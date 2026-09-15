use eframe::egui_wgpu::RenderState;
use eframe::wgpu;

use super::{COLOR_FORMAT, DEPTH_FORMAT};

const SHADER: &str = include_str!("tape.wgsl");

/// Floats per ribbon vertex: this end of a segment, the other end, and which
/// edge of the tape the vertex sits on.
const RIBBON_FLOATS: usize = 7;

/// A matrix, the viewport and half-width, and the colour.
const UNIFORM_BYTES: u64 = 96;

/// The tape over the body: a strip of quads widened on the GPU to a fixed
/// pixel width, drawn in the body's own pass against the body's depth.
///
/// Its vertices are uploaded only when a tape is laid, never per frame; a
/// paint writes nothing but the camera into it.
pub struct TapeLayer {
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    vcap: u64,
    n_vertex: u32,
    ubuf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl TapeLayer {
    pub fn new(device: &wgpu::Device) -> Self {
        let (pipeline, bgl) = build_pipeline(device);
        let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("toile-tape-vbuf"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("toile-tape-ubuf"),
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
            vcap: 0,
            n_vertex: 0,
            ubuf,
            bind_group,
        }
    }

    /// Replaces the strip with `verts` (built by [`ribbon`]), growing the
    /// buffer only when it outgrows it. Empty takes the tape away.
    pub fn upload(&mut self, rs: &RenderState, verts: &[f32]) {
        let bytes = std::mem::size_of_val(verts) as u64;
        if bytes > self.vcap {
            self.vbuf = rs.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("toile-tape-vbuf"),
                size: bytes,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.vcap = bytes;
        }
        rs.queue
            .write_buffer(&self.vbuf, 0, bytemuck::cast_slice(verts));
        self.n_vertex = (verts.len() / RIBBON_FLOATS) as u32;
    }

    /// The camera, the target's pixel size, the half-width in pixels and the
    /// colour, for the next paint.
    pub fn set_uniforms(&self, rs: &RenderState, uniforms: &[f32; 24]) {
        rs.queue
            .write_buffer(&self.ubuf, 0, bytemuck::cast_slice(uniforms));
    }

    /// Draws the strip into a pass that already holds the body's depth.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.n_vertex == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vbuf.slice(..));
        pass.draw(0..self.n_vertex, 0..1);
    }
}

/// The strip's vertices for a line through `points`: two triangles per
/// segment, and a closing segment when the line is `closed`.
pub fn ribbon(points: &[[f32; 3]], closed: bool) -> Vec<f32> {
    let n = points.len();
    let segments = if closed && n > 2 {
        n
    } else {
        n.saturating_sub(1)
    };
    let mut out = Vec::with_capacity(segments * 6 * RIBBON_FLOATS);
    for i in 0..segments {
        let (a, b) = (points[i], points[(i + 1) % n]);
        // Seen from `b` the segment runs the other way, so the side flips to
        // stay on the same edge of the tape.
        for (here, there, side) in [
            (a, b, 1.0),
            (a, b, -1.0),
            (b, a, 1.0),
            (a, b, 1.0),
            (b, a, 1.0),
            (b, a, -1.0),
        ] {
            out.extend_from_slice(&here);
            out.extend_from_slice(&there);
            out.push(side);
        }
    }
    out
}

fn build_pipeline(device: &wgpu::Device) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("toile-tape"),
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
        label: Some("toile-tape"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: (RIBBON_FLOATS * 4) as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32],
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            // A strip seen from behind its own winding is still the tape.
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            // Tested against the body, never written: the tape does not hide
            // itself where a loop crosses its own far half on screen.
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    (pipeline, bgl)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A loop gets the segment that closes it and an open line does not, so a
    /// length never draws a chord from its end back to its start.
    #[test]
    fn a_loop_closes_and_a_path_does_not() {
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]];
        let quad = 6 * RIBBON_FLOATS;
        assert_eq!(ribbon(&points, true).len(), 3 * quad);
        assert_eq!(ribbon(&points, false).len(), 2 * quad);
        assert!(ribbon(&points[..1], false).is_empty());
    }
}
