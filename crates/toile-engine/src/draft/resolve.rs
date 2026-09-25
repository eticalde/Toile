use std::collections::BTreeMap;

use toile_doc::formula::EvalError;
use toile_doc::{Axis, Doc, EdgeAnchor, EdgeRange, Piece, Point, PointKey};
use toile_geom::{length, validate};

use super::defect::Defect;
use super::env::Env;
use super::resolved::Resolved;
use super::{contour, fold};

/// Centimetres in a metre. The document counts in the first, the solver in the
/// second, and this is the only place the two meet.
const CM_PER_M: f64 = 100.0;

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

/// One piece as a closed contour, flattened, with the cloth it is cut from.
///
/// The arc lengths are measured along the flattened line rather than between
/// nodes, so a curved tract is as long as the cloth it will need.
///
/// `fold` is the axis the piece is drawn against, when it is drawn against one.
/// This is the one place in the program that reads it: the unfold happens here
/// and hangs on the result, so nothing downstream asks whether a piece was
/// drawn whole or drawn half.
///
/// # Errors
/// Every defect the piece carries: one per coordinate that does not resolve,
/// handles included; when they all do, the fault that stops the flattened
/// contour from being a simple closed polygon; and when it folds, an axis with
/// nothing to mirror or a cloth that lies over itself.
pub fn piece(
    held: &Piece,
    fold: Option<EdgeRange>,
    good: &BTreeMap<PointKey, [f64; 2]>,
    broken: &BTreeMap<PointKey, (Axis, EvalError)>,
) -> Result<Resolved, Vec<Defect>> {
    let tracts = contour::tracts(held, good, broken)?;
    let points: Vec<(PointKey, [f64; 2])> =
        tracts.iter().map(|one| (one.node, one.start)).collect();
    let (flat_cm, starts) = contour::flatten(&tracts);
    let drawn_m: Vec<[f64; 2]> = flat_cm.iter().map(|&p| to_metres(p)).collect();
    validate::check_closed(&drawn_m).map_err(|fault| vec![Defect::Contour(fault)])?;
    let along = length::cumulative(&flat_cm);
    let cum: Vec<f64> = starts
        .iter()
        .map(|&start| along[start])
        .chain(along.last().copied())
        .collect();
    let cloth = match fold {
        None => None,
        Some(axis) => {
            let at = |anchor: &EdgeAnchor| arc(&points, &cum, anchor);
            let (Some(head), Some(tail)) = (at(&axis.head), at(&axis.tail)) else {
                return Err(vec![Defect::FoldAxis]);
            };
            Some(fold::unfold(&flat_cm, &along, head, tail).map_err(|fault| vec![fault])?)
        }
    };
    Ok(Resolved {
        points,
        flat_cm,
        drawn_m,
        starts,
        cum,
        cloth,
    })
}

/// How far along the drawn contour one place on it falls, in centimetres.
///
/// `None` for a node the contour does not run through, which is what a place
/// nobody can point at is worth.
pub fn arc(points: &[(PointKey, [f64; 2])], cum: &[f64], at: &EdgeAnchor) -> Option<f64> {
    let k = points.iter().position(|&(key, _)| key == at.from)?;
    let (from, to) = (*cum.get(k)?, *cum.get(k + 1)?);
    Some(from + (to - from) * at.t)
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
        piece(held, None, &good, &broken).expect("the block is a closed contour")
    }

    /// The length of the run from node `from` to node `to`, in centimetres.
    fn run(of: &Resolved, from: usize, to: usize) -> f64 {
        of.cum[to] - of.cum[from]
    }

    #[test]
    fn resolve_converts_centimetres_once() {
        let front = resolved(&block::trouser_front());
        assert_eq!(front.points[2].1, [25.5, 20.0]);
        // The waist opens the hip curve, so it is a node and the first sample
        // of its own tract at once.
        assert_eq!(front.flat_cm[1], [22.0, 0.0]);
        assert_eq!(front.drawn_m[1], [0.22, -0.0]);
    }

    #[test]
    fn the_two_bent_tracts_are_the_whole_difference_in_the_flattening() {
        let front = resolved(&block::trouser_front());
        // Seven straight tracts give a point each; the hip gives twenty-four
        // and the crotch sixteen.
        assert_eq!(front.points.len(), 9);
        assert_eq!(front.flat_cm.len(), 7 + 24 + 16);
        assert_eq!(front.cum.len(), front.points.len() + 1);
        // Each node opens its own tract, and the two bent ones are the only
        // places the flattening runs on past a single point.
        assert_eq!(front.starts, [0, 1, 25, 26, 27, 28, 29, 30, 46]);
    }

    #[test]
    fn the_y_axis_is_negated_exactly_once() {
        assert_eq!(to_metres([25.5, 20.0]), [0.255, -0.2]);
        assert_eq!(to_document(to_metres([25.5, 20.0])), [25.5, 20.0]);
        assert_eq!(to_document([0.255, -0.2]), [25.5, 20.0]);
    }

    #[test]
    fn etienne_resolves_the_side_seam_to_104_6_cm() {
        let front = resolved(&block::trouser_front());
        assert!((run(&front, 1, 4) - 104.60).abs() < 0.01);
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
        assert!((perimeter - 256.16).abs() < 0.01, "{perimeter} cm");
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
        assert_ne!(after.drawn_m, before.drawn_m);
        assert!((run(&before, 1, 4) - 104.60).abs() < 0.01);
        assert!((run(&after, 1, 4) - 106.79).abs() < 0.01);
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
            piece(held, None, &good, &broken),
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
            piece(&held, None, &good, &broken).expect_err("a bowtie is not a piece")[0],
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
            piece(&held, None, &good, &broken),
            Err(vec![Defect::NoSuchPoint { point: lost }])
        );
    }
}
