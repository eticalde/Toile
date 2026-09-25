use eframe::egui::{Painter, Pos2, Shape, Stroke};
use toile_engine::draft::{Cloth, Draft, PieceKey};

use super::canvas::paper_and_outline;
use super::inner::{Drawn, Tick};
use super::view::View;
use crate::theme::Theme;

/// How much of its ink the half nobody drew is painted in.
///
/// Faint enough that the drawing is plainly the drawing, strong enough that the
/// cut piece's real shape can be read off the mat at a glance.
const GHOST: f32 = 0.45;

/// The dash and the gap the crease is drawn with, in screen points.
const CREASE: (f32, f32) = (11.0, 5.0);

/// The half nobody drew, under the drawing, and the crease between them.
///
/// Drawn and not editable, which is what the ink says: the mirror is a
/// consequence of the axis, so nothing on it answers a press, and no node, no
/// tract and no mark of it is ever offered to the pointer. `shift` is how far
/// the view moves the piece from its own coordinates.
pub fn ghost(
    p: &Painter,
    theme: &Theme,
    (draft, piece): (&Draft, PieceKey),
    (view, shift): (View, [f64; 2]),
) {
    let Some(cloth) = draft.cloth(piece) else {
        return;
    };
    let outline: Vec<[f64; 2]> = cloth.cm.iter().map(|&at| moved(at, shift)).collect();
    let ink = Stroke::new(1.0, theme.outline.gamma_multiply(GHOST));
    paper_and_outline(p, &outline, view, &[theme.paper.gamma_multiply(GHOST)], ink);
}

/// The crease itself, over the paper: the one line on the mat that is neither
/// cut nor drawn, but where the cloth turns back.
///
/// Unmistakable on purpose, in the ink of a measurement and at a dash nothing
/// else on the mat wears: a person who cannot tell the fold from a cut line
/// lays the paper on the wrong edge and the piece comes out short.
pub fn crease(
    p: &Painter,
    theme: &Theme,
    (draft, piece): (&Draft, PieceKey),
    (view, shift): (View, [f64; 2]),
) {
    let Some(cloth) = draft.cloth(piece) else {
        return;
    };
    let ends: Vec<Pos2> = cloth
        .axis
        .iter()
        .map(|&at| view.to_screen(moved(at, shift)))
        .collect();
    let ink = Stroke::new(2.0, theme.measure);
    p.extend(Shape::dashed_line(&ends, ink, CREASE.0, CREASE.1));
}

/// Every internal line of the piece as the mirror shows it.
///
/// The same lines, reflected, with nothing to take hold of: the places of a
/// mirrored mark belong to the mark on the drawn half, and a hand that grabbed
/// one would be dragging a reflection. A mark is one entry of the document
/// however many halves of the cloth it appears on.
pub fn mirrored(draft: &Draft, piece: PieceKey, drawn: &[Drawn], shift: [f64; 2]) -> Vec<Drawn> {
    let Some(cloth) = draft.cloth(piece) else {
        return Vec::new();
    };
    drawn
        .iter()
        .map(|it| Drawn {
            run: it.run.iter().map(|&at| across(cloth, at, shift)).collect(),
            loose: Vec::new(),
            ..it.clone()
        })
        .collect()
}

/// Every notch of the piece as the mirror shows it.
///
/// The cut piece carries each of these twice, once on each side of the fold,
/// and the scissors go into both — so the reflection is drawn in the ink of a
/// cut like the mark it reflects, and not faintly. What it is not is a second
/// mark: the pointer is only ever offered the notches the document holds, so
/// nothing here answers a press, and a hand that took hold of one would be
/// dragging a reflection.
pub fn reflected(draft: &Draft, piece: PieceKey, ticks: &[Tick], shift: [f64; 2]) -> Vec<Tick> {
    let Some(cloth) = draft.cloth(piece) else {
        return Vec::new();
    };
    ticks
        .iter()
        .map(|tick| {
            let at = across(cloth, tick.at, shift);
            // The way the contour runs there, reflected as a direction and not
            // as a place: the cut goes across the line the mark sits on, and
            // here that line is the mirrored one.
            let ahead = [0, 1].map(|k| tick.at[k] + tick.along[k]);
            let ahead = across(cloth, ahead, shift);
            Tick {
                at,
                along: [0, 1].map(|k| ahead[k] - at[k]),
                ..*tick
            }
        })
        .collect()
}

/// One place of the drawing where its reflection falls, both already moved by
/// the view's own shift.
fn across(cloth: &Cloth, at: [f64; 2], shift: [f64; 2]) -> [f64; 2] {
    moved(cloth.mirror(moved(at, [-shift[0], -shift[1]])), shift)
}

fn moved(at: [f64; 2], shift: [f64; 2]) -> [f64; 2] {
    [at[0] + shift[0], at[1] + shift[1]]
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a reflection across an axis of the grid is exact in a double"
    )]

    use toile_engine::draft::{
        Command, Doc, EdgeRange, Identity, LineKind, MeasureSet, NotchCount, NotchKey, Piece,
        Point, Symmetry, Winding,
    };

    use super::*;

    /// One line of the mat, drawn from `run`, with `loose` to take hold of.
    fn line(run: Vec<[f64; 2]>, loose: Vec<(toile_engine::draft::PointKey, [f64; 2])>) -> Drawn {
        Drawn {
            line: toile_engine::draft::LineKey::new(0, 0),
            kind: LineKind::Placement,
            ordinal: 1,
            label: None,
            places: run.len(),
            run,
            loose,
        }
    }

    /// A ten-by-ten square folded on its left side, with one line drawn across
    /// it, as the table would hold it.
    fn folded() -> (Draft, PieceKey) {
        let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
        let corners = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let points: Vec<_> = corners
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        let piece = doc
            .pieces
            .insert(Piece::polygon("Pretina", points.clone(), Winding::Cw));
        let axis = EdgeRange::between(piece, points[3], points[0]);
        Command::AddSymmetry {
            identity: Identity::New,
            symmetry: Symmetry::fold(axis),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the square");
        (Draft::from_doc(doc).expect("the square resolves"), piece)
    }

    #[test]
    fn a_mark_on_the_drawn_half_is_shown_on_the_other_with_nothing_to_grab() {
        let (draft, piece) = folded();
        let crease = draft.cloth(piece).expect("the square is folded").axis;
        assert_eq!(crease, [[0.0, 0.0], [0.0, 10.0]]);
        // The mat hands these already moved by the view's own shift, so the
        // mark drawn two centimetres in from the crease is painted at 102.
        let held = draft.points_cm(piece)[0].0;
        let drawn = vec![line(
            vec![[102.0, 3.0], [108.0, 3.0]],
            vec![(held, [102.0, 3.0])],
        )];
        let ghosts = mirrored(&draft, piece, &drawn, [100.0, 0.0]);
        assert_eq!(ghosts.len(), 1);
        assert_eq!(ghosts[0].run, [[98.0, 3.0], [92.0, 3.0]]);
        assert!(ghosts[0].loose.is_empty(), "a reflection is not a handle");
    }

    /// A notch is reflected like the lines are, and so is the way the contour
    /// runs at it: the cut is drawn across that line, and on this half the line
    /// is the mirrored one.
    #[test]
    fn a_notch_is_shown_on_the_other_half_and_across_the_mirrored_line() {
        let (draft, piece) = folded();
        let tick = Tick {
            notch: NotchKey::new(0, 0),
            count: NotchCount::Double,
            at: [102.0, 3.0],
            along: [1.0, 0.0],
        };
        let ghosts = reflected(&draft, piece, &[tick], [100.0, 0.0]);
        assert_eq!(ghosts.len(), 1);
        assert_eq!(ghosts[0].at, [98.0, 3.0]);
        assert_eq!(ghosts[0].along, [-1.0, 0.0], "the crease runs down x = 0");
        assert_eq!(ghosts[0].notch, tick.notch, "one mark, drawn twice");
        assert_eq!(ghosts[0].count, tick.count, "with the cuts it was given");
    }

    #[test]
    fn a_piece_nobody_folded_shows_no_second_half_at_all() {
        let (mut draft, piece) = folded();
        let (key, _) = draft
            .doc()
            .symmetry_of(piece)
            .expect("the square is folded");
        draft
            .edit(Command::RemoveSymmetry { symmetry: key })
            .expect("the axis is live");
        assert_eq!(draft.cloth(piece), None);
        let drawn = [line(vec![[2.0, 3.0], [8.0, 3.0]], Vec::new())];
        assert!(mirrored(&draft, piece, &drawn, [0.0, 0.0]).is_empty());
        let tick = Tick {
            notch: NotchKey::new(0, 0),
            count: NotchCount::Single,
            at: [2.0, 3.0],
            along: [1.0, 0.0],
        };
        assert!(reflected(&draft, piece, &[tick], [0.0, 0.0]).is_empty());
    }
}
