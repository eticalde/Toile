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
            "grainline" => {
                let mut attrs = Attrs::new(child);
                attrs.expect("visible", "false")?;
                attrs.ignore(&["arrows", "length", "rotation"]);
                attrs.finish()?;
            }
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
