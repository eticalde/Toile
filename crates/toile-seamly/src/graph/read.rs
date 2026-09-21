use std::collections::BTreeMap;

use roxmltree::{Document, Node};

use super::index::Index;
use super::{calc, modeling, pieces};
use crate::xml::{Attrs, elements, text, unsupported};
use crate::{Block, Comment, Error, Pattern, Variable};

/// How Seamly opens the note it stamps on every pattern it saves.
const SIGNATURE: &str = "Pattern created with Seamly2D";

impl Pattern {
    /// Reads the text of a Seamly pattern file drafted in centimetres.
    ///
    /// # Errors
    ///
    /// Malformed XML; a unit other than centimetres; any element, object
    /// type or attribute outside the model, named with its line; a reference
    /// to an id that is not defined above or is of the wrong kind; a formula
    /// that does not parse.
    pub fn parse(xml: &str) -> Result<Self, Error> {
        let doc = Document::parse(xml).map_err(|e| Error::Xml(e.to_string()))?;
        let root = doc.root_element();
        if root.tag_name().name() != "pattern" {
            return Err(unsupported(root, "a file other than a `<pattern>`"));
        }
        let mut pattern = Pattern {
            version: String::new(),
            measurements: None,
            variables: Vec::new(),
            blocks: Vec::new(),
            comments: comments(&doc),
        };
        let mut index = Index::default();
        for child in elements(root) {
            match child.tag_name().name() {
                "version" => text(child).clone_into(&mut pattern.version),
                "unit" => {
                    if text(child) != "cm" {
                        return Err(unsupported(child, format!("the unit `{}`", text(child))));
                    }
                }
                "measurements" => {
                    pattern.measurements = Some(text(child).to_owned()).filter(|p| !p.is_empty());
                }
                "variables" => pattern.variables = variables(child)?,
                "draftBlock" => pattern.blocks.push(block(child, &mut index)?),
                // Free text for people; nothing downstream reads it.
                "description" | "notes" => {}
                _ => return Err(unsupported(child, "this element")),
            }
        }
        Ok(pattern)
    }
}

/// Where every comment of the file sits, prolog included.
fn comments(doc: &Document<'_>) -> Vec<Comment> {
    let root = doc.root_element();
    doc.root()
        .descendants()
        .filter(Node::is_comment)
        .map(|node| Comment {
            line: doc.text_pos_at(node.range().start).row,
            block: node
                .ancestors()
                .find(|a| a.has_tag_name("draftBlock"))
                .and_then(|block| block.attribute("name"))
                .map(str::to_owned),
            signature: node.parent() == Some(root)
                && node
                    .text()
                    .is_some_and(|text| text.trim_start().starts_with(SIGNATURE)),
        })
        .collect()
}

fn variables(node: Node<'_, '_>) -> Result<Vec<Variable>, Error> {
    let mut variables: Vec<Variable> = Vec::new();
    for child in elements(node) {
        if child.tag_name().name() != "variable" {
            return Err(unsupported(child, "this element"));
        }
        let mut attrs = Attrs::new(child);
        let variable = Variable {
            name: attrs.req("name")?.to_owned(),
            formula: attrs.formula("formula")?,
            description: attrs.opt("description").unwrap_or_default().to_owned(),
            at: attrs.at(),
        };
        attrs.finish()?;
        if variables.iter().any(|v| v.name == variable.name) {
            return Err(Error::Malformed {
                what: format!("`{}` is defined twice", variable.name),
                at: variable.at,
            });
        }
        variables.push(variable);
    }
    Ok(variables)
}

fn block(node: Node<'_, '_>, index: &mut Index) -> Result<Block, Error> {
    let mut attrs = Attrs::new(node);
    let name = attrs.req("name")?.to_owned();
    attrs.finish()?;
    let mut block = Block {
        name,
        objects: Vec::new(),
        copies: BTreeMap::new(),
        internal_paths: Vec::new(),
        pieces: Vec::new(),
    };
    for child in elements(node) {
        match child.tag_name().name() {
            "calculation" => {
                for element in elements(child) {
                    block.objects.push(calc::object(element, index)?);
                }
            }
            "modeling" => modeling::read(child, index, &mut block)?,
            "pieces" => pieces::read(child, index, &mut block)?,
            "groups" | "images" => {
                if let Some(first) = elements(child).next() {
                    return Err(unsupported(first, "this element"));
                }
            }
            _ => return Err(unsupported(child, "this element")),
        }
    }
    Ok(block)
}
