//! Documento 2D — la fuente de verdad de Toile.
//!
//! Entidades (Pieza, PuntoDeControl, Piquete, Costura, Simetría, Pin) en
//! structs planos con keys generacionales estables que nunca se reciclan.
//! Mutación exclusivamente por comandos reversibles (apply/invert) con
//! coalescing por gesto. Serialización JSON canónica y diffable.
//!
//! Ver `docs/architecture.html` §2.1.

pub mod model;
