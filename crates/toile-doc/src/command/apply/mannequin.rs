use std::collections::BTreeMap;

use super::Naming;
use crate::measure::finite;
use crate::{
    Applied, BodyShape, ChangeClass, Command, Doc, DocError, Identity, MannequinKey, MeasureSet,
    Origin,
};

/// Writes a measurement the body already carries.
///
/// Which measurements a body has is the mannequin's own business; an edit that
/// could introduce one could not be undone by another edit of the same shape.
pub(crate) fn set_measure(
    doc: &mut Doc,
    mannequin: MannequinKey,
    name: String,
    to: f64,
) -> Result<Applied, DocError> {
    // JSON has no NaN or infinity: the writer would spell one as `null` and the
    // next open would refuse the whole file.
    if !to.is_finite() {
        return Err(DocError::NonFinite(name));
    }
    let touched = if mannequin == doc.resolve_with {
        doc.piece_keys()
    } else {
        Vec::new()
    };
    let set = doc
        .mannequins
        .get_mut(mannequin)
        .ok_or_else(|| DocError::stale(mannequin))?;
    let slot = set
        .values
        .get_mut(&name)
        .ok_or_else(|| DocError::UnknownMeasure(name.clone()))?;
    let from = std::mem::replace(slot, to);
    Ok(Applied {
        inverse: Command::SetMeasure {
            mannequin,
            name,
            to: from,
        },
        touched,
        class: ChangeClass::Shape,
    })
}

pub(crate) fn resolve_with(doc: &mut Doc, mannequin: MannequinKey) -> Result<Applied, DocError> {
    if doc.mannequins.get(mannequin).is_none() {
        return Err(DocError::stale(mannequin));
    }
    let from = std::mem::replace(&mut doc.resolve_with, mannequin);
    Ok(Applied {
        inverse: Command::ResolveWith { mannequin: from },
        touched: doc.piece_keys(),
        class: ChangeClass::Shape,
    })
}

/// Gives a body its phenotype, or takes it away. No piece is touched.
pub(crate) fn set_phenotype(
    doc: &mut Doc,
    mannequin: MannequinKey,
    to: Option<BodyShape>,
) -> Result<Applied, DocError> {
    if let Some(shape) = &to {
        shape.check()?;
    }
    let set = doc
        .mannequins
        .get_mut(mannequin)
        .ok_or_else(|| DocError::stale(mannequin))?;
    let from = std::mem::replace(&mut set.phenotype, to);
    Ok(Applied {
        inverse: Command::SetPhenotype {
            mannequin,
            to: from,
        },
        touched: Vec::new(),
        class: ChangeClass::Sim,
    })
}

/// Adds a body, refusing a name another body carries.
///
/// A body is chosen by its name, in the chooser and on the command line, so
/// two under one name would leave the choice to key order.
pub(crate) fn add_mannequin(
    doc: &mut Doc,
    identity: Identity<MeasureSet>,
    mannequin: MeasureSet,
    naming: Naming,
) -> Result<Applied, DocError> {
    if naming == Naming::Checked && doc.mannequin_named(&mannequin.name).is_some() {
        return Err(DocError::DuplicateMannequinName(mannequin.name));
    }
    let key = match identity {
        Identity::New => doc.mannequins.insert(mannequin),
        Identity::Restored(key) => {
            doc.mannequins.restore(key, mannequin)?;
            key
        }
    };
    Ok(Applied {
        inverse: Command::RemoveMannequin { mannequin: key },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

/// Takes a body away, refusing the one the pattern resolves against.
///
/// The refusal is what keeps `resolve_with` from ever naming nothing. The
/// history never runs into it: a body can only be chosen after it was added,
/// so undo takes the choice back before it takes the body.
pub(crate) fn remove_mannequin(
    doc: &mut Doc,
    mannequin: MannequinKey,
) -> Result<Applied, DocError> {
    let held = doc
        .mannequins
        .get(mannequin)
        .ok_or_else(|| DocError::stale(mannequin))?;
    if mannequin == doc.resolve_with {
        return Err(DocError::BodyInUse(held.name.clone()));
    }
    let held = doc.mannequins.remove(mannequin)?;
    Ok(Applied {
        inverse: Command::AddMannequin {
            identity: Identity::Restored(mannequin),
            mannequin: held,
        },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

/// Renames a body, refusing a name another body carries.
pub(crate) fn rename_mannequin(
    doc: &mut Doc,
    mannequin: MannequinKey,
    to: String,
    naming: Naming,
) -> Result<Applied, DocError> {
    if naming == Naming::Checked
        && let Some(other) = doc.mannequin_named(&to)
        && other != mannequin
    {
        return Err(DocError::DuplicateMannequinName(to));
    }
    let set = doc
        .mannequins
        .get_mut(mannequin)
        .ok_or_else(|| DocError::stale(mannequin))?;
    let from = std::mem::replace(&mut set.name, to);
    Ok(Applied {
        inverse: Command::RenameMannequin {
            mannequin,
            to: from,
        },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

/// Replaces what a body carries from the library — its tape, its phenotype
/// and its link — and keeps its name.
///
/// Every check runs before anything is written, so a refusal leaves the body
/// as it was. The name stays the document's: a body is chosen by the name its
/// document gave it, and an edit that never writes a name cannot give two
/// bodies the same one.
pub(crate) fn refresh_mannequin(
    doc: &mut Doc,
    mannequin: MannequinKey,
    values: BTreeMap<String, f64>,
    phenotype: Option<BodyShape>,
    origin: Option<Origin>,
) -> Result<Applied, DocError> {
    finite(&values)?;
    if let Some(shape) = &phenotype {
        shape.check()?;
    }
    if let Some(link) = &origin {
        link.check()?;
    }
    let touched = if mannequin == doc.resolve_with {
        doc.piece_keys()
    } else {
        Vec::new()
    };
    let set = doc
        .mannequins
        .get_mut(mannequin)
        .ok_or_else(|| DocError::stale(mannequin))?;
    let inverse = Command::RefreshMannequin {
        mannequin,
        values: std::mem::replace(&mut set.values, values),
        phenotype: std::mem::replace(&mut set.phenotype, phenotype),
        origin: std::mem::replace(&mut set.origin, origin),
    };
    Ok(Applied {
        inverse,
        touched,
        class: ChangeClass::Shape,
    })
}
