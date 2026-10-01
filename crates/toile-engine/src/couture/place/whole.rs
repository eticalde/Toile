/// The ordinates every panel of a strip has cloth at.
///
/// A strip's pieces need not all reach the same height — the back of the
/// trouser block rises 25 mm above its front — and where one of them stops
/// there is no hoop the strip agrees on. Inside this range a panel's cloth has
/// one reading; outside it the surface carries the last whole hoop straight on,
/// which is what a free flap of a garment does, floored panel by panel at the
/// cloth really there so that no hoop comes out shorter than what it carries.
///
/// Letting the hoop follow the cloth that is left instead was measured: the
/// radius the cloth alone asks for falls 34.5 mm across the front's top edge —
/// 0.03567 m half a millimetre above it against 0.07015 m on it — and that is a
/// cliff no clearance table leaning one band of radius per band of height can
/// bridge. The floor makes none, because it only ever adds to a reading the
/// whole strip agreed on.
#[derive(Debug, Clone, Copy)]
pub(super) struct Whole {
    lo: f64,
    hi: f64,
}

impl Whole {
    /// The range every one of `spans` covers.
    ///
    /// Spans that share no height at all collapse to a single ordinate rather
    /// than to a range read backwards, which `f64::clamp` panics on: a strip
    /// whose pieces never overlap has one surface as much as none, and the walk
    /// that built it has already decided they belong on the same one. That
    /// collapse leaves the whole strip outside its own range, so every reading
    /// of it is the two-ordinate one, and what keeps such a hoop from coming
    /// out shorter than the cloth on it is that each panel is floored at
    /// its own cloth. No walk reaches here with none at all; with none,
    /// nothing is held.
    pub(super) fn of(spans: impl Iterator<Item = (f64, f64)>) -> Whole {
        let (lo, hi) = spans.fold((f64::MIN, f64::MAX), |(lo, hi), (low, high)| {
            (lo.max(low), hi.min(high))
        });
        let (lo, hi) = if lo <= hi { (lo, hi) } else { (lo, lo) };
        Whole { lo, hi }
    }

    /// `y` held inside the range.
    pub(super) fn at(&self, y: f64) -> f64 {
        y.clamp(self.lo, self.hi)
    }

    /// Whether the strip is whole at `y`, so that holding it changes nothing.
    ///
    /// The cheap half of the pair: inside the range a panel's cloth has one
    /// reading and outside it two, and the surface is read once per released
    /// vertex per round of the clearance walk.
    pub(super) fn holds(&self, y: f64) -> bool {
        y >= self.lo && y <= self.hi
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a clamp hands back one of the three ordinates it was given"
    )]

    use super::*;

    /// The range is the overlap, so a piece that stops early stops the strip.
    #[test]
    fn the_strip_is_whole_where_all_of_its_pieces_are() {
        let whole = Whole::of([(-1.04, 0.0), (-1.04, 0.025)].into_iter());
        assert_eq!(
            whole.at(0.025),
            0.0,
            "above the front's top it reads its top"
        );
        assert_eq!(whole.at(0.0), 0.0);
        assert_eq!(whole.at(-0.50), -0.50, "and inside the range nothing moves");
        assert_eq!(whole.at(-2.0), -1.04, "below it, the hem");
        assert!(
            whole.holds(-0.50),
            "and it says where holding changes nothing"
        );
        assert!(whole.holds(0.0), "its own top edge included");
        assert!(
            !whole.holds(0.025),
            "and where it does not, it says that too"
        );
    }

    /// Pieces that share no height at all name one ordinate, not a range read
    /// backwards, which `f64::clamp` would panic on.
    #[test]
    fn pieces_that_never_meet_collapse_to_one_ordinate() {
        let whole = Whole::of([(0.0, 0.10), (0.50, 0.60)].into_iter());
        assert_eq!(whole.at(0.0), 0.50);
        assert_eq!(whole.at(10.0), 0.50);
        // And nowhere but that one ordinate is whole, so such a strip is read
        // panel by panel everywhere: it is the arrangement the floor is for.
        assert!(whole.holds(0.50));
        assert!(!whole.holds(0.0) && !whole.holds(10.0));
        let none = Whole::of(std::iter::empty());
        assert_eq!(none.at(0.0), 0.0, "and with no pieces nothing is held");
    }
}
