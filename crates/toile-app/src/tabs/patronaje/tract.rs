use toile_engine::draft::{Draft, PieceKey, PointKey};

use super::pick::{Nearest, away, nearest_on};

/// One tract of a contour as the pointer meets it.
///
/// A straight tract is its two ends; a bent one carries the flattening the
/// document asked for. The pointer therefore catches the line the drawing
/// paints rather than the chord underneath it, which on a hip curve runs half
/// a centimetre away from where the line looks.
#[derive(Debug, Clone, PartialEq)]
pub struct Tract {
    /// The node the tract leaves.
    pub node: PointKey,
    /// The node it runs to.
    pub to: PointKey,
    /// The line itself, node to node, in centimetres.
    pub line: Vec<[f64; 2]>,
}

/// Every tract of a piece, in contour order.
///
/// The flattening comes from the draft, so nothing is evaluated twice and the
/// samples the document asked for are the samples the pointer walks.
pub fn of(draft: &Draft, piece: PieceKey) -> Vec<Tract> {
    let nodes = draft.points_cm(piece);
    let flat = draft.flat_cm(piece);
    let starts = draft.flat_starts(piece);
    if nodes.len() < 2 || starts.len() != nodes.len() {
        return Vec::new();
    }
    (0..nodes.len())
        .map(|index| {
            let next = (index + 1) % nodes.len();
            let head = starts[index];
            let tail = if next == 0 { flat.len() } else { starts[next] };
            let mut line = flat.get(head..tail).unwrap_or_default().to_vec();
            line.push(nodes[next].1);
            Tract {
                node: nodes[index].0,
                to: nodes[next].0,
                line,
            }
        })
        .collect()
}

/// The straight tracts of a closed polygon of nodes.
#[cfg(test)]
pub fn straight(nodes: &[(PointKey, [f64; 2])]) -> Vec<Tract> {
    (0..nodes.len())
        .map(|index| {
            let next = (index + 1) % nodes.len();
            Tract {
                node: nodes[index].0,
                to: nodes[next].0,
                line: vec![nodes[index].1, nodes[next].1],
            }
        })
        .collect()
}

/// The place on any tract nearest `at`.
///
/// The tracts that touch anything in `held` are left out: a node cannot be
/// placed against the very lines it is dragging along with it.
pub fn nearest(at: [f64; 2], tracts: &[Tract], held: &[PointKey]) -> Option<Nearest> {
    let mut best: Option<Nearest> = None;
    for (index, tract) in tracts.iter().enumerate() {
        if held.contains(&tract.node) || held.contains(&tract.to) {
            continue;
        }
        let Some(found) = on(tract, at, index) else {
            continue;
        };
        if best.is_none_or(|kept| found.away < kept.away) {
            best = Some(found);
        }
    }
    best
}

/// The nearest place on one tract, with the fraction measured along it.
///
/// The fraction is arc length along the drawn line and not along one of its
/// sub-segments, so it means the same thing on a bent tract as on a straight
/// one: how far into the tract the pointer landed.
fn on(tract: &Tract, q: [f64; 2], from: usize) -> Option<Nearest> {
    let mut best: Option<(Nearest, f64)> = None;
    let mut along = 0.0;
    for pair in tract.line.windows(2) {
        let span = away(pair[0], pair[1]);
        let found = nearest_on(pair[0], pair[1], q, from);
        if best.is_none_or(|(kept, _)| found.away < kept.away) {
            best = Some((found, along + found.t * span));
        }
        along += span;
    }
    let (found, arc) = best?;
    Some(Nearest {
        t: if along > 0.0 { arc / along } else { 0.0 },
        ..found
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a tract opens on the very coordinates its node resolved to"
    )]

    use toile_engine::draft::block;

    use super::*;

    /// A square, in contour order, with a node key per corner.
    fn square() -> Vec<(PointKey, [f64; 2])> {
        [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
            .into_iter()
            .enumerate()
            .map(|(i, at)| (PointKey::new(i as u32, 0), at))
            .collect()
    }

    /// The block on the table, and the tracts its contour draws.
    fn front() -> (Draft, PieceKey) {
        let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
        let piece = draft
            .doc()
            .piece_named(block::FRONT)
            .expect("the block draws one piece");
        (draft, piece)
    }

    #[test]
    fn a_hit_reports_its_fraction_along_the_tract() {
        let nodes = square();
        let found = nearest([2.5, 0.3], &straight(&nodes), &[]).expect("the square has tracts");
        assert_eq!(found.from, 0);
        assert!((found.t - 0.25).abs() < 1.0e-9, "{found:?}");
        assert!((found.away - 0.3).abs() < 1.0e-9, "{found:?}");
    }

    #[test]
    fn the_node_in_hand_hides_the_tracts_that_hang_from_it() {
        let nodes = square();
        let tracts = straight(&nodes);
        let found = nearest([0.2, 0.2], &tracts, &[nodes[0].0]).expect("two tracts are left");
        assert!(found.from == 1 || found.from == 2, "{found:?}");
    }

    #[test]
    fn every_node_of_the_block_opens_a_tract_of_its_own() {
        let (draft, piece) = front();
        let tracts = of(&draft, piece);
        assert_eq!(tracts.len(), draft.points_cm(piece).len());
        let ends: usize = tracts.iter().map(|tract| tract.line.len()).sum();
        // Every tract carries its own nodes, so the two ends of each are
        // counted once more than the flattening holds them.
        assert_eq!(ends, draft.flat_cm(piece).len() + tracts.len());
        for (index, tract) in tracts.iter().enumerate() {
            assert_eq!(tract.line[0], draft.points_cm(piece)[index].1);
        }
    }

    #[test]
    fn a_bent_tract_is_caught_where_it_is_drawn_and_not_on_its_chord() {
        let (draft, piece) = front();
        let tracts = of(&draft, piece);
        // A quarter of the way along the hip: on the curve, and well off the
        // straight line between the waist and the hip point. Halfway would
        // not do — the tract is symmetric, so its middle sits on its chord.
        let hip = &tracts[1];
        let quarter = hip.line[hip.line.len() / 4];
        let found = nearest(quarter, &tracts, &[]).expect("the block has tracts");
        assert_eq!(found.from, 1);
        assert!(found.away < 1.0e-9, "{found:?}");
        // Nearly five millimetres off the chord, which at true scale is more
        // than twice the budget the pointer picks a tract within: catching by
        // the chord would have missed the line the drawing paints.
        let chord = nearest_on(hip.line[0], *hip.line.last().expect("two ends"), quarter, 1);
        assert!(chord.away > 0.4, "{chord:?}");
    }

    #[test]
    fn a_piece_with_no_flattening_draws_no_tracts() {
        let (draft, _) = front();
        assert!(of(&draft, PieceKey::new(9, 0)).is_empty());
    }
}
