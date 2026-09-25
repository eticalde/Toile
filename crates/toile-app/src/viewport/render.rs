mod avatar;
mod ground;
mod layout;
mod pipeline;
mod solid;
mod tape;
mod targets;

pub use avatar::Avatar;
use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
use layout::BufferPlan;
use pipeline::build_pipeline;
pub use solid::SolidRenderer;
pub use tape::ribbon;
use targets::Targets;

use crate::theme::Theme;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SHADER: &str = include_str!("render/shader.wgsl");

/// Sixteen matrix floats plus a padded light vector.
const UNIFORM_BYTES: u64 = 80;

/// Renders the drape to an offscreen texture that egui shows as an image.
///
/// One pipeline and two meshes in one pair of buffers: the cloth, re-uploaded
/// each frame, and the body it falls on, written on every (re)allocation and
/// whenever another body is solved.
pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    ubuf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    targets: Targets,
    clear: wgpu::Color,
    n_cloth_verts: usize,
    /// The index list as uploaded, kept so [`Renderer::fits`] can answer by
    /// content: after a swap that happens to keep the counts, only the
    /// triangles themselves say the buffers are stale.
    indices: Vec<u32>,
    /// The body the cloth falls on, kept so a resize can re-upload it at its
    /// new offset.
    avatar: Avatar,
}

impl Renderer {
    /// `cloth_tris` indexes the cloth vertices; `avatar` is the body they fall
    /// on, which lives behind the cloth in the same buffers.
    pub fn new(
        rs: &RenderState,
        theme: &Theme,
        n_cloth_verts: usize,
        cloth_tris: &[u32],
        avatar: Avatar,
    ) -> Self {
        let device = &rs.device;
        let (pipeline, bgl) = build_pipeline(device);

        let plan = layout::plan(n_cloth_verts, cloth_tris, &avatar);
        let (vbuf, ibuf) = alloc_buffers(rs, &plan, &avatar);

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
            targets: Targets::default(),
            clear: theme.clear_color(),
            n_cloth_verts,
            indices: plan.indices,
            avatar,
        }
    }

    /// The cloth vertex count the buffers are sized for.
    pub fn n_cloth_verts(&self) -> usize {
        self.n_cloth_verts
    }

    /// Whether the buffers on hand were built for exactly this cloth.
    pub fn fits(&self, n_verts: usize, cloth_tris: &[u32]) -> bool {
        let n_cloth_idx = self.indices.len() - self.avatar.idx.len();
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
        let plan = layout::plan(n_verts, cloth_tris, &self.avatar);
        let (vbuf, ibuf) = alloc_buffers(rs, &plan, &self.avatar);
        self.vbuf = vbuf;
        self.ibuf = ibuf;
        self.n_cloth_verts = n_verts;
        self.indices = plan.indices;
    }

    /// Puts another body behind the cloth.
    ///
    /// A body is re-solved whenever a measurement moves, and the new one need
    /// not carry the old one's vertex count, so both buffers are laid out
    /// again and the cloth's triangles rebased over it. The cloth's own path
    /// is untouched: it goes on writing into the same vertex buffer at the
    /// same offset, which is why a body arriving does not interrupt the drape
    /// drawn over it.
    pub fn set_avatar(&mut self, rs: &RenderState, avatar: Avatar) {
        let n_cloth_idx = self.indices.len() - self.avatar.idx.len();
        let cloth_tris = self.indices[..n_cloth_idx].to_vec();
        self.avatar = avatar;
        let plan = layout::plan(self.n_cloth_verts, &cloth_tris, &self.avatar);
        let (vbuf, ibuf) = alloc_buffers(rs, &plan, &self.avatar);
        self.vbuf = vbuf;
        self.ibuf = ibuf;
        self.indices = plan.indices;
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
        self.targets.ensure(rs, w.max(8), h.max(8));
        debug_assert_eq!(cloth_vertices.len(), self.n_cloth_verts * 9);
        rs.queue
            .write_buffer(&self.vbuf, 0, bytemuck::cast_slice(cloth_vertices));
        rs.queue
            .write_buffer(&self.ubuf, 0, bytemuck::cast_slice(uniforms));

        let mut encoder = rs
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("toile"),
            });
        {
            let mut pass = self.targets.pass(&mut encoder, "toile-pass", self.clear);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vbuf.slice(..));
            pass.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
        }
        rs.queue.submit([encoder.finish()]);
    }

    /// The egui handle for the offscreen texture, once a paint has made one.
    pub fn texture_id(&self) -> Option<eframe::egui::TextureId> {
        self.targets.texture_id()
    }
}

/// Creates the vertex and index buffers for a plan and uploads what only
/// changes with the topology: the body's vertices and the index list.
///
/// Both writes are skipped when there is nothing to write. Before the first
/// body is solved, and over a table with nothing draping, the buffers are
/// empty and the queue has no business being handed a zero-length slice.
fn alloc_buffers(
    rs: &RenderState,
    plan: &BufferPlan,
    avatar: &Avatar,
) -> (wgpu::Buffer, wgpu::Buffer) {
    let vbuf = rs.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("toile-vbuf"),
        size: plan.vbuf_bytes,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    if !avatar.verts.is_empty() {
        rs.queue.write_buffer(
            &vbuf,
            plan.avatar_offset,
            bytemuck::cast_slice(&avatar.verts),
        );
    }
    let ibuf = rs.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("toile-ibuf"),
        size: (plan.indices.len() * 4) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    if !plan.indices.is_empty() {
        rs.queue
            .write_buffer(&ibuf, 0, bytemuck::cast_slice(&plan.indices));
    }
    (vbuf, ibuf)
}
