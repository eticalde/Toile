use crate::asset::{PathId, RingId, RingPoint};
use crate::mesh::decoded;

mod geom;
#[cfg(test)]
mod on_skin;
#[cfg(test)]
mod tests;

pub use geom::{at, centroid};
use geom::{perimeter, walk};

/// The number of body vertices every baked ring and path indexes into: the
/// fixed shape [`measure`] requires of its `positions` argument.
const BODY_VERTEX_COUNT: usize = 13_380;

/// The 20 catalogue measurements read directly off a generated Anny mesh, in
/// centimetres.
///
/// Field names are English identifiers — the Spanish catalogue names these
/// map to are the boundary layer's business (`toile_engine::body`), never
/// this crate's, per STD-001.
///
/// This is the *measured* half of the tab's dado/medido/Δ row: what this
/// body actually comes to, as opposed to what the tape (a [`crate::Phenotype`]
/// does not itself carry a tape) was written down as.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measures {
    /// The mesh's own bounding-box height (`estatura`): a height against a
    /// wall, not a line on the skin.
    pub height: f32,
    /// The neck ring's perimeter (`cuello`).
    pub neck: f32,
    /// The bust ring's perimeter (`pecho`).
    pub bust: f32,
    /// The upper-chest ring's perimeter (`pecho_alto`).
    ///
    /// This mesh's horizontal section shrinks monotonically from the armpit
    /// down to the waist, so a ring cut anywhere above `pecho`'s own
    /// necessarily reads *more* than `pecho`. A tape asking for less than
    /// its own `pecho` therefore cannot be met at any honest height, and
    /// this row has no lever of its own to close the rest.
    pub upper_chest: f32,
    /// The underbust ring's perimeter (`bajo_pecho`).
    pub underbust: f32,
    /// The waist ring's perimeter (`cintura`).
    pub waist: f32,
    /// The hip ring's perimeter (`cadera`).
    pub hip: f32,
    /// The thigh ring's perimeter (`muslo`).
    pub thigh: f32,
    /// The knee ring's perimeter (`rodilla`).
    pub knee: f32,
    /// The ankle ring's perimeter (`tobillo`).
    pub ankle: f32,
    /// The upper-arm ring's perimeter (`brazo_contorno`).
    pub upper_arm: f32,
    /// The wrist ring's perimeter (`muneca`).
    pub wrist: f32,
    /// The head ring's perimeter (`cabeza`).
    pub head: f32,
    /// Down the side from the waist to the fork's height (`tiro`): see
    /// [`PathId::Rise`].
    pub rise: f32,
    /// Down the side from the waist to the hip's height (`altura_cadera`):
    /// see [`PathId::HipDrop`].
    ///
    /// Always shorter than `rise`, structurally rather than by luck: its path
    /// is the start of `rise`'s own, point for point, so it sums the first
    /// few of the very same chords.
    pub hip_drop: f32,
    /// Down the inside of the leg from the fork to the floor
    /// (`entrepierna`): see [`PathId::Inseam`] and
    /// [`PathId::ends_on_floor`]. The floor is the one the stature stands
    /// on.
    pub inseam: f32,
    /// Down the outside of the leg from the waist to the ankle
    /// (`largo_lateral`): see [`PathId::Outseam`].
    pub outseam: f32,
    /// Down the spine from the nape to the waist (`largo_espalda`): see
    /// [`PathId::Back`].
    pub back_length: f32,
    /// From the shoulder point over the elbow's point to the wrist (`brazo`):
    /// see [`PathId::Arm`].
    pub arm_length: f32,
    /// Across the back from shoulder point to shoulder point (`hombros`):
    /// see [`PathId::Shoulders`].
    pub shoulder_width: f32,
}

/// The floor the body stands on and the crown of its head: the lowest and
/// highest y in the mesh, in metres.
pub fn floor_and_crown(positions: &[f32]) -> (f32, f32) {
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &y in positions.iter().skip(1).step_by(3) {
        lo = lo.min(y);
        hi = hi.max(y);
    }
    (lo, hi)
}

/// How far `top` stands above `bottom`, in centimetres.
///
/// Named so that every row of [`Measures`] converts through a helper, the
/// hundred written once: a centimetre figure a hundred times short is a
/// plausible number rather than a crash, and the solver would chase it.
fn vertical_cm(top: f32, bottom: f32) -> f32 {
    (top - bottom) * 100.0
}

/// The mesh's own bounding-box height along y, in centimetres.
///
/// The one implementation of the stature, so [`Measures::height`] and whatever
/// a client shows beside the body cannot be changed apart. The identity panel
/// prints this number and its own Δ against the tape side by side, and two
/// hand-written scans of the same vertices is how one of them comes to
/// contradict the other.
pub fn height_cm(positions: &[f32]) -> f32 {
    let (floor, crown) = floor_and_crown(positions);
    vertical_cm(crown, floor)
}

/// One baked ring, as the shipped asset stores it.
///
/// Public so a client can draw exactly the loop [`measure`] reads: its points
/// go through [`at`], the function every measurement here is built on.
#[derive(Debug, Clone, Copy)]
pub struct Ring {
    /// The ring's points, still to be evaluated against a body's positions.
    pub points: &'static [RingPoint],
    /// The unit normal of the plane the girth is summed in. See
    /// [`crate::asset::RingEntry::normal`].
    pub normal: [f32; 3],
}

/// The ring a [`RingId`] names, sliced out of the shipped asset.
pub fn ring(id: RingId) -> Ring {
    let baked = decoded();
    let e = baked.ring_entries[id as usize];
    let start = e.offset as usize;
    Ring {
        points: &baked.ring_points[start..start + e.length as usize],
        normal: e.normal,
    }
}

/// The points of the path a [`PathId`] names, sliced out of the shipped
/// asset, still to be evaluated against a body's positions with [`at`].
///
/// Public so a client can draw exactly the line [`measure`] reads.
pub fn path(id: PathId) -> &'static [RingPoint] {
    let baked = decoded();
    let e = baked.path_entries[id as usize];
    let start = e.offset as usize;
    &baked.path_points[start..start + e.length as usize]
}

/// A path's length on `positions`, in centimetres: its chords, then the drop
/// from its last point to `floor` when [`PathId::ends_on_floor`] says so.
fn length_cm(positions: &[f32], id: PathId, floor: f32) -> f32 {
    let points = path(id);
    let on_skin = walk(positions, points) * 100.0;
    match points.last() {
        Some(&end) if id.ends_on_floor() => on_skin + vertical_cm(at(positions, end)[1], floor),
        _ => on_skin,
    }
}

/// Measures every catalogue value off `positions` (already morphed by a
/// phenotype), by walking the rings and paths baked into the shipped asset.
///
/// Both are cut once at bake time and only ever walked here. Every step is
/// `+ - * / sqrt` over their fixed `(vertex, vertex, t)` data, in the fixed
/// order below — the same regime [`crate::body_mesh`] runs in, so a
/// measurement is a continuous function of the phenotype rather than a step
/// function of which triangle a plane happened to cross.
///
/// A girth is summed in the plane its ring was cut on ([`geom::perimeter`]),
/// a length in space along the skin ([`geom::walk`]).
///
/// # Panics
/// If `positions` does not hold exactly [`BODY_VERTEX_COUNT`] xyz triples.
/// The baked rings and paths index into that exact vertex layout, so a
/// mismatched slice would otherwise read out of bounds or silently measure
/// the wrong vertices.
pub fn measure(positions: &[f32]) -> Measures {
    assert_eq!(
        positions.len(),
        BODY_VERTEX_COUNT * 3,
        "measure() only understands the Anny body's own {BODY_VERTEX_COUNT}-vertex layout, \
         got {} vertices",
        positions.len() / 3
    );

    let per = |id: RingId| {
        let Ring { points, normal } = ring(id);
        perimeter(positions, points, normal) * 100.0
    };
    let (floor, crown) = floor_and_crown(positions);
    let len = |id: PathId| length_cm(positions, id, floor);

    Measures {
        height: vertical_cm(crown, floor),
        neck: per(RingId::Neck),
        bust: per(RingId::Bust),
        upper_chest: per(RingId::UpperChest),
        underbust: per(RingId::Underbust),
        waist: per(RingId::Waist),
        hip: per(RingId::Hip),
        thigh: per(RingId::Thigh),
        knee: per(RingId::Knee),
        ankle: per(RingId::Ankle),
        upper_arm: per(RingId::UpperArm),
        wrist: per(RingId::Wrist),
        head: per(RingId::Head),
        rise: len(PathId::Rise),
        hip_drop: len(PathId::HipDrop),
        inseam: len(PathId::Inseam),
        outseam: len(PathId::Outseam),
        back_length: len(PathId::Back),
        arm_length: len(PathId::Arm),
        shoulder_width: len(PathId::Shoulders),
    }
}
