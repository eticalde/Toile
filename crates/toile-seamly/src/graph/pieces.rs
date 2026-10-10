use roxmltree::Node;

use super::index::{Index, Kind};
use super::modeling::{Owner, nodes};
use crate::xml::{Attrs, elements, place, unsupported};
use crate::{Block, Error, Piece};

/// Reads a `<pieces>` list.
pub(super) fn read(node: Node<'_, '_>, index: &mut Index, block: &mut Block) -> Result<(), Error> {
    for child in elements(node) {
        if child.tag_name().name() != "piece" {
            return Err(unsupported(child, "this element"));
        }
        let piece = piece(child, index)?;
        block.pieces.push(piece);
    }
    Ok(())
}

fn piece(node: Node<'_, '_>, index: &mut Index) -> Result<Piece, Error> {
    let mut attrs = Attrs::new(node);
    let at = attrs.at();
    let id = attrs.id("id")?;
    let name = attrs.req("name")?.to_owned();
    let seam_allowance = if attrs.flag("seamAllowance")? == Some(true) {
        Some(attrs.number("width")?)
    } else {
        attrs.ignore(&["width"]);
        None
    };
    attrs.expect("version", "2")?;
    let placement = [offset(&mut attrs, "mx")?, offset(&mut attrs, "my")?];
    // Layout and display switches: none of them moves a line of the piece.
    attrs.ignore(&["forbidFlipping", "hideMainPath", "inLayout", "united"]);
    attrs.finish()?;
    let mut piece = Piece {
        id,
        name,
        seam_allowance,
        letter: String::new(),
        quantity: 1,
        on_fold: false,
        labels: Vec::new(),
        grain: None,
        outline: Vec::new(),
        internal_paths: Vec::new(),
        placement,
    };
    for child in elements(node) {
        match child.tag_name().name() {
            "data" => label(child, &mut piece)?,
            "nodes" => piece.outline = nodes(child, index, Owner::Piece)?,
            "iPaths" => {
                for record in elements(child) {
                    let mut attrs = Attrs::new(record);
                    let path = attrs.id("path")?;
                    attrs.finish()?;
                    let path = index.expect(path, Kind::InternalPath, &place(record))?;
                    piece.internal_paths.push(path);
                }
            }
            "grainline" => piece.grain = grainline(child)?,
            "patternInfo" => {
                let mut attrs = Attrs::new(child);
                attrs.ignore(&["fontSize", "height", "visible", "width", "rotation"]);
                attrs.finish()?;
            }
            _ => return Err(unsupported(child, "this element inside a piece")),
        }
    }
    index.insert(id, Kind::Piece, &at)?;
    Ok(piece)
}

/// The angle a grain line declares, in degrees, when it declares one.
///
/// A piece nobody put a grain line on keeps the element with an empty
/// rotation, which is the absence of an angle and not an angle of zero.
/// Whether Seamly draws the arrow, how long it draws it and which ends it
/// barbs are the drawing's business, the way a line's colour is: the warp runs
/// the way it runs whether or not the piece shows it. An angle Seamly wrote as
/// a formula is refused out loud, because reading it would need the body and
/// guessing at it would be inventing the grain.
fn grainline(node: Node<'_, '_>) -> Result<Option<f64>, Error> {
    let mut attrs = Attrs::new(node);
    attrs.ignore(&["arrows", "length", "visible"]);
    let Some(raw) = attrs.opt("rotation").filter(|raw| !raw.is_empty()) else {
        attrs.finish()?;
        return Ok(None);
    };
    let degrees = raw
        .parse::<f64>()
        .ok()
        .filter(|degrees| degrees.is_finite())
        .ok_or_else(|| {
            unsupported(
                node,
                format!("`rotation=\"{raw}\"`, which is no plain angle"),
            )
        })?;
    attrs.finish()?;
    Ok(Some(degrees))
}

/// A layout offset of the piece; a piece the file never moved writes none.
fn offset(attrs: &mut Attrs<'_, '_>, name: &'static str) -> Result<f64, Error> {
    match attrs.opt(name) {
        Some(_) => attrs.number(name),
        None => Ok(0.0),
    }
}

fn label(node: Node<'_, '_>, piece: &mut Piece) -> Result<(), Error> {
    let mut attrs = Attrs::new(node);
    attrs.req("letter")?.clone_into(&mut piece.letter);
    piece.quantity = attrs.count("quantity")?;
    piece.on_fold = attrs.flag("onFold")?.unwrap_or(false);
    attrs.ignore(&["fontSize", "height", "visible", "width", "rotation"]);
    attrs.finish()?;
    for line in elements(node) {
        if line.tag_name().name() != "line" {
            return Err(unsupported(line, "this element inside a label"));
        }
        let mut attrs = Attrs::new(line);
        piece.labels.push(attrs.req("text")?.to_owned());
        attrs.ignore(&["alignment", "bold", "italic", "sfIncrement"]);
        attrs.finish()?;
    }
    Ok(())
}
