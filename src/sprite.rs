use std::{marker::PhantomData, sync::Arc};
use image::GenericImageView;
use wgpu::util::{DeviceExt, TextureFormatExt};

pub struct Sprite {
    texture: UniformTexture,
    mesh: GpuMesh<Vertex>
}

pub struct GpuMesh<T> {
    vertex_buffer: wgpu::Buffer,
    _phantom_vertex: PhantomData<T>,
    index_buffer: wgpu::Buffer,
    len_indices: u32,
    len_vertices: u32,
} impl<T> GpuMesh<T> where T: bytemuck::Pod {
    pub fn from_slices(gpu: &Arc<crate::Gpu>, vertices: &[T], indices: &[u32]) -> Self {
        let vertex_buffer = gpu.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = gpu.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        Self {
            vertex_buffer,
            index_buffer,
            len_indices: indices.len() as u32,
            len_vertices: vertices.len() as u32,
            _phantom_vertex: PhantomData,
        }
    }

    pub fn vertices(&self) -> &wgpu::Buffer { &self.vertex_buffer }
    pub fn indices(&self) -> &wgpu::Buffer { &self.index_buffer }

    pub fn len_indices(&self) -> u32 {
        self.len_indices
    }

    pub fn len_vertices(&self) -> u32 {
        self.len_vertices
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
} impl Vertex {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        }
    }
}


pub struct UniformTexture {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    size: (u32, u32),
} impl UniformTexture {
    pub fn from_image(
        gpu: &Arc<crate::Gpu>,
        img: &image::DynamicImage,
        label: &str,
    ) -> Self {
        let label = (if label.is_empty() { "Unnamed" } else { label }).to_string() ;

        let rgba = img.to_rgba8();

        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = gpu.device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some(label.as_str()),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }
        );

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = gpu.device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            }
        );

        let texture_bind_group_layout = Self::bind_group_layout(gpu);

        let bind_group = gpu.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    }
                ],
                label: Some((label + " Bind Group").as_str()),
            }
        );

        Self { 
            _texture: texture, 
            _view: view, 
            _sampler: sampler, 
            bind_group, 
            size: img.dimensions(),
        }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(gpu: &crate::Gpu) -> wgpu::BindGroupLayout {
        gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("Texture Bind Group Layout"),
        })
    }
    
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn size_float(&self) -> (f32, f32) {
        let (w, h) = self.size;

        (w as f32, h as f32)
    }
}

pub struct UniformMatrix {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
} impl UniformMatrix {
    pub fn new(gpu: &Arc<crate::Gpu>) -> Self {
        let zero = [[0f32; 4]; 4];
        let buffer = gpu.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(format!("Mat4x4 Uniform Buffer").as_str()),
                contents: bytemuck::cast_slice(&[zero]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::bind_group_layout(gpu),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }
            ],
            label: Some(format!("UniformMatrix Bind Group").as_str()),
        });

        Self {
            buffer,
            bind_group,
        }
    }

    pub fn bind_group_layout(gpu: &crate::Gpu) -> wgpu::BindGroupLayout {
        gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: 
                        wgpu::ShaderStages::FRAGMENT 
                        | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some(format!("Matrix Bind Group Layout").as_str()),
        })
    }
}

pub fn create_pipeline(gpu: &Arc<crate::Gpu>, surface_format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = gpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("sprite_shader.wgsl").into(),
            ),
        });
    let sprite_pipeline_layout =
        gpu.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&UniformTexture::bind_group_layout(gpu)),
                    Some(&UniformMatrix::bind_group_layout(gpu)),
                ],
                immediate_size: 0,
            });

    gpu.device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&sprite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::layout())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: 
                wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
            depth_stencil: None,
            multisample: 
                wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
            cache: None,
            multiview_mask: None,
        })
}
