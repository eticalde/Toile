use std::fmt::Write as _;

use roxmltree::Node;

use crate::{Error, Formula, Id, Place};

/// Attributes that only steer how Seamly draws an object on screen, or its
/// own bookkeeping: none of them changes a coordinate.
const DISPLAY: &[&str] = &[
    "mx",
    "my",
    "showPointName",
    "lineColor",
    "lineWeight",
    "color",
    "penStyle",
    "inUse",
];

/// Where `node` sits in its file.
pub(crate) fn place(node: Node<'_, '_>) -> Place {
    let line = node.document().text_pos_at(node.range().start).row;
    let mut element = format!("<{}", node.tag_name().name());
    for key in ["type", "id", "name"] {
        if let Some(value) = node.attribute(key) {
            let _ = write!(element, " {key}=\"{value}\"");
        }
    }
    element.push('>');
    Place { line, element }
}

/// The element children of `node`, in file order; comments and layout
/// whitespace are not part of the model.
pub(crate) fn elements<'a, 'input>(
    node: Node<'a, 'input>,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(Node::is_element)
}

/// The trimmed text inside `node`.
pub(crate) fn text<'a>(node: Node<'a, '_>) -> &'a str {
    node.text().map_or("", str::trim)
}

/// An error naming `node` for carrying something outside the model.
pub(crate) fn unsupported(node: Node<'_, '_>, what: impl Into<String>) -> Error {
    Error::Unsupported {
        at: place(node),
        what: what.into(),
    }
}

/// An element's attributes, read so that none can go unread: [`Attrs::finish`]
/// fails on any attribute nothing asked for, so an attribute this crate does
/// not understand is an error instead of lost meaning.
pub(crate) struct Attrs<'a, 'input> {
    node: Node<'a, 'input>,
    read: Vec<&'static str>,
}

impl<'a, 'input> Attrs<'a, 'input> {
    pub(crate) fn new(node: Node<'a, 'input>) -> Self {
        Self {
            node,
            read: DISPLAY.to_vec(),
        }
    }

    pub(crate) fn at(&self) -> Place {
        place(self.node)
    }

    fn malformed(&self, what: String) -> Error {
        Error::Malformed {
            at: self.at(),
            what,
        }
    }

    /// Marks attributes as understood and deliberately unused.
    pub(crate) fn ignore(&mut self, names: &[&'static str]) {
        self.read.extend_from_slice(names);
    }

    pub(crate) fn opt(&mut self, name: &'static str) -> Option<&'a str> {
        self.read.push(name);
        self.node.attribute(name)
    }

    pub(crate) fn req(&mut self, name: &'static str) -> Result<&'a str, Error> {
        self.opt(name)
            .ok_or_else(|| self.malformed(format!("no `{name}` attribute")))
    }

    pub(crate) fn id(&mut self, name: &'static str) -> Result<Id, Error> {
        self.count(name)
    }

    pub(crate) fn count(&mut self, name: &'static str) -> Result<u32, Error> {
        let raw = self.req(name)?;
        raw.parse()
            .map_err(|_| self.malformed(format!("`{name}=\"{raw}\"` is not a whole number")))
    }

    pub(crate) fn number(&mut self, name: &'static str) -> Result<f64, Error> {
        let raw = self.req(name)?;
        raw.parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| self.malformed(format!("`{name}=\"{raw}\"` is not a number")))
    }

    pub(crate) fn formula(&mut self, name: &'static str) -> Result<Formula, Error> {
        let raw = self.req(name)?;
        Formula::parse(raw).map_err(|source| Error::Formula {
            at: self.at(),
            attribute: name,
            formula: raw.to_owned(),
            source,
        })
    }

    /// A boolean, written `true`/`false` or `1`/`0` depending on the
    /// attribute; `None` when absent.
    pub(crate) fn flag(&mut self, name: &'static str) -> Result<Option<bool>, Error> {
        match self.opt(name) {
            None => Ok(None),
            Some("true" | "1") => Ok(Some(true)),
            Some("false" | "0") => Ok(Some(false)),
            Some(raw) => Err(self.malformed(format!("`{name}=\"{raw}\"` is not a boolean"))),
        }
    }

    /// Requires `name` to hold exactly `value`; any other value is a variant
    /// this crate does not model.
    pub(crate) fn expect(&mut self, name: &'static str, value: &str) -> Result<(), Error> {
        match self.opt(name) {
            Some(found) if found == value => Ok(()),
            Some(found) => Err(unsupported(self.node, format!("`{name}=\"{found}\"`"))),
            None => Err(self.malformed(format!("no `{name}` attribute"))),
        }
    }

    pub(crate) fn finish(self) -> Result<(), Error> {
        match self
            .node
            .attributes()
            .find(|attribute| !self.read.contains(&attribute.name()))
        {
            Some(attribute) => Err(unsupported(
                self.node,
                format!("the attribute `{}`", attribute.name()),
            )),
            None => Ok(()),
        }
    }
}
