use thiserror::Error;

use crate::{Elastic, Key, SAMPLES};

/// What can go wrong while reading or writing the document.
///
/// Every variant is bad input rather than a broken invariant: keys reach the
/// document from the interface, so a dead one is an error to report and never
/// a panic.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DocError {
    /// A key no live entry answers to.
    #[error("`{entity}` has no entry {index}.{generation}")]
    StaleKey {
        /// The entity the key names.
        entity: &'static str,
        /// The index the key carries.
        index: u32,
        /// The generation the key carries.
        generation: u32,
    },
    /// A restore aimed at an entry that is still taken.
    #[error("`{entity}` already holds an entry at {index}")]
    Occupied {
        /// The entity the key names.
        entity: &'static str,
        /// The index the key carries.
        index: u32,
    },
    /// A stored slot count no pattern could have reached.
    #[error("`{entity}` claims {issued} slots, far more than a pattern holds")]
    ImplausibleStore {
        /// The entity the store holds.
        entity: &'static str,
        /// The count of slots the file claims.
        issued: u32,
    },
    /// A measurement the chosen measure set does not carry.
    #[error("the measure set has no measurement named `{0}`")]
    UnknownMeasure(String),
    /// A measurement, a phenotype scale or a placement that is not a finite
    /// number, which the file could not spell and would refuse on the way
    /// back in.
    #[error("`{0}` must be a finite number")]
    NonFinite(String),
    /// A day a session or a link carries that is not written `YYYY-MM-DD`.
    #[error("`{0}` is not a day written as YYYY-MM-DD")]
    NotADay(String),
    /// A stem that could name no library file.
    #[error("`{0}` cannot name a library file: a stem is lowercase letters, digits, `_` and `-`")]
    NotAStem(String),
    /// A fingerprint that is not sixteen lowercase hex digits.
    #[error("`{0}` is not a fingerprint, which is sixteen lowercase hex digits")]
    NotAFingerprint(String),
    /// A label another point of the same piece already shows.
    #[error("the piece already shows a point named `{0}`")]
    DuplicateLabel(String),
    /// A name another piece already carries.
    #[error("the document already has a piece named `{0}`")]
    DuplicatePieceName(String),
    /// A name another mannequin already carries.
    #[error("the document already has a mannequin named `{0}`")]
    DuplicateMannequinName(String),
    /// The mannequin the pattern resolves against, asked to be removed.
    #[error("the pattern resolves against `{0}`, so it cannot be removed")]
    BodyInUse(String),
    /// A point the piece's contour does not run through.
    #[error("the piece has no node at that point")]
    NoSuchNode,
    /// An anchor whose fraction is not one the tract leaving its node can
    /// answer for, which is where a NaN the file cannot spell lands as well.
    #[error("a stretch is anchored at a fraction of its tract outside 0 to 1")]
    AnchorFraction,
    /// A seam side whose two ends sit on different pieces.
    #[error("a seam side has to start and end on one piece")]
    SplitSeamSide,
    /// An elastic whose two ends sit on different pieces.
    #[error("an elastic has to start and end on one piece")]
    SplitElastic,
    /// An internal line anchored to the contour of a piece it is not drawn on.
    #[error("an internal line is drawn on one piece, and anchors only to it")]
    SplitInternalLine,
    /// An axis of symmetry whose two ends sit on different pieces.
    #[error("an axis of symmetry has to start and end on one piece")]
    SplitSymmetry,
    /// An axis of symmetry whose two ends name one place, which is a point and
    /// not a line, and no reflection is defined across it.
    #[error("an axis of symmetry runs between two places, so its ends cannot be one")]
    FoldAxis,
    /// A second axis asked for on a piece that already carries one.
    #[error("the piece is already drawn against an axis of symmetry")]
    AlreadySymmetric,
    /// An internal line that runs through fewer than two places.
    #[error("an internal line runs from one place to another, so it needs two")]
    ShortLine,
    /// A ratio no elastic holds a stretch to, which is where the numbers JSON
    /// cannot spell land as well.
    #[error(
        "an elastic holds a stretch to a ratio in (0, {}] of the drawn length",
        Elastic::MAX_RATIO
    )]
    ElasticRatio,
    /// A strength under the floor an elastic is written at, or not finite.
    #[error(
        "an elastic pulls with a finite strength of at least {}",
        Elastic::MIN_STRENGTH
    )]
    ElasticStrength,
    /// A point another piece still draws itself with.
    #[error("the point still belongs to `{0}`")]
    Shared(String),
    /// A flattening no tract can be asked for.
    #[error("a curve is flattened at {floor} to {ceiling} samples, and this asks for {got}")]
    Sampling {
        /// The count asked for.
        got: u16,
        /// The fewest samples a curve is flattened at.
        floor: u16,
        /// The most.
        ceiling: u16,
    },
    /// An edit whose tool has not been built yet.
    #[error("that edit is not implemented yet")]
    NotYetImplemented,
}

impl DocError {
    /// The error a key that names no live entry produces.
    pub fn stale<T>(key: Key<T>) -> DocError {
        DocError::StaleKey {
            entity: entity_of::<T>(),
            index: key.index(),
            generation: key.generation(),
        }
    }

    /// The error a restore onto a live entry produces.
    pub fn occupied<T>(key: Key<T>) -> DocError {
        DocError::Occupied {
            entity: entity_of::<T>(),
            index: key.index(),
        }
    }

    /// The error a flattening no tract can carry produces.
    pub fn sampling(got: u16) -> DocError {
        DocError::Sampling {
            got,
            floor: SAMPLES.0,
            ceiling: SAMPLES.1,
        }
    }

    /// The error a slot count past what a pattern can hold produces.
    pub fn implausible_store<T>(issued: u32) -> DocError {
        DocError::ImplausibleStore {
            entity: entity_of::<T>(),
            issued,
        }
    }
}

/// The entity's own name, without the module path that leads to it.
fn entity_of<T>() -> &'static str {
    let path = std::any::type_name::<T>();
    match path.rsplit_once("::") {
        Some((_, name)) => name,
        None => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;

    #[test]
    fn a_stale_key_names_its_entity_and_its_id() {
        let error = DocError::stale(Key::<Point>::new(3, 0));
        assert_eq!(error.to_string(), "`Point` has no entry 3.0");
    }

    #[test]
    fn an_occupied_slot_names_the_entity_it_holds() {
        let error = DocError::occupied(Key::<Point>::new(7, 0));
        assert_eq!(error.to_string(), "`Point` already holds an entry at 7");
    }
}
