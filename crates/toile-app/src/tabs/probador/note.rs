use toile_engine::body::Station;
use toile_engine::body::bake::Crossings;

use crate::fitting::Fitting;

/// How many places are named before the rest are left to the count.
const NAMED: usize = 2;

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
