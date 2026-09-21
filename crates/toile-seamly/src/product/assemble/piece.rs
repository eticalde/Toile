use std::collections::BTreeSet;

use toile_doc::{
    Command, ContourNode, Doc, EdgeAnchor, Grain, Identity, Notch, Piece, PieceKey, Placement,
    Point, PointKey, Segment, Winding,
};

use super::super::outline::Tract;
use super::super::report::{InternalNote, NotchNote, PieceNote};
use super::super::samples;
use super::super::source::Source;
use super::super::translate::{Coords, Translator};
use super::{Placed, Walked, binding};
use crate::{Error, Xy};

/// Puts one piece into the document: its corners, its handles, the piece
/// itself and its notches.
pub(super) fn place(
    tr: &mut Translator<'_>,
    doc: &mut Doc,
    walked: &Walked<'_>,
    placed: &mut Placed,
) -> Result<PieceNote, Error> {
    let Walked {
        block,
        piece,
        tracts,
    } = walked;
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
        grain: Grain::default(),
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
    let internal = piece
        .internal_paths
        .iter()
        .filter_map(|id| block.internal_paths.iter().find(|path| path.id == *id))
        .map(|path| InternalNote {
            name: path.name.clone(),
            line_type: path.line_type.clone(),
            cut: path.cut,
        })
        .collect();
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
    })
}

/// The name each corner shows: its construction point's, when it is one.
fn labels(tr: &Translator<'_>, tracts: &[Tract]) -> Result<Vec<Option<String>>, Error> {
    tracts
        .iter()
        .map(|tract| {
            tract
                .from
                .point
                .map(|id| tr.point_name(id).map(str::to_owned))
                .transpose()
        })
        .collect()
}

/// A corner's name, or its place in the contour when it has none.
fn shown(labels: &[Option<String>], index: usize) -> String {
    labels[index]
        .clone()
        .unwrap_or_else(|| format!("P{}", index + 1))
}

/// Where the imported body puts a point of the product.
fn locate(tr: &Translator<'_>, source: Source) -> Result<Xy, Error> {
    source
        .locate(tr.reference)
        .ok_or_else(|| Error::Product(format!("{source:?} has no place on the imported body")))
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

fn at(coords: &Coords, what: &str) -> Result<Point, Error> {
    Ok(Point::at(
        binding(&coords.x.source(), what)?,
        binding(&coords.y.source(), what)?,
    ))
}

fn insert(
    doc: &mut Doc,
    point: Point,
    coords: &Coords,
    source: Source,
    placed: &mut Placed,
) -> PointKey {
    let key = doc.points.insert(point);
    placed.sources.insert(key, source);
    let frozen: BTreeSet<_> = coords
        .x
        .frozen()
        .union(coords.y.frozen())
        .copied()
        .collect();
    if !frozen.is_empty() {
        placed.frozen.insert(key, frozen);
    }
    key
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
