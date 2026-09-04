use thiserror::Error;

use crate::Key;

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
    /// A measurement the chosen measure set does not carry.
    #[error("the measure set has no measurement named `{0}`")]
    UnknownMeasure(String),
    /// A label another point of the same piece already shows.
    #[error("the piece already shows a point named `{0}`")]
    DuplicateLabel(String),
    /// A name another piece already carries.
    #[error("the document already has a piece named `{0}`")]
    DuplicatePieceName(String),
    /// A point the piece's contour does not run through.
    #[error("the piece has no node at that point")]
    NoSuchNode,
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
