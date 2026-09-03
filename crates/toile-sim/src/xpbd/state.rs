/// Particle state, structure-of-arrays.
///
/// `qx/qy/qz` hold the position before the current substep: PBD derives
/// velocity from the difference rather than integrating it separately.
#[allow(
    missing_docs,
    reason = "SoA buffers are named by their axis; a doc
    per field would only restate the name"
)]
#[derive(Debug, Clone, Default)]
pub struct State {
    pub px: Vec<f32>,
    pub py: Vec<f32>,
    pub pz: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub vz: Vec<f32>,
    pub qx: Vec<f32>,
    pub qy: Vec<f32>,
    pub qz: Vec<f32>,
    pub inv_mass: Vec<f32>,
}

impl State {
    /// `n` particles at the origin, at rest, with unit mass.
    pub fn new(n: usize) -> Self {
        Self {
            px: vec![0.0; n],
            py: vec![0.0; n],
            pz: vec![0.0; n],
            vx: vec![0.0; n],
            vy: vec![0.0; n],
            vz: vec![0.0; n],
            qx: vec![0.0; n],
            qy: vec![0.0; n],
            qz: vec![0.0; n],
            inv_mass: vec![1.0; n],
        }
    }

    /// Number of particles.
    pub fn len(&self) -> usize {
        self.px.len()
    }

    /// Whether there are no particles.
    pub fn is_empty(&self) -> bool {
        self.px.is_empty()
    }
}

/// Distance constraints, structure-of-arrays: edge `i` joins `a[i]`–`b[i]`.
#[allow(
    missing_docs,
    reason = "the endpoint and rest arrays are named by their role"
)]
#[derive(Debug, Clone, Default)]
pub struct DistanceConstraints {
    pub a: Vec<u32>,
    pub b: Vec<u32>,
    pub rest: Vec<f32>,
    /// Per-edge XPBD compliance. Warp/weft anisotropy lives here.
    pub compliance: Vec<f32>,
    /// Post-solve elongation cap as a ratio of rest length, e.g. `1.05`;
    /// `0.0` disables it.
    ///
    /// Without a cap, a closed garment's hem elongates under its own weight
    /// until the cloth passes through the avatar.
    pub strain_limit: f32,
    /// Clamp sweeps per substep. Values below four are raised to four.
    ///
    /// This changes the effective physics, so it belongs to a scene, never to
    /// a global default: four is interactive (~1.4% residual on long chains),
    /// sixteen is measurement grade (~0.9%).
    pub strain_sweeps: u32,
}

impl DistanceConstraints {
    /// Number of constraints.
    pub fn len(&self) -> usize {
        self.a.len()
    }

    /// Whether there are no constraints.
    pub fn is_empty(&self) -> bool {
        self.a.is_empty()
    }
}

/// Inter-piece seam attachments with rest length zero.
///
/// Indices are global across every piece: they address the combined solver
/// state, not one piece's local numbering.
///
/// High `compliance` is a soft seam, used to sew progressively; near zero is a
/// firm seam once the drape has settled. `max_step` caps the per-substep
/// correction so the first frames of a drape cannot generate extreme forces.
#[allow(missing_docs, reason = "the endpoint arrays are named by their role")]
#[derive(Debug, Clone, Default)]
pub struct Seams {
    pub a: Vec<u32>,
    pub b: Vec<u32>,
    pub compliance: f32,
    pub max_step: f32,
    /// Pass iterations per substep; zero is treated as one. Seams are few and
    /// dominate conditioning, so iterating them is cheap and closes gaps that
    /// a single capped pass cannot.
    pub iterations: u32,
}

impl Seams {
    /// Number of sewn pairs.
    pub fn len(&self) -> usize {
        self.a.len()
    }

    /// Whether nothing is sewn.
    pub fn is_empty(&self) -> bool {
        self.a.is_empty()
    }
}
