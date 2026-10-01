use toile_sim::xpbd::State;

use super::pipeline::ShapePipeline;

mod profile;
mod room;
mod round;
mod whole;

pub use round::Round;

/// Where one piece's own width runs once the product is rolled round the body.
#[derive(Debug, Clone, Copy)]
pub struct Wrap {
    /// The turn a pattern abscissa of zero sits at on the crest's own hoop, in
    /// radians.
    ///
    /// One hoop's worth and not the piece's whole placement: the turn a point
    /// sits at is read off the cloth at its own ordinate, so a piece whose
    /// width changes with height has no single one. This is the crest's own
    /// reading, which is the one a client is shown, and nothing the
    /// placement keys on.
    pub turn: f64,
    /// `1.0` when the piece's rising abscissa runs with the turn, `-1.0` when
    /// it runs against it.
    pub sense: f64,
}

/// Where a sewn product is let go: the surface it is rolled onto, and every
/// piece's place on it.
///
/// Rolling is exact along a hoop and not down a meridian. Along a hoop it is
/// exact by construction — the arc at any ordinate is as long as the cloth
/// there — so the line a garment hangs by comes out within −0.68 % of flat on
/// the reference adult's skirt and −0.01 % on the wide-hipped one, and a closed
/// garment closes at every height rather than at one ring's alone.
///
/// Down a meridian it is not, and not in one direction only. Measured at
/// release on those same two skirts, the whole cloth comes out 4.04 % and
/// 5.57 % longer than flat, the worst one per cent of edges reach 1.289× and
/// 1.668×, and the worst single edge 2.93× and 2.17×. In the other direction
/// 31 % and 36 % of the edges come out *shorter*, the worst at 0.322× and
/// 0.587×: a flare carries more cloth than the path it runs, so that much of it
/// bunches. Both are at the hip, where the girth climbs 0.611 m to 1.166 m over
/// 22 cm of pattern.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The surface's centre: x, then z, in metres.
    pub axis: [f32; 2],
    /// How far from that axis the cloth is rolled at the crest, in metres.
    ///
    /// One hoop of the many the surface carries; [`Round::radius`] reads the
    /// rest.
    pub radius: f64,
    /// How high the surface sits, in metres: where the crest is let go.
    ///
    /// A garment goes where on the body it belongs, so this is the height of
    /// the body's own ring it was matched to — not a height above the crown.
    /// Once a pattern is a hoop there is no direction above the head for it to
    /// fall from that is round the body.
    pub stand: f32,
    /// The pattern ordinate the height names: the line of cloth the product
    /// hangs by.
    ///
    /// Everything else sits above or below it by the distance the pattern puts
    /// between them, so the pieces keep the rise they are drafted with: a back
    /// two centimetres above the front on the page sits two centimetres above
    /// it on the body. With nothing holding the product up this is its
    /// topmost point; with an elastic it is the elastic.
    pub crest: f64,
    /// One entry per piece, in the order the combined state holds them;
    /// `None` for a piece the seams do not place.
    pub wraps: Vec<Option<Wrap>>,
    /// The surface the strip is rolled onto, and where the strip lies on it.
    ///
    /// Why the cloth comes out longer than flat, and why no amount of tuning
    /// takes it back. Two terms, and the first is the larger. The surface is a
    /// cone wherever the girth changes and a cone's profile is longer than the
    /// height it spans, so a line down it is stretched by `hypot(1, dr/dy)`:
    /// measured, that slope reaches 1.40 on the wide-hipped skirt and the
    /// factor 1.72, which covers the whole of its worst one per cent. The
    /// second is that a line at a fixed abscissa is *not* a meridian where
    /// the width changes — only the profile's own edges and middle are — so
    /// it shears too; on a piece flared ten to one the shear adds 26 % to
    /// what the profile alone accounts for. The solver takes both up as
    /// stretch in the first substeps, as it does a seam's own slack.
    pub round: Round,
}

impl Layout {
    /// The strip on a surface, read at the crest for the one hoop a client is
    /// shown.
    pub(crate) fn on(
        round: Round,
        axis: [f32; 2],
        stand: f32,
        crest: f64,
        pieces: usize,
    ) -> Layout {
        let mut layout = Layout {
            axis,
            radius: 0.0,
            stand,
            crest,
            wraps: vec![None; pieces],
            round,
        };
        layout.reread();
        layout
    }

    /// Where a pattern point of piece `at` is let go, in metres; `None` for a
    /// piece this does not place.
    pub fn point(&self, at: usize, p: [f64; 2]) -> Option<[f32; 3]> {
        // Destructured rather than read through `self`, because this is the one
        // function every released vertex goes through: a field added to the
        // layout and not answered for here would be dropped from every
        // placement, silently and with nothing red. The two spelt `_` are the
        // crest's own readings, which this recomputes per ordinate.
        let Layout {
            axis,
            stand,
            crest,
            round,
            radius: _,
            wraps: _,
        } = self;
        let (turn, radius) = round.at(at, p)?;
        // `libm` rather than the intrinsic, for the reason the bake pins it:
        // what makes an angle deterministic here is that it is the same code
        // on every machine, not that it is correctly rounded.
        let (sin, cos) = libm::sincos(turn);
        Some([
            axis[0] + (radius * sin) as f32,
            *stand + (p[1] - *crest) as f32,
            axis[1] + (radius * cos) as f32,
        ])
    }

    /// Opens the surface at the ordinates the body is still swallowing cloth
    /// at, by `by` and no further out than `ceiling`. Answers whether anything
    /// moved.
    pub(crate) fn open(&mut self, at: &[f64], by: f64, ceiling: f64) -> bool {
        let moved = self.round.open(at, by, ceiling);
        if moved {
            self.reread();
        }
        moved
    }

    /// Reads the crest again, which is where [`Layout::radius`] and every
    /// [`Wrap`] come from and the only place they are written.
    fn reread(&mut self) {
        self.radius = self.round.radius(self.crest);
        let strip: Vec<(usize, f64)> = self.round.strip().collect();
        self.wraps.fill(None);
        for (piece, sense) in strip {
            let Some((turn, _)) = self.round.at(piece, [0.0, self.crest]) else {
                continue;
            };
            if let Some(slot) = self.wraps.get_mut(piece) {
                *slot = Some(Wrap { turn, sense });
            }
        }
    }

    /// Writes one piece's released block into a state that holds the whole
    /// product, beginning at `base`. Answers whether it placed the piece.
    pub(super) fn wrap_into(
        &self,
        state: &mut State,
        base: usize,
        pipe: &ShapePipeline,
        at: usize,
    ) -> bool {
        if self.wraps.get(at).copied().flatten().is_none() {
            return false;
        }
        for (i, &p) in pipe.pos2d.iter().enumerate() {
            let Some([x, y, z]) = self.point(at, p) else {
                return false;
            };
            state.px[base + i] = x;
            state.py[base + i] = y;
            state.pz[base + i] = z;
        }
        true
    }
}

#[cfg(test)]
mod tests;
