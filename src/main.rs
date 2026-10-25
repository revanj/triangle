mod sprite;
mod camera;

use std::{marker::PhantomData, sync::{Arc, mpsc::{Receiver, Sender, TryRecvError}}};

use nalgebra::vector;
use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, EventLoop}, window::Window};

fn main() {
    run_game::<FlockOfBirdsGame>();
}

fn run_game<G: Game + 'static>() {
    let event_loop = EventLoop::new().unwrap();
    let engine = Engine::<G>::new();

        let mut engine = engine;
        event_loop.run_app(&mut engine).unwrap();
}

struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

struct Renderer {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    gpu: Arc<Gpu>,
    _adapter: wgpu::Adapter,
    surface_config: wgpu::SurfaceConfiguration,
    sprite_pipeline: wgpu::RenderPipeline,

    window: Arc<winit::window::Window>,
} impl Renderer {
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Renderer failed to create surface from window");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .expect("Renderer failed o request adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .expect("Renderer failed to find device and queue");

        let gpu = Arc::new(Gpu { device, queue });

        let size = window.inner_size();
        let (min_1_width, min_1_height) = (size.width.max(1), size.height.max(1));

        let mut surface_config = surface
            .get_default_config(&adapter, min_1_width, min_1_height)
            .expect("Renderer failed to get default config from surface");

        surface_config.view_formats = vec![surface_config.format.add_srgb_suffix()];
        surface_config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;

        surface.configure(&gpu.device, &surface_config);

        let sprite_pipeline = sprite::create_pipeline(&gpu, surface_config.format);

        Self {
            instance,
            surface,
            _adapter: adapter,
            gpu,
            surface_config,
            sprite_pipeline,
            window,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.gpu.device, &self.surface_config);
        }
    }

    pub fn size_vec(&self) -> nalgebra::Vector2<f32> {
        let extent = self.window.inner_size();
        nalgebra::vector![extent.width as f32, extent.height as f32]
    }

    pub fn render(&mut self, world: &hecs::World) {

    }

    pub fn request_redraw(&mut self) {
        self.window.request_redraw();
    }
}

struct Engine<G: Game> {
    finished_init: bool,
    global: hecs::Entity,
    world: hecs::World,
    _phantom_game: PhantomData<G>,
} impl<G: Game> Engine<G> {
    pub fn new() -> Self {
        let mut world = hecs::World::new();
        let global = world.spawn(());

        Self {
            finished_init: false,
            global,
            world,
            _phantom_game: PhantomData,
        }
    }

    fn update(&mut self) {}
}

impl<G: Game> ApplicationHandler for Engine<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        let renderer = pollster::block_on(Renderer::new(window));
        self.world.insert(self.global, (renderer,)).unwrap();
        G::init(&mut self.world, self.global);
        self.finished_init = true;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::RedrawRequested => {
                if self.finished_init {
                    self.update();
                }
            },
            WindowEvent::Resized(size) 
            if self.finished_init && size.width != 0 && size.height != 0 =>
            {
                let mut renderer = self.world.entity(self.global).unwrap().get::<&mut Renderer>().unwrap();
                renderer.resize(size.width, size.height);
                renderer.request_redraw();
            }
            _ => (),
        }
    }
}

trait Game {
    fn init(world: &mut hecs::World, global: hecs::Entity);
}


struct FlockOfBirdsGame {}
impl Game for FlockOfBirdsGame {
    fn init(world: &mut hecs::World, global: hecs::Entity) {
        let renderer = world.entity(global).unwrap().get::<&Renderer>().unwrap();
        let default_camera = camera::Camera::new(vector![0.0, 0.0], renderer.size_vec());
        drop(renderer);
        world.insert(global, (default_camera,)).unwrap();
    }
}


