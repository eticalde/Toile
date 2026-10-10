use std::f64::consts::TAU;

use super::group::Group;
use super::stands::stands;
use super::strip::{Step, across, adjacency, walk};
use super::{BAND_HEIGHT, Band, Elsewhere, OPENING, Sewn};
use crate::body::Collider;
use crate::couture::{Layout, Pin, Round, ShapePipeline};

/// One group of a product rolled onto one surface.
pub(super) struct Rolled {
    /// The surface, and where the strip lies on it.
    pub(super) layout: Layout,
    /// The two rings, when the declaration and the body's own reading name
    /// different ones.
    pub(super) elsewhere: Option<Elsewhere>,
    /// The cloth the strip carries across, in metres.
    pub(super) girth: f64,
    /// How far round the ring the hung line landed on goes, in metres.
    pub(super) hoop: f64,
}

/// One group of a product rolled onto one surface, and what the body had to say
/// about the station it declared.
///
/// Three readings, from three places. The seams chain the group's pieces into a
/// strip and say which way round each one runs. The cloth says how big the
/// surface is at every height. The body says where on itself a garment that
/// size belongs, and which of those heights have to be opened to go round it at
/// all.
///
/// One radius cannot do the middle reading. A skirt's waistband carries 61 cm
/// of cloth and its hip 117, and the one hoop that cleared the person was 130
/// cm round: the band — the line that has to close first and holds the garment
/// up — was let go at 47 % coverage, its two ends most of its own length apart.
/// A hoop per ordinate lands every line of cloth on a hoop its own size, so the
/// arc a seam walks is the arc its cloth gives it.
pub(super) fn ring(
    group: &Group,
    sewn: &[Sewn],
    pipes: &[&ShapePipeline],
    collider: &Collider,
    (band, pin): (Option<Band>, Option<Pin>),
) -> Option<Rolled> {
    let (steps, closed) = chain(group, sewn, pipes)?;
    if steps.iter().any(|s| s.hi - s.lo <= f64::EPSILON) {
        return None;
    }
    // Across the cloth, never between the seams' middles. A seam runs down the
    // edge of a piece and the cloth reaches past it to the piece's own edge, so
    // measuring middle to middle loses everything outside the two seams — a
    // fifth of this block's trousers, which would then wrap a ring a fifth too
    // small and overlap each other on it.
    let girth: f64 = steps.iter().map(|s| s.hi - s.lo).sum();
    let (top, hem) = rise(&steps, pipes);
    // A garment is held up by one line of itself, so that line is the one
    // matched to a ring and the one stood on it. With an elastic that is the
    // elastic; without one it is the topmost edge, as it always was. Read the
    // widest girth for both and a tube skirt is matched by its hip to the ring
    // nearest that — the upper chest, measured — and then stood with its
    // waistline up there, two rings above where a waistband belongs.
    let (hangs_by, crest) = band.map_or((girth, top), |band| (band.girth, band.at));
    // A declared station names the line of cloth as well as the ring. The run
    // a person hung is the one that goes to that height, and every other line
    // keeps the rise the pattern puts between them — which is the same rule the
    // elastic already follows, applied to the louder of the two declarations.
    let station = group.worn.as_ref().map(|worn| worn.station.as_str());
    let crest = group.worn.as_ref().map_or(crest, |worn| worn.at);
    let (axis, stand, elsewhere) = stands(collider, station, hangs_by, girth, crest - hem);

    // The garment's own size first, opened only at the heights the person under
    // it makes necessary. Cloth let go inside the body starts past the band
    // where the field is saturated flat and no contact solve has a normal to
    // carry it out along.
    let far = outermost(collider, axis);
    let order: Vec<(usize, f64)> = steps.iter().map(|s| (s.piece, s.sense)).collect();
    // The pin and not the heading: which way round the body a line of cloth
    // goes is a reading of the pattern, taken where the runs are resolved, and
    // the surface is handed the line it is to fix rather than the document.
    // Every ring is offered it and [`Round::over`] keeps it on the one ring
    // whose cloth carries it, so a heading declared on a collar does not turn
    // the body of the shirt.
    let round = Round::over(&order, pipes, closed, BAND_HEIGHT, pin)?;
    let mut layout = Layout::on(round, axis, stand, crest, pipes.len());
    // Band by band rather than the whole surface at once, which is the point:
    // a garment that fits the person over most of its height is opened only
    // where it does not, and everywhere else the hoops stay the size of the
    // cloth and the seams meet. The walk cannot circle — every round either
    // stops or opens a band it has not yet walked out to `far`.
    while layout.open(&swallowed(&layout, pipes, collider), OPENING, far) {}
    // The hoop read after the opening walk and the cloth as the strip carries
    // it across, which is the pair the ring was matched from. Read at the hung
    // line, because that is the line a declaration names and the one a garment
    // is held up by.
    let hoop = layout.radius * TAU;
    Some(Rolled {
        layout,
        elsewhere,
        girth,
        hoop,
    })
}

/// The order one group's pieces are rolled in, and whether its strip closes.
///
/// Two ways in, and the declaration is the first of them. A group its seams
/// chain is walked, exactly as the one strip of a product always was. A group
/// of a single piece has no chain to walk and is placed only where a person
/// said where it goes — a collar strip on the neck ring — because a lone piece
/// nobody placed is the panel the tree releases flat, and that release is what
/// the drape goldens hash.
fn chain(group: &Group, sewn: &[Sewn], pipes: &[&ShapePipeline]) -> Option<(Vec<Step>, bool)> {
    if group.seams.is_empty() {
        let &[piece] = group.pieces.as_slice() else {
            return None;
        };
        let (lo, hi) = across(pipes.get(piece)?);
        group.worn.as_ref()?;
        return Some((
            vec![Step {
                piece,
                sense: 1.0,
                lo,
                hi,
            }],
            false,
        ));
    }
    let seams: Vec<&Sewn> = group.seams.iter().filter_map(|&k| sewn.get(k)).collect();
    let joins = adjacency(&seams, pipes.len())?;
    let (steps, closed) = walk(&seams, pipes, &joins)?;
    (steps.len() == group.pieces.len()).then_some((steps, closed))
}

/// Every pattern ordinate at which the body has swallowed some of the rolled
/// cloth.
///
/// The ordinates and not the vertices, because what is opened is a height: a
/// panel caught at the hip says the hip's hoop is too small, and says nothing
/// at all about the waistband two hand's breadths above it.
///
/// At the band and not a step short of it, which was measured and costs the
/// whole point of the rule. Stopping the walk one opening early does give the
/// release a real margin — 5.2 mm and 5.7 mm on the two skirts, against the
/// 51 µm and 12 µm `fit.rs` reads — and it shuts the shipped block's seams
/// besides, 1 pair of 381 standing open at the mark instead of 5. It pays for
/// that at the one place that must not pay: the tightest hoop in a garment is
/// the line it hangs by, so the waistband is the first band a margin opens,
/// and the reference skirt's band comes off a hoop 3.5 % longer than its own
/// cloth with the held run stretched 2.87 % — a garment let go too big at the
/// band, which is the defect the ordinate-by-ordinate hoop exists to end.
fn swallowed(layout: &Layout, pipes: &[&ShapePipeline], collider: &Collider) -> Vec<f64> {
    let mut inside = Vec::new();
    for (at, pipe) in pipes.iter().enumerate() {
        for &p in &pipe.pos2d {
            if layout.point(at, p).is_some_and(|q| collider.swallows(q)) {
                inside.push(p[1]);
            }
        }
    }
    inside
}

/// The radius past which none of the body is left to clear: from the axis to
/// the furthest corner of the body's own box.
///
/// The opening stops here whatever the field says, so a garment the field
/// never reports clear cannot be walked outwards for ever.
fn outermost(collider: &Collider, axis: [f32; 2]) -> f64 {
    let (lo, hi) = collider.extent();
    let reach = |far: f32, near: f32, from: f32| {
        f64::from(far - from)
            .abs()
            .max(f64::from(from - near).abs())
    };
    let x = reach(hi[0], lo[0], axis[0]);
    let z = reach(hi[2], lo[2], axis[1]);
    (x * x + z * z).sqrt()
}

/// The highest and lowest pattern ordinate over the pieces a walk places.
fn rise(steps: &[Step], pipes: &[&ShapePipeline]) -> (f64, f64) {
    steps.iter().fold((f64::MIN, f64::MAX), |(hi, lo), step| {
        pipes[step.piece]
            .pos2d
            .iter()
            .fold((hi, lo), |(h, l), p| (h.max(p[1]), l.min(p[1])))
    })
}
