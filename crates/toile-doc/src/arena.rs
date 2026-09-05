use crate::{DocError, Key};

/// A store that never recycles an index.
///
/// Removing an entry leaves its slot empty and keeps its generation, so
/// restoring it puts the value back under the very same key. That is what
/// makes a delete reversible without breaking every reference to the deleted
/// entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    issued: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// The most slots a stored arena is allowed to claim.
///
/// Rebuilding opens every slot the count names, so an unchecked count lets a
/// file choose how much memory reading it takes. A pattern holds tens to
/// hundreds of entries, orders of magnitude below this, so a file past the cap
/// is corrupt or hostile rather than merely big.
const MAX_ISSUED: u32 = 1_000_000;

impl<T> Arena<T> {
    /// An arena with nothing in it.
    pub fn new() -> Arena<T> {
        Arena {
            slots: Vec::new(),
            issued: 0,
        }
    }

    /// Stores `value` in a slot of its own and hands back its key.
    pub fn insert(&mut self, value: T) -> Key<T> {
        let key = Key::new(self.issued, 0);
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        self.issued += 1;
        key
    }

    /// Puts `value` back under the key it had before it was removed.
    ///
    /// # Errors
    /// `DocError::StaleKey` if no slot answers to the key, and
    /// `DocError::Occupied` if the slot still holds a value.
    pub fn restore(&mut self, key: Key<T>, value: T) -> Result<(), DocError> {
        let slot = self.slot_mut(key)?;
        if slot.value.is_some() {
            return Err(DocError::occupied(key));
        }
        slot.value = Some(value);
        Ok(())
    }

    /// Whether the arena would take a value back under `key`.
    ///
    /// An edit that creates several entries at once has to know its whole plan
    /// fits before it changes anything: half an edit leaves a document no
    /// inverse describes.
    pub(crate) fn is_vacant(&self, key: Key<T>) -> bool {
        match self.slots.get(key.index() as usize) {
            Some(slot) => slot.generation == key.generation() && slot.value.is_none(),
            None => false,
        }
    }

    /// The arena a stored count of slots and a set of entries describe.
    ///
    /// A slot no entry claims is left empty at the generation a slot opens
    /// with, which is the only state an arena that never recycles can leave
    /// one in.
    ///
    /// # Errors
    /// `DocError::ImplausibleStore` for a count of slots past `MAX_ISSUED`,
    /// `DocError::StaleKey` for a key past the slots the arena ever opened,
    /// and `DocError::Occupied` for two entries claiming one key.
    pub(crate) fn rebuild(
        issued: u32,
        entries: impl IntoIterator<Item = (Key<T>, T)>,
    ) -> Result<Arena<T>, DocError> {
        if issued > MAX_ISSUED {
            return Err(DocError::implausible_store::<T>(issued));
        }
        let mut slots = Vec::with_capacity(issued as usize);
        slots.resize_with(issued as usize, || Slot {
            generation: 0,
            value: None,
        });
        let mut arena = Arena { slots, issued };
        for (key, value) in entries {
            let slot = arena
                .slots
                .get_mut(key.index() as usize)
                .ok_or_else(|| DocError::stale(key))?;
            if slot.value.is_some() {
                return Err(DocError::occupied(key));
            }
            slot.generation = key.generation();
            slot.value = Some(value);
        }
        Ok(arena)
    }

    /// Takes the value out, leaving its slot empty and its key restorable.
    ///
    /// # Errors
    /// `DocError::StaleKey` if the key names no live entry.
    pub fn remove(&mut self, key: Key<T>) -> Result<T, DocError> {
        self.slot_mut(key)?
            .value
            .take()
            .ok_or_else(|| DocError::stale(key))
    }

    /// The value the key names, if the key still names one.
    pub fn get(&self, key: Key<T>) -> Option<&T> {
        let slot = self.slots.get(key.index() as usize)?;
        if slot.generation == key.generation() {
            slot.value.as_ref()
        } else {
            None
        }
    }

    /// The value the key names, to be written.
    pub fn get_mut(&mut self, key: Key<T>) -> Option<&mut T> {
        let slot = self.slots.get_mut(key.index() as usize)?;
        if slot.generation == key.generation() {
            slot.value.as_mut()
        } else {
            None
        }
    }

    /// Every live entry with its key, in index order.
    pub fn iter(&self) -> impl Iterator<Item = (Key<T>, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            let value = slot.value.as_ref()?;
            Some((Key::new(index as u32, slot.generation), value))
        })
    }

    /// Every live key, in index order.
    pub fn keys(&self) -> impl Iterator<Item = Key<T>> {
        self.iter().map(|(key, _)| key)
    }

    /// How many entries are live.
    pub fn len(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.value.is_some())
            .count()
    }

    /// Whether no entry is live.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// How many slots the arena has ever opened, live or not.
    ///
    /// A reader that skips the empty slots needs this to know where the next
    /// insertion lands, so that reloading a document cannot collide with a key
    /// that is still in use.
    pub fn issued(&self) -> u32 {
        self.issued
    }

    fn slot_mut(&mut self, key: Key<T>) -> Result<&mut Slot<T>, DocError> {
        match self.slots.get_mut(key.index() as usize) {
            Some(slot) if slot.generation == key.generation() => Ok(slot),
            _ => Err(DocError::stale(key)),
        }
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Arena<T> {
        Arena::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn letters() -> (Arena<String>, Vec<Key<String>>) {
        let mut arena = Arena::new();
        let keys = ["a", "b", "c"]
            .into_iter()
            .map(|value| arena.insert(value.to_owned()))
            .collect();
        (arena, keys)
    }

    #[test]
    fn an_empty_arena_holds_nothing() {
        let arena: Arena<String> = Arena::new();
        assert!(arena.is_empty());
        assert_eq!(arena.iter().count(), 0);
        assert_eq!(arena.get(Key::new(0, 0)), None);
    }

    #[test]
    fn delete_then_restore_keeps_the_same_key() {
        let (mut arena, keys) = letters();
        let value = arena.remove(keys[1]).expect("the key is live");
        assert_eq!(arena.get(keys[1]), None);
        arena.restore(keys[1], value).expect("the slot is free");
        assert_eq!(arena.get(keys[1]).map(String::as_str), Some("b"));
    }

    #[test]
    fn insert_never_reuses_an_index() {
        let (mut arena, keys) = letters();
        arena.remove(keys[0]).expect("the key is live");
        arena.remove(keys[1]).expect("the key is live");
        let fresh = arena.insert("d".to_owned());
        assert_eq!(fresh.index(), 3);
        assert_eq!(arena.issued(), 4);
    }

    #[test]
    fn a_key_from_another_arena_is_stale() {
        let (mut arena, _) = letters();
        let mut other: Arena<String> = Arena::new();
        let mut far = other.insert("z".to_owned());
        for _ in 0..9 {
            far = other.insert("z".to_owned());
        }
        assert_eq!(arena.get(far), None);
        assert_eq!(arena.remove(far), Err(DocError::stale(far)));
    }

    #[test]
    fn a_key_with_the_wrong_generation_reads_nothing() {
        let (arena, keys) = letters();
        let forged = Key::new(keys[0].index(), 7);
        assert_eq!(arena.get(forged), None);
    }

    #[test]
    fn removing_twice_is_an_error_not_a_panic() {
        let (mut arena, keys) = letters();
        arena.remove(keys[0]).expect("the key is live");
        assert_eq!(arena.remove(keys[0]), Err(DocError::stale(keys[0])));
    }

    #[test]
    fn restoring_onto_a_live_entry_is_an_error() {
        let (mut arena, keys) = letters();
        assert_eq!(
            arena.restore(keys[0], "z".to_owned()),
            Err(DocError::occupied(keys[0]))
        );
    }

    #[test]
    fn a_vacant_slot_is_the_only_one_a_restore_can_land_on() {
        let (mut arena, keys) = letters();
        assert!(!arena.is_vacant(keys[0]));
        arena.remove(keys[0]).expect("the key is live");
        assert!(arena.is_vacant(keys[0]));
        assert!(!arena.is_vacant(Key::new(9, 0)));
        assert!(!arena.is_vacant(Key::new(keys[0].index(), 7)));
    }

    #[test]
    fn iteration_is_index_ordered() {
        let (mut arena, keys) = letters();
        arena.remove(keys[1]).expect("the key is live");
        arena.insert("d".to_owned());
        let seen: Vec<(u32, &str)> = arena
            .iter()
            .map(|(key, value)| (key.index(), value.as_str()))
            .collect();
        assert_eq!(seen, [(0, "a"), (2, "c"), (3, "d")]);
        assert_eq!(arena.len(), 3);
    }

    #[test]
    fn writing_through_a_key_writes_the_entry() {
        let (mut arena, keys) = letters();
        *arena.get_mut(keys[2]).expect("the key is live") = "z".to_owned();
        assert_eq!(arena.get(keys[2]).map(String::as_str), Some("z"));
    }
}
