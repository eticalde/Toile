use std::collections::BTreeMap;

use toile_doc::formula::EvalError;
use toile_doc::{Axis, ContourNode, Piece, PointKey};
use toile_geom::curve;

use super::defect::Defect;

/// One tract of a contour with everything it needs already resolved.
///
/// A handle is a point of the document like any other, so by the time a tract
/// reaches here its handles are two coordinates and nothing more: flattening
/// looks up no key and cannot fail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tract {
    /// The node the tract leaves.
    pub node: PointKey,
    /// Where that node sits, in centimetres with y downward.
    pub start: [f64; 2],
    /// The two handles, when the tract bends.
    pub handles: Option<([f64; 2], [f64; 2])>,
    /// How many points the tract contributes to the flattened contour.
    pub samples: u16,
}

/// Every tract of a piece, with its node and its handles resolved.
///
/// # Errors
/// One defect per coordinate that does not resolve, handles included: a
/// tangent written as a formula that stops resolving costs the piece exactly
/// what a node that stops resolving costs it.
pub fn tracts(
    held: &Piece,
    good: &BTreeMap<PointKey, [f64; 2]>,
    broken: &BTreeMap<PointKey, (Axis, EvalError)>,
) -> Result<Vec<Tract>, Vec<Defect>> {
    let mut resolved = Vec::with_capacity(held.contour.len());
    let mut defects = Vec::new();
    for node in &held.contour {
        match tract(node, good, broken) {
            Ok(one) => resolved.push(one),
            Err(mut faults) => defects.append(&mut faults),
        }
    }
    if defects.is_empty() {
        Ok(resolved)
    } else {
        Err(defects)
    }
}

/// The closed polyline the tracts flatten to, and where each node lands in it.
///
/// Every tract gives its own node first and stops short of the next one, so
/// the polyline closes on itself without a doubled point. A straight tract
/// gives its node and nothing else whatever its sample count says:
/// subdividing a straight line moves no cloth and buys only triangles.
///
/// The second vector holds each node's index into the polyline, which is what
/// turns a node into an arc length without measuring the contour twice.
pub fn flatten(tracts: &[Tract]) -> (Vec<[f64; 2]>, Vec<usize>) {
    let mut flat = Vec::with_capacity(tracts.len());
    let mut starts = Vec::with_capacity(tracts.len());
    for (index, tract) in tracts.iter().enumerate() {
        starts.push(flat.len());
        let Some((out, into)) = tract.handles else {
            flat.push(tract.start);
            continue;
        };
        let end = tracts[(index + 1) % tracts.len()].start;
        flat.extend(curve::flatten(tract.start, out, into, end, tract.samples));
    }
    (flat, starts)
}

/// One tract resolved, or everything that stops it resolving.
fn tract(
    node: &ContourNode,
    good: &BTreeMap<PointKey, [f64; 2]>,
    broken: &BTreeMap<PointKey, (Axis, EvalError)>,
) -> Result<Tract, Vec<Defect>> {
    let mut defects = Vec::new();
    let mut resolve = |key: PointKey| match at(key, good, broken) {
        Ok(cm) => Some(cm),
        Err(defect) => {
            defects.push(defect);
            None
        }
    };
    let start = resolve(node.point);
    let bend = node
        .segment
        .handles()
        .map(|(out, into)| (resolve(out), resolve(into)));
    let handles = match bend {
        None => None,
        Some((Some(out), Some(into))) => Some((out, into)),
        Some(_) => return Err(defects),
    };
    match start {
        Some(start) => Ok(Tract {
            node: node.point,
            start,
            handles,
            samples: node.samples,
        }),
        None => Err(defects),
    }
}

/// Where one point of a contour resolved to, or what is wrong with it.
fn at(
    point: PointKey,
    good: &BTreeMap<PointKey, [f64; 2]>,
    broken: &BTreeMap<PointKey, (Axis, EvalError)>,
) -> Result<[f64; 2], Defect> {
    if let Some((axis, error)) = broken.get(&point) {
        return Err(Defect::Binding {
            point,
            axis: *axis,
            error: error.clone(),
        });
    }
    good.get(&point)
        .copied()
        .ok_or(Defect::NoSuchPoint { point })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a node lands exactly where the document put it"
    )]

    use toile_doc::{Segment, Winding};

    use super::*;

    /// The four corners of a hundred-centimetre square, in contour order.
    const SQUARE: [[f64; 2]; 4] = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

    /// The square as a piece, and where its points resolve to.
    ///
    /// Keys 0 to 3 are the corners; keys 4 and 5 are handles bowing the tract
    /// that leaves corner 1 out to the right.
    fn square() -> (Piece, BTreeMap<PointKey, [f64; 2]>) {
        let keys: Vec<PointKey> = (0..4).map(|i| PointKey::new(i, 0)).collect();
        let mut good: BTreeMap<PointKey, [f64; 2]> = keys.iter().copied().zip(SQUARE).collect();
        good.insert(PointKey::new(4, 0), [130.0, 33.0]);
        good.insert(PointKey::new(5, 0), [130.0, 67.0]);
        (Piece::polygon("Delantero", keys, Winding::Cw), good)
    }

    /// The square with the tract leaving corner 1 bowed, at `samples` points.
    fn bowed(samples: u16) -> (Piece, BTreeMap<PointKey, [f64; 2]>) {
        let (mut piece, good) = square();
        piece.contour[1].segment = Segment::Cubic {
            out: PointKey::new(4, 0),
            into: PointKey::new(5, 0),
        };
        piece.contour[1].samples = samples;
        (piece, good)
    }

    #[test]
    fn a_straight_contour_flattens_to_its_own_nodes() {
        let (piece, good) = square();
        let tracts = tracts(&piece, &good, &BTreeMap::new()).expect("the square resolves");
        let (flat, starts) = flatten(&tracts);
        assert_eq!(flat, SQUARE);
        assert_eq!(starts, [0, 1, 2, 3]);
    }

    #[test]
    fn a_curved_tract_contributes_exactly_its_sample_count() {
        let (piece, good) = bowed(8);
        let tracts = tracts(&piece, &good, &BTreeMap::new()).expect("the square resolves");
        let (flat, starts) = flatten(&tracts);
        assert_eq!(flat.len(), 3 + 8);
        assert_eq!(starts, [0, 1, 9, 10]);
        assert_eq!(flat[1], SQUARE[1], "the tract opens on its own node");
        assert_eq!(flat[9], SQUARE[2], "and stops short of the next one");
        assert!(flat[5][0] > 100.0, "the curve bows out past the chord");
    }

    #[test]
    fn moving_a_handle_leaves_the_point_count_where_it_was() {
        let (piece, good) = bowed(24);
        let count = |good: &BTreeMap<PointKey, [f64; 2]>| {
            let tracts = tracts(&piece, good, &BTreeMap::new()).expect("the square resolves");
            flatten(&tracts).0.len()
        };
        let before = count(&good);
        let mut pulled = good.clone();
        pulled.insert(PointKey::new(4, 0), [190.0, 10.0]);
        assert_eq!(count(&pulled), before);
    }

    #[test]
    fn a_handle_that_does_not_resolve_is_a_defect_of_the_piece() {
        let (piece, good) = bowed(8);
        let handle = PointKey::new(5, 0);
        let broken = BTreeMap::from([(
            handle,
            (
                Axis::Y,
                EvalError::UnknownName("largo_del_brazo".to_owned()),
            ),
        )]);
        assert_eq!(
            tracts(&piece, &good, &broken),
            Err(vec![Defect::Binding {
                point: handle,
                axis: Axis::Y,
                error: EvalError::UnknownName("largo_del_brazo".to_owned()),
            }])
        );
    }

    #[test]
    fn a_handle_the_document_lost_names_itself() {
        let (piece, mut good) = bowed(8);
        let handle = PointKey::new(4, 0);
        good.remove(&handle);
        assert_eq!(
            tracts(&piece, &good, &BTreeMap::new()),
            Err(vec![Defect::NoSuchPoint { point: handle }])
        );
    }

    #[test]
    fn a_contour_with_no_tracts_flattens_to_nothing() {
        assert_eq!(flatten(&[]), (Vec::new(), Vec::new()));
    }
}
