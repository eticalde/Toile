use roxmltree::Document;

use crate::xml::{Attrs, elements, place, text, unsupported};
use crate::{Error, Formula, FormulaError};

/// A body's measurements, as a Seamly individual measurement file holds them.
///
/// The file's `<personal>` block — name, birth date, gender, email — is never
/// read: no field of this type could hold it.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurements {
    entries: Vec<Measurement>,
}

/// One measurement, under the name the file gives it.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// The name formulas cite it by.
    pub name: String,
    /// The value as written: a number, or a formula over the measurements
    /// above it.
    pub formula: Formula,
    /// The value in centimetres.
    pub value: f64,
}

impl Measurements {
    /// Reads the text of an individual measurement file in centimetres.
    ///
    /// # Errors
    ///
    /// Malformed XML, a unit other than centimetres, an element outside the
    /// model, a repeated name, or a formula citing a measurement below it.
    pub fn parse(xml: &str) -> Result<Self, Error> {
        let doc = Document::parse(xml).map_err(|e| Error::Xml(e.to_string()))?;
        let root = doc.root_element();
        if root.tag_name().name() != "smis" {
            return Err(unsupported(root, "a measurement file other than `<smis>`"));
        }
        let mut entries = Vec::new();
        for child in elements(root) {
            match child.tag_name().name() {
                "unit" => {
                    if text(child) != "cm" {
                        return Err(unsupported(child, format!("the unit `{}`", text(child))));
                    }
                }
                "body-measurements" => {
                    for m in elements(child) {
                        let entry = measurement(m, &entries)?;
                        entries.push(entry);
                    }
                }
                // `personal` is private by design: nothing here looks inside.
                "personal" | "version" | "read-only" | "notes" | "pm_system" => {}
                _ => return Err(unsupported(child, "this element")),
            }
        }
        Ok(Self { entries })
    }

    /// Every measurement, in file order.
    pub fn entries(&self) -> &[Measurement] {
        &self.entries
    }

    /// The value of the measurement called `name`.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.entries
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.value)
    }

    /// The same body with `name` measuring `value` instead, and every
    /// measurement written as a formula over it measured again.
    ///
    /// `None` when the body has no measurement of that name.
    pub fn with_value(&self, name: &str, value: f64) -> Option<Self> {
        self.get(name)?;
        // Written as the number it now is, so a later change of another
        // measurement measures this one again as that number.
        let written = Formula::parse(&format!("{value}")).ok()?;
        let mut entries: Vec<Measurement> = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            if entry.name == name {
                entries.push(Measurement {
                    name: entry.name.clone(),
                    formula: written.clone(),
                    value,
                });
                continue;
            }
            let lookup = |cited: &str| {
                entries
                    .iter()
                    .find(|m| m.name == cited)
                    .map(|m| m.value)
                    .ok_or_else(|| FormulaError::UnknownName(cited.to_owned()))
            };
            // It evaluated over these very names when the file was read.
            let value = entry.formula.eval(&lookup).ok()?;
            entries.push(Measurement {
                value,
                ..entry.clone()
            });
        }
        Some(Self { entries })
    }
}

fn measurement(node: roxmltree::Node<'_, '_>, above: &[Measurement]) -> Result<Measurement, Error> {
    if node.tag_name().name() != "m" {
        return Err(unsupported(node, "this element"));
    }
    let mut attrs = Attrs::new(node);
    let name = attrs.req("name")?.to_owned();
    let formula = attrs.formula("value")?;
    attrs.ignore(&["full_name", "description"]);
    attrs.finish()?;
    if above.iter().any(|m| m.name == name) {
        return Err(Error::Malformed {
            at: place(node),
            what: format!("`{name}` is measured twice"),
        });
    }
    // File order is evaluation order, so a formula may cite only what is
    // above it and no cycle can form.
    let lookup = |cited: &str| {
        above
            .iter()
            .find(|m| m.name == cited)
            .map(|m| m.value)
            .ok_or_else(|| FormulaError::UnknownName(cited.to_owned()))
    };
    let value = formula.eval(&lookup).map_err(|source| Error::Formula {
        at: place(node),
        attribute: "value",
        formula: formula.source().to_owned(),
        source,
    })?;
    Ok(Measurement {
        name,
        formula,
        value,
    })
}
