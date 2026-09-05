use std::fmt;
use std::marker::PhantomData;

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Key;

/// A key is written as `index.generation`.
///
/// Two anonymous integers would say the same thing; one string that reads as
/// `3.0` is what a person, or a language model, can follow from the entry that
/// declares it to the seam that cites it.
impl<T> Serialize for Key<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("{}.{}", self.index(), self.generation()))
    }
}

impl<'de, T> Deserialize<'de> for Key<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Key<T>, D::Error> {
        deserializer.deserialize_str(Id(PhantomData))
    }
}

struct Id<T>(PhantomData<fn() -> T>);

impl<T> Visitor<'_> for Id<T> {
    type Value = Key<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an entity id, written as index.generation")
    }

    fn visit_str<E: Error>(self, text: &str) -> Result<Key<T>, E> {
        read(text).ok_or_else(|| {
            E::custom(format!(
                "`{text}` is not an entity id: an id reads as index.generation, as in `3.0`"
            ))
        })
    }
}

/// The key some stored text names, when the text names one.
fn read<T>(text: &str) -> Option<Key<T>> {
    let (index, generation) = text.split_once('.')?;
    Some(Key::new(index.parse().ok()?, generation.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use crate::PointKey;

    #[test]
    fn a_key_is_written_as_its_index_and_its_generation() {
        let written =
            serde_json::to_string(&PointKey::new(3, 0)).expect("a key writes as a string");
        assert_eq!(written, "\"3.0\"");
    }

    #[test]
    fn a_stored_id_reads_back_as_the_key_that_wrote_it() {
        let key: PointKey = serde_json::from_str("\"12.4\"").expect("the id is well formed");
        assert_eq!(key, PointKey::new(12, 4));
    }

    #[test]
    fn an_id_that_is_not_one_says_what_an_id_looks_like() {
        for text in ["\"3\"", "\"3.0.1\"", "\"a.b\"", "\"-1.0\"", "\"\"", "3"] {
            let read = serde_json::from_str::<PointKey>(text);
            let error = read.expect_err("the id is malformed").to_string();
            assert!(
                error.contains("index.generation"),
                "{text} was refused with {error}"
            );
        }
    }
}
