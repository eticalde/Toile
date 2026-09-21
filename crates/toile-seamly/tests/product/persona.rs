use toile_doc::{Doc, FORMAT_VERSION_LINKED, Persona, Snapshot};

use super::product;

#[test]
fn the_person_is_the_product_body_and_reads_back_through_the_library_format() {
    let product = product();
    let persona = product.persona("Ana María", "2026-09-05", "cuerpo.smis");
    let body = product.doc.measures().expect("one body");
    let current = persona.current().expect("one session");
    assert_eq!(current.values, body.values);
    assert_eq!(current.date, "2026-09-05");
    assert_eq!(current.phenotype, None);
    assert!(persona.notes.contains("«cuerpo.smis»"), "{}", persona.notes);
    let text = persona.to_canonical_json().expect("writable");
    assert!(text.starts_with("{\n  \"toile_persona\": 1,"), "{text}");
    assert_eq!(Persona::from_json(&text), Ok(persona));
    assert!(!text.contains("SENTINEL"), "the personal block reached her");
}

#[test]
fn a_linked_product_carries_the_person_fingerprint_and_asks_for_the_link_version() {
    let mut product = product();
    let unlinked = product.doc.to_canonical_json();
    let persona = product.persona("Ana María", "2026-09-05", "cuerpo.smis");
    product.link(&persona, "ana-maria").expect("links");
    let body = product.doc.measures().expect("one body");
    assert_eq!(body.name, "Ana María");
    assert_eq!(product.report.body, "Ana María");
    let origin = body.origin.as_ref().expect("linked");
    assert_eq!(origin.persona, "ana-maria");
    assert_eq!(origin.taken, "2026-09-05");
    assert_eq!(
        Some(&origin.fnv),
        persona.current().map(Snapshot::fingerprint).as_ref()
    );
    assert_eq!(origin.fnv, body.fingerprint());
    assert_eq!(product.doc.format_version(), FORMAT_VERSION_LINKED);
    let written = product.doc.to_canonical_json();
    assert_eq!(
        Doc::from_json(&written).map(|d| d.to_canonical_json()),
        Ok(written.clone())
    );
    // Only the body changed: the pieces and the points are the unlinked ones.
    let head = |text: &str| text[..text.find("\"mannequins\"").expect("bodies")].to_owned();
    assert_eq!(
        head(&written).replacen("\"toile\": 3", "\"toile\": 1", 1),
        head(&unlinked)
    );
    assert!(
        !written.contains("SENTINEL"),
        "the personal block reached the product"
    );
}

#[test]
fn a_person_measured_otherwise_or_a_stem_no_file_could_have_is_refused() {
    let mut product = product();
    let before = product.clone();
    let mut other = product.persona("Ana", "2026-09-05", "cuerpo.smis");
    if let Some(value) = other.taken[0].values.get_mut("cadera") {
        *value += 1.0;
    }
    assert!(product.link(&other, "ana").is_err());
    let ana = product.persona("Ana", "2026-09-05", "cuerpo.smis");
    assert!(product.link(&ana, "../ana").is_err());
    let undated = product.persona("Ana", "5/9/2026", "cuerpo.smis");
    assert!(product.link(&undated, "ana").is_err());
    assert_eq!(product, before);
}
