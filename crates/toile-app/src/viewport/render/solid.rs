use eframe::egui_wgpu::RenderState;
use eframe::wgpu;

use super::pipeline::build_pipeline;
use super::{COLOR_FORMAT, DEPTH_FORMAT, UNIFORM_BYTES};

/// Renders one arbitrary triangle mesh in the 9-float vertex format to an
/// offscreen texture egui shows as an image.
///
/// A leaner sibling of [`super::Renderer`]: one mesh, no per-frame cloth and no
/// avatar sphere. The whole mesh is re-uploaded when it changes, and the
/// buffers only grow, so a measurement edit that keeps the topology writes into
/// the buffers it already has.
pub struct SolidRenderer {
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    ubuf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    color: Option<(wgpu::Texture, wgpu::TextureView)>,
    depth: Option<wgpu::TextureView>,
    texture_id: Option<eframe::egui::TextureId>,
    size: (u32, u32),
    clear: wgpu::Color,
    n_index: usize,
    /// Bytes the vertex and index buffers hold, so a mesh that fits is written
    /// in place rather than reallocated.
    vcap: u64,
    icap: u64,
}

impl SolidRenderer {
    pub fn new(rs: &RenderState, clear: wgpu::Color) -> Self {
        let device = &rs.device;
        let (pipeline, bgl) = build_pipeline(device);

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
            color: None,
            depth: None,
            texture_id: None,
            size: (0, 0),
            clear,
            n_index: 0,
            vcap: 0,
            icap: 0,
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

    /// Draws the current mesh to the offscreen texture at the given size.
    pub fn paint(&mut self, rs: &RenderState, w: u32, h: u32, uniforms: &[f32; 20]) {
        self.ensure_targets(rs, w.max(8), h.max(8));
        rs.queue
            .write_buffer(&self.ubuf, 0, bytemuck::cast_slice(uniforms));
        if self.n_index == 0 {
            return;
        }

        let (_, color_view) = self.color.as_ref().expect("targets created above");
        let depth_view = self.depth.as_ref().expect("targets created above");
        let mut encoder = rs
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("toile-solid"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("toile-solid-pass"),
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
            pass.draw_indexed(0..self.n_index as u32, 0, 0..1);
        }
        rs.queue.submit([encoder.finish()]);
    }

    /// The egui handle for the offscreen texture, once a paint has made one.
    pub fn texture_id(&self) -> Option<eframe::egui::TextureId> {
        self.texture_id
    }
}
