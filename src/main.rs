use std::{fs,
    time::Instant,
    path::{PathBuf,Path},
    sync::mpsc::channel};
use winit::{
    event::{Event,Event::{RedrawRequested,MainEventsCleared,UserEvent},WindowEvent,KeyboardInput,VirtualKeyCode,ElementState},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::{WindowBuilder, Window}};
use pollster::block_on;
use notify::{RawEvent, RecommendedWatcher, Watcher};
use wgpu::{Color, Operations, LoadOp, Surface, Features, Limits, Device, Queue, Instance, DeviceDescriptor, PipelineLayout, RenderPipeline, Buffer, BindGroup, ShaderModule, ShaderSource, ShaderModuleDescriptor, BindGroupEntry, RequestAdapterOptions, ShaderStages, BindingType, BufferBindingType, BufferUsages, RenderPipelineDescriptor, VertexState, FragmentState, PowerPreference, BindGroupDescriptor, BindGroupLayoutDescriptor, RenderPassDescriptor, PipelineLayoutDescriptor, BindGroupLayoutEntry, RenderPassColorAttachment, CommandEncoderDescriptor, PrimitiveState, SurfaceError, MultisampleState, SurfaceConfiguration};
use wgpu::util::{DeviceExt, BufferInitDescriptor};
use naga::{valid::{ValidationFlags, Validator, Capabilities}};
use bytemuck;

#[derive(Debug)]
struct ReloadEvent;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
  pub resolution: [f32; 2],
  pub playime: f32,
  pub mouse: [f32; 3],
}

impl Uniforms {
  fn as_bytes(&self)-> &[u8] { bytemuck::bytes_of(self) }
}

struct State {
  window: Window,
  surface: Surface,
  config: SurfaceConfiguration,
  device: Device,
  queue: Queue,

  size: winit::dpi::PhysicalSize<u32>,
  pipeline_layout: PipelineLayout,
  render_pipeline: RenderPipeline,
  uniforms: Uniforms,
  uniforms_buffer: Buffer,
  uniforms_bind_group: BindGroup,

  fragment_path: PathBuf,
  vertex_shader: ShaderModule,
}

impl State {
  // Creating some of the wgpu types requires async code
  async fn new(window: Window, fragment_path: &Path) -> Self {
    let size = window.inner_size();
    let instance = Instance::new(wgpu::Backends::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance.request_adapter(&RequestAdapterOptions {
      power_preference: PowerPreference::default(),
      compatible_surface: Some(&surface),
      force_fallback_adapter: false},
    ).await.unwrap();
    let config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: surface.get_preferred_format(&adapter).unwrap(),
      width: size.width, height: size.height,
      present_mode: wgpu::PresentMode::Mailbox};
    let (device, queue) = adapter.request_device(
      &DeviceDescriptor {features: Features::default(), limits: Limits::default(), label: None}, None,
    ).await.unwrap();
    surface.configure(&device, &config);
    let vertex_shader = device.create_shader_module(&ShaderModuleDescriptor {
        label: Some("Vertex Shader"),
        source: ShaderSource::Wgsl(include_str!("./vertex.wgsl").into())});
    let uniforms = Uniforms{resolution: [size.width as f32, size.height as f32], playime: 0.0, mouse: [0.0, 0.0, 0.0]};
    let uniforms_buffer = device.create_buffer_init(
      &BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::bytes_of(&uniforms),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST});
    let uniforms_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("uniform_bind_group_layout"),
        entries: &[
          BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
              ty: BufferBindingType::Uniform,
              has_dynamic_offset: false,
              min_binding_size: None},
            count: None}]});
    let uniforms_bind_group = device.create_bind_group(&BindGroupDescriptor {
        layout: &uniforms_bind_group_layout,
        entries: &[ BindGroupEntry {
          binding: 0,
          resource: uniforms_buffer.as_entire_binding()}],
        label: Some("uniform_bind_group")});
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&uniforms_bind_group_layout],
        push_constant_ranges: &[]});
    let pathbuf = fragment_path.to_path_buf();
    let render_pipeline = create_pipeline(&device, config.format, &vertex_shader, &pipeline_layout, &pathbuf).unwrap();
    Self{window, surface, config, device, queue, size, pipeline_layout,
        uniforms, uniforms_buffer, uniforms_bind_group, render_pipeline, fragment_path: pathbuf, vertex_shader}
  }

  fn render(&mut self) -> Result<(), SurfaceError> {
    let frame = self.surface.get_current_texture().expect("Surface error");
    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor{label: Some("Render Encoder")});
    {
      let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[
          RenderPassColorAttachment {
            view: &view, resolve_target: None,
            ops: Operations {load: LoadOp::Clear(Color{r:0.1,g:0.2,b:0.3,a:1.0}),store: true}}],
        depth_stencil_attachment: None});
      render_pass.set_pipeline(&self.render_pipeline);
      render_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
      render_pass.draw(0..3, 0..1);
    }
    self.queue.submit(std::iter::once(encoder.finish()));
    frame.present();
    Ok(())
  }

  fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
    if new_size.width > 0 && new_size.height > 0 {
      self.size = new_size;
      self.config.width = new_size.width;
      self.config.height = new_size.height;
      self.surface.configure(&self.device, &self.config);
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
    match create_pipeline(&self.device, self.config.format, &self.vertex_shader, &self.pipeline_layout, &self.fragment_path) {
        Ok(render_pipeline) => self.render_pipeline = render_pipeline,
        Err(e) => println!("{}", e),
      }
    self.window.request_redraw();
  }
}

fn create_pipeline(
    device: &Device,
    format: wgpu::TextureFormat,
    vertex_shader: &ShaderModule,
    pipeline_layout: &PipelineLayout,
    fragment_path: &PathBuf,
) -> Result<RenderPipeline, String> {
    let fragment_content = fs::read_to_string(fragment_path).unwrap();
    let module = naga::front::wgsl::parse_str(&fragment_content).map_err(|e| format!("Parse Error: {}", &e))?;
    Validator::new(ValidationFlags::all(), Capabilities::all()).validate(&module).map_err(|e| format!("Validation error: {}", &e))?;
    let fragment_shader = device.create_shader_module(&ShaderModuleDescriptor {
        label: Some("Fragment shader"),
        source: ShaderSource::Wgsl(fragment_content.into())});
    Ok(device.create_render_pipeline(&RenderPipelineDescriptor {
      label: Some("Render Pipeline"),
      layout: Some(&pipeline_layout),
      vertex: VertexState {
        module: &vertex_shader,
        entry_point: "main",
        buffers: &[]},
      fragment: Some(FragmentState {
        module: &fragment_shader,
        entry_point: "main",
        targets: &[format.into()]}),
      primitive: PrimitiveState::default(),
      depth_stencil: None,
      multisample: MultisampleState{count: 1, mask: !0, alpha_to_coverage_enabled: false}})
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

  let window = WindowBuilder::new().with_title("Slim Shader").build(&event_loop).unwrap();
  let mut state = block_on(State::new(window, path.as_path()));
  let instant = Instant::now();

  event_loop.run(move |event, _, control_flow| {
    match event {
      RedrawRequested(_) => {
        state.uniforms.playime = instant.elapsed().as_secs_f32();
        state.update();
        match state.render() {
          Ok(_) => {}
          // Err(SwapChainError::Lost) => state.resize(state.size),
          // Err(SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
          Err(e) => eprintln!("{:?}", e),
        }
      }
      MainEventsCleared => { state.window.request_redraw(); }
      UserEvent(ReloadEvent) => { state.reload(); }
      Event::WindowEvent {ref event, window_id} if window_id == state.window.id() => if !state.input(event) {
        match event {
          WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
            input: KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Escape), .. }, ..
          } => *control_flow = ControlFlow::Exit,
          WindowEvent::Resized(physical_size) => {
            state.resize(*physical_size);
          }
          WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
            state.resize(**new_inner_size);
          }
          WindowEvent::CursorMoved {position, ..} => {
            let size = state.window.inner_size();
            let normalized_x = position.x as f32 / size.width as f32;
            let normalized_y = position.y as f32 / size.height as f32;
            state.uniforms.mouse = [normalized_x * 2.0 - 1.0, -normalized_y * 2.0 + 1.0, 0.0];
          }
          _ => {}
      }
    }
    _ => {}
    }
  });
}
