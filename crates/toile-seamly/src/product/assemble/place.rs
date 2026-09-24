use std::collections::BTreeSet;

use toile_doc::{Doc, Point, PointKey};

use super::super::outline::Vertex;
use super::super::source::Source;
use super::super::translate::{Coords, Translator};
use super::{Placed, binding};
use crate::{Error, Xy};

/// The name a place shows: its construction point's, when it is one.
pub(super) fn label(tr: &Translator<'_>, vertex: &Vertex) -> Result<Option<String>, Error> {
    vertex
        .point
        .map(|id| tr.point_name(id).map(str::to_owned))
        .transpose()
}

/// A place's name, or where it sits in the walk when it has none.
pub(super) fn shown(labels: &[Option<String>], index: usize) -> String {
    labels[index]
        .clone()
        .unwrap_or_else(|| format!("P{}", index + 1))
}

/// Where the imported body puts a point of the product.
pub(super) fn locate(tr: &Translator<'_>, source: Source) -> Result<Xy, Error> {
    source
        .locate(tr.reference)
        .ok_or_else(|| Error::Product(format!("{source:?} has no place on the imported body")))
}

/// A pair of coordinates as a point of the document.
pub(super) fn at(coords: &Coords, what: &str) -> Result<Point, Error> {
    Ok(Point::at(
        binding(&coords.x.source(), what)?,
        binding(&coords.y.source(), what)?,
    ))
}

/// Puts a point into the document and remembers where it came from.
pub(super) fn insert(
    doc: &mut Doc,
    point: Point,
    coords: &Coords,
    source: Source,
    placed: &mut Placed,
) -> PointKey {
    let key = doc.points.insert(point);
    remember(key, coords, source, placed);
    key
}

/// Remembers where a point of the product comes from, and what it depends on
/// that was frozen at the imported body.
///
/// Apart from [`insert`] because a command creates the two handles of a curve
/// itself: the document hands their keys back, and what those keys are worth
/// checking against is still known only here.
pub(super) fn remember(key: PointKey, coords: &Coords, source: Source, placed: &mut Placed) {
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
}
