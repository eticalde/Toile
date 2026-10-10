use std::collections::BTreeSet;

use toile_doc::{
    Command, ContourNode, Doc, EdgeAnchor, Grain, Identity, Notch, Piece, PieceKey, Placement,
    PointKey, Segment, Winding,
};

use super::super::outline::Tract;
use super::super::report::{NotchNote, PieceNote};
use super::super::samples;
use super::super::source::Source;
use super::super::translate::{Coords, Translator};
use super::place::{at, insert, label, locate, shown};
use super::{Placed, Walked, inner};
use crate::{Error, Xy};

/// Puts one piece into the document: its corners, its handles, the piece
/// itself and its notches.
pub(super) fn place(
    tr: &mut Translator<'_>,
    doc: &mut Doc,
    walked: &Walked<'_>,
    placed: &mut Placed,
) -> Result<PieceNote, Error> {
    let Walked { piece, tracts, .. } = walked;
    let labels = labels(tr, tracts)?;
    let mut taken = BTreeSet::new();
    for label in labels.iter().flatten() {
        if !taken.insert(label) {
            return Err(Error::Product(format!(
                "`{}` runs through `{label}` twice",
                piece.name
            )));
        }
    }
    let mut contour = Vec::with_capacity(tracts.len());
    let mut flat: Vec<([Xy; 4], Option<u16>)> = Vec::with_capacity(tracts.len());
    let mut curves = Vec::new();
    for (index, tract) in tracts.iter().enumerate() {
        let next = (index + 1) % tracts.len();
        let node = corner(tr, doc, tract, labels[index].as_deref(), placed)?;
        let from = locate(tr, tract.from.source)?;
        let to = locate(tr, tracts[next].from.source)?;
        let Some(bend) = &tract.bend else {
            flat.push(([from, from, to, to], None));
            contour.push(ContourNode::line(node));
            continue;
        };
        let control = [from, locate(tr, bend.out.1)?, locate(tr, bend.into.1)?, to];
        let (count, stray) = samples::samples(control);
        let (a, b) = (shown(&labels, index), shown(&labels, next));
        let out = handle(doc, &bend.out, &format!("manija_{a}_{b}_1"), placed)?;
        let into = handle(doc, &bend.into, &format!("manija_{a}_{b}_2"), placed)?;
        flat.push((control, Some(count)));
        curves.push((format!("{a} → {b}"), count, stray));
        contour.push(ContourNode {
            point: node,
            segment: Segment::Cubic { out, into },
            samples: count,
        });
    }
    let winding = Winding::of_area(samples::signed_area(&samples::flattened(&flat)));
    let placement = piece
        .placement
        .iter()
        .any(|offset| offset.abs() > 0.0)
        .then(|| Placement::new(piece.placement[0], piece.placement[1]));
    let nodes = contour.len();
    let held = Piece {
        name: piece.name.clone(),
        contour,
        winding,
        grain: grain(piece.grain),
        seam_allowance: piece.seam_allowance,
        quantity: piece.quantity,
        letter: (!piece.letter.is_empty()).then(|| piece.letter.clone()),
        labels: piece.labels.clone(),
        placement,
    };
    let applied = Command::AddPiece {
        identity: Identity::New,
        piece: held,
    }
    .apply(doc)
    .map_err(|refused| Error::Product(format!("`{}`: {refused}", piece.name)))?;
    let key = applied.touched[0];
    let notches = notches(tr, doc, piece, key, placed)?;
    // After the piece, because a place of a line that is a corner of it is
    // addressed as a node of the contour the piece has only just gained.
    let internal = inner::draw(tr, doc, walked, key, placed)?;
    Ok(PieceNote {
        name: piece.name.clone(),
        letter: piece.letter.clone(),
        quantity: piece.quantity,
        on_fold: piece.on_fold,
        seam_allowance: piece.seam_allowance,
        labels: piece.labels.clone(),
        nodes,
        curves,
        winding,
        notches,
        internal,
        placement: piece.placement,
        grain: piece.grain,
    })
}

/// The grain the file declares, or the vertical a piece with no grain line is
/// left on.
///
/// Seamly measures the angle counter-clockwise on a page whose y grows
/// downward and the document measures it from x toward y, which is down the
/// page, so the sign turns over. The half turn is folded away because a grain
/// is the direction the warp runs and not an arrow along it: `90`, the angle
/// Seamly writes for a grain line straight up the piece, is the document's
/// vertical, straight down it, and the two name one cloth.
fn grain(degrees: Option<f64>) -> Grain {
    match degrees {
        None => Grain::default(),
        Some(degrees) => Grain::Angle((-degrees).rem_euclid(180.0).to_radians()),
    }
}

/// The name each corner shows: its construction point's, when it is one.
fn labels(tr: &Translator<'_>, tracts: &[Tract]) -> Result<Vec<Option<String>>, Error> {
    tracts.iter().map(|tract| label(tr, &tract.from)).collect()
}

/// The document point a corner is: the one its construction point already
/// became, or a new one.
fn corner(
    tr: &mut Translator<'_>,
    doc: &mut Doc,
    tract: &Tract,
    label: Option<&str>,
    placed: &mut Placed,
) -> Result<PointKey, Error> {
    let vertex = &tract.from;
    if let Some(id) = vertex.point
        && let Some(&known) = placed.points.get(&id)
    {
        return Ok(known);
    }
    let coords = match vertex.point {
        Some(id) => tr.node(id)?,
        None => vertex.coords.clone(),
    };
    let mut point = at(&coords, label.unwrap_or("corner"))?;
    point.label = label.map(str::to_owned);
    let key = insert(doc, point, &coords, vertex.source, placed);
    if let Some(id) = vertex.point {
        placed.points.insert(id, key);
    }
    Ok(key)
}

/// A handle as a document point.
fn handle(
    doc: &mut Doc,
    (coords, source): &(Coords, Source),
    label: &str,
    placed: &mut Placed,
) -> Result<PointKey, Error> {
    let point = at(coords, label)?.named(label);
    Ok(insert(doc, point, coords, *source, placed))
}

/// The piece's notches, each at the corner the file cuts it at, and what the
/// product cannot say of them.
fn notches(
    tr: &Translator<'_>,
    doc: &mut Doc,
    piece: &crate::Piece,
    key: PieceKey,
    placed: &Placed,
) -> Result<Vec<NotchNote>, Error> {
    let mut notes = Vec::new();
    for node in &piece.outline {
        let Some(notch) = &node.notch else {
            continue;
        };
        let at = placed.points.get(&node.object).copied().ok_or_else(|| {
            Error::Product(format!(
                "`{}` has a notch at object {}, which is not one of its corners",
                piece.name, node.object
            ))
        })?;
        doc.notches
            .insert(Notch::lone(EdgeAnchor::at_node(key, at)));
        notes.push(NotchNote {
            at: tr.point_name(node.object)?.to_owned(),
            kind: notch.kind.clone(),
            length: notch.length,
        });
    }
    Ok(notes)
}
