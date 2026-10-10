use std::cmp::Ordering;

use toile_doc::{HangKey, Heading, Sense};
use toile_geom::validate;

use super::super::Session;
use super::super::run::{Run, edge_bases};
use crate::couture::{Pin, ShapePipeline};

impl Session {
    /// Where the document turns the product to face: every line of cloth a
    /// heading pins, and the turn each is pinned at, highest first.
    ///
    /// Highest first, by the rule and for the reason [`Session::declared`]
    /// takes the topmost station: a ring faces one way, so two headings on
    /// one ring are two answers to one question, and the line it hangs by
    /// is the one that settles them. "The first" would be the document's
    /// storage order, which is the very thing a heading exists to replace.
    ///
    /// Every pin and no longer one, because a garment need not go round one
    /// ring: a shirt's body faces front and so does its collar, and those are
    /// two declarations the placement reads one per ring. Which ring a pin
    /// belongs to is the piece it is written on, so nothing here has to choose.
    ///
    /// Read against the meshes on the stand, again for that reason: a pin is an
    /// abscissa of cloth the placement will roll, and only the cloth has one.
    pub(in crate::session) fn pinned(&self) -> Vec<Pin> {
        let Some(draft) = self.draft.as_ref() else {
            return Vec::new();
        };
        let pipes = self.pipelines();
        let bases = edge_bases(&pipes);
        let mut found: Vec<(f64, Pin)> = Vec::new();
        for (_, hang) in draft.doc().hangs.iter() {
            let Some(heading) = hang.heading else {
                continue;
            };
            for run in self.runs_of(draft, &pipes, &bases, hang.at) {
                if let Some(pinned) = pinned_at(pipes[run.at], &run, heading) {
                    found.push(pinned);
                }
            }
        }
        found.sort_by(|a, b| b.0.total_cmp(&a.0));
        found.into_iter().map(|(_, pin)| pin).collect()
    }

    /// Every hang no heading could turn: the ones whose run pins nothing
    /// whatever a person declares on it.
    ///
    /// Asked of the cloth and answered here rather than in the interface,
    /// because it is the same reading [`Session::pinned`] takes and a second
    /// one taken elsewhere could disagree with it. What it is for is the press:
    /// a press that writes a declaration the placement then drops is a press
    /// that lies, and a gap with its reason is worth more than one.
    pub fn headless(&self) -> Vec<HangKey> {
        let Some(draft) = self.draft.as_ref() else {
            return Vec::new();
        };
        let pipes = self.pipelines();
        let bases = edge_bases(&pipes);
        // The very heading the press would write, so what this answers is
        // whether that press would do anything.
        let facing = Heading::facing(0.0, Sense::Leftward);
        draft
            .doc()
            .hangs
            .iter()
            .filter(|(_, hang)| {
                !self
                    .runs_of(draft, &pipes, &bases, hang.at)
                    .iter()
                    .any(|run| pinned_at(pipes[run.at], run, facing).is_some())
            })
            .map(|(key, _)| key)
            .collect()
    }
}

/// Where a heading's pin lands on one stretch of cloth: the ordinate of the
/// pinned point, and the pin the surface is to fix.
///
/// The head of the run is its first vertex and the tail its last, in the order
/// the contour walks them, so a pin of 0 is the head whatever the mesh laid in
/// between. Pinning the head is what makes a declaration survive a redraft: a
/// run's middle moves with the cloth either side of it — one back read 46.0° as
/// drawn, 48.8° with ease on the fronts and 44.1° with the same ease on itself
/// — while its head says the sentence of the trade, "this edge is the centre
/// front", and on a piece drawn against a fold the head of the run *is* the
/// crease.
///
/// A run whose two ends stand at one abscissa is pinned by the cloth beside
/// it: see [`beside`]. `None` only where that reading has no answer either.
pub(in crate::session) fn pinned_at(
    pipe: &ShapePipeline,
    run: &Run,
    heading: Heading,
) -> Option<(f64, Pin)> {
    let head = pipe.pos2d[*run.verts.first()? as usize];
    let tail = pipe.pos2d[*run.verts.last()? as usize];
    // At one abscissa within the mesh's own step down this run, and not within
    // a hair: an edge drawn straight down the pattern comes off the mesher with
    // its ends a fraction of a millimetre apart in x, which is a lean the cloth
    // cannot assert and nothing a person declared.
    let step = (tail[1] - head[1]).abs() / run.verts.len().saturating_sub(1).max(1) as f64;
    // Which way the run's own abscissa grows, which is what turns a sense
    // written about the run into one about the piece it is drawn on: a run
    // walked from its high abscissa down to its low one goes leftward on the
    // body while the piece's abscissa falls.
    let runs = if (tail[0] - head[0]).abs() > step {
        if tail[0] > head[0] { 1.0 } else { -1.0 }
    } else {
        beside(pipe, run)?
    };
    let along = |end: usize| head[end] + heading.pin * (tail[end] - head[end]);
    Some((
        along(1),
        Pin {
            piece: run.at,
            at: along(0),
            turn: heading.radians(),
            leftward: heading.leftward() * runs,
        },
    ))
}

/// Which side of a run that crosses no cloth the piece's own cloth lies on:
/// `1.0` at the higher abscissae, `-1.0` at the lower.
///
/// A run whose two ends stand at one abscissa is a vertical canto of the
/// pattern, and a vertical canto is what the trade calls a centre front. It is
/// a canto because the cloth is on one side of it, and which side is the
/// sentence the run itself cannot say: "leftward from this edge" means the way
/// into the piece.
///
/// Decided off the contour's own walk, segment by segment, and never by how far
/// the cloth reaches either way. Reach reads the whole piece and answers the
/// wrong question on a concave one: the left wall of a placket has the cloth to
/// its left while the piece's right-hand canto is the further off, so reach
/// declares the cloth to the right and the garment goes on inside out.
///
/// `None` where the run's own two ends are no canto after all: see the arms at
/// the foot of it.
fn beside(pipe: &ShapePipeline, run: &Run) -> Option<f64> {
    let turn = contour_turn(pipe)?;
    let at = |v: u32| pipe.pos2d[v as usize];
    let (lo, hi) = run.verts.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &v| {
        (lo.min(at(v)[1]), hi.max(at(v)[1]))
    });
    // What a rise has to beat to be one. A segment drawn level says nothing
    // whatever about left and right, and a straight canto comes off the mesher
    // with the last bits of its ordinates in no particular order, which is a
    // reversal the cloth never asserted.
    let hair = (hi - lo) * 1.0e-6;
    let (mut above, mut below) = (false, false);
    for pair in run.verts.windows(2) {
        let rise = at(pair[1])[1] - at(pair[0])[1];
        if rise.abs() <= hair {
            continue;
        }
        // The interior of a contour lies to the left of a counterclockwise walk
        // and to the right of a clockwise one, and the left of a rise is the
        // lower abscissa.
        if turn * rise > 0.0 {
            below = true;
        } else {
            above = true;
        }
    }
    match (above, below) {
        (true, false) => Some(1.0),
        (false, true) => Some(-1.0),
        // Both sides: a vent narrower than the mesh's step down the run, whose
        // two walls have the cloth on opposite sides of the same ordinates.
        // Neither wall answers for the other, and half a heading is worse than
        // none — a wrong sense puts the garment on mirrored where a gap reaches
        // a person through [`Session::headless`]. Neither side: a run with no
        // ordinate to read over, which is a stretch of contour nobody drew.
        _ => None,
    }
}

/// Which way round its own cloth a contour runs: `1.0` counterclockwise with y
/// upward, `-1.0` clockwise, `None` for one enclosing nothing.
///
/// The whole lap through the same reading one stretch of it comes through, so
/// the walk this is the sense of is the walk [`beside`] steps along. Off the
/// mesh and not off the document's declared winding, which speaks for the drawn
/// half of a piece cut against a fold.
fn contour_turn(pipe: &ShapePipeline) -> Option<f64> {
    let walk: Vec<[f64; 2]> = pipe
        .boundary_run((0.0, 1.0))
        .iter()
        .map(|&v| pipe.pos2d[v as usize])
        .collect();
    match validate::signed_area(&walk).partial_cmp(&0.0)? {
        Ordering::Greater => Some(1.0),
        Ordering::Less => Some(-1.0),
        Ordering::Equal => None,
    }
}

#[cfg(test)]
mod tests;
