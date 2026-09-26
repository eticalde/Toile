use eframe::egui::Pos2;
use toile_engine::draft::{Doc, PointKey};

use super::super::super::gesture::{EditContext, Mods};
use super::super::super::snap::SnapKind;
use super::super::caught;
use super::{
    CURVED, DARTED, Declaring, FLAT, IN_LINE, NOT_BEYOND, NOT_NEXT, OFF_NODE, SAME_NODE, anchor,
};

/// How many nodes have to be chosen before the walk has a direction.
const WALKING: usize = 2;

/// Which node a press chooses, or why it chooses none.
///
/// A node of the contour and nowhere else: the point of declaring a dart is
/// that the three places are drawn already, so there is nothing here for a
/// press to place. Every refusal in this module is one the document would make
/// too, and this is where each is worded in the language the panels are written
/// in — before anything reaches the document to be refused there in another
/// one.
pub(super) fn chosen(
    declaring: &Declaring,
    at: Pos2,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> Result<(PointKey, [f64; 2]), &'static str> {
    let caught = caught(at, anchor(declaring, at, ctx), mods, ctx);
    let Some(SnapKind::Node(node)) = caught.kind else {
        return Err(OFF_NODE);
    };
    if declaring.nodes.iter().any(|&(key, _)| key == node) {
        return Err(SAME_NODE);
    }
    vacant(ctx.doc, node)?;
    next_to(declaring, node, ctx)?;
    // The two the cut makes on its own side, which the declaration owes for the
    // same reasons: a wedge whose sides bend is not the straight-sided triangle
    // the sheet draws a dart's marks against, and three nodes in line enclose
    // nothing, so the seam would close a wedge of no width — the argument
    // `dart::apex` makes when it keeps an apex off the contour.
    if let Some(&(head, _)) = declaring.nodes.last()
        && bends_between(ctx, head, node)
    {
        return Err(CURVED);
    }
    if let [(_, leg), (_, tip)] = declaring.nodes[..]
        && in_line(leg, tip, caught.at)
    {
        return Err(IN_LINE);
    }
    // Last, because it is the one no press can reach: two nodes written alike
    // are two nodes at one place, and the ladder answers a press there with
    // whichever of them the contour reaches first. It stands so that the
    // command this gesture sends cannot come back refused in another language.
    if declaring
        .nodes
        .iter()
        .any(|&(key, _)| written_alike(ctx.doc, key, node))
    {
        return Err(FLAT);
    }
    Ok((node, caught.at))
}

/// Refuses a node any dart of the document already names.
///
/// The whole document and not the one piece, because a point can be drawn by
/// two pieces at once — the argument the document's own check makes.
pub(super) fn vacant(doc: &Doc, node: PointKey) -> Result<(), &'static str> {
    let named = doc
        .darts
        .iter()
        .any(|(_, dart)| [dart.legs.0, dart.apex, dart.legs.1].contains(&node));
    if named { Err(DARTED) } else { Ok(()) }
}

/// Whether two nodes are written to one place.
///
/// The bindings and not the numbers they resolve to, because the bindings are
/// what the document holds and compares: the same document is the same document
/// whichever body it is read against.
fn written_alike(doc: &Doc, one: PointKey, other: PointKey) -> bool {
    match (doc.points.get(one), doc.points.get(other)) {
        (Some(a), Some(b)) => a.x == b.x && a.y == b.y,
        _ => false,
    }
}

/// Refuses a node that does not carry the walk on from the ones already chosen.
///
/// The second press goes beside the first, either way round; the third goes
/// past the apex on the side the first two already chose, so there is exactly
/// one node it can be.
fn next_to(
    declaring: &Declaring,
    node: PointKey,
    ctx: &EditContext<'_>,
) -> Result<(), &'static str> {
    let chosen: Vec<usize> = declaring
        .nodes
        .iter()
        .filter_map(|&(key, _)| seat(ctx, key))
        .collect();
    let walks = match (chosen.as_slice(), seat(ctx, node)) {
        ([leg], Some(apex)) => apex.abs_diff(*leg) == 1,
        ([leg, apex], Some(far)) => beyond(*leg, *apex) == Some(far),
        _ => false,
    };
    if walks {
        return Ok(());
    }
    Err(if declaring.nodes.len() < WALKING {
        NOT_NEXT
    } else {
        NOT_BEYOND
    })
}

/// The seat the walk reaches past `apex`, having come to it from `leg`.
///
/// One step on in the same direction, and no seat at all where that step would
/// run off the head of the contour: the document counts a wedge's seats forward
/// from its first leg and never wraps them, so a wedge straddling the place
/// where the contour closes is one it will not hold.
fn beyond(leg: usize, apex: usize) -> Option<usize> {
    if apex > leg {
        Some(apex + 1)
    } else {
        apex.checked_sub(1)
    }
}

/// Where a node sits in the contour, counted from its head.
///
/// Read off the document's own contour by the accessor the document counts a
/// wedge's seats with, and not off `ctx.nodes`: a piece that stops resolving
/// keeps its last good geometry and gains a defect, so the drawn list can be a
/// node short of the contour and the two would disagree about which nodes stand
/// together. Then the refusal would come back from the document, in English.
pub(super) fn seat(ctx: &EditContext<'_>, node: PointKey) -> Option<usize> {
    ctx.doc.pieces.get(ctx.piece)?.node_index(node)
}

/// Whether the contour bends on its way from one of these nodes to the other.
///
/// A tract is named by the node it leaves, and the walk crosses the two nodes
/// in contour order, so the tract between them is the one leaving the earlier
/// seat.
fn bends_between(ctx: &EditContext<'_>, one: PointKey, other: PointKey) -> bool {
    let leaves = match (seat(ctx, one), seat(ctx, other)) {
        (Some(a), Some(b)) if a < b => one,
        (Some(_), Some(_)) => other,
        _ => return false,
    };
    ctx.bends.iter().any(|bend| bend.node == leaves)
}

/// Whether three places stand in one straight line.
///
/// Twice the area of the triangle they enclose, against the slop of the
/// arithmetic and not against a smallest dart worth sewing: how shallow a dart
/// may be is a question about patterns, and this only refuses the one that is
/// not a triangle at all.
fn in_line(leg: [f64; 2], tip: [f64; 2], far: [f64; 2]) -> bool {
    let (one, other) = (
        [tip[0] - leg[0], tip[1] - leg[1]],
        [far[0] - leg[0], far[1] - leg[1]],
    );
    (one[0] * other[1] - one[1] * other[0]).abs() <= 1.0e-9
}

#[cfg(test)]
mod tests;
