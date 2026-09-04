use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::{Dart, MeasureSet, Notch, Piece, Pin, Point, Seam, Symmetry, Variable};

/// The stable identity of one entity of the document.
///
/// The traits are written by hand: a derive would ask `T` for them, and the
/// tag carries no value. `PhantomData<fn() -> T>` rather than `PhantomData<T>`
/// so that a key is `Send` and `Sync` whatever `T` is.
pub struct Key<T> {
    index: u32,
    generation: u32,
    tag: PhantomData<fn() -> T>,
}

impl<T> Key<T> {
    /// The key an entry stored at `index` under `generation` answers to.
    ///
    /// Reading a key back from a stored id is what this is for; a key that
    /// names a live entry comes from the arena that issued it.
    pub const fn new(index: u32, generation: u32) -> Key<T> {
        Key {
            index,
            generation,
            tag: PhantomData,
        }
    }

    /// Where in its arena the entry sits. Indices are never recycled.
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Which occupant of that slot the key means.
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

impl<T> Clone for Key<T> {
    fn clone(&self) -> Key<T> {
        *self
    }
}

impl<T> Copy for Key<T> {}

impl<T> PartialEq for Key<T> {
    fn eq(&self, other: &Key<T>) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Key<T> {}

impl<T> Ord for Key<T> {
    fn cmp(&self, other: &Key<T>) -> Ordering {
        (self.index, self.generation).cmp(&(other.index, other.generation))
    }
}

impl<T> PartialOrd for Key<T> {
    fn partial_cmp(&self, other: &Key<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Hash for Key<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> fmt::Debug for Key<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key({}.{})", self.index, self.generation)
    }
}

/// Which key a created entity takes.
///
/// The inverse of a delete carries the key the entity had, so undoing a delete
/// puts the same key back and every reference to it stays live.
pub enum Identity<T> {
    /// A key the arena has not issued yet.
    New,
    /// The key the entity carried before it was removed.
    Restored(Key<T>),
}

impl<T> Clone for Identity<T> {
    fn clone(&self) -> Identity<T> {
        *self
    }
}

impl<T> Copy for Identity<T> {}

impl<T> PartialEq for Identity<T> {
    fn eq(&self, other: &Identity<T>) -> bool {
        match (self, other) {
            (Identity::New, Identity::New) => true,
            (Identity::Restored(a), Identity::Restored(b)) => a == b,
            _ => false,
        }
    }
}

impl<T> Eq for Identity<T> {}

impl<T> fmt::Debug for Identity<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Identity::New => f.write_str("New"),
            Identity::Restored(key) => write!(f, "Restored({key:?})"),
        }
    }
}

/// The identity of a piece.
pub type PieceKey = Key<Piece>;
/// The identity of a control point.
pub type PointKey = Key<Point>;
/// The identity of a seam.
pub type SeamKey = Key<Seam>;
/// The identity of a notch.
pub type NotchKey = Key<Notch>;
/// The identity of a dart.
pub type DartKey = Key<Dart>;
/// The identity of a symmetry.
pub type SymmetryKey = Key<Symmetry>;
/// The identity of a pin.
pub type PinKey = Key<Pin>;
/// The identity of a pattern variable.
pub type VariableKey = Key<Variable>;
/// The identity of a measure set.
pub type MannequinKey = Key<MeasureSet>;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn keys_of_the_same_entry_are_equal() {
        assert_eq!(PointKey::new(3, 0), PointKey::new(3, 0));
        assert_ne!(PointKey::new(3, 0), PointKey::new(3, 1));
        assert_ne!(PointKey::new(3, 0), PointKey::new(4, 0));
    }

    #[test]
    fn keys_order_by_index_before_generation() {
        let mut set = BTreeSet::new();
        set.insert(PointKey::new(2, 9));
        set.insert(PointKey::new(1, 4));
        set.insert(PointKey::new(1, 0));
        let order: Vec<(u32, u32)> = set.iter().map(|k| (k.index(), k.generation())).collect();
        assert_eq!(order, [(1, 0), (1, 4), (2, 9)]);
    }

    #[test]
    fn a_key_reads_as_its_stored_id() {
        assert_eq!(format!("{:?}", PointKey::new(3, 0)), "Key(3.0)");
    }

    #[test]
    fn an_identity_is_new_or_a_key() {
        assert_eq!(Identity::<Point>::New, Identity::New);
        assert_ne!(Identity::New, Identity::Restored(PointKey::new(0, 0)));
        assert_eq!(
            format!("{:?}", Identity::Restored(PointKey::new(1, 0))),
            "Restored(Key(1.0))"
        );
    }
}
