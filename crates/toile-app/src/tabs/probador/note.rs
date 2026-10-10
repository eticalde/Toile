use toile_engine::body::Station;
use toile_engine::body::bake::Crossings;
use toile_engine::session::{Elsewhere, Loose};
use toile_engine::sync::Hanging;

use crate::fitting::Fitting;

/// How many places are named before the rest are left to the count.
const NAMED: usize = 2;

/// Where the reading changes unit, in metres as the solver holds it:
/// millimetres under this, centimetres from it up.
///
/// A waistline that settles a tenth of a millimetre off its ring is at it, and
/// a box reading `0.0 cm` says so much less than one reading `0.1 mm` that it
/// would be read as a box that failed to fill.
const FINE: f32 = 0.01;

/// What the body on the stand cost, and where it came from.
///
/// The bake runs on its own thread and the drape is never held up waiting for
/// it, so a person who moves a measurement sees the body change before the
/// field behind it does. This is the whole of the report, and it belongs on
/// the bar that names the body rather than in a dialog nobody asked for.
pub fn cost(fitting: &Fitting) -> String {
    if let Some(why) = fitting.refused.as_deref() {
        return why.to_owned();
    }
    match &fitting.cost {
        Some(cost) if cost.cached => "en caché".to_owned(),
        Some(cost) => format!("horneado en {:.0} ms", cost.ms),
        None => "horneando…".to_owned(),
    }
}

/// Where the body on the stand passes through itself, when it does so
/// somewhere a garment can be: how many pairs of triangles, and the parts of
/// the body they are on.
///
/// Nothing at all for a body that only crosses under its soles. The body the
/// app opens with does, it was accepted knowing so, and a box that lit up on
/// every launch would teach a person to stop reading it before the day it
/// names an armpit.
pub fn crossed(found: &Crossings) -> Option<String> {
    let pairs = found.exposed().count();
    if pairs == 0 {
        return None;
    }
    let places = found.stations();
    if places.is_empty() {
        return Some(pairs.to_string());
    }
    let named: Vec<&str> = places.iter().take(NAMED).map(|s| place(*s)).collect();
    let more = if places.len() > NAMED { "…" } else { "" };
    Some(format!("{pairs} · {}{more}", named.join(", ")))
}

/// What the body is holding up, and how near the rings it names it is holding
/// it: nothing at all for a garment hung from nothing.
///
/// A distance and never a verdict. Whether a tenth of a millimetre matters is
/// the person's to judge and depends on the garment, so the bar says how far
/// the cloth is and leaves the reading to whoever asked for the hang. It is
/// measured at the end of the substep and not where the anchor pulls: the body
/// and the ground both move the cloth afterwards, and read at the pull this
/// same skirt said 1.6 mm where its run settles 1.1 mm from its ring.
pub fn hanging(held: Option<Hanging>) -> Option<String> {
    let Hanging { runs, gap } = held?;
    let run = if runs == 1 { "tramo" } else { "tramos" };
    let off = if gap < FINE {
        format!("{:.1} mm", gap * 1000.0)
    } else {
        format!("{:.1} cm", gap * 100.0)
    };
    Some(format!("{runs} {run} · {off} del anillo"))
}

/// Where the product was let go, when the ring it was asked for is not the
/// ring its own size points at: nothing at all when the two agree.
///
/// Both names, because either alone is half the sentence. The declared one won
/// and the garment is there now, so this is not a complaint — it is the reading
/// that keeps a garment put down a hand's breadth from where it was drafted
/// for from being a silence. Quiet on agreement for the reason [`crossed`] is
/// quiet on a body that only crosses under its soles.
pub fn worn(elsewhere: Option<&Elsewhere>) -> Option<String> {
    let Elsewhere { declared, inferred } = elsewhere?;
    Some(format!("{declared} · inferido {inferred}"))
}

/// How much bigger than the cloth on it a ring had to be opened, and the ring
/// it was: nothing at all for a garment let go on a hoop near its own size.
///
/// Not a complaint about the placement, which is doing its work: the opening
/// walk lets a surface out until the person under it is clear. It is the one
/// symptom of declaring the wrong ring, and the only form that mistake can
/// reach anybody in — a 40 cm panel hung from this body's waist comes off an
/// 83.6 cm hoop and covers less than half of it. Quiet for the reason
/// [`crossed`] is quiet on a body that only crosses under its soles: every
/// garment this tree places today reads under 1.2×.
pub fn loose(read: Option<&Loose>) -> Option<String> {
    let read = read?;
    let ring = match &read.station {
        Some(station) => format!(" \u{b7} {station}"),
        None => String::new(),
    };
    Some(format!("{:.1}\u{d7} su tela{ring}", read.hoop / read.cloth))
}

/// How many pieces of the product the release could not put on the body:
/// nothing at all when it placed every one of them.
///
/// A count and not a verdict, for the reason [`hanging`] gives one. What is
/// behind it is a ring of the garment one of whose pieces carries three of that
/// ring's own seams — a shirt's back under a collar nobody said where to put —
/// and those pieces are let go flat over the person instead of on them. That is
/// worth a reading: refusing to drape the garment would be worth less, and
/// saying nothing is how a garment comes to be a metre overhead with the bar
/// reporting a good day.
///
/// Quiet at zero, which is nearly every product and every launch: the document
/// the app opens is sewn into one chain, so the box takes no room on the bar
/// until the day a person draws a piece it cannot place.
pub fn adrift(pieces: usize) -> Option<String> {
    if pieces == 0 {
        return None;
    }
    let piece = if pieces == 1 { "pieza" } else { "piezas" };
    Some(format!("{pieces} {piece} sin sitio"))
}

/// A station as a person names that part of a body.
///
/// The mesh tags a whole foot with its ankle and a whole hand with its wrist,
/// so those two say both.
fn place(station: Station) -> &'static str {
    match station {
        Station::Ankle => "tobillo y pie",
        Station::Calf => "pantorrilla",
        Station::Knee => "rodilla",
        Station::Thigh => "muslo",
        Station::Crotch => "entrepierna",
        Station::Hip => "cadera",
        Station::Waist => "cintura",
        Station::Underbust => "bajo pecho",
        Station::Bust => "pecho",
        Station::Armpit => "axila",
        Station::Shoulder => "hombro",
        Station::NeckBase => "base del cuello",
        Station::NeckTop => "cuello",
        Station::Jaw => "mandíbula",
        Station::Cheek => "cara",
        Station::HeadMax => "cabeza",
        Station::Crown => "coronilla",
        Station::Wrist => "muñeca y mano",
        Station::Forearm => "antebrazo",
        Station::Elbow => "codo",
        Station::Biceps => "brazo",
        Station::Deltoid => "deltoides",
    }
}
