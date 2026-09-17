use super::super::sew::Sewn;
use crate::couture::ShapePipeline;

/// One piece as the walk crosses it: which way round it runs, and the stretch
/// of cloth it carries across.
pub(super) struct Step {
    pub(super) piece: usize,
    pub(super) sense: f64,
    pub(super) lo: f64,
    pub(super) hi: f64,
}

/// Which seams sit on each piece; `None` when one of them cannot be walked.
pub(super) fn adjacency(sewn: &[Sewn], pieces: usize) -> Option<Vec<Vec<usize>>> {
    let mut on: Vec<Vec<usize>> = vec![Vec::new(); pieces];
    for (k, one) in sewn.iter().enumerate() {
        if one.sides[0] == one.sides[1] {
            return None;
        }
        on[one.sides[0]].push(k);
        on[one.sides[1]].push(k);
    }
    (!sewn.is_empty() && on.iter().all(|seams| seams.len() <= 2)).then_some(on)
}

/// Follows the seams from piece to piece, and says whether the strip closed.
pub(super) fn walk(
    sewn: &[Sewn],
    pipes: &[&ShapePipeline],
    on: &[Vec<usize>],
) -> Option<(Vec<Step>, bool)> {
    let (start, first) = opening(sewn, on)?;
    let mut steps = Vec::new();
    let mut piece = start;
    let mut exit = first;
    let mut entry = on[start].iter().copied().find(|&j| j != first);
    loop {
        let out = at(sewn, exit, piece)?;
        let came = match entry {
            Some(j) => at(sewn, j, piece)?,
            None => far_from(pipes[piece], out),
        };
        steps.push(step(piece, pipes[piece], came, out));
        let next = across_seam(sewn, exit, piece)?;
        if next == start {
            return Some((steps, true));
        }
        if steps.len() > pipes.len() {
            return None;
        }
        piece = next;
        entry = Some(exit);
        let Some(onward) = on[piece].iter().copied().find(|&j| j != exit) else {
            // Nothing else is sewn to this piece, so the strip ends here and
            // the cloth past its one seam reaches to its own edge.
            let end = at(sewn, exit, piece)?;
            steps.push(step(piece, pipes[piece], end, far_from(pipes[piece], end)));
            return Some((steps, false));
        };
        exit = onward;
    }
}

/// The stretch of cloth a piece carries across, in its own frame.
///
/// The piece's own edges, not the seams that run down it. A seam lies inside
/// the cloth it joins, so the garment is wider than the distance between its
/// seams by whatever hangs past them on either side.
pub(super) fn across(pipe: &ShapePipeline) -> (f64, f64) {
    pipe.pos2d
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), p| (l.min(p[0]), h.max(p[0])))
}

/// One piece of the walk: the cloth it carries, and which way the seams run
/// it round the body.
fn step(piece: usize, pipe: &ShapePipeline, entry: f64, exit: f64) -> Step {
    let (lo, hi) = across(pipe);
    Step {
        piece,
        sense: if exit > entry { 1.0 } else { -1.0 },
        lo,
        hi,
    }
}

/// Where the walk starts, and the seam it leaves by.
///
/// Two tie-breaks live here, and neither is in the seams. A strip with two
/// free ends is walked from the first of them, and a closed one is opened at
/// its first piece by whichever of its two seams sits further along it — so
/// that first piece runs with the turn rather than against it. Start at the
/// other end, or leave by the other seam, and the garment comes out mirrored.
fn opening(sewn: &[Sewn], on: &[Vec<usize>]) -> Option<(usize, usize)> {
    let ends: Vec<usize> = (0..on.len()).filter(|&p| on[p].len() == 1).collect();
    match ends.len() {
        0 => {
            let start = (0..on.len()).find(|&p| on[p].len() == 2)?;
            let (x, y) = (on[start][0], on[start][1]);
            Some((
                start,
                if at(sewn, x, start)? > at(sewn, y, start)? {
                    x
                } else {
                    y
                },
            ))
        }
        2 => Some((ends[0], on[ends[0]][0])),
        _ => None,
    }
}

/// Where a seam runs across one of the two pieces it joins.
fn at(sewn: &[Sewn], seam: usize, piece: usize) -> Option<f64> {
    let one = sewn.get(seam)?;
    one.sides
        .iter()
        .position(|&p| p == piece)
        .map(|s| one.at[s])
}

/// The piece on the other side of a seam.
fn across_seam(sewn: &[Sewn], seam: usize, piece: usize) -> Option<usize> {
    let one = sewn.get(seam)?;
    let side = one.sides.iter().position(|&p| p == piece)?;
    Some(one.sides[1 - side])
}

/// The edge of a piece furthest from a seam that runs across it.
///
/// Which way a piece at the end of an open strip is turned: it has one seam,
/// and the cloth beyond it reaches to the piece's own edge.
fn far_from(pipe: &ShapePipeline, seam: f64) -> f64 {
    let (lo, hi) = across(pipe);
    if (seam - lo).abs() >= (hi - seam).abs() {
        lo
    } else {
        hi
    }
}
