use std::collections::{BTreeMap, BTreeSet};

use toile_doc::{Binding, Doc, MeasureSet, PointKey, Variable};

use super::names::Names;
use super::outline::Tract;
use super::report::{FrozenNote, HelperNote, LengthNote, Measure, Report, VariableNote};
use super::source::Source;
use super::translate::Translator;
use super::{Frozen, Product};
use crate::eval::env::SPLINE;
use crate::{Block, Construction, Error, Id, ObjectKind, Piece};

/// The internal lines a piece is drawn with and not cut on.
mod inner;
/// The points and the pieces of the document.
mod piece;
/// Putting a point of the pattern into the document, and remembering it.
mod place;

/// One piece of the pattern and the outline the translation walked for it.
pub(super) struct Walked<'p> {
    pub(super) block: &'p Block,
    pub(super) piece: &'p Piece,
    pub(super) tracts: Vec<Tract>,
}

/// What the pieces put into the document besides themselves.
#[derive(Default)]
struct Placed {
    /// The document point each construction point became, shared by every
    /// piece that runs through it.
    points: BTreeMap<Id, PointKey>,
    sources: BTreeMap<PointKey, Source>,
    frozen: BTreeMap<PointKey, BTreeSet<Frozen>>,
}

pub(super) fn assemble(
    mut tr: Translator<'_>,
    walked: &[Walked<'_>],
    name: &str,
) -> Result<Product, Error> {
    let mut variables = Vec::new();
    for variable in &tr.pattern.variables {
        let formula = tr.names.respell(&variable.formula)?;
        let binding = binding(&formula, &variable.name)?;
        let toile = tr
            .names
            .variable(&variable.name)
            .unwrap_or_default()
            .to_owned();
        let note = VariableNote {
            seamly: variable.name.clone(),
            toile,
            formula,
            description: variable.description.clone(),
        };
        variables.push((note, binding));
    }
    let mut doc = Doc::new(tape(&tr.names, name));
    for (note, binding) in &variables {
        add_variable(&mut doc, &note.toile, binding.clone())?;
    }
    let mut placed = Placed::default();
    let mut pieces = Vec::new();
    for one in walked {
        pieces.push(piece::place(&mut tr, &mut doc, one, &mut placed)?);
    }
    let mut helpers = Vec::new();
    for helper in &tr.helpers {
        let formula = helper.value.source();
        add_variable(&mut doc, &helper.name, binding(&formula, &helper.name)?)?;
        helpers.push(HelperNote {
            name: helper.name.clone(),
            formula,
            stands_for: helper.stands_for.clone(),
        });
    }
    let (mapped, carried, left_out) = body(&mut doc, &tr.names);
    let frozen = frozen(&tr, &doc, &placed)?;
    let directions = tr
        .directions
        .iter()
        .map(|&id| tr.point_name(id).map(str::to_owned))
        .collect::<Result<_, _>>()?;
    let constructed = tr.pattern.objects().filter(|o| o.name().is_some()).count();
    let lengths = lengths(&tr)?;
    let report = Report {
        body: name.to_owned(),
        mapped,
        carried,
        left_out,
        variables: variables.into_iter().map(|(note, _)| note).collect(),
        helpers,
        frozen,
        directions,
        lengths,
        pieces,
        construction: (constructed, tr.defined()),
    };
    Ok(Product {
        doc,
        report,
        sources: placed.sources,
        frozen: placed.frozen,
    })
}

/// Adds a variable, refusing a name the document already gives another: the
/// two a new document starts with, which a pattern could name too.
fn add_variable(doc: &mut Doc, name: &str, value: Binding) -> Result<(), Error> {
    if doc.variable_named(name).is_some() {
        return Err(Error::Product(format!(
            "the variable `{name}` is one a Toile document already has"
        )));
    }
    doc.variables.insert(Variable::new(name, value));
    Ok(())
}

/// The binding a translated formula reads back as.
fn binding(source: &str, what: &str) -> Result<Binding, Error> {
    Binding::parse(source).map_err(|error| {
        Error::Product(format!(
            "the formula written for `{what}`, `{source}`, does not read back: {error}"
        ))
    })
}

/// Every measurement the body could be asked for, under the Toile name it
/// would go by: the tape the translation writes its formulas against.
///
/// Wider than the body the product keeps, and it has to be. Which measurements
/// the product reads is the answer to which formulas it carries, and the last
/// of those is written when the last piece is drawn — so the tape stays whole
/// until then and [`body`] cuts it down afterwards.
fn tape(names: &Names, name: &str) -> MeasureSet {
    let values = names
        .measurements()
        .values()
        .filter_map(|m| Some((m.toile.as_deref()?, m.value)));
    MeasureSet::new(name, values)
}

/// The product's body: every measurement with a catalogue name, and every
/// other one a formula reads; the rest left out of the document and said so in
/// the report.
///
/// Asked after the pieces are placed, because a measurement is read for the
/// first time by whichever formula cites it and a piece's internal lines are
/// the last formulas written. Sorted any earlier, a measurement only an
/// internal line reads counted as read by nothing: the body went out without
/// it, and the line's own points then cited a name the document did not carry,
/// so they resolved nowhere and the drawing lost the line.
fn body(doc: &mut Doc, names: &Names) -> (Vec<Measure>, Vec<Measure>, Vec<Measure>) {
    let (mut mapped, mut carried, mut left_out) = (Vec::new(), Vec::new(), Vec::new());
    for (seamly, measure) in names.measurements() {
        let note = Measure {
            seamly: seamly.clone(),
            toile: measure.toile.clone(),
            value: measure.value,
        };
        if measure.mapped {
            mapped.push(note);
        } else if names.is_read(seamly) {
            carried.push(note);
        } else {
            left_out.push(Measure {
                toile: None,
                ..note
            });
        }
    }
    let held: BTreeSet<&str> = mapped
        .iter()
        .chain(&carried)
        .filter_map(|m| m.toile.as_deref())
        .collect();
    // Not `if let`: a body this never found would ship the whole tape under a
    // report that lists half of it as left out, and say nothing.
    let set = doc
        .mannequins
        .get_mut(doc.resolve_with)
        .expect("Doc::new inserted the body the document resolves against");
    set.values.retain(|name, _| held.contains(name.as_str()));
    (mapped, carried, left_out)
}

/// Every frozen quantity with what it reaches in the product, and every cut
/// point of the pattern the product did not need, which would be.
fn frozen(tr: &Translator<'_>, doc: &Doc, placed: &Placed) -> Result<Vec<FrozenNote>, Error> {
    let mut notes = Vec::new();
    for (&frozen, &value) in &tr.frozen {
        let name = match frozen {
            Frozen::SplineExcess(id) => spline_name(tr, id)?,
            Frozen::CutParameter(id) => tr.point_name(id)?.to_owned(),
        };
        let reaches = placed
            .frozen
            .iter()
            .filter(|(_, set)| set.contains(&frozen))
            .filter_map(|(key, _)| doc.points.get(*key)?.label.clone())
            .collect();
        notes.push(FrozenNote {
            frozen,
            name,
            value,
            reaches,
        });
    }
    for object in tr.pattern.objects() {
        let ObjectKind::Point {
            name,
            construction: Construction::CutSpline { .. },
            ..
        } = &object.kind
        else {
            continue;
        };
        let frozen = Frozen::CutParameter(object.id);
        if let (false, Some(cut)) = (
            tr.frozen.contains_key(&frozen),
            tr.reference.cuts.get(&object.id),
        ) {
            notes.push(FrozenNote {
                frozen,
                name: name.clone(),
                value: cut.t,
                reaches: Vec::new(),
            });
        }
    }
    Ok(notes)
}

/// Every spline the file writes a length on, beside the curve's own and the
/// points that cite it.
fn lengths(tr: &Translator<'_>) -> Result<Vec<LengthNote>, Error> {
    let mut notes = Vec::new();
    for object in tr.pattern.objects() {
        let ObjectKind::Spline(spline) = &object.kind else {
            continue;
        };
        let (Some(written), Some(curve)) =
            (spline.written_length, tr.reference.splines.get(&object.id))
        else {
            continue;
        };
        let name = spline_name(tr, object.id)?;
        let cited_by = tr
            .pattern
            .objects()
            .filter(|o| {
                o.formulas()
                    .iter()
                    .any(|f| f.names().contains(name.as_str()))
            })
            .filter_map(|o| o.name().map(str::to_owned))
            .collect();
        notes.push(LengthNote {
            name,
            written,
            arc: curve.length(),
            cited_by,
        });
    }
    Ok(notes)
}

/// `Spl_` and the names of a spline's two points.
fn spline_name(tr: &Translator<'_>, id: Id) -> Result<String, Error> {
    let ObjectKind::Spline(spline) = &tr.object(id)?.1.kind else {
        return Err(Error::Product(format!("object {id} is not a spline")));
    };
    Ok(format!(
        "{SPLINE}{}_{}",
        tr.point_name(spline.start)?,
        tr.point_name(spline.end)?
    ))
}
