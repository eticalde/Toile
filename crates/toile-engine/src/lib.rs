//! Orquestación headless — la API pública del motor.
//!
//! Módulos: `couture` (compilador de ediciones: comando → RestStateDelta /
//! RemeshPlan), `avatar` (gltf → bake SDF cacheado), `sync` (canales
//! latest-wins hacia la sim, snapshots arc-swap hacia los clientes), hilos.
//!
//! Todo lo que la UI puede hacer es expresable por esta API — y por tanto
//! testeable en CI sin ventana. La consumen `toile-cli` y `toile-app`.
//!
//! Ver `docs/architecture.html` §2.4 y §2.7.

pub mod couture;
pub mod golden;
pub mod session;
pub mod sync;
