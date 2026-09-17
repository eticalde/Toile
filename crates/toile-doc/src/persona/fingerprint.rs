use std::collections::BTreeMap;

use serde::Serialize;

use crate::BodyShape;
use crate::json::canonical_bytes;

/// What a body copies from a session, as the fingerprint reads it.
///
/// A struct of its own rather than the session, so that the fields in the
/// text are the ones named here, in this order, whatever a session comes to
/// carry besides.
#[derive(Serialize)]
struct Copied<'a> {
    values: &'a BTreeMap<String, f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phenotype: Option<&'a BodyShape>,
}

/// The FNV-1a 64 offset basis.
const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// The FNV-1a 64 prime.
const PRIME: u64 = 0x0100_0000_01b3;

/// The fingerprint of a tape and a phenotype, as `Snapshot::fingerprint`
/// defines it.
pub(crate) fn of(values: &BTreeMap<String, f64>, phenotype: Option<&BodyShape>) -> String {
    let text = canonical_bytes(&Copied { values, phenotype });
    format!("{:016x}", fnv1a(&text))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(OFFSET, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_is_fnv_1a_64_as_its_authors_publish_it() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn an_empty_unshaped_tape_is_fingerprinted_as_its_one_text() {
        let values = BTreeMap::new();
        let text = canonical_bytes(&Copied {
            values: &values,
            phenotype: None,
        });
        assert_eq!(text, b"{\n  \"values\": {}\n}");
        assert_eq!(of(&values, None), format!("{:016x}", fnv1a(&text)));
        assert_eq!(of(&values, None).len(), 16);
    }
}
