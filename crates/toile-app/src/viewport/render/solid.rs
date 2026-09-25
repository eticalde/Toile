use eframe::egui_wgpu::RenderState;
use eframe::wgpu;

use super::tape::TapeLayer;
use super::targets::Targets;
use super::{UNIFORM_BYTES, pipeline};

/// Renders one arbitrary triangle mesh in the 9-float vertex format to an
/// offscreen texture egui shows as an image.
///
/// A leaner sibling of [`super::Renderer`]: one mesh, no per-frame cloth and no
/// avatar sphere. The whole mesh is re-uploaded when it changes, and the
/// buffers only grow, so a measurement edit that keeps the topology writes into
/// the buffers it already has. A tape can be laid over it, uploaded apart so
/// laying one never touches the mesh.
pub struct SolidRenderer {
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    ubuf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    targets: Targets,
    clear: wgpu::Color,
    n_index: usize,
    /// Bytes the vertex and index buffers hold, so a mesh that fits is written
    /// in place rather than reallocated.
    vcap: u64,
    icap: u64,
    tape: TapeLayer,
}

impl SolidRenderer {
    pub fn new(rs: &RenderState, clear: wgpu::Color) -> Self {
        let device = &rs.device;
        let (pipeline, bgl) = pipeline::build_pipeline(device);

        let empty = |usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("toile-solid-buf"),
                size: 0,
                usage,
                mapped_at_creation: false,
            })
        };
        let vbuf = empty(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST);
        let ibuf = empty(wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST);

        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("toile-solid-ubuf"),
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
            clear,
            n_index: 0,
            vcap: 0,
            icap: 0,
            tape: TapeLayer::new(device),
        }
    }

    /// Uploads a new mesh, reallocating the buffers only when it outgrows them.
    pub fn upload(&mut self, rs: &RenderState, verts9: &[f32], indices: &[u32]) {
        let vbytes = std::mem::size_of_val(verts9) as u64;
        let ibytes = std::mem::size_of_val(indices) as u64;
        if vbytes > self.vcap {
            self.vbuf = rs.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("toile-solid-vbuf"),
                size: vbytes,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.vcap = vbytes;
        }
        if ibytes > self.icap {
            self.ibuf = rs.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("toile-solid-ibuf"),
                size: ibytes,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.icap = ibytes;
        }
        rs.queue
            .write_buffer(&self.vbuf, 0, bytemuck::cast_slice(verts9));
        rs.queue
            .write_buffer(&self.ibuf, 0, bytemuck::cast_slice(indices));
        self.n_index = indices.len();
    }

    /// Replaces the tape over the mesh with a strip built by
    /// [`super::ribbon`]; empty takes it away.
    pub fn upload_tape(&mut self, rs: &RenderState, verts: &[f32]) {
        self.tape.upload(rs, verts);
    }

    /// Draws the current mesh, and the tape over it, to the offscreen texture
    /// at the given size.
    pub fn paint(
        &mut self,
        rs: &RenderState,
        w: u32,
        h: u32,
        uniforms: &[f32; 20],
        tape: &[f32; 24],
    ) {
        self.targets.ensure(rs, w.max(8), h.max(8));
        rs.queue
            .write_buffer(&self.ubuf, 0, bytemuck::cast_slice(uniforms));
        self.tape.set_uniforms(rs, tape);
        if self.n_index == 0 {
            return;
        }

        let mut encoder = rs
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("toile-solid"),
            });
        {
            let mut pass = self
                .targets
                .pass(&mut encoder, "toile-solid-pass", self.clear);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vbuf.slice(..));
            pass.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.n_index as u32, 0, 0..1);
            self.tape.draw(&mut pass);
        }
        rs.queue.submit([encoder.finish()]);
    }

    /// The egui handle for the offscreen texture, once a paint has made one.
    pub fn texture_id(&self) -> Option<eframe::egui::TextureId> {
        self.targets.texture_id()
    }
}
