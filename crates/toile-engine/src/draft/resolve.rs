use std::collections::BTreeMap;

use thiserror::Error;
use toile_doc::formula::EvalError;
use toile_doc::{Axis, Doc, Piece, Point, PointKey};
use toile_geom::{length, validate};

use super::env::Env;

/// Centimetres in a metre. The document counts in the first, the solver in the
/// second, and this is the only place the two meet.
const CM_PER_M: f64 = 100.0;

/// What is wrong with a piece, in terms the drawing can point at.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Defect {
    /// A coordinate that does not resolve to a number.
    #[error("the {axis:?} of point {} does not resolve: {error}", point.index())]
    Binding {
        /// The point that carries it.
        point: PointKey,
        /// Which of its two coordinates.
        axis: Axis,
        /// Why it did not resolve.
        error: EvalError,
    },
    /// A contour running through a point the document does not carry.
    #[error("the contour runs through point {}, which the document has lost", point.index())]
    NoSuchPoint {
        /// The point the contour still names.
        point: PointKey,
    },
    /// A contour that is not a simple closed polygon. Its indices are node
    /// positions in the piece's contour.
    #[error(transparent)]
    Contour(validate::ContourFault),
}

/// A piece as the rest of the program sees it: the same contour twice over,
/// once for the drawing and once for the solver.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Resolved {
    /// The contour nodes in centimetres, y downward, in contour order.
    pub points: Vec<(PointKey, [f64; 2])>,
    /// The same contour in metres, y upward: what the mesher takes.
    pub outline: Vec<[f64; 2]>,
    /// Arc length in centimetres up to each node, and round to the first, so
    /// the last entry is the perimeter.
    pub cum: Vec<f64>,
}

/// The metres the solver works in, from the centimetres the document holds.
///
/// The document draws downward from the waist, the way the trade does; the
/// solver has y upward. The division and the negation happen here and nowhere
/// else, which is what stops a coordinate from being converted twice.
pub fn to_metres(cm: [f64; 2]) -> [f64; 2] {
    [cm[0] / CM_PER_M, -cm[1] / CM_PER_M]
}

/// The centimetres the document holds, from the metres the solver works in.
pub fn to_document(metres: [f64; 2]) -> [f64; 2] {
    [metres[0] * CM_PER_M, -metres[1] * CM_PER_M]
}

/// Where every point of the document resolves to, in centimetres.
///
/// A point whose binding fails lands in the second map instead of the first,
/// so one bad formula costs its own piece and not the whole table.
pub type Resolutions = (
    BTreeMap<PointKey, [f64; 2]>,
    BTreeMap<PointKey, (Axis, EvalError)>,
);

/// Resolves every point of the document, good and bad alike.
pub fn points(doc: &Doc, env: &Env) -> Resolutions {
    let mut good = BTreeMap::new();
    let mut broken = BTreeMap::new();
    for (key, point) in doc.points.iter() {
        match coordinates(point, env) {
            Ok(at) => {
                good.insert(key, at);
            }
            Err(fault) => {
                broken.insert(key, fault);
            }
        }
    }
    (good, broken)
}

/// One piece as a closed contour in both units.
///
/// # Errors
/// Every defect the piece carries: one per coordinate that does not resolve,
/// or, when they all do, the fault that stops the contour from being a simple
/// closed polygon.
pub fn piece(
    held: &Piece,
    good: &BTreeMap<PointKey, [f64; 2]>,
    broken: &BTreeMap<PointKey, (Axis, EvalError)>,
) -> Result<Resolved, Vec<Defect>> {
    let mut points = Vec::with_capacity(held.contour.len());
    let mut cm = Vec::with_capacity(held.contour.len());
    let mut defects = Vec::new();
    for node in held.anchors() {
        match (good.get(&node), broken.get(&node)) {
            (_, Some((axis, error))) => defects.push(Defect::Binding {
                point: node,
                axis: *axis,
                error: error.clone(),
            }),
            (Some(&at), None) => {
                points.push((node, at));
                cm.push(at);
            }
            (None, None) => defects.push(Defect::NoSuchPoint { point: node }),
        }
    }
    if !defects.is_empty() {
        return Err(defects);
    }
    let outline: Vec<[f64; 2]> = cm.iter().map(|&p| to_metres(p)).collect();
    validate::check_closed(&outline).map_err(|fault| vec![Defect::Contour(fault)])?;
    Ok(Resolved {
        points,
        outline,
        cum: length::cumulative(&cm),
    })
}

/// A point's two coordinates in centimetres, y downward.
fn coordinates(point: &Point, env: &Env) -> Result<[f64; 2], (Axis, EvalError)> {
    let x = point.x.eval(env).map_err(|e| (Axis::X, e))?;
    let y = point.y.eval(env).map_err(|e| (Axis::Y, e))?;
    Ok([x, y])
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a conversion by a power of ten is exact at these magnitudes"
    )]

    use toile_doc::block::{self, FRONT};
    use toile_doc::{Binding, Command, MeasureSet, Winding};

    use super::*;
    use crate::draft::env;

    fn resolved(doc: &Doc) -> Resolved {
        let env = env::build(doc).expect("the block resolves");
        let (good, broken) = points(doc, &env);
        let key = doc.piece_named(FRONT).expect("the block draws one piece");
        let held = doc.pieces.get(key).expect("the key is live");
        piece(held, &good, &broken).expect("the block is a closed contour")
    }

    /// The length of the run from node `from` to node `to`, in centimetres.
    fn run(of: &Resolved, from: usize, to: usize) -> f64 {
        of.cum[to] - of.cum[from]
    }

    #[test]
    fn resolve_converts_centimetres_once() {
        let front = resolved(&block::trouser_front());
        assert_eq!(front.points[2].1, [25.5, 20.0]);
        assert_eq!(front.outline[2], [0.255, -0.2]);
    }

    #[test]
    fn the_y_axis_is_negated_exactly_once() {
        assert_eq!(to_metres([25.5, 20.0]), [0.255, -0.2]);
        assert_eq!(to_document(to_metres([25.5, 20.0])), [25.5, 20.0]);
        assert_eq!(to_document([0.255, -0.2]), [25.5, 20.0]);
    }

    #[test]
    fn etienne_resolves_the_side_seam_to_104_5_cm() {
        let front = resolved(&block::trouser_front());
        assert!((run(&front, 1, 4) - 104.48).abs() < 0.01);
    }

    #[test]
    fn etienne_resolves_the_inseam_to_77_2_cm() {
        let front = resolved(&block::trouser_front());
        assert!((run(&front, 5, 7) - 77.20).abs() < 0.01);
    }

    #[test]
    fn etienne_resolves_the_perimeter_to_two_and_a_half_metres() {
        let front = resolved(&block::trouser_front());
        let perimeter = front.cum[front.points.len()];
        assert!((perimeter - 255.21).abs() < 0.01, "{perimeter} cm");
    }

    #[test]
    fn changing_the_mannequin_keeps_the_node_count() {
        let mut doc = block::trouser_front();
        let before = resolved(&doc);
        let other = doc
            .mannequin_named("Talla 42")
            .expect("the block carries a second body");
        Command::ResolveWith { mannequin: other }
            .apply(&mut doc)
            .expect("the second body is live");
        let after = resolved(&doc);

        assert_eq!(after.points.len(), before.points.len());
        let keys = |of: &Resolved| of.points.iter().map(|&(key, _)| key).collect::<Vec<_>>();
        assert_eq!(keys(&after), keys(&before));
        assert_ne!(after.outline, before.outline);
        assert!((run(&before, 1, 4) - 104.48).abs() < 0.01);
        assert!((run(&after, 1, 4) - 106.63).abs() < 0.01);
    }

    #[test]
    fn a_coordinate_that_does_not_resolve_names_its_point_and_its_axis() {
        let mut doc = block::trouser_front();
        let key = doc.points.keys().next().expect("the block has points");
        doc.points.get_mut(key).expect("the key is live").y =
            Binding::parse("largo_del_brazo").expect("the source parses");
        let env = env::build(&doc).expect("the variables still resolve");
        let (good, broken) = points(&doc, &env);
        let front = doc.piece_named(FRONT).expect("the block draws one piece");
        let held = doc.pieces.get(front).expect("the key is live");
        assert_eq!(
            piece(held, &good, &broken),
            Err(vec![Defect::Binding {
                point: key,
                axis: Axis::Y,
                error: EvalError::UnknownName("largo_del_brazo".to_owned()),
            }])
        );
    }

    #[test]
    fn a_contour_that_crosses_itself_is_a_defect_not_a_mesh() {
        let mut doc = Doc::new(MeasureSet::default());
        let corners = [[0.0, 0.0], [10.0, 0.0], [0.0, 10.0], [10.0, 10.0]];
        let keys: Vec<PointKey> = corners
            .iter()
            .map(|&[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        let held = Piece::polygon("Nudo", keys, Winding::Cw);
        let env = env::build(&doc).expect("an empty document resolves");
        let (good, broken) = points(&doc, &env);
        assert!(matches!(
            piece(&held, &good, &broken).expect_err("a bowtie is not a piece")[0],
            Defect::Contour(validate::ContourFault::SelfIntersects { .. })
        ));
    }

    #[test]
    fn a_contour_that_lost_a_point_says_which_one() {
        let mut doc = Doc::new(MeasureSet::default());
        let live = doc.points.insert(Point::at(0.0, 0.0));
        let lost = doc.points.insert(Point::at(1.0, 0.0));
        let held = Piece::polygon("Roto", [live, lost], Winding::Cw);
        doc.points.remove(lost).expect("the key is live");
        let env = env::build(&doc).expect("an empty document resolves");
        let (good, broken) = points(&doc, &env);
        assert_eq!(
            piece(&held, &good, &broken),
            Err(vec![Defect::NoSuchPoint { point: lost }])
        );
    }
}
