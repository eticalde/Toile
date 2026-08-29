//! Mallado por pieza y reproyección del interior.
//!
//! CDT + refinement (spade, inserción canónica por ID), matriz PMVC cacheada
//! detrás de un trait interpolador (fallback armónico), transferencia
//! baricéntrica para cambios de topología, métricas de calidad de malla.
//!
//! Ver `docs/architecture.html` §2.2 y §2.7.

pub mod cdt;
pub mod interp;
