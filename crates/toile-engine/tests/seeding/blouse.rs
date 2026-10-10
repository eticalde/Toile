use shape::{BESIDE, draw, range, sew};
use toile_engine::draft::{Command, Doc, Hang, Heading, Identity, MeasureSet, Sense};

/// The station a blouse is hung from: the ring a bodice sits on.
///
/// `cintura` is what every fixture before this one hung from, and it is the
/// wrong ring for a garment whose own line of cloth is at the chest. Reading
/// the two apart is the whole point of the scenes here.
pub const CHEST: &str = "pecho_alto";

/// The names the three panels carry in the product tree, in the order a walk
/// round the body meets them.
pub const PANELS: [&str; 3] = ["Delantero izquierdo", "Espalda", "Delantero derecho"];

/// The tag each panel's nodes are suffixed with, in the same order.
pub const TAGS: [&str; 3] = ["_izq", "_esp", "_der"];

/// One blouse's measurements, in centimetres of pattern.
///
/// A cut and not four constants, for the reason the skirt's is one: the same
/// draft rule taken off a second person is a second cut of the one fixture.
#[derive(Debug, Clone, Copy)]
pub struct Cut {
    /// How wide each front panel is.
    pub front: f64,
    /// How wide the back is.
    pub back: f64,
    /// Chest line to hem.
    pub length: f64,
}

impl Cut {
    /// A blouse that fits the reference adult's chest.
    ///
    /// Two fronts and a back make 1.0400 m of cloth, within a centimetre of
    /// that body's own 1.0475 m upper chest: the garment is the size of the
    /// person. Its hem reaches past the hip, and the hip's 1.0299 m is just
    /// inside the garment — which is what leaves the chest wearable at all,
    /// since a garment is only worn where everything it passes is narrower.
    pub const REFERENCE: Cut = Cut {
        front: 26.0,
        back: 52.0,
        length: 55.0,
    };

    /// The same blouse cut for a narrower chest: 0.8800 m of cloth.
    ///
    /// The interesting one, and it is not a strange garment — it is the same
    /// drafting rule run against a chest sixteen centimetres smaller, which is
    /// one person in a family. On the reference body its cloth is nearer the
    /// waist than the chest, and the waist cannot carry it because the hip
    /// below is wider than the garment: that is how its own size comes to point
    /// at a ring 38 cm from the one it was drafted for.
    pub const NARROW: Cut = Cut {
        front: 22.0,
        back: 44.0,
        length: 55.0,
    };

    /// How much cloth the blouse carries round the body, in metres.
    pub fn girth(self) -> f64 {
        (self.front + self.front + self.back) / 100.0
    }
}

/// Which panel the document holds first, second and third.
///
/// The three panels are the same garment in every one of these: the drawing,
/// the widths and the seams do not move. What moves is the order the file keeps
/// them in, which is the one thing no part of a placement should be able to
/// read — and which every reading of this blouse before a heading could be
/// declared came out of.
pub const ORDERS: [([usize; 3], &str); 3] = [
    ([0, 1, 2], "delantero izquierdo primero"),
    ([1, 0, 2], "espalda primera"),
    ([2, 1, 0], "delantero derecho primero"),
];

/// One blouse to let go: how it is stored, how it closes, and what it declares.
#[derive(Debug, Clone, Copy)]
pub struct Scene {
    /// The panels in the order the document is to hold them.
    pub order: [usize; 3],
    /// Whether the two fronts are sewn to each other at the centre front.
    ///
    /// Buttoned, the strip closes into a tube and has no free end to be walked
    /// from, so the walk opens it at the first piece carrying two seams — the
    /// storage order again, and with a whole lap to be wrong by rather than
    /// half a panel.
    pub buttoned: bool,
    /// The ring the chest line is hung from, or `None`.
    pub station: Option<&'static str>,
    /// Which way the left front's centre-front edge is turned, or `None`.
    pub heading: Option<Heading>,
}

impl Scene {
    /// The blouse as it was measured before any heading: open at the centre
    /// front, stored front first, declaring the chest and nothing else.
    pub const PLAIN: Scene = Scene {
        order: [0, 1, 2],
        buttoned: false,
        station: Some(CHEST),
        heading: None,
    };

    /// The sentence of the trade this bench declares: the left front's own
    /// centre-front edge is the centre front, and the chest line runs on from
    /// it toward the wearer's left.
    ///
    /// That edge is one of the strip's two free ends, which is what makes it
    /// the opening of the garment, and the hang's run begins exactly there —
    /// so a pin of 0 is that edge and nothing else.
    pub fn facing_the_front(self) -> Scene {
        Scene {
            heading: Some(Heading::facing(0.0, Sense::Leftward)),
            ..self
        }
    }
}

/// A blouse of three panels at one height: two fronts, a back, and the side
/// seams that chain them into a strip.
///
/// Open at the centre front, so the strip has two free ends and the walk places
/// it with the rule that is already in the tree. What a third seam on one panel
/// costs is a scene of its own.
///
/// `station` is the ring the chest line is hung from, or `None` for the same
/// blouse declaring nothing — the control. Every scene here has both, because a
/// reading of the declared one alone says nothing at all.
pub fn blouse(cut: Cut, station: Option<&str>) -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let panels = draw(&mut doc, cut, [0, 1, 2]);
    sew(&mut doc, &panels, false);
    if let Some(station) = station {
        hang_by_the_chest(&mut doc, station, None);
    }
    doc
}

/// The same blouse built to a scene: stored in its order, closed as it says,
/// and declaring what it declares.
pub fn staged(cut: Cut, scene: Scene) -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let panels = draw(&mut doc, cut, scene.order);
    sew(&mut doc, &panels, scene.buttoned);
    if let Some(station) = scene.station {
        hang_by_the_chest(&mut doc, station, scene.heading);
    }
    doc
}

/// The pattern abscissa, in metres, a fraction `along` across panel `k`.
///
/// In metres because that is the unit the cloth carries: the fixture draws in
/// centimetres and the mesher hands back metres, so a reading taken against the
/// drawing in the drawing's own unit lands a hundred panels away.
pub fn across(cut: Cut, k: usize, along: f64) -> f64 {
    let widths = [cut.front, cut.back, cut.front];
    (BESIDE * k as f64 + along * widths[k]) / 100.0
}

/// Hangs every panel's chest line from `station`, turning the left front's to
/// face `heading`.
///
/// All three and not one, because the line a product hangs by runs round the
/// whole of it: a blouse hung by its back alone would be held at one ring and
/// let go from the other two.
///
/// One heading and not three, which is the figure the whole bench turns on: a
/// single declared rumbo has to fix the garment entire, whatever order the file
/// keeps its panels in. Three would hide a reader that silently took whichever
/// one it met first.
pub fn hang_by_the_chest(doc: &mut Doc, station: &str, heading: Option<Heading>) {
    for (k, name) in PANELS.iter().enumerate() {
        let piece = doc
            .piece_named(name)
            .expect("the fixture draws every panel before this is called");
        let tag = TAGS[k];
        let at = range(
            doc,
            piece,
            &format!("pecho_izq{tag}"),
            &format!("pecho_der{tag}"),
        );
        let hang = match heading.filter(|_| k == 0) {
            Some(heading) => Hang::facing(at, station, heading),
            None => Hang::new(at, station),
        };
        Command::AddHang {
            identity: Identity::New,
            hang,
        }
        .apply(doc)
        .expect("the chest line runs between two nodes of the piece it names");
    }
}

/// How the panels are drawn and chained on the table.
mod shape;

/// A blouse whose collar strip gives one of its panels a third seam.
mod collar;

/// The same defect read the other way up: a band across the three hems.
mod band;

/// Which ring a declared blouse is let go on, measured against the same blouse
/// declaring nothing.
mod station;

/// Which way round the body a declared blouse faces, in every order a document
/// could hold its panels in.
mod heading;

/// The same blouse with a collar strip and a band at once: three rings, and
/// one heading to turn all of them.
mod shirt;

/// The same blouse with a second component beside it that nothing declares.
mod loose;
