use toile_doc::{
    Command, Doc, EdgeAnchor, Identity, LineEdit, LineKey, LineKind, LineVertex, PieceKey,
    PointKey, SegmentEdit,
};

use super::super::outline::{self, Bend, Run, Vertex};
use super::super::report::{Carried, InternalNote, Refusal};
use super::super::samples;
use super::super::source::Source;
use super::super::translate::{Coords, Translator};
use super::place::{at, insert, label, locate, remember, shown};
use super::{Placed, Walked};
use crate::{Error, InternalPath};

/// The stroke Seamly draws a line with when something lands on the piece
/// along it, as against the dashes it reasons with.
const SOLID: &str = "solidLine";

/// Draws every internal path a piece cites as an internal line of that piece.
///
/// One note per path whether it was drawn or not: a path the product has
/// nowhere to put is said out loud in the report, never dropped quietly.
pub(super) fn draw(
    tr: &mut Translator<'_>,
    doc: &mut Doc,
    walked: &Walked<'_>,
    piece: PieceKey,
    placed: &mut Placed,
) -> Result<Vec<InternalNote>, Error> {
    let mut notes = Vec::new();
    for id in &walked.piece.internal_paths {
        let path = walked
            .block
            .internal_paths
            .iter()
            .find(|held| held.id == *id)
            .ok_or_else(|| {
                Error::Product(format!(
                    "`{}` draws the internal path {id}, which its block does not hold",
                    walked.piece.name
                ))
            })?;
        let run = outline::line(tr, walked.block, path)?;
        notes.push(InternalNote {
            name: path.name.clone(),
            line_type: path.line_type.clone(),
            cut: path.cut,
            carried: one(tr, doc, path, &run, piece, placed)?,
        });
    }
    Ok(notes)
}

/// One path as one line of the document, or why it became none.
fn one(
    tr: &mut Translator<'_>,
    doc: &mut Doc,
    path: &InternalPath,
    run: &Run,
    piece: PieceKey,
    placed: &mut Placed,
) -> Result<Carried, Error> {
    if run.spans.is_empty() {
        return Ok(Carried::Refused(Refusal::OnePlace));
    }
    let places: Vec<&Vertex> = std::iter::once(&run.head)
        .chain(run.spans.iter().map(|span| &span.to))
        .collect();
    let labels: Vec<Option<String>> = places
        .iter()
        .map(|vertex| label(tr, vertex))
        .collect::<Result<_, _>>()?;
    let kind = kind(path);
    let mut edit = LineEdit::new(piece, kind, place(tr, doc, &run.head, piece, placed)?);
    let mut handles: Vec<&(Coords, Source)> = Vec::new();
    let mut stray: f64 = 0.0;
    for (index, span) in run.spans.iter().enumerate() {
        let to = place(tr, doc, &span.to, piece, placed)?;
        let Some(bend) = &span.bend else {
            edit = edit.to(to);
            continue;
        };
        let control = [
            locate(tr, places[index].source)?,
            locate(tr, bend.out.1)?,
            locate(tr, bend.into.1)?,
            locate(tr, span.to.source)?,
        ];
        let (count, off) = samples::samples(control);
        stray = stray.max(off);
        let (a, b) = (shown(&labels, index), shown(&labels, index + 1));
        let curve = curve(bend, &format!("manija_{a}_{b}"))?;
        handles.extend([&bend.out, &bend.into]);
        edit = edit.curving(to, curve, count);
    }
    let anchored = edit
        .vertices()
        .filter(|vertex| vertex.anchor().is_some())
        .count();
    let curves = edit
        .spans
        .iter()
        .filter(|span| span.segment.bends())
        .count();
    let line = drawn(doc, &path.name, edit.named(&path.name))?;
    keep(doc, line, &handles, placed)?;
    Ok(Carried::Line {
        kind,
        places: places.len(),
        anchored,
        curves,
        stray,
    })
}

/// What the pattern draws an internal path as, which is all the file says.
///
/// Seamly keeps no meaning for an internal path: a stroke, the name its author
/// wrote, and a flag for whether the cutter opens the cloth along it. The flag
/// is taken at its word, since it is the one thing the file states outright,
/// and the stroke decides the rest — solid for something that lands on the
/// piece, dashed for a line the draft was only reasoned with. Neither of those
/// two costs cloth or thread, so a wrong guess spoils nothing, and the report
/// lists which path got which so its author can put it right in the app.
fn kind(path: &InternalPath) -> LineKind {
    if path.cut {
        LineKind::Slit
    } else if path.line_type == SOLID {
        LineKind::Placement
    } else {
        LineKind::Reference
    }
}

/// One place of a line: on the contour of the piece it is drawn on when the
/// construction point it stands on is a corner of that very piece, a point of
/// the document otherwise.
///
/// A corner is worth an anchor because the line then follows the cloth: move
/// the corner and the pocket mouth moves with it. Anywhere else an anchor
/// would have to name a fraction along a tract, and that fraction is a number
/// no formula follows — a point of its own, carrying the construction as
/// formulas, follows a change of body exactly as the pattern does.
fn place(
    tr: &mut Translator<'_>,
    doc: &mut Doc,
    vertex: &Vertex,
    piece: PieceKey,
    placed: &mut Placed,
) -> Result<LineVertex, Error> {
    if let Some(id) = vertex.point
        && let Some(&known) = placed.points.get(&id)
        && doc
            .pieces
            .get(piece)
            .is_some_and(|held| held.node_index(known).is_some())
    {
        return Ok(LineVertex::Contour(EdgeAnchor::at_node(piece, known)));
    }
    Ok(LineVertex::free(point(tr, doc, vertex, placed)?))
}

/// The document point a place is: the one its construction point already
/// became, for any piece, or a new one.
fn point(
    tr: &mut Translator<'_>,
    doc: &mut Doc,
    vertex: &Vertex,
    placed: &mut Placed,
) -> Result<PointKey, Error> {
    if let Some(id) = vertex.point
        && let Some(&known) = placed.points.get(&id)
    {
        return Ok(known);
    }
    let named = label(tr, vertex)?;
    let coords = match vertex.point {
        Some(id) => tr.node(id)?,
        None => vertex.coords.clone(),
    };
    let mut held = at(&coords, named.as_deref().unwrap_or("place"))?;
    held.label = named;
    let key = insert(doc, held, &coords, vertex.source, placed);
    if let Some(id) = vertex.point {
        placed.points.insert(id, key);
    }
    Ok(key)
}

/// A curved span's two handles, as the edit that draws the line carries them.
fn curve(bend: &Bend, name: &str) -> Result<SegmentEdit, Error> {
    let out = at(&bend.out.0, name)?.named(&format!("{name}_1"));
    let into = at(&bend.into.0, name)?.named(&format!("{name}_2"));
    Ok(SegmentEdit::cubic(out, into))
}

/// Applies the edit and gives back the key the line took.
///
/// The key travels in the inverse, which is where a command that creates
/// something keeps it: the alternative is looking for the newest entry of the
/// arena, which is the same answer read off a guess instead of off the edit.
fn drawn(doc: &mut Doc, name: &str, edit: LineEdit) -> Result<LineKey, Error> {
    let applied = Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    }
    .apply(doc)
    .map_err(|refused| Error::Product(format!("`{name}`: {refused}")))?;
    match applied.inverse {
        Command::RemoveLine { line } => Ok(line),
        other => Err(Error::Product(format!(
            "drawing `{name}` is undone by {other:?}, which names no line"
        ))),
    }
}

/// Remembers where the handles the command created come from.
///
/// `handles` holds each curved span's two handles in span order, which is the
/// order `InternalLine::handles` hands the keys back in, so the two zip.
fn keep(
    doc: &Doc,
    line: LineKey,
    handles: &[&(Coords, Source)],
    placed: &mut Placed,
) -> Result<(), Error> {
    let held = doc
        .lines
        .get(line)
        .ok_or_else(|| Error::Product("the line just drawn is not in the document".to_owned()))?;
    let keys: Vec<PointKey> = held.handles().collect();
    if keys.len() != handles.len() {
        return Err(Error::Product(format!(
            "the line took {} handles for the {} written",
            keys.len(),
            handles.len()
        )));
    }
    for (key, (coords, source)) in keys.into_iter().zip(handles) {
        remember(key, coords, *source, placed);
    }
    Ok(())
}
