use eframe::egui::{self, Color32, RichText};
use toile_engine::draft::MeasureSet;

use crate::theme::Theme;
use crate::widgets::PAD;

/// The bands a Δ falls into, in centimetres: quiet up to here, amber up to
/// here, alert past it.
const QUIET_CM: f64 = 1.0;
const AMBER_CM: f64 = 3.0;

/// The colour a Δ paints with, by how far the body's own measurement sits
/// from what was written down.
fn band_color(theme: &Theme, delta_cm: f64) -> Color32 {
    let magnitude = delta_cm.abs();
    if magnitude <= QUIET_CM {
        theme.muted
    } else if magnitude <= AMBER_CM {
        theme.accent
    } else {
        theme.alert
    }
}

/// The *medido* and Δ line under a slider: what the Anny body actually
/// measures for `name`, and how far that sits from `dado_cm` (what the
/// slider says). Draws nothing when `measured` carries no such name — the
/// tailor's dummy is never handed a `measured` set, so it never gains this
/// line at all.
///
/// Nothing here ever moves `dado_cm` or the body: this slice only reports
/// the gap. Closing it is the next slice's per-part solver.
pub fn row(
    ui: &mut egui::Ui,
    theme: &Theme,
    name: &str,
    dado_cm: f64,
    measured: Option<&MeasureSet>,
) {
    let Some(medido) = measured.and_then(|m| m.get(name)) else {
        return;
    };
    let delta = medido - dado_cm;
    let color = band_color(theme, delta);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.label(
            RichText::new(format!("medido {medido:.1} cm · Δ {delta:+.1}"))
                .size(10.5)
                .monospace()
                .color(color),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_match_the_documented_thresholds() {
        let theme = Theme::sastreria();
        assert_eq!(band_color(&theme, 0.0), theme.muted);
        assert_eq!(band_color(&theme, 1.0), theme.muted);
        assert_eq!(band_color(&theme, -1.0), theme.muted);
        assert_eq!(band_color(&theme, 1.1), theme.accent);
        assert_eq!(band_color(&theme, 3.0), theme.accent);
        assert_eq!(band_color(&theme, -3.0), theme.accent);
        assert_eq!(band_color(&theme, 3.1), theme.alert);
        assert_eq!(band_color(&theme, -10.0), theme.alert);
    }
}
