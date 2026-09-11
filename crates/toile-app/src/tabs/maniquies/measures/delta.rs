use eframe::egui::{self, Color32, RichText};
use toile_engine::body::AnnySolve;

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

/// The *medido* and Δ line under a slider: what the last solve's Anny body
/// actually measures for `name`, how far that sits from `dado_cm` (what
/// the slider says), and — when it matters — why a gap remains. Draws
/// nothing when `solved` is `None` (the tailor's dummy never solves, so it
/// never gains this line) or when `name` names no catalogue row (which
/// does not happen for the panel's own rows).
///
/// A row with no lever of its own (`tiro`, `pecho_alto`, `entrepierna` and
/// `cabeza` — see `toile_engine::body::solve_anny`'s doc) still shows a
/// real Δ, but is marked "sin palanca propia" rather than left to look like
/// the solver merely failed there. A row whose lever hit its own `±1`
/// bound without closing Δ is marked "tope del modelo" instead: the target
/// sits outside what this body can become, not a solver giving up early.
pub fn row(ui: &mut egui::Ui, theme: &Theme, name: &str, dado_cm: f64, solved: Option<&AnnySolve>) {
    let Some(r) = solved.and_then(|s| s.rows.get(name)) else {
        return;
    };
    let delta = r.delta_cm.unwrap_or(r.medido_cm - dado_cm);
    let color = band_color(theme, delta);
    let note = if r.saturated {
        " · tope del modelo"
    } else if !r.has_lever {
        " · sin palanca propia"
    } else {
        ""
    };
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.label(
            RichText::new(format!(
                "medido {:.1} cm · Δ {delta:+.1}{note}",
                r.medido_cm
            ))
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
