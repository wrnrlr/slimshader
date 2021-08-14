use std::{fs};
use std::path::{PathBuf,Path};
use std::sync::mpsc::channel;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::{WindowBuilder, Window}};
use pollster;
use notify::{RawEvent, RecommendedWatcher, Watcher};
use naga::{front::wgsl, valid::{ValidationFlags, Validator, Capabilities}};

struct ReloadEvent;

struct State {
    // window: Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,

    fragment_path: PathBuf,
    validator: Validator,
    vertex_shader: wgpu::ShaderModule,
}

impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: &Window, fragment_path: &Path) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
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
        let swapchain_format = adapter.get_swap_chain_preferred_format(&surface).unwrap();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo}; // Faster framerate with Immediate or Mailbox but this is not optimal for mobile... 
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        // let shader_str = include_str!("shader.wgsl");
        let vertex_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Vertex Shader"),
            flags: wgpu::ShaderFlags::all(),
            source: wgpu::ShaderSource::Wgsl(include_str!("./vertex.wgsl").into())});
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[], // TODO add bind group for uniform buffer with screen resolution
            push_constant_ranges: &[]});
        let pathbuf = fragment_path.to_path_buf();
        let render_pipeline = create_pipeline(&device, &vertex_shader, &render_pipeline_layout, swapchain_format, &pathbuf).unwrap();
        let validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        Self{surface, device, queue, sc_desc, swap_chain, size, render_pipeline, validator, fragment_path: pathbuf, vertex_shader}
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
            render_pass.draw(0..3, 0..1);
        }
        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.sc_desc.width = new_size.width;
            self.sc_desc.height = new_size.height;
            self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        }
    }

    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
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

    let content = fs::read_to_string(path.to_str().unwrap()).expect("could not read file");
    println!("{}", content);

    env_logger::init();

    let event_loop:EventLoop<ReloadEvent> = EventLoop::with_user_event();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = pollster::block_on(State::new(&window, path.as_path()));

    {
        let fragment_path = path.clone();
        std::thread::spawn(move || {
            let (tx, rx) = channel();
            let mut watcher: RecommendedWatcher = Watcher::new_raw(tx).unwrap();
            watcher.watch(&fragment_path, notify::RecursiveMode::NonRecursive).unwrap();
            loop {
                match rx.recv() {
                    Ok(RawEvent {path: Some(_), op: Ok(_), ..}) => println!("reload fragment shader"),
                    Ok(event) => println!("broken event: {:?}", event),
                    Err(e) => println!("watch error: {:?}", e),
                }
            }
        });
    }

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(_) => {
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
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            Event::WindowEvent {ref event, window_id} if window_id == window.id() => if !state.input(event) {
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
