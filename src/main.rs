use std::{fs,
    time::Instant,
    path::{PathBuf,Path},
    sync::mpsc::channel};
use winit::{
    event::{Event,WindowEvent,KeyboardInput,VirtualKeyCode,ElementState},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::{WindowBuilder, Window}};
use pollster;
use notify::{RawEvent, RecommendedWatcher, Watcher};
use wgpu::util::DeviceExt;
use naga::{valid::{ValidationFlags, Validator, Capabilities}};
use bytemuck;

#[derive(Debug)]
struct ReloadEvent;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    pub resolution: [f32; 2],
    pub playime: f32,
}

impl Uniforms {
    fn as_bytes(&self)-> &[u8] { bytemuck::bytes_of(self) }
}

struct State {
    window: Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swapchain_format: wgpu::TextureFormat,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
    uniforms: Uniforms,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,

    fragment_path: PathBuf,
    vertex_shader: wgpu::ShaderModule,
}

impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: Window, fragment_path: &Path) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface)},
        ).await.unwrap();
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::default(),
                limits: wgpu::Limits::default(),
                label: None},
            None,
        ).await.unwrap();
        let vertex_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Vertex Shader"),
            flags: wgpu::ShaderFlags::all(),
            source: wgpu::ShaderSource::Wgsl(include_str!("./vertex.wgsl").into())});
        let uniforms = Uniforms{resolution: [size.width as f32, size.height as f32], playime: 0.0};
        let uniforms_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST});
        let uniforms_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None},
                    count: None}]});
        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniforms_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms_buffer.as_entire_binding()}],
            label: Some("uniform_bind_group")});
        let swapchain_format = adapter.get_swap_chain_preferred_format(&surface).unwrap();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo}; // Faster framerate with Immediate or Mailbox but this is not optimal for mobile... 
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&uniforms_bind_group_layout],
            push_constant_ranges: &[]});
        let pathbuf = fragment_path.to_path_buf();
        let render_pipeline = create_pipeline(&device, &vertex_shader, &pipeline_layout, swapchain_format, &pathbuf).unwrap();
        Self{window, surface, device, queue, sc_desc, swapchain_format, swap_chain, size, pipeline_layout,
            uniforms, uniforms_buffer, uniforms_bind_group, render_pipeline, fragment_path: pathbuf, vertex_shader}
    }

    fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{label: Some("Render Encoder")});
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    wgpu::RenderPassColorAttachment {
                        view: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color{r: 0.1, g: 0.2, b: 0.3, a: 1.0}),
                            store: true}}],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.sc_desc.width = new_size.width;
            self.sc_desc.height = new_size.height;
            self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
            self.uniforms.resolution = [new_size.width as f32, new_size.height as f32];
        }
    }

    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        self.queue.write_buffer(&self.uniforms_buffer, 0, self.uniforms.as_bytes());
    }

    fn reload(&mut self) {
        println!("reload fragment shader");
        match create_pipeline(&self.device, &self.vertex_shader, &self.pipeline_layout, self.swapchain_format, &self.fragment_path) {
            Ok(render_pipeline) => self.render_pipeline = render_pipeline,
            Err(e) => println!("{}", e),
        }
        self.window.request_redraw();
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    vertex_shader: &wgpu::ShaderModule,
    pipeline_layout: &wgpu::PipelineLayout,
    swapchain_format: wgpu::TextureFormat,
    fragment_path: &PathBuf,
) -> Result<wgpu::RenderPipeline, String> {
    let fragment_content = fs::read_to_string(fragment_path).unwrap();
    let module = naga::front::wgsl::parse_str(&fragment_content).map_err(|e| format!("Parse Error: {}", &e))?;
    Validator::new(ValidationFlags::all(), Capabilities::all()).validate(&module).map_err(|e| format!("Validation error: {}", &e))?;
    let fragment_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some("Fragment shader"),
        source: wgpu::ShaderSource::Wgsl(fragment_content.into()),
        flags: wgpu::ShaderFlags::all()});
    Ok(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vertex_shader,
            entry_point: "main",
            buffers: &[]},
        fragment: Some(wgpu::FragmentState {
            module: &fragment_shader,
            entry_point: "main",
            targets: &[wgpu::ColorTargetState {
                format: swapchain_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrite::ALL}]}),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState{count: 1, mask: !0, alpha_to_coverage_enabled: false}})
    )
}

fn main() {
    println!("Slim Shader!");

    let filename = std::env::args().nth(1).expect("no path to file fiven");
    let path = PathBuf::from(filename.as_str());
    println!("{:?}", path);

    fs::read_to_string(path.to_str().unwrap()).expect("could not read file");

    env_logger::init();

    let event_loop:EventLoop<ReloadEvent> = EventLoop::with_user_event();
    let proxy:EventLoopProxy<ReloadEvent> = event_loop.create_proxy();

    {
        let fragment_path = path.clone();
        std::thread::spawn(move || {
            let (tx, rx) = channel();
            let mut watcher: RecommendedWatcher = Watcher::new_raw(tx).unwrap();
            watcher.watch(&fragment_path, notify::RecursiveMode::NonRecursive).unwrap();
            loop {
                match rx.recv() {
                    Ok(RawEvent {path: Some(_), op: Ok(_), ..}) => proxy.send_event(ReloadEvent).unwrap(),
                    Ok(event) => println!("broken event: {:?}", event),
                    Err(e) => println!("watch error: {:?}", e),
                }
            }
        });
    }

    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = pollster::block_on(State::new(window, path.as_path()));
    let instant = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(_) => {
                state.uniforms.playime = instant.elapsed().as_secs_f32();
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Recreate the swap_chain if lost
                    Err(wgpu::SwapChainError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => { state.window.request_redraw(); }
            Event::UserEvent(ReloadEvent) => { state.reload(); }
            Event::WindowEvent {ref event, window_id} if window_id == state.window.id() => if !state.input(event) {
                match event {
                    WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                        input: KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
}
