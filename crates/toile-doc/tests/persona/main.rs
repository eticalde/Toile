#![allow(missing_docs, reason = "a test crate publishes no API surface")]

mod file;
mod fingerprint;
mod refresh;

use serde_json::Value;
use toile_doc::Persona;

/// Ana as her library file holds her: two sessions with the tape, oldest
/// first, and a shaped body under the newer one.
const ANA: &str = r#"{
  "toile_persona": 1,
  "name": "Ana",
  "notes": "NOTA PRIVADA: prefiere el tiro alto",
  "taken": [
    {
      "date": "2026-03-02",
      "values": {
        "cadera": 96,
        "cintura": 72,
        "estatura": 165
      }
    },
    {
      "date": "2026-09-01",
      "values": {
        "cadera": 95.5,
        "cintura": 71.25,
        "tiro": 27
      },
      "phenotype": {
        "sex": 1,
        "age_years": 34,
        "build": 0.62,
        "muscle": 0.4,
        "proportions": 0.25
      }
    }
  ]
}
"#;

/// What the notes open with, which nothing outside the library may repeat.
const PRIVATE: &str = "NOTA PRIVADA";

/// The stem Ana's file is saved under.
const STEM: &str = "ana";

fn ana() -> Persona {
    Persona::from_json(ANA).unwrap_or_else(|error| panic!("Ana reads: {error}"))
}

/// Ana, read from her file with the first `from` written as `to`.
fn edited(from: &str, to: &str) -> Persona {
    assert!(
        ANA.contains(from),
        "the fixture moved under the test: {from}"
    );
    Persona::from_json(&ANA.replacen(from, to, 1))
        .unwrap_or_else(|error| panic!("{from} as {to} reads: {error}"))
}

/// Ana's file as someone else's tool might write it: no indentation, and
/// every object's keys in another order, the version header among them.
fn retyped() -> String {
    let value: Value = serde_json::from_str(ANA).expect("the fixture is JSON");
    let text = reversed(&value).to_string();
    assert_ne!(text, ANA);
    assert!(!text.starts_with("{\"toile_persona\""), "{text}");
    text
}

fn reversed(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .rev()
                .map(|(key, inner)| (key.clone(), reversed(inner)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(reversed).collect()),
        other => other.clone(),
    }
}
