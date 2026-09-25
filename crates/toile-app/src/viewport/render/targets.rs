use eframe::egui_wgpu::RenderState;
use eframe::wgpu;

use super::{COLOR_FORMAT, DEPTH_FORMAT};

/// The offscreen colour-and-depth pair a renderer draws into, and the handle
/// egui shows the colour half through.
///
/// Both renderers in this module draw into the same pair, at the same two
/// formats, cleared the same way; what differs between them is the mesh and the
/// labels, not the target. Holding the pair once is what stops one of them from
/// being resized, re-registered or cleared differently from the other.
#[derive(Default)]
pub(super) struct Targets {
    color: Option<(wgpu::Texture, wgpu::TextureView)>,
    depth: Option<wgpu::TextureView>,
    texture_id: Option<eframe::egui::TextureId>,
    size: (u32, u32),
}

impl Targets {
    /// The egui handle for the colour texture, once a paint has made one.
    pub(super) fn texture_id(&self) -> Option<eframe::egui::TextureId> {
        self.texture_id
    }

    /// (Re)creates the pair at `w`×`h` and hands egui the new colour texture.
    ///
    /// A size that has not moved costs nothing, which is what lets the handle
    /// egui is already showing stay valid across the frames between two
    /// resizes.
    pub(super) fn ensure(&mut self, rs: &RenderState, w: u32, h: u32) {
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

    /// A pass that clears the pair and draws into it.
    ///
    /// # Panics
    /// If [`Targets::ensure`] has not run since the last resize.
    pub(super) fn pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        label: &str,
        clear: wgpu::Color,
    ) -> wgpu::RenderPass<'a> {
        let (_, color_view) = self.color.as_ref().expect("ensure created the targets");
        let depth_view = self.depth.as_ref().expect("ensure created the targets");
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
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
        })
    }
}
