//! Golden de determinismo (ADR §2.4): drapear → editar → re-drapear debe
//! producir SIEMPRE estos bits. CI lo corre en release sobre macOS ARM y
//! Linux x86 contra la misma constante — si ambas pasan, el solver f32 es
//! bit-idéntico cross-arquitectura (§3.3).
//!
//! Correr con: `cargo test --release -- --ignored`
//! Si un cambio LEGÍTIMO de física altera el hash, regenerarlo con
//! `toile drape` y actualizar la constante en el mismo commit, explicando
//! el porqué en el mensaje.

#[test]
#[ignore = "golden en release: cargo test --release -- --ignored"]
fn drape_bodice_golden() {
    assert_eq!(
        toile_engine::golden::drape_bodice_hash(),
        0xdb04a0b231ac923f,
        "el drapeado dorado cambió de bits: o hay no-determinismo, o un \
         cambio de física legítimo que exige regenerar el golden (toile drape)"
    );
}
