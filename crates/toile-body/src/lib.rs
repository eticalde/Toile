//! Procedural tailor's dummy: measurements lofted into a 3D body mesh.

mod loft;
mod mesh;
mod params;
mod ring;

pub use mesh::{BodyMesh, body_mesh};
pub use params::{BodyMeasures, BodyRes};

#[cfg(test)]
mod tests;
