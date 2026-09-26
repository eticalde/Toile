use toile_anny::asset::{PathId, RingId};

use super::BodyMesh;

mod lay;
#[cfg(test)]
mod tests;

/// The line a tailor's tape lies along for one catalogue measurement, on one
/// body: what the interface draws while the person handles that row.
///
/// It carries two lines. `points` is the one measured: its length is the
/// measurement, to rounding. The other is the one drawn ([`Tape::lifted`]),
/// and no part of that length.
///
/// For a length the two are the same: the baked path's own points on the
/// skin, and for the inseam the plumb drop to the floor after them; the
/// stature is a plumb line beside the body, crown to floor. A girth is where
/// they part. It is measured as its ring's loop flattened into the plane the
/// girth is summed in, and drawn through the ring's own crossings on the skin,
/// because a morph buckles the ring out of that plane by centimetres and the
/// flattened loop cuts into the body and stands off it by as much.
#[derive(Debug, Clone, PartialEq)]
pub struct Tape {
    /// The measured line, in the body mesh's own frame and unit (metres).
    pub points: Vec<[f32; 3]>,
    /// Whether the last point runs back to the first, as a girth's does, in
    /// both lines.
    pub closed: bool,
    /// The drawn line before its lift: on the skin, except where `points`
    /// stands beside the body or drops to the floor on purpose.
    drawn: Vec<[f32; 3]>,
    /// Per drawn point, the unit direction that leads away from the skin.
    out: Vec<[f32; 3]>,
}

impl Tape {
    /// The measured line's length in centimetres: the sum of its chords, with
    /// the closing one when it is a loop.
    pub fn length_cm(&self) -> f64 {
        let n = self.points.len();
        let chords = if self.closed { n } else { n.saturating_sub(1) };
        let chord = |i: usize| {
            let (a, b) = (self.points[i], self.points[(i + 1) % n]);
            let d = [0, 1, 2].map(|k| f64::from(a[k]) - f64::from(b[k]));
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
        };
        (0..chords).map(chord).sum::<f64>() * 100.0
    }

    /// Where to draw the tape: every drawn point moved `gap` metres off the
    /// skin, the same on every body, so the tape stands as far off a heavy
    /// body as off a slight one.
    pub fn lifted(&self, gap: f32) -> Vec<[f32; 3]> {
        self.drawn
            .iter()
            .zip(&self.out)
            .map(|(p, o)| [0, 1, 2].map(|k| p[k] + gap * o[k]))
            .collect()
    }
}

/// How one catalogue measurement lies on the body.
#[derive(Debug, Clone, Copy)]
enum Lay {
    /// The ring's own loop.
    Girth(RingId),
    /// The path's own points on the skin.
    Skin(PathId),
    /// A plumb line from the crown's height to the floor.
    Stature,
}

/// How each catalogue name lies on the body: the one mapping from the
/// catalogue to what the interface draws, read off `measure` name by name.
///
/// `None` outside the catalogue. A catalogue name that falls through to it
/// fails `every_catalogue_tape_is_as_long_as_its_measurement`.
fn lay(name: &str) -> Option<Lay> {
    use PathId::{Arm, Back, HipDrop, Inseam, Outseam, Rise, Shoulders};
    use RingId::{
        Ankle, Bust, Head, Hip, Knee, Neck, Thigh, Underbust, UpperArm, UpperChest, Waist, Wrist,
    };
    Some(match name {
        "cuello" => Lay::Girth(Neck),
        "pecho_alto" => Lay::Girth(UpperChest),
        "pecho" => Lay::Girth(Bust),
        "bajo_pecho" => Lay::Girth(Underbust),
        "cintura" => Lay::Girth(Waist),
        "cadera" => Lay::Girth(Hip),
        "muslo" => Lay::Girth(Thigh),
        "rodilla" => Lay::Girth(Knee),
        "tobillo" => Lay::Girth(Ankle),
        "brazo_contorno" => Lay::Girth(UpperArm),
        "muneca" => Lay::Girth(Wrist),
        "cabeza" => Lay::Girth(Head),
        "tiro" => Lay::Skin(Rise),
        "altura_cadera" => Lay::Skin(HipDrop),
        "entrepierna" => Lay::Skin(Inseam),
        "largo_lateral" => Lay::Skin(Outseam),
        "largo_espalda" => Lay::Skin(Back),
        "brazo" => Lay::Skin(Arm),
        "hombros" => Lay::Skin(Shoulders),
        "estatura" => Lay::Stature,
        _ => return None,
    })
}

/// The body's own ring for catalogue measurement `name`; `None` for a name that
/// is not a girth.
///
/// A second question of the one mapping, rather than a second mapping. A
/// garment hung from a station needs the ring behind the name, and which ring a
/// name means is written once, in [`lay`], where the tape already needed it.
pub(crate) fn ring_of(name: &str) -> Option<RingId> {
    match lay(name)? {
        Lay::Girth(id) => Some(id),
        // A length runs down the body and a stature stands beside it: neither
        // is a loop, so neither names a height a garment can be held at.
        Lay::Skin(_) | Lay::Stature => None,
    }
}

/// The tape for catalogue measurement `name` on `mesh`, or `None` for a name
/// outside the catalogue.
///
/// # Panics
/// If `mesh` is not the Anny body's own vertex layout, which every baked
/// ring and path indexes into.
pub fn tape(name: &str, mesh: &BodyMesh) -> Option<Tape> {
    lay(name).map(|how| lay::on(how, mesh))
}
