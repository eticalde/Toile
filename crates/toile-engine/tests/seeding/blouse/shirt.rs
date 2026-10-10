use toile_engine::draft::{Doc, Heading, MeasureSet, Sense};

use super::band::{HIP, TO_THE_HIP, band, band_seams, hang_the_band};
use super::collar::{NECK, collar_seams, hang_the_strip, strip};
use super::shape::{draw, sew};
use super::{CHEST, Cut, hang_by_the_chest};

mod scene;

/// Which of the shirt's three hangs carries a heading.
///
/// A garment is one object, so the interesting column is the middle one: it is
/// what a person gets for one press of "Dar rumbo" on the tract they chose,
/// and what the whole of this scene is about is that the other two rings have
/// to follow it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Faced {
    /// Nothing declared, which leaves the storage order turning every ring.
    Nothing,
    /// The body of the shirt alone: one press, on one tract.
    TheBody,
    /// All three, which is the same garment declared three times over.
    Every,
}

impl Faced {
    /// The heading this writes on the body's own chest line.
    fn body(self) -> Option<Heading> {
        (self != Faced::Nothing).then(|| Heading::facing(0.0, Sense::Leftward))
    }

    /// And the one it writes on the collar strip and on the band.
    fn rest(self) -> Option<Heading> {
        (self == Faced::Every).then(|| Heading::facing(0.0, Sense::Leftward))
    }
}

/// The three rings of the shirt, and where each one's centre front is drawn.
///
/// The piece as the document stores it, the abscissa in metres, and a name for
/// the print. The three are the same line of the garment read on three
/// different pieces: the left front's own centre-front edge, and the two nodes
/// of the strip and the band that are sewn to it.
pub const FRONTS: [(usize, &str); 3] = [(0, "cuerpo"), (3, "cuello"), (4, "pretina")];

/// Where each of those three lines is drawn, in metres of pattern.
pub fn fronts(cut: Cut) -> [f64; 3] {
    [
        super::across(cut, 0, 0.0),
        super::collar::centre_front(),
        super::band::centre_front(),
    ]
}

/// The cut every scene here uses: the blouse whose hem lands on the hip ring.
pub const CUT: Cut = TO_THE_HIP;

/// A shirt of five pieces: three panels, a collar strip on the neck ring and a
/// band on the hip ring.
///
/// The garment the heading bench was missing. Everything before it went round
/// one ring or two, and the figure "one declared heading fixes the garment
/// entire" was measured when there was one. Three rings is where the rule has
/// to be read: the body's chest line, the strip's own neck line and the band's
/// own hip line are three declarations of the same garment, and a person who
/// presses "Dar rumbo" once has turned one of them.
///
/// Every station is declared in every scene, because what is read here is the
/// heading and a ring nobody placed has no turn to read at all.
pub fn shirt(cut: Cut, faced: Faced) -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let panels = draw(&mut doc, cut, [0, 1, 2]);
    sew(&mut doc, &panels, false);
    let collar = strip(&mut doc, cut);
    collar_seams(&mut doc, collar, &panels);
    let waist = band(&mut doc, cut);
    band_seams(&mut doc, waist, &panels);
    hang_by_the_chest(&mut doc, CHEST, faced.body());
    hang_the_strip(&mut doc, collar, NECK, faced.rest());
    hang_the_band(&mut doc, waist, HIP, faced.rest());
    doc
}
