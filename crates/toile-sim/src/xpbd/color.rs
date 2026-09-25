use super::bare::Dropped;
use super::state::DistanceConstraints;

/// Constraints reordered so that no two in the same colour share a vertex.
///
/// Within a colour the writes are disjoint, so a colour can be solved with no
/// atomics and no scheduler-dependent reductions — which is what makes the
/// coloured paths give the same bits on one thread or eight.
///
/// The fields are private so that [`color_constraints`] is the only way to
/// obtain one: it is where a set carrying a pass the coloured paths lack is
/// turned away, and a hand-built value would walk round that.
#[derive(Debug)]
pub struct ColoredConstraints {
    pub(super) cons: DistanceConstraints,
    pub(super) ranges: Vec<std::ops::Range<usize>>,
}

impl ColoredConstraints {
    /// How many colours the constraint graph needed.
    pub fn colors(&self) -> usize {
        self.ranges.len()
    }
}

/// Greedy colouring in the constraints' canonical input order.
///
/// Deterministic by construction: first free colour on both endpoints, chosen
/// from a `u64` bitmask. A cloth mesh needs around fifteen colours.
///
/// # Errors
/// [`Dropped`] when the set carries a pass the coloured paths do not run.
/// Refused here rather than at the substep because the colouring is what an
/// elastic cannot survive: it permutes the edges, so an index into the set
/// handed in names another edge in the set handed back.
///
/// # Panics
/// If the constraint graph needs more than 64 colours.
pub fn color_constraints(
    cons: &DistanceConstraints,
    n_verts: usize,
) -> Result<ColoredConstraints, Dropped> {
    // Taken apart field by field for the reason `Bare::of` is: a fifth thing
    // asked of the constraint set has to be answered for on this line, where
    // reading through `cons.` would let the coloured paths drop it in silence.
    let DistanceConstraints {
        a: _,
        b: _,
        rest: _,
        compliance: _,
        strain_limit,
        strain_sweeps: _,
        held,
        held_passes: _,
    } = cons;
    if *strain_limit > 0.0 {
        return Err(Dropped::StrainLimit);
    }
    if !held.is_empty() {
        return Err(Dropped::Held);
    }
    let m = cons.len();
    let mut used: Vec<u64> = vec![0; n_verts];
    let mut color_of = vec![0u32; m];
    let mut n_colors = 0usize;
    for ((&a, &b), color) in cons.a.iter().zip(&cons.b).zip(color_of.iter_mut()) {
        let (a, b) = (a as usize, b as usize);
        let col = (!(used[a] | used[b])).trailing_zeros() as usize;
        assert!(col < 64, "constraint graph needs more than 64 colours");
        used[a] |= 1 << col;
        used[b] |= 1 << col;
        *color = col as u32;
        n_colors = n_colors.max(col + 1);
    }

    // Within a colour, order by lowest vertex: colouring destroys cache
    // locality and this wins it back. The key is a total order — lowest
    // vertex, then original index — so the result stays deterministic.
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); n_colors];
    for (i, &c) in color_of.iter().enumerate() {
        groups[c as usize].push(i);
    }
    for g in &mut groups {
        g.sort_by_key(|&i| (cons.a[i].min(cons.b[i]), i));
    }

    // Everything beyond the four arrays was refused above, so the set the
    // coloured paths get carries none of it rather than a field nothing reads.
    // Written out rather than filled from the default, so that a new field
    // cannot arrive here defaulted away without anyone deciding it should be.
    let mut cc = DistanceConstraints {
        a: Vec::with_capacity(m),
        b: Vec::with_capacity(m),
        rest: Vec::with_capacity(m),
        compliance: Vec::with_capacity(m),
        strain_limit: 0.0,
        strain_sweeps: 0,
        held: Vec::new(),
        held_passes: 0,
    };
    let mut ranges = Vec::with_capacity(n_colors);
    for g in &groups {
        let start = cc.a.len();
        for &i in g {
            cc.a.push(cons.a[i]);
            cc.b.push(cons.b[i]);
            cc.rest.push(cons.rest[i]);
            cc.compliance.push(cons.compliance[i]);
        }
        ranges.push(start..cc.a.len());
    }
    Ok(ColoredConstraints { cons: cc, ranges })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path graph: 0–1–2–3–4.
    fn path(n: u32) -> DistanceConstraints {
        DistanceConstraints {
            a: (0..n - 1).collect(),
            b: (1..n).collect(),
            rest: vec![1.0; (n - 1) as usize],
            compliance: vec![0.0; (n - 1) as usize],
            ..DistanceConstraints::default()
        }
    }

    /// The colouring of a path graph, which carries nothing that is refused.
    fn colored(cons: &DistanceConstraints) -> ColoredConstraints {
        color_constraints(cons, 64).expect("a path graph is plain stretch")
    }

    #[test]
    fn no_colour_repeats_a_vertex() {
        let cons = path(64);
        let cc = colored(&cons);
        for r in &cc.ranges {
            let mut seen = std::collections::HashSet::new();
            for c in r.clone() {
                assert!(seen.insert(cc.cons.a[c]), "vertex twice in one colour");
                assert!(seen.insert(cc.cons.b[c]), "vertex twice in one colour");
            }
        }
    }

    #[test]
    fn colouring_preserves_every_constraint() {
        let cons = path(64);
        let cc = colored(&cons);
        assert_eq!(cc.cons.len(), cons.len());
        assert_eq!(
            cc.ranges.iter().map(ExactSizeIterator::len).sum::<usize>(),
            cons.len()
        );
    }

    #[test]
    fn a_path_needs_two_colours() {
        assert_eq!(colored(&path(64)).colors(), 2);
    }

    #[test]
    fn colouring_is_reproducible() {
        let cons = path(64);
        let (x, y) = (colored(&cons), colored(&cons));
        assert_eq!(x.cons.a, y.cons.a);
        assert_eq!(x.cons.b, y.cons.b);
        assert_eq!(x.ranges, y.ranges);
    }
}
