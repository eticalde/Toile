/// The fraction of the whole perimeter at a node-local place on the contour.
///
/// `cum` is the table `length::cumulative` builds; `node` names a tract by the
/// node it leaves, and `t` runs from 0 at that node to 1 at the next. Called
/// inside every derive, and its result is never persisted: the node is the
/// address that survives an edit, the global fraction is not.
pub fn global_fraction(cum: &[f64], node: usize, t: f64) -> f64 {
    let Some(tracts) = tract_count(cum) else {
        return 0.0;
    };
    let node = node.min(tracts - 1);
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    (cum[node] + (cum[node + 1] - cum[node]) * t) / cum[tracts]
}

/// The node and local fraction a fraction of the whole perimeter lands on.
///
/// Inverse of `global_fraction`, up to the places two addresses name one
/// point: the end of one tract is the start of the next, and a fraction on a
/// zero-length tract resolves past it. What round-trips exactly is the
/// fraction itself, which is all the pairing ever reads.
pub fn local_anchor(cum: &[f64], fraction: f64) -> (usize, f64) {
    let Some(tracts) = tract_count(cum) else {
        return (0, 0.0);
    };
    if !fraction.is_finite() {
        return (0, 0.0);
    }
    let target = fraction.clamp(0.0, 1.0) * cum[tracts];
    let node = cum[..tracts]
        .partition_point(|&along| along <= target)
        .saturating_sub(1);
    let length = cum[node + 1] - cum[node];
    let t = if length > 0.0 {
        (target - cum[node]) / length
    } else {
        0.0
    };
    (node, t)
}

/// How many tracts the table describes, when it can describe any.
///
/// A table too short to hold one tract, or one measuring a perimeter of
/// nothing, anchors nothing; both come from degenerate contours the validator
/// reports elsewhere, so here they answer with the contour's start.
fn tract_count(cum: &[f64]) -> Option<usize> {
    let tracts = cum.len().checked_sub(2)? + 1;
    let perimeter = cum[tracts];
    (perimeter.is_finite() && perimeter > 0.0).then_some(tracts)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the addresses of a degenerate table are exact by definition"
    )]

    use super::*;
    use crate::length;

    /// An irregular pentagon: tracts of five different lengths.
    fn contour() -> Vec<[f64; 2]> {
        vec![[0.0, 0.0], [4.0, 0.0], [5.0, 2.0], [3.0, 6.0], [-1.0, 3.0]]
    }

    #[test]
    fn global_fraction_is_monotonic_around_the_contour() {
        let cum = length::cumulative(&contour());
        let mut walked = Vec::new();
        for node in 0..contour().len() {
            for step in 0..10 {
                walked.push(global_fraction(&cum, node, f64::from(step) / 10.0));
            }
        }
        walked.push(global_fraction(&cum, contour().len() - 1, 1.0));
        assert_eq!(walked[0], 0.0);
        assert_eq!(*walked.last().expect("the walk has steps"), 1.0);
        for pair in walked.windows(2) {
            assert!(pair[0] < pair[1], "{pair:?} went backwards");
        }
    }

    #[test]
    fn local_anchor_inverts_global_fraction_within_1e_12() {
        let cum = length::cumulative(&contour());
        for node in 0..contour().len() {
            for step in 0..=8 {
                let there = global_fraction(&cum, node, f64::from(step) / 8.0);
                let (back_node, back_t) = local_anchor(&cum, there);
                let back = global_fraction(&cum, back_node, back_t);
                assert!(
                    (there - back).abs() < 1.0e-12,
                    "node {node} step {step}: {there} came back as {back}"
                );
            }
        }
    }

    #[test]
    fn the_whole_perimeter_lands_on_the_end_of_the_last_tract() {
        let cum = length::cumulative(&contour());
        assert_eq!(local_anchor(&cum, 1.0), (contour().len() - 1, 1.0));
        assert_eq!(local_anchor(&cum, 0.0), (0, 0.0));
    }

    #[test]
    fn a_zero_length_tract_is_resolved_past_and_never_divided_by() {
        let cum = length::cumulative(&[[0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        let fraction = global_fraction(&cum, 1, 0.5);
        assert_eq!(fraction, global_fraction(&cum, 1, 0.0));
        let (node, t) = local_anchor(&cum, fraction);
        assert_eq!((node, t), (2, 0.0));
    }

    #[test]
    fn a_degenerate_table_answers_with_the_start_instead_of_panicking() {
        for table in [vec![], vec![0.0], vec![0.0, 0.0]] {
            assert_eq!(global_fraction(&table, 3, 0.5), 0.0);
            assert_eq!(local_anchor(&table, 0.5), (0, 0.0));
        }
        let cum = length::cumulative(&contour());
        assert_eq!(local_anchor(&cum, f64::NAN), (0, 0.0));
        assert_eq!(global_fraction(&cum, 0, f64::NAN), 0.0);
        assert_eq!(local_anchor(&cum, 2.0), (contour().len() - 1, 1.0));
        assert_eq!(
            global_fraction(&cum, 99, 0.0),
            global_fraction(&cum, 4, 0.0)
        );
    }
}
