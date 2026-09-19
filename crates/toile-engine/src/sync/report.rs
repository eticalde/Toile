use thiserror::Error;

/// Why the sim thread would not take a message.
///
/// A refusal is loud rather than silent for one reason: every case here is a
/// message compiled against a mesh the solver has already replaced, and taking
/// it would warm-start the drape over a topology that no longer exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StaleMessage {
    /// A message from before the one the solver is already running.
    #[error("a message at generation {got} arrived after generation {applied}")]
    Generation {
        /// The generation the solver is at.
        applied: u64,
        /// The generation the message names.
        got: u64,
    },
    /// Rest lengths for another mesh's constraints.
    #[error("{got} rest lengths for a mesh of {expected} constraints")]
    RestCount {
        /// Constraints the solver holds.
        expected: usize,
        /// Rest lengths the message carries.
        got: usize,
    },
    /// Compliances for another mesh's constraints.
    ///
    /// Its own variant and not a count folded into [`StaleMessage::RestCount`]:
    /// the two lists arrive together and are checked together, and a refusal
    /// that named the rest lengths when the compliances were the short list
    /// would send whoever read it looking at the half that was right.
    #[error("{got} compliances for a mesh of {expected} constraints")]
    ComplianceCount {
        /// Constraints the solver holds.
        expected: usize,
        /// Compliances the message carries.
        got: usize,
    },
    /// An elastic naming an edge the solver's constraint set does not hold.
    ///
    /// The solver indexes these directly, so one past the end is a panic on
    /// the sim thread rather than a wrong drape — the same reason a sewn
    /// vertex is checked before it is taken.
    #[error("an elastic on edge {edge}, in a set of {len}")]
    HeldRange {
        /// The edge the message named.
        edge: u32,
        /// Constraints the solver holds.
        len: usize,
    },
    /// A swap naming a run of the combined state the solver does not hold.
    #[error("a swap of {replacing} vertices at {at}, in a state of {len}")]
    SwapRange {
        /// Where the run the swap replaces begins.
        at: u32,
        /// How many vertices it claims to replace.
        replacing: u32,
        /// How many the solver holds in all.
        len: usize,
    },
    /// A seam sewing a vertex the solver does not hold.
    #[error("a seam names vertex {vertex}, in a state of {len}")]
    SeamRange {
        /// The first vertex past the end of the state.
        vertex: u32,
        /// How many the solver holds in all.
        len: usize,
    },
}

/// What the sim thread publishes after every tick.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// Last rest-state generation applied before these substeps.
    pub generation: u64,
    /// Substeps run since the thread started.
    pub substeps: u64,
    /// The sim is asleep, waiting for an edit.
    pub converged: bool,
    /// Interleaved xyz positions.
    pub positions: Vec<f32>,
    /// Interleaved xyz vertex normals.
    pub normals: Vec<f32>,
    /// The last message the sim refused, if it ever refused one.
    pub refused: Option<StaleMessage>,
}
