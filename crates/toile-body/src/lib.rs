//! Procedural tailor's dummy: measurements lofted into a 3D body mesh.

mod derive;
mod landmarks;
mod loft;
mod mesh;
mod params;
mod parts;
mod profile;
mod ring;
mod station;

pub use derive::PartialMeasures;
pub use mesh::{BodyMesh, body_mesh};
pub use params::{BodyMeasures, BodyRes};
pub use station::Station;

#[cfg(test)]
mod tests;
