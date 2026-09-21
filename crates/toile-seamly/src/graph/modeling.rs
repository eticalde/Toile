use roxmltree::Node;

use super::index::{Index, Kind};
use crate::xml::{Attrs, elements, unsupported};
use crate::{Block, Error, InternalPath, NodeKind, Notch, PathNode};

/// Who a list of path nodes belongs to. Only a piece outline carries notches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Owner {
    InternalPath,
    Piece,
}

/// Reads a `<modeling>`: the copies of construction objects that paths cite,
/// and the internal paths built from them.
pub(super) fn read(node: Node<'_, '_>, index: &mut Index, block: &mut Block) -> Result<(), Error> {
    for child in elements(node) {
        let copied = match (child.tag_name().name(), child.attribute("type")) {
            ("point", Some("modeling")) => Kind::Point,
            ("spline", Some("modelingSpline")) => Kind::Spline,
            ("spline", Some("modelingPath")) => Kind::SplinePath,
            ("arc", Some("modeling")) => Kind::Arc,
            ("path", Some("2")) => {
                let path = internal_path(child, index)?;
                block.internal_paths.push(path);
                continue;
            }
            _ => return Err(unsupported(child, "this modeling object")),
        };
        let mut attrs = Attrs::new(child);
        let at = attrs.at();
        let copy = attrs.id("id")?;
        let original = attrs.id("idObject")?;
        attrs.ignore(&["type"]);
        attrs.finish()?;
        index.add_copy(copy, original, copied, &at)?;
        block.copies.insert(copy, original);
    }
    Ok(())
}

// `type="2"` is read as an internal path because pieces list exactly these
// under `<iPaths>`; any other path type is refused above.
fn internal_path(node: Node<'_, '_>, index: &mut Index) -> Result<InternalPath, Error> {
    let mut attrs = Attrs::new(node);
    let at = attrs.at();
    let mut path = InternalPath {
        id: attrs.id("id")?,
        name: attrs.req("name")?.to_owned(),
        line_type: attrs.req("lineType")?.to_owned(),
        cut: attrs.flag("cut")?.unwrap_or(false),
        nodes: Vec::new(),
    };
    attrs.ignore(&["type"]);
    attrs.finish()?;
    for child in elements(node) {
        if child.tag_name().name() != "nodes" {
            return Err(unsupported(child, "this element inside a path"));
        }
        path.nodes = nodes(child, index, Owner::InternalPath)?;
    }
    index.insert(path.id, Kind::InternalPath, &at)?;
    Ok(path)
}

/// Reads a `<nodes>` list, resolving each node through its modeling copy.
pub(super) fn nodes(
    list: Node<'_, '_>,
    index: &Index,
    owner: Owner,
) -> Result<Vec<PathNode>, Error> {
    elements(list)
        .map(|node| path_node(node, index, owner))
        .collect()
}

fn path_node(node: Node<'_, '_>, index: &Index, owner: Owner) -> Result<PathNode, Error> {
    let (kind, copied) = match (node.tag_name().name(), node.attribute("type")) {
        ("node", Some("NodePoint")) => (NodeKind::Point, Kind::Point),
        ("node", Some("NodeSpline")) => (NodeKind::Spline, Kind::Spline),
        ("node", Some("NodeSplinePath")) => (NodeKind::SplinePath, Kind::SplinePath),
        ("node", Some("NodeArc")) => (NodeKind::Arc, Kind::Arc),
        _ => return Err(unsupported(node, "this path node")),
    };
    let mut attrs = Attrs::new(node);
    let at = attrs.at();
    let object = index.original(attrs.id("idObject")?, copied, &at)?;
    let reverse = attrs.flag("reverse")?.unwrap_or(false);
    let notch = match owner {
        Owner::Piece => notch(&mut attrs)?,
        Owner::InternalPath => None,
    };
    attrs.ignore(&["type"]);
    attrs.finish()?;
    Ok(PathNode {
        kind,
        object,
        reverse,
        notch,
    })
}

fn notch(attrs: &mut Attrs<'_, '_>) -> Result<Option<Notch>, Error> {
    attrs.ignore(&["showNotch"]);
    if attrs.flag("notch")? != Some(true) {
        attrs.ignore(&["notchType", "notchLength"]);
        return Ok(None);
    }
    Ok(Some(Notch {
        kind: attrs.req("notchType")?.to_owned(),
        length: attrs.number("notchLength")?,
    }))
}
