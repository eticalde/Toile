use eframe::egui::{Painter, Pos2, Stroke, vec2};
use toile_engine::draft::{Doc, PieceKey};

use super::super::tract::Tract;
use super::super::view::View;
use super::run;
use crate::theme::Theme;

/// How long one hook is drawn, across the cut line, in screen points.
///
/// Longer than the elastic's tape is wide, so a tract wearing both shows the
/// hooks standing out past the band on either side rather than buried under it.
const HOOK: f32 = 11.0;

/// How thick one hook is drawn.
const WIRE: f32 = 1.4;

/// How far apart the hooks are along the tract, in screen points.
const STEP: f32 = 11.0;

/// The hooks across every tract the body holds up, on the piece being drawn.
///
/// A comb and not a band. The elastic already wears a band, a waistband is
/// hung and elasticated over the very same stretch of contour, and two bands
/// drawn down one line would leave whichever went second as the only one
/// anybody could see — so this mark takes up different room as well as
/// different ink, crossing the line where the tape lies along it.
///
/// `shift` is how far the view moves the piece from its own coordinates, which
/// the whole product does and a piece on its own does not.
pub fn hangs(
    p: &Painter,
    theme: &Theme,
    doc: &Doc,
    at: (PieceKey, &[Tract], [f64; 2]),
    view: View,
) {
    let (piece, tracts, shift) = at;
    let ink = Stroke::new(WIRE, theme.hang);
    for (_, held) in doc.hangs.iter() {
        if held.at.piece() != Some(piece) {
            continue;
        }
        for tract in run(tracts, held.at) {
            let line: Vec<Pos2> = tract
                .line
                .iter()
                .map(|&[x, y]| view.to_screen([x + shift[0], y + shift[1]]))
                .collect();
            hooks(p, ink, &line);
        }
    }
}

/// The hooks along one polyline, evenly spaced by the length it has on the
/// glass and each one square across it.
///
/// Spaced on the glass for the reason the tape is drawn a width on the glass:
/// what the mark has to stay is legible, and a comb spaced in centimetres turns
/// into a solid block the moment the product is zoomed out. The first hook sits
/// on the node the run leaves, so the comb says where the run begins as well as
/// how far it goes.
fn hooks(p: &Painter, ink: Stroke, line: &[Pos2]) {
    let mut along = 0.0;
    for pair in line.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let leg = to - from;
        let span = leg.length();
        if span < f32::EPSILON {
            continue;
        }
        let step = leg / span;
        let across = vec2(-step.y, step.x) * (HOOK / 2.0);
        while along < span {
            let at = from + step * along;
            p.line_segment([at - across, at + across], ink);
            along += STEP;
        }
        along -= span;
    }
}
