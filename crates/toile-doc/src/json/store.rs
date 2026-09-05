use serde::de::Error;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Arena, Key};

/// An arena is written as the entries that are live, plus the count of slots
/// it has ever opened.
///
/// The empty slots are not written — nothing is left of them but their index —
/// yet the count is, so that reopening a document and inserting into it cannot
/// hand out a key some seam still holds.
impl<T: Serialize> Serialize for Arena<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut store = serializer.serialize_struct("Arena", 2)?;
        store.serialize_field("issued", &self.issued())?;
        store.serialize_field("entries", &Entries(self))?;
        store.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Arena<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Arena<T>, D::Error> {
        let stored = Stored::deserialize(deserializer)?;
        let entries = stored.entries.into_iter().map(|held| (held.id, held.value));
        Arena::rebuild(stored.issued, entries).map_err(D::Error::custom)
    }
}

/// The live entries, in index order, each one carrying its own id.
struct Entries<'a, T>(&'a Arena<T>);

impl<T: Serialize> Serialize for Entries<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(|(id, value)| Entry { id, value }))
    }
}

#[derive(Serialize)]
struct Entry<'a, T> {
    id: Key<T>,
    #[serde(flatten)]
    value: &'a T,
}

#[derive(Deserialize)]
struct Stored<T> {
    issued: u32,
    entries: Vec<Held<T>>,
}

#[derive(Deserialize)]
struct Held<T> {
    id: Key<T>,
    #[serde(flatten)]
    value: T,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, PointKey};

    fn arena() -> Arena<Point> {
        let mut arena = Arena::new();
        arena.insert(Point::at(1.0, 2.0));
        let second = arena.insert(Point::at(3.0, 4.0).named("cadera_lat"));
        arena.insert(Point::at(5.0, 6.0));
        arena.remove(second).expect("the key is live");
        arena
    }

    #[test]
    fn an_entry_carries_its_id_beside_its_own_fields() {
        let written = serde_json::to_string(&arena()).expect("an arena of points writes");
        assert!(written.starts_with("{\"issued\":3,\"entries\":[{\"id\":\"0.0\","));
        assert!(
            !written.contains("\"1.0\""),
            "the empty slot is written: {written}"
        );
    }

    #[test]
    fn an_empty_slot_survives_the_round_trip_as_the_gap_it_is() {
        let written = serde_json::to_string(&arena()).expect("an arena of points writes");
        let read: Arena<Point> = serde_json::from_str(&written).expect("what was written reads");
        assert_eq!(read, arena());
        assert_eq!(read.issued(), 3);
        assert_eq!(read.get(PointKey::new(1, 0)), None);
        assert_eq!(read.len(), 2);
    }

    #[test]
    fn an_id_past_the_slots_the_arena_opened_is_refused() {
        let stored = "{\"issued\":1,\"entries\":[{\"id\":\"4.0\",\"x\":0,\"y\":0}]}";
        let error = serde_json::from_str::<Arena<Point>>(stored)
            .expect_err("the id names no slot")
            .to_string();
        assert!(error.contains("Point"), "{error}");
    }

    #[test]
    fn two_entries_under_one_id_are_refused() {
        let entry = "{\"id\":\"0.0\",\"x\":0,\"y\":0}";
        let stored = format!("{{\"issued\":1,\"entries\":[{entry},{entry}]}}");
        let error = serde_json::from_str::<Arena<Point>>(&stored)
            .expect_err("the id is taken twice")
            .to_string();
        assert!(error.contains("already holds"), "{error}");
    }
}
