//! Renderer wgpu minimal (ADR §2.6) — módulo de la app, no un motor.
//!
//! Render a textura offscreen con depth buffer propio (patrón rerun.io),
//! registrada en egui como textura nativa: el panel 3D la muestra con
//! `ui.image`. Un pipeline, dos mallas en un buffer (tela dinámica +
//! avatar estático), iluminación lambertiana a dos caras.

use eframe::egui_wgpu::RenderState;
use eframe::wgpu;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const SHADER: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
    light: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
) -> VsOut {
    var out: VsOut;
    out.clip = u.mvp * vec4<f32>(pos, 1.0);
    out.normal = normal;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let shade = 0.22 + 0.78 * abs(dot(n, u.light.xyz));
    return vec4<f32>(in.color * shade, 1.0);
}
"#;

/// Vértice: posición + normal + color (interleaved, 9 f32).
const VERTEX_STRIDE: u64 = 9 * 4;

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    ubuf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    color: Option<(wgpu::Texture, wgpu::TextureView)>,
    depth: Option<wgpu::TextureView>,
    pub texture_id: Option<eframe::egui::TextureId>,
    size: (u32, u32),
    n_cloth_verts: usize,
    n_indices: u32,
}

impl Renderer {
    /// `cloth_tris` indexa los vértices de la tela; el avatar (esfera) se
    /// genera aquí y vive tras la tela en el mismo par de buffers.
    pub fn new(
        rs: &RenderState,
        n_cloth_verts: usize,
        cloth_tris: &[u32],
        avatar_radius: f32,
    ) -> Self {
        let device = &rs.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("toile"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("toile-mesh"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: VERTEX_STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None, // tela a dos caras
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        // Avatar: esfera UV estática, gris, apenas bajo el radio real para
        // no pelear el z-buffer con la tela apoyada.
        let (sphere_verts, sphere_idx) = uv_sphere(avatar_radius * 0.995, 40, 20);
        let n_sphere_verts = sphere_verts.len() / 9;

        let total_verts = n_cloth_verts + n_sphere_verts;
        let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("toile-vbuf"),
            size: total_verts as u64 * VERTEX_STRIDE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // La sección del avatar se escribe una sola vez.
        rs.queue.write_buffer(
            &vbuf,
            n_cloth_verts as u64 * VERTEX_STRIDE,
            bytemuck::cast_slice(&sphere_verts),
        );

        let mut indices: Vec<u32> = cloth_tris.to_vec();
        indices.extend(sphere_idx.iter().map(|&i| i + n_cloth_verts as u32));
        let ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("toile-ibuf"),
            size: (indices.len() * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        rs.queue
            .write_buffer(&ibuf, 0, bytemuck::cast_slice(&indices));

        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("toile-ubuf"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            vbuf,
            ibuf,
            ubuf,
            bind_group,
            color: None,
            depth: None,
            texture_id: None,
            size: (0, 0),
            n_cloth_verts,
            n_indices: indices.len() as u32,
        }
    }

    fn ensure_targets(&mut self, rs: &RenderState, w: u32, h: u32) {
        if self.size == (w, h) && self.color.is_some() {
            return;
        }
        let device = &rs.device;
        let make = |format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let color = make(
            COLOR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let color_view = color.create_view(&Default::default());
        let depth = make(DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT);
        self.depth = Some(depth.create_view(&Default::default()));

        let mut renderer = rs.renderer.write();
        if let Some(old) = self.texture_id.take() {
            renderer.free_texture(&old);
        }
        self.texture_id =
            Some(renderer.register_native_texture(device, &color_view, wgpu::FilterMode::Linear));
        self.color = Some((color, color_view));
        self.size = (w, h);
    }

    /// Sube la tela del frame (pos+normal+color intercalados) y dibuja la
    /// escena a la textura offscreen.
    pub fn paint(
        &mut self,
        rs: &RenderState,
        w: u32,
        h: u32,
        cloth_vertices: &[f32],
        uniforms: &[f32; 20],
    ) {
        self.ensure_targets(rs, w.max(8), h.max(8));
        debug_assert_eq!(cloth_vertices.len(), self.n_cloth_verts * 9);
        rs.queue
            .write_buffer(&self.vbuf, 0, bytemuck::cast_slice(cloth_vertices));
        rs.queue
            .write_buffer(&self.ubuf, 0, bytemuck::cast_slice(uniforms));

        let (_, color_view) = self.color.as_ref().unwrap();
        let depth_view = self.depth.as_ref().unwrap();
        let mut encoder = rs
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("toile"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("toile-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.055,
                            g: 0.07,
                            b: 0.065,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vbuf.slice(..));
            pass.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.n_indices, 0, 0..1);
        }
        rs.queue.submit([encoder.finish()]);
    }
}

/// Esfera UV con normales, como vértices intercalados pos+normal+color.
fn uv_sphere(r: f32, seg: u32, rings: u32) -> (Vec<f32>, Vec<u32>) {
    let color = [0.30f32, 0.33, 0.32];
    let mut v = Vec::new();
    for j in 0..=rings {
        let phi = std::f32::consts::PI * j as f32 / rings as f32;
        let (sp, cp) = phi.sin_cos();
        for i in 0..=seg {
            let theta = std::f32::consts::TAU * i as f32 / seg as f32;
            let (st, ct) = theta.sin_cos();
            let n = [sp * ct, cp, sp * st];
            v.extend_from_slice(&[r * n[0], r * n[1], r * n[2], n[0], n[1], n[2]]);
            v.extend_from_slice(&color);
        }
    }
    let mut idx = Vec::new();
    let stride = seg + 1;
    for j in 0..rings {
        for i in 0..seg {
            let a = j * stride + i;
            let b = a + stride;
            idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    (v, idx)
}
