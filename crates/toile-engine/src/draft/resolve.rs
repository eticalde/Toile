use std::collections::BTreeMap;

use toile_doc::formula::EvalError;
use toile_doc::{Axis, Doc, Piece, Point, PointKey};
use toile_geom::{length, validate};

use super::contour;
use super::defect::Defect;
use super::env::Env;

/// Centimetres in a metre. The document counts in the first, the solver in the
/// second, and this is the only place the two meet.
const CM_PER_M: f64 = 100.0;

/// A piece as the rest of the program sees it: its nodes, the line they draw
/// once the curves are flattened, and that same line in the solver's units.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Resolved {
    /// The contour nodes in centimetres, y downward, in contour order.
    pub points: Vec<(PointKey, [f64; 2])>,
    /// The whole contour flattened, in centimetres with y downward: the line
    /// the table draws, curves and all.
    pub flat: Vec<[f64; 2]>,
    /// The same flattened contour in metres, y upward: what the mesher takes.
    pub outline: Vec<[f64; 2]>,
    /// Where each node opens in the flattening, in contour order.
    pub starts: Vec<usize>,
    /// Flattened arc length in centimetres up to each node, and round to the
    /// first, so the last entry is the perimeter.
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

/// One piece as a closed contour, flattened, in both units.
///
/// The arc lengths are measured along the flattened line rather than between
/// nodes, so a curved tract is as long as the cloth it will need.
///
/// # Errors
/// Every defect the piece carries: one per coordinate that does not resolve,
/// handles included, or, when they all do, the fault that stops the flattened
/// contour from being a simple closed polygon.
pub fn piece(
    held: &Piece,
    good: &BTreeMap<PointKey, [f64; 2]>,
    broken: &BTreeMap<PointKey, (Axis, EvalError)>,
) -> Result<Resolved, Vec<Defect>> {
    let tracts = contour::tracts(held, good, broken)?;
    let points = tracts.iter().map(|one| (one.node, one.start)).collect();
    let (flat, starts) = contour::flatten(&tracts);
    let outline: Vec<[f64; 2]> = flat.iter().map(|&p| to_metres(p)).collect();
    validate::check_closed(&outline).map_err(|fault| vec![Defect::Contour(fault)])?;
    let along = length::cumulative(&flat);
    let cum = starts
        .iter()
        .map(|&start| along[start])
        .chain(along.last().copied())
        .collect();
    Ok(Resolved {
        points,
        flat,
        outline,
        starts,
        cum,
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
        // The waist opens the hip curve, so it is a node and the first sample
        // of its own tract at once.
        assert_eq!(front.flat[1], [22.0, 0.0]);
        assert_eq!(front.outline[1], [0.22, -0.0]);
    }

    #[test]
    fn the_two_bent_tracts_are_the_whole_difference_in_the_flattening() {
        let front = resolved(&block::trouser_front());
        // Seven straight tracts give a point each; the hip gives twenty-four
        // and the crotch sixteen.
        assert_eq!(front.points.len(), 9);
        assert_eq!(front.flat.len(), 7 + 24 + 16);
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
        assert_ne!(after.outline, before.outline);
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
