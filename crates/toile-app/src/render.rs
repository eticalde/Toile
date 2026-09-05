mod layout;
mod pipeline;
mod sphere;

use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
use layout::BufferPlan;
use pipeline::build_pipeline;

use crate::theme::Theme;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SHADER: &str = include_str!("render/shader.wgsl");

/// Sixteen matrix floats plus a padded light vector.
const UNIFORM_BYTES: u64 = 80;

/// Renders the drape to an offscreen texture that egui shows as an image.
///
/// One pipeline and two meshes in one pair of buffers: the cloth, re-uploaded
/// each frame, and the avatar, written on every (re)allocation.
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
    clear: wgpu::Color,
    n_cloth_verts: usize,
    /// The index list as uploaded, kept so [`Renderer::fits`] can answer by
    /// content: after a swap that happens to keep the counts, only the
    /// triangles themselves say the buffers are stale.
    indices: Vec<u32>,
    /// The avatar's mesh, kept so a resize can re-upload it at its new offset.
    sphere_verts: Vec<f32>,
    sphere_idx: Vec<u32>,
}

impl Renderer {
    /// `cloth_tris` indexes the cloth vertices; the avatar sphere is generated
    /// here and lives behind the cloth in the same buffers.
    pub fn new(
        rs: &RenderState,
        theme: &Theme,
        n_cloth_verts: usize,
        cloth_tris: &[u32],
        avatar_radius: f32,
    ) -> Self {
        let device = &rs.device;
        let (pipeline, bgl) = build_pipeline(device);

        // Just inside the real radius, so the avatar does not fight the cloth
        // resting on it for the depth buffer.
        let (sphere_verts, sphere_idx) =
            sphere::uv_sphere(avatar_radius * 0.995, 40, 20, theme.avatar);

        let plan = layout::plan(n_cloth_verts, cloth_tris, &sphere_verts, &sphere_idx);
        let (vbuf, ibuf) = alloc_buffers(rs, &plan, &sphere_verts);

        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("toile-ubuf"),
            size: UNIFORM_BYTES,
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
            clear: theme.clear_color(),
            n_cloth_verts,
            indices: plan.indices,
            sphere_verts,
            sphere_idx,
        }
    }

    /// The cloth vertex count the buffers are sized for.
    pub fn n_cloth_verts(&self) -> usize {
        self.n_cloth_verts
    }

    /// Whether the buffers on hand were built for exactly this cloth.
    pub fn fits(&self, n_verts: usize, cloth_tris: &[u32]) -> bool {
        let n_cloth_idx = self.indices.len() - self.sphere_idx.len();
        n_verts == self.n_cloth_verts && *cloth_tris == self.indices[..n_cloth_idx]
    }

    /// Reallocates the mesh buffers for a new cloth topology and re-uploads
    /// what does not change per frame: the indices and the avatar.
    ///
    /// The offscreen texture is left alone on purpose. It keeps showing the
    /// last painted frame until the first [`Renderer::paint`] of the new
    /// topology, which is what a mesh swap looks like when it does not
    /// flicker.
    pub fn resize(&mut self, rs: &RenderState, n_verts: usize, cloth_tris: &[u32]) {
        let plan = layout::plan(n_verts, cloth_tris, &self.sphere_verts, &self.sphere_idx);
        let (vbuf, ibuf) = alloc_buffers(rs, &plan, &self.sphere_verts);
        self.vbuf = vbuf;
        self.ibuf = ibuf;
        self.n_cloth_verts = n_verts;
        self.indices = plan.indices;
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
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = make(DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT);
        self.depth = Some(depth.create_view(&wgpu::TextureViewDescriptor::default()));

        let mut renderer = rs.renderer.write();
        if let Some(old) = self.texture_id.take() {
            renderer.free_texture(&old);
        }
        self.texture_id =
            Some(renderer.register_native_texture(device, &color_view, wgpu::FilterMode::Linear));
        self.color = Some((color, color_view));
        self.size = (w, h);
    }

    /// Uploads this frame's cloth and draws the scene to the offscreen texture.
    ///
    /// # Panics
    /// If called before [`Renderer::new`] has established the targets, or if
    /// `cloth_vertices` does not match the count the buffers are sized for.
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

        let (_, color_view) = self.color.as_ref().expect("targets created above");
        let depth_view = self.depth.as_ref().expect("targets created above");
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
                        load: wgpu::LoadOp::Clear(self.clear),
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
            pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
        }
        rs.queue.submit([encoder.finish()]);
    }
}

/// Creates the vertex and index buffers for a plan and uploads what only
/// changes with the topology: the avatar's vertices and the index list.
fn alloc_buffers(
    rs: &RenderState,
    plan: &BufferPlan,
    sphere_verts: &[f32],
) -> (wgpu::Buffer, wgpu::Buffer) {
    let vbuf = rs.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("toile-vbuf"),
        size: plan.vbuf_bytes,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    rs.queue.write_buffer(
        &vbuf,
        plan.sphere_offset,
        bytemuck::cast_slice(sphere_verts),
    );
    let ibuf = rs.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("toile-ibuf"),
        size: (plan.indices.len() * 4) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    rs.queue
        .write_buffer(&ibuf, 0, bytemuck::cast_slice(&plan.indices));
    (vbuf, ibuf)
}
