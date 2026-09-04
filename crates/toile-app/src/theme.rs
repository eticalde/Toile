use eframe::egui::{self, Color32, Stroke};
use eframe::wgpu;

/// Every colour the app paints, named by role rather than by value.
///
/// Widgets and painters ask for a role — `accent`, `measure`, `alert` — never
/// for a colour, so a second palette is a second constructor and nothing else
/// changes. Roles follow the craft: chalk for selection, tape for measurements,
/// marking thread for alerts, pattern paper for pieces.
#[derive(Debug, Clone)]
#[allow(
    dead_code,
    reason = "the palette is a complete contract; a role waits for the widget that needs it"
)]
pub struct Theme {
    /// Canvas background: the cutting mat.
    pub mat: Color32,
    /// Side panels and bars.
    pub panel: Color32,
    /// Inputs and cards sitting on a panel.
    pub raised: Color32,
    /// Borders and separators.
    pub line: Color32,
    /// Canvas grid lines, drawn over `mat`.
    pub grid: Color32,
    /// Primary text.
    pub ink: Color32,
    /// Secondary text: list rows, field labels.
    pub ink_soft: Color32,
    /// Captions and section headers.
    pub muted: Color32,
    /// Text drawn on top of `accent`.
    pub on_accent: Color32,
    /// Selection, seams, primary actions.
    pub accent: Color32,
    /// Measurements, dimensions and guide lines.
    pub measure: Color32,
    /// Warnings and the point being dragged.
    pub alert: Color32,
    /// Fill of a pattern piece; carries its own alpha.
    pub paper: Color32,
    /// Outline of a pattern piece.
    pub outline: Color32,
    /// Default cloth colour, linear RGB for the renderer.
    pub cloth: [f32; 3],
    /// Avatar colour, linear RGB for the renderer.
    pub avatar: [f32; 3],
}

impl Theme {
    /// The tailoring palette: warm dark, brass for selection, chalk teal for
    /// measurements, marking thread for alerts. Values mirror the canvas
    /// tokens.
    pub fn sastreria() -> Self {
        Self {
            mat: Color32::from_rgb(30, 26, 23),
            panel: Color32::from_rgb(38, 33, 29),
            raised: Color32::from_rgb(21, 18, 15),
            line: Color32::from_rgb(59, 51, 44),
            grid: Color32::from_rgba_unmultiplied(239, 231, 218, 15),
            ink: Color32::from_rgb(239, 231, 218),
            ink_soft: Color32::from_rgb(207, 197, 182),
            muted: Color32::from_rgb(161, 149, 138),
            on_accent: Color32::from_rgb(26, 20, 16),
            accent: Color32::from_rgb(212, 162, 76),
            measure: Color32::from_rgb(98, 184, 173),
            alert: Color32::from_rgb(226, 97, 79),
            paper: Color32::from_rgba_unmultiplied(239, 231, 218, 18),
            outline: Color32::from_rgb(239, 231, 218),
            cloth: [0.79, 0.48, 0.33],
            avatar: [0.29, 0.26, 0.24],
        }
    }

    /// Installs the palette into egui's widget visuals.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut v = egui::Visuals::dark();
        v.panel_fill = self.panel;
        v.window_fill = self.panel;
        v.window_stroke = Stroke::new(1.0, self.line);
        v.extreme_bg_color = self.raised;
        v.faint_bg_color = self.raised;
        v.code_bg_color = self.raised;
        v.hyperlink_color = self.accent;
        v.warn_fg_color = self.measure;
        v.error_fg_color = self.alert;
        v.selection.bg_fill = self.accent.gamma_multiply(0.35);
        v.selection.stroke = Stroke::new(1.0, self.accent);

        let w = &mut v.widgets;
        w.noninteractive.bg_fill = self.panel;
        w.noninteractive.weak_bg_fill = self.panel;
        w.noninteractive.bg_stroke = Stroke::new(1.0, self.line);
        w.noninteractive.fg_stroke = Stroke::new(1.0, self.ink_soft);
        w.inactive.bg_fill = self.raised;
        w.inactive.weak_bg_fill = self.raised;
        w.inactive.bg_stroke = Stroke::new(1.0, self.line);
        w.inactive.fg_stroke = Stroke::new(1.0, self.ink_soft);
        w.hovered.bg_fill = self.line;
        w.hovered.weak_bg_fill = self.line;
        w.hovered.bg_stroke = Stroke::new(1.0, self.muted);
        w.hovered.fg_stroke = Stroke::new(1.5, self.ink);
        w.active.bg_fill = self.accent;
        w.active.weak_bg_fill = self.accent;
        w.active.bg_stroke = Stroke::new(1.0, self.accent);
        w.active.fg_stroke = Stroke::new(2.0, self.on_accent);
        w.open.bg_fill = self.raised;
        w.open.weak_bg_fill = self.raised;
        w.open.bg_stroke = Stroke::new(1.0, self.line);
        w.open.fg_stroke = Stroke::new(1.0, self.ink);
        ctx.set_visuals(v);
    }

    /// The cloth as egui paints it, out of the renderer's linear RGB.
    pub fn cloth_color(&self) -> Color32 {
        linear(self.cloth)
    }

    /// The avatar as egui paints it, out of the renderer's linear RGB.
    pub fn avatar_color(&self) -> Color32 {
        linear(self.avatar)
    }

    /// The mat as a render-target clear colour, in the linear space wgpu
    /// expects for an sRGB target.
    pub fn clear_color(&self) -> wgpu::Color {
        let lin = |c: u8| {
            let s = f64::from(c) / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        wgpu::Color {
            r: lin(self.mat.r()),
            g: lin(self.mat.g()),
            b: lin(self.mat.b()),
            a: 1.0,
        }
    }
}

fn linear(rgb: [f32; 3]) -> Color32 {
    let [r, g, b] = rgb;
    Color32::from(egui::Rgba::from_rgb(r, g, b))
}
