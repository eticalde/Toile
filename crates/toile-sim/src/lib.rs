//! Solver de tela XPBD small-steps residente.
//!
//! Buffers SoA, coloreo de grafo determinista, anisotropía urdimbre/trama,
//! costuras inter-pieza globales, colisión por SDF, kinetic damping y sleep
//! por islas de acoplamiento. El solver nunca se resetea: los cambios de
//! forma llegan como nuevo estado de reposo (hot-swap entre substeps).
//!
//! Expone el trait `ClothSolver` para que la implementación sea desafiable
//! sin re-arquitectura. Ver `docs/architecture.html` §2.3–2.5.

pub mod xpbd;
