use std::collections::{BTreeMap, BTreeSet};

use toile_doc::{Doc, PointKey};

use crate::{Error, Evaluation, Frozen, Measurements, Pattern, SplineLength};

/// The document put together from the translated pieces.
mod assemble;
/// The `Line_`, `AngleLine_` and `Spl_` names drawings define.
mod drawn;
/// Seamly names respelled as Toile names.
mod names;
/// Piece outlines as runs of corners and curves.
mod outline;
/// The product's body as a person in the user's library.
mod persona;
/// What the import tells a person.
mod report;
/// How finely a curve is flattened, and which way a contour runs.
mod samples;
/// Where each point of the product comes from.
mod source;
/// Constructions as formulas.
mod translate;

pub use report::{
    Carried, FrozenNote, HelperKind, HelperNote, InternalNote, LengthNote, Measure, NotchNote,
    PieceNote, Refusal, Report, VariableNote,
};
pub use source::{Curve, End, Source};

/// A Seamly pattern as a Toile product, with what the translation could not
/// carry.
#[derive(Debug, Clone, PartialEq)]
pub struct Product {
    /// The product.
    pub doc: Doc,
    /// What the import did, for a person to read.
    pub report: Report,
    /// Where each point of the product comes from in the pattern.
    pub sources: BTreeMap<PointKey, Source>,
    /// What each point depends on that was frozen at the imported body; a
    /// point absent here follows the body exactly as the pattern does.
    pub frozen: BTreeMap<PointKey, BTreeSet<Frozen>>,
}

/// Translates `pattern` into a Toile product whose one body is `body`,
/// named `name`.
///
/// Every corner and every handle becomes a point whose formulas are the
/// construction unrolled over the measurements and the variables, so the
/// product follows a change of body the way the pattern does. What cannot
/// follow is frozen at `body` and said in the report, and so is everything
/// the product has no place for.
///
/// # Errors
///
/// Whatever stops the pattern from evaluating for `body`, a construction the
/// translation does not model, and a name or an outline a Toile product
/// cannot hold.
pub fn import(pattern: &Pattern, body: &Measurements, name: &str) -> Result<Product, Error> {
    let reference = Evaluation::new(pattern, body, SplineLength::ArcLength)?;
    let names = names::Names::new(pattern, body)?;
    let mut translator = translate::Translator::new(pattern, &reference, names);
    let mut walked = Vec::new();
    for block in &pattern.blocks {
        for piece in &block.pieces {
            let tracts = outline::walk(&mut translator, block, piece)?;
            walked.push(assemble::Walked {
                block,
                piece,
                tracts,
            });
        }
    }
    assemble::assemble(translator, &walked, name)
}
