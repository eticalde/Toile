use std::f64::consts::TAU;

use heading::{into, sense_of};

use super::super::pipeline::ShapePipeline;
use super::profile::Profile;
use super::room::Room;
use super::whole::Whole;

mod heading;

pub use heading::Pin;

/// One piece as the strip crosses it: which way round it runs, and how wide
/// its cloth is at every ordinate.
#[derive(Debug, Clone)]
struct Panel {
    piece: usize,
    sense: f64,
    cloth: Profile,
}

/// The surface a sewn product is rolled onto, and where the strip lies on it.
///
/// A surface of revolution about the body's own vertical axis, whose hoop at
/// each pattern ordinate is the cloth the strip carries there — plus whatever
/// room the body underneath asked for. So a waistband of 61 cm lands on a 61 cm
/// hoop even though the skirt below it carries 117, which one radius for the
/// whole garment cannot do: it has to serve both, and serving both means
/// serving neither.
///
/// Two readings and not one, because a strip's pieces need not all reach the
/// same height. [`Round::girth`] is the cloth that is really at an ordinate;
/// the hoop is read inside the strip's whole range, so where only some of the
/// pieces are left the surface continues straight instead of closing on what
/// remains — floored, panel by panel, at the cloth that is there, because a
/// hoop shorter than its cloth has the pieces lying inside each other.
#[derive(Debug, Clone)]
pub struct Round {
    /// The strip, in the order the seams walk it.
    panels: Vec<Panel>,
    /// Whether the strip closes into a tube.
    closed: bool,
    /// The line of cloth a declared heading fixes, once the walk that carries
    /// it has been found; `None` for a product that declares none.
    ///
    /// Its turn is added as an angle and after the division by the hoop, which
    /// is why the surface survives a heading at all: the whole of it rotates
    /// rigidly and every meridian stays one. Added to the walked cloth instead,
    /// the same declaration would come to a different angle on every hoop —
    /// 1.0400 m of cloth at the chest against 1.2570 m at the hip — and the
    /// centre front would spiral down the body.
    pin: Option<Pin>,
    /// Which way the turn grows as the walk goes on: `1.0` with it, `-1.0`
    /// against it.
    ///
    /// `1.0` with nothing declared, which is every release there was before a
    /// heading could be written. It is the pinned piece's own declaration read
    /// against the direction the seams walk that piece, because those are two
    /// different facts: a run declared leftward on a panel the walk crosses
    /// backwards is a strip that has to go round the other way.
    winds: f64,
    room: Room,
    whole: Whole,
}

impl Round {
    /// Reads the pieces of a walked strip into the surface they roll onto.
    ///
    /// `order` is the walk: which piece, and `1.0` or `-1.0` for which way its
    /// rising abscissa runs. `band` is how tall one band of the clearance table
    /// is. `pin` is the line a declared heading fixes, or `None`.
    ///
    /// `None` when the walk is empty or a piece of it is too degenerate to have
    /// a width, which is the same answer the strip walk itself gives: the
    /// placement those want is not a turn around one axis.
    pub(crate) fn over(
        order: &[(usize, f64)],
        pipes: &[&ShapePipeline],
        closed: bool,
        band: f64,
        pin: Option<Pin>,
    ) -> Option<Round> {
        let mut panels: Vec<Panel> = Vec::with_capacity(order.len());
        for &(piece, sense) in order {
            panels.push(Panel {
                piece,
                sense,
                cloth: Profile::of(pipes.get(piece)?)?,
            });
        }
        panels.first()?;
        // A pin on a piece this walk does not carry is dropped here and not
        // taken into `at`, where it would match no panel: the turn would then
        // open at a cloth of zero and the garment come out turned by whatever
        // the storage order gave it — 44.6° measured, in silence — or, with no
        // zero to be found at all, go to the flat release a metre overhead.
        let pin = pin.filter(|pin| panels.iter().any(|panel| panel.piece == pin.piece));
        let winds = pin.map_or(1.0, |pin| pin.leftward * sense_of(&panels, pin.piece));
        let room = Room::over(spanned(&panels), band);
        let whole = Whole::of(panels.iter().map(|panel| panel.cloth.span()));
        Some(Round {
            panels,
            closed,
            pin,
            winds,
            room,
            whole,
        })
    }

    /// How much cloth the product really carries round at one pattern ordinate,
    /// in metres.
    ///
    /// A piece that does not reach that high contributes nothing, so this reads
    /// the trouser back's top 25 mm as the back's own cloth. It is not the hoop
    /// that cloth lands on — see [`Round::radius`] — and asking it for one was
    /// how a hoop came to be sized from a width that is not there.
    ///
    /// Nor can the hoop come out under it, and not because anything compares
    /// the two: [`Round::reach`] floors every panel at its own cloth, so the
    /// hoop is a sum of terms each of which is one of this sum's terms or
    /// larger.
    pub fn girth(&self, y: f64) -> f64 {
        self.panels.iter().map(|panel| panel.cloth.carries(y)).sum()
    }

    /// How far from the axis the cloth is rolled at one pattern ordinate, in
    /// metres.
    pub fn radius(&self, y: f64) -> f64 {
        self.hoop(y) / TAU + self.room.at(y)
    }

    /// How far round the surface goes at one pattern ordinate, before the room
    /// the body asked for.
    ///
    /// Panel by panel, and not one `max` of the two whole sums. The arc a point
    /// gets is its own panel's share of this, so a hoop long enough in total
    /// while one panel is read short gives that panel an arc shorter than its
    /// cloth and hands the difference to its neighbours — which is the very
    /// overlap a floor is for.
    fn hoop(&self, y: f64) -> f64 {
        self.panels
            .iter()
            .map(|panel| {
                let (lo, hi) = self.reach(panel, y);
                hi - lo
            })
            .sum()
    }

    /// The abscissae one panel is given at an ordinate: as wide as the wider of
    /// two readings, and slid over the cloth that is really there.
    ///
    /// Inside the strip's whole range the two ordinates are one and this is a
    /// single reading; outside it both answers are owed, for the reason
    /// [`Whole`] carries.
    ///
    /// Two invariants, arithmetic rather than asserted. The width is the larger
    /// of the two readings, so the hoop is a sum of terms each at least as long
    /// as [`Round::girth`]'s. And the span covers the cloth at `y`, so the arc
    /// [`Round::at`] measures a vertex into is an arc that vertex is on: a
    /// piece whose cloth stands further across the pattern the higher it is
    /// read — any slanted panel — has edges the held reading does not
    /// reach, and measured from those the cloth walks off its own arc onto
    /// its neighbour's.
    fn reach(&self, panel: &Panel, y: f64) -> (f64, f64) {
        let held = panel.cloth.extent(self.whole.at(y));
        if self.whole.holds(y) {
            return held;
        }
        // Outside its own span a piece carries nothing, and `extent` there
        // hands back the very edge `held` already is, because the strip is
        // whole no further than that piece reaches. So the two readings agree
        // exactly where the cloth is absent, and the floor is the cloth.
        let here = panel.cloth.extent(y);
        let wide = (held.1 - held.0).max(here.1 - here.0);
        // Slid the least it takes to cover the cloth, so wherever the held
        // reading already covers it this hands that reading back unchanged and
        // the surface is the one measured.
        let lo = held.0.min(here.0).max(here.1 - wide);
        (lo, lo + wide)
    }

    /// The lowest and highest pattern ordinate the strip reaches.
    pub fn span(&self) -> (f64, f64) {
        spanned(&self.panels)
    }

    /// The strip as the surface holds it: each piece, and whether its rising
    /// abscissa runs with the turn or against it.
    ///
    /// The walk's own sense times the way the turn grows, and not the walk's
    /// alone. A declared heading can send the strip round the other way, and
    /// then a panel the walk crosses forwards is a panel whose cloth runs
    /// backwards on the body — which is the question this answers.
    pub(super) fn strip(&self) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.panels
            .iter()
            .map(|panel| (panel.piece, panel.sense * self.winds))
    }

    /// Where a pattern point of `piece` lands: the turn it sits at, in radians,
    /// and how far from the axis. `None` for a piece the strip does not carry.
    ///
    /// The turn is the cloth walked to reach the point divided by the hoop it
    /// lands on, which is what makes a closed garment close at every height
    /// rather than at the one a ring was sized from: two pieces sharing an
    /// ordinate share the whole turn, however wide either is.
    ///
    /// It opens at the pinned line where a heading declares one, and at the
    /// first panel's own middle where none does — read at the point's own
    /// ordinate and never once at the crest, so that line comes out a meridian
    /// and the product faces one way at every height instead of at one.
    ///
    /// All three readings come from [`Round::reach`], so the hoop a point is
    /// divided by is the hoop it is placed on: read the widths at one ordinate
    /// while the hoop is read at another and the trouser back's centre line
    /// swings 1.604 rad across its own top 25 mm.
    pub(super) fn at(&self, piece: usize, p: [f64; 2]) -> Option<(f64, f64)> {
        // The hoop and not the girth, which is the distinction this whole
        // module turns on: what a point's walked cloth is divided by is
        // the surface it is going onto, never the cloth that happens to
        // be at that height.
        let mut hoop = 0.0;
        let mut opens = 0.0;
        let mut found = None;
        for (k, panel) in self.panels.iter().enumerate() {
            let (lo, hi) = self.reach(panel, p[1]);
            match self.pin {
                Some(pin) if pin.piece == panel.piece => opens = hoop + into(panel, pin.at, lo, hi),
                None if k == 0 => opens = (hi - lo) / 2.0,
                _ => {}
            }
            if panel.piece == piece {
                found = Some(hoop + into(panel, p[0], lo, hi));
            }
            hoop += hi - lo;
        }
        let walked = found?;
        // One walk of the panels and not two: the clearance walk asks this once
        // per vertex per round, and reading the extents again to size the
        // radius doubled what a placement costs for nothing.
        let radius = hoop / TAU + self.room.at(p[1]);
        let from = self.pin.map_or(0.0, |pin| pin.turn);
        if hoop <= f64::EPSILON {
            // A hoop that has closed has one place on it, and every vertex of
            // that ordinate belongs there: that is what a point is. A gore's
            // apex and a tab's tip are ordinary pattern shapes, and refusing
            // them took the whole piece off the body — one `None` here and
            // `wrap_into` drops the lot to the flat release, 1.2 m up.
            return Some((from, radius));
        }
        let along = if self.closed {
            TAU * (walked - opens) / hoop
        } else {
            // An open strip has no whole turn to divide, so its pieces abut at
            // the surface's own size, exactly as they abutted on the cylinder.
            (walked - opens) / radius
        };
        Some((from + self.winds * along, radius))
    }

    /// Opens the bands the given ordinates fall in, none of them past
    /// `ceiling`, and answers whether anything moved.
    pub(super) fn open(&mut self, at: &[f64], by: f64, ceiling: f64) -> bool {
        for &y in at {
            self.room.ask(y);
        }
        self.room.open(by, ceiling)
    }
}

/// The ordinate range a set of panels spans between them.
fn spanned(panels: &[Panel]) -> (f64, f64) {
    panels.iter().fold((f64::MAX, f64::MIN), |(lo, hi), panel| {
        let (low, high) = panel.cloth.span();
        (lo.min(low), hi.max(high))
    })
}

#[cfg(test)]
mod tests;
