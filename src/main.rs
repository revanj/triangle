mod sprite;
mod camera;

use std::{f32::consts::PI, marker::PhantomData, sync::Arc};

use hecs::Entity;
use nalgebra::{Matrix4, vector};
use rand::RngExt;
use wgpu::wgc::MAX_COLOR_ATTACHMENTS;
use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, EventLoop}, window::Window};

use crate::sprite::Sprite;

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
    _instance: wgpu::Instance,
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
            _instance: instance,
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

    pub fn request_redraw(&mut self) {
        self.window.request_redraw();
    }
}

fn render(world: &mut hecs::World, global: hecs::Entity) {
    let rdr: &Renderer = &world.get::<&Renderer>(global).unwrap();
    let output = match rdr.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
        wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
            surface_texture
        }
        wgpu::CurrentSurfaceTexture::Timeout
        | wgpu::CurrentSurfaceTexture::Occluded
        | wgpu::CurrentSurfaceTexture::Validation => {
            return;
        }
        wgpu::CurrentSurfaceTexture::Outdated => {
            rdr.surface.configure(&rdr.gpu.device, &rdr.surface_config);
            return;
        }
        wgpu::CurrentSurfaceTexture::Lost => {
            panic!("Lost device");
        }
    };
    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = rdr.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    {
        let mut render_pass: wgpu::RenderPass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        render_pass.set_pipeline(&rdr.sprite_pipeline);

        let mut query = world.query::<(&mut Sprite,)>();
        for (sprite,) in query.iter() {
            sprite.draw(&mut render_pass)
        }
    }

    rdr.gpu.queue.submit(std::iter::once(encoder.finish()));
    rdr.gpu.queue.present(output);
}

struct MousePosition(nalgebra::Vector2<f32>);

struct Engine<G: Game> {
    finished_init: bool,
    global: hecs::Entity,
    world: hecs::World,
    _phantom_game: PhantomData<G>,
} impl<G: Game> Engine<G> {
    pub fn new() -> Self {
        let mut world = hecs::World::new();
        let global = world.spawn(());
        world.insert_one(global, MousePosition(nalgebra::Vector2::zeros())).unwrap();

        Self {
            finished_init: false,
            global,
            world,
            _phantom_game: PhantomData,
        }
    }
}

impl<G: Game> ApplicationHandler for Engine<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        let renderer = pollster::block_on(Renderer::new(window));
        let default_camera = camera::Camera::new(vector![0f32, 0f32], renderer.size_vec());
        self.world.insert(self.global, (default_camera,)).unwrap();
        self.world.insert(self.global, (renderer,)).unwrap();
        G::init(&mut self.world, self.global);
        self.finished_init = true;
        self.world.get::<&mut Renderer>(self.global).unwrap().request_redraw();
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
                    G::update(&mut self.world, self.global);
                    {
                        let camera: &camera::Camera = &self.world.get::<&camera::Camera>(self.global).unwrap();
                        let pv_matrix = camera.pv_matrix();
                        let rdr: &Renderer = &self.world.get::<&Renderer>(self.global).unwrap();
                        let mut query = self.world.query::<(&mut Sprite, &nalgebra::Matrix4<f32>)>();
                        for (sprite, transform) in query.iter() {
                            sprite.set_mvp(&rdr.gpu, pv_matrix * transform);
                        }
                    }

                    render(&mut self.world, self.global);

                    let mut renderer = self.world.get::<&mut Renderer>(self.global).unwrap();
                    renderer.request_redraw();
                }
            },
            WindowEvent::Resized(size) 
            if self.finished_init && size.width != 0 && size.height != 0 => {
                let mut renderer = self.world.get::<&mut Renderer>(self.global).unwrap();
                renderer.resize(size.width, size.height);
                renderer.request_redraw();
                let mut camera = self.world.get::<&mut camera::Camera>(self.global).unwrap();
                camera.resize(vector![size.width as f32, size.height as f32]);
            }
            WindowEvent::CursorMoved { device_id: _, position } => {
                let mut mouse_position = self.world.get::<&mut MousePosition>(self.global).unwrap();
                let screen_size = self.world.get::<&Renderer>(self.global).unwrap().size_vec();
                *mouse_position = MousePosition(nalgebra::vector![position.x as f32, screen_size.y - position.y as f32]);
            },
            _ => ()
        }
    }
}

trait Game {
    fn init(world: &mut hecs::World, global: hecs::Entity);
    fn update(world: &mut hecs::World, global: hecs::Entity);
}

struct Position(nalgebra::Vector2<f32>);
struct Velocity(nalgebra::Vector2<f32>);

struct FlockOfBirdsGame {}
impl Game for FlockOfBirdsGame {
    fn init(world: &mut hecs::World, global: hecs::Entity) {
        let renderer = world.entity(global).unwrap().get::<&Renderer>().unwrap();
        let screen_size = renderer.size_vec();
        let mut rng = rand::rng(); 
        drop(renderer);
        
        for _ in 0..100 {
            let renderer = world.entity(global).unwrap().get::<&Renderer>().unwrap();
            let sprite = Sprite::new_arrow(&renderer.gpu);
            drop(renderer);
            let x: f32 = rng.random();
            let y: f32 = rng.random();
            let pos = (nalgebra::vector![x, y] - vector![0.5, 0.5]).component_mul(&screen_size);
            let mut transform = nalgebra::Matrix4::<f32>::new_scaling(30f32);
            let position = Position(pos);
            let rand_angle = rng.random::<f32>() * 2f32 * PI;
            let velocity = Velocity(vector![f32::cos(rand_angle), f32::sin(rand_angle)]);
            world.spawn((sprite, position, velocity, transform));
        }

    }

    fn update(world: &mut hecs::World, global: hecs::Entity) {
        let camera = world.get::<&camera::Camera>(global).unwrap();
        let mouse_pos = world.get::<&MousePosition>(global).unwrap().0;
        let left_margin = -350f32;
        let right_margin = 350f32;
        let bottom_margin = -350f32;
        let top_margin = 350f32;
        let avoid_factor = 0.01f32;
        let matching_factor = 0.05f32;
        let centering_factor = 0.0005f32;
        let turn_factor = 0.2f32;
        let max_speed = 1.0f32;
        let min_speed = 0.5f32;
        let visual_range = 50.0f32;
        let protected_range = 10.0f32;
        let mouse_attraction = 0.001f32;
        let mouse_rotation = 0.08f32;
        let min_radius = 10f32;
        let mouse_follow = false;

        let mouse_pos_world = camera.screen_pt_to_world(mouse_pos);
        {
            let mut query = world.query::<(Entity, &mut Velocity, &mut Position)>();
            let arrow_etts: Vec<hecs::Entity> = query.iter().map(|x| x.0).collect();
            let mut view = query.view();

            for i in 0..arrow_etts.len() {
                let mut vel_avoid = vector![0f32, 0f32];
                let mut vel_avg = vector![0f32, 0f32];
                let mut pos_avg = vector![0f32, 0f32];

                let mut count = 0;
                for j in i..arrow_etts.len() {
                    if i == j {continue;}
                    let [this, other] = view.get_disjoint_mut([arrow_etts[i], arrow_etts[j]]);
                    let [(_, _, this_pos), (_, other_vel, other_pos)] = [this.unwrap(), other.unwrap()];
                    if (this_pos.0 - other_pos.0).norm() > visual_range { continue; }
                    vel_avg += other_vel.0;
                    pos_avg += other_pos.0;
                    count += 1;
                    if (this_pos.0 - other_pos.0).norm() > protected_range { continue; }
                    vel_avoid += this_pos.0 - other_pos.0;
                }

                vel_avg /= count as f32;
                pos_avg /= count as f32;

                let (_, this_vel, this_pos) = view.get_mut(arrow_etts[i]).unwrap();
                let vel_delta = vel_avg - this_vel.0;
                let pos_delta = pos_avg - this_pos.0;
                let mouse_delta = mouse_pos_world - this_pos.0;

                let mouse_tangent = vector![mouse_delta.y,  -mouse_delta.x];
                let radius = mouse_tangent.norm();
                let mouse_tangent = mouse_tangent.normalize();

                if !mouse_tangent.x.is_nan() && !mouse_tangent.y.is_nan() && mouse_follow {
                    this_vel.0 += mouse_tangent * mouse_rotation * ((min_radius-radius).max(-0f32)).exp();
                }
                
                this_vel.0 += vel_avoid * avoid_factor;

                if radius > min_radius  && mouse_follow {
                    this_vel.0 += mouse_delta * mouse_attraction;
                }

                if count != 0 {
                    this_vel.0 += vel_delta * matching_factor;
                    this_vel.0 += pos_delta * centering_factor;
                }

                if this_pos.0.x < left_margin {
                    this_vel.0.x += turn_factor;
                }        
                if this_pos.0.x > right_margin {
                    this_vel.0.x -= turn_factor;
                }
                if this_pos.0.y < bottom_margin {
                    this_vel.0.y += turn_factor;
                }
                if this_pos.0.y > top_margin {
                    this_vel.0.y -= turn_factor;
                }
                let speed = this_vel.0.norm();
                if speed > max_speed {
                    this_vel.0 = (this_vel.0 / speed) * max_speed;
                }
                if speed < min_speed {
                    this_vel.0 = (this_vel.0 / speed) * min_speed;
                }

                this_pos.0 += this_vel.0;
            }
        }

        for (pos, heading, tf) in world.query::<(&Position, &Velocity, &mut Matrix4<f32>,)>().iter() {
            tf[(0, 3)] = pos.0.x;
            tf[(1, 3)] = pos.0.y;
            let scaled_heading = heading.0.normalize() * 15f32;
            // basis x is (heading.y -heading.x)
            // basis y is (heading.x heading.y)
            tf.set_column(0, &vector![scaled_heading.y, -scaled_heading.x, 0f32, 0f32]);
            tf.set_column(1, &vector![scaled_heading.x, scaled_heading.y, 0f32, 0f32]); 
        }

    }
}


