use std::f64::consts::TAU;

use toile_engine::body::bake;
use toile_engine::couture::{Layout, Wrap};

use super::{let_go, placed, report};
use crate::extremes::wide_hipped;
use crate::skirt::Cut;
use crate::watch::{buried, reference};

/// Every field of a placement and of a wrap, named once so that a new one has
/// to be answered for somewhere.
///
/// The half that matters is in `Layout::point` itself, which destructures for
/// the same reason `Bare::of` does: a field added and forgotten there would be
/// dropped by the one function every released vertex goes through, and it
/// breaks the crate's own build rather than a test's. What is left for here is
/// `Wrap`, which `point` never opens, and the one crossing the two: that the
/// radius a client is shown is the surface read at the crest.
fn every_field_of(ring: Layout) -> (f64, f32, f64) {
    let Layout {
        axis,
        radius,
        stand,
        crest,
        wraps,
        round,
    } = ring;
    let turns: Vec<f64> = wraps
        .into_iter()
        .flatten()
        .map(|wrap| {
            let Wrap { turn, sense } = wrap;
            turn * sense
        })
        .collect();
    assert!(!turns.is_empty(), "the walk placed something");
    assert!(
        (round.radius(crest) - radius).abs() < 1.0e-12,
        "the crest's radius is the surface read there and nothing else: {radius} \
         against {}",
        round.radius(crest)
    );
    assert!(axis[0].is_finite() && axis[1].is_finite());
    (radius, stand, crest)
}

/// The surface two bodies' skirts are let go on, and what each one releases
/// open.
///
/// Read and not hashed, deliberately. A `Layout` comes out of the body as much
/// as out of the cloth — the profile is opened band by band until the person
/// stops swallowing the panels — so a hash would re-pin the bake.
///
/// What is asserted instead is what the rule promises: the line the garment
/// hangs by is let go on a hoop of its own length, no hoop anywhere is shorter
/// than the cloth it carries, and the surface stands within the body's own
/// height rather than over its crown. Which ring it stands at is sharper, and
/// `blouse/station.rs` asks it: these two declare no station.
///
/// What is *not* asserted, though the name this test carried said it was: that
/// nothing is released inside the person. Plenty is, and how far in is printed
/// and not held to anything — there is no bar for it, and the field cannot
/// carry one.
#[test]
#[ignore = "release-only: two real bodies baked"]
fn the_surface_lands_every_line_of_cloth_on_a_hoop_its_own_size() {
    for (scene, mesh, cut) in [
        ("reference adult", reference(), Cut::REFERENCE),
        ("wide-hipped body", wide_hipped(), Cut::WIDE_HIPPED),
    ] {
        let (session, sdf) = let_go(&mesh, cut);
        let (lo, hi) = session.collider().extent();
        let ring = session.layout().expect("the seams place the skirt");
        let hoop = ring.radius * TAU;
        let crest = ring.round.girth(ring.crest);
        // The widest cloth the product carries, in metres: both halves of this
        // fixture run at their full width from the hip to the hem.
        let widest = 2.0 * cut.hip / 100.0;
        let start = placed(&session.released());
        let (deep, all) = buried(&sdf, &start);
        // How near the band the deepest released particle stands, and how many
        // are under the skin at all. `place::around` opens bands until
        // `Collider::swallows` reports none, off this very field baked
        // again, so the count past the band is its own stopping rule
        // read back. These two are what it actually left behind: measured,
        // 6,664 of 22,826 and 8,846 of 36,951 under the skin, and the
        // deepest of them 51 µm and 12 µm clear of the band — 1 µm on the
        // same bodies with no waistband. Clear of the *band*, which is
        // itself 25 mm in: that particle stands 24.95 mm under the skin,
        // and 27 mm and 32 mm from the nearest skin vertex. Nothing here
        // reads as standing clear of the person, and the field cannot say
        // it does — it saturates at the band, and the walk stops the
        // moment the reading rounds above it.
        let (mut margin, mut under) = (f32::MAX, 0usize);
        for p in &start {
            let d = sdf.sample(p[0], p[1], p[2]);
            margin = margin.min(d + bake::BAND as f32);
            under += usize::from(d < 0.0);
        }
        println!(
            "{scene}: crest {:.4} carries {crest:.5} m of cloth onto a {hoop:.5} m hoop at \
             {:.4}, {:.1} % coverage · widest cloth {widest:.5} m on {:.5} m · body \
             {:.4}..{:.4} · {deep} of {all} released past the band, {under} under the skin, \
             the deepest of them {:.1} µm clear of the band",
            ring.crest,
            ring.stand,
            100.0 * crest / hoop,
            TAU * ring.round.radius(-cut.hip_drop / 100.0),
            lo[1],
            hi[1],
            margin * 1.0e6,
        );
        report(scene, "at release", &session.sewn_pairs(), &start);
        // A coverage is a ratio of cloth to hoop, so it takes cloth to have
        // one, and the crest is not always a line that carries any: the
        // shipped block's is the tip of the tab its back ends in,
        // 0.00000 m of cloth on a 0.44075 m hoop, and a figure read
        // there is 0 % and says nothing about anything. Both scenes
        // here hang by an elastic, so their crest is the band's own
        // line and the ratio below means what it says. This is
        // what stops it being asked where it cannot be answered.
        assert!(
            crest > 0.0,
            "{scene}: the line the garment hangs by carries no cloth, so a \
             coverage read at it is no reading of this surface at all"
        );
        // The defect this rule exists to end. One radius for the whole garment
        // had to serve the waistband and the hip at once: on the wide-hipped
        // body the hip carries 1.17 m of cloth and the band 0.614, and
        // the one hoop that cleared the person let the band go at 47 %
        // of its own length, with its two ends most of its own length
        // apart.
        assert!(
            crest / hoop > 0.95,
            "{scene}: the line the garment hangs by is let go at {:.1} % of its \
             own length",
            100.0 * crest / hoop
        );
        // And nowhere is a hoop shorter than the cloth on it. Rolling preserves
        // length along a hoop, so a hoop short of its cloth could only be
        // filled by the pieces overlapping each other on it.
        let (from, to) = ring.round.span();
        for k in 0..=400 {
            let y = from + (to - from) * f64::from(k) / 400.0;
            let (hoop, cloth) = (TAU * ring.round.radius(y), ring.round.girth(y));
            assert!(
                hoop >= cloth - 1.0e-9,
                "{scene}: at ordinate {y} the cloth is {cloth} round and the hoop \
                 it lands on is {hoop}"
            );
        }
        // Nearly the walk's own stopping rule read back, and two ways short of
        // it: a garment the field never reports clear leaves the walk on the
        // ceiling with cloth still swallowed, and a piece the strip does not
        // place falls back to the flat release, which the walk never asks about
        // and this counts. Neither is reachable in the scenes here, so on these
        // two bodies it is a reading.
        //
        // Giving it room of its own was measured and costs the rule: stop the
        // walk one opening short of the band and the release stands 5.2 mm and
        // 5.7 mm clear instead of 50.7 µm and 12.4 µm — and the waistband,
        // which is the tightest hoop in the garment and so the first
        // band a margin opens, comes off a hoop 3.5 % longer than its
        // own cloth with the held run stretched 2.87 % against the 1 %
        // `stretch.rs` holds it to.
        assert_eq!(
            deep, 0,
            "{scene}: the surface released {deep} of {all} particles past the band, \
             where the field is saturated flat and no contact solve has a normal \
             to carry them out along"
        );
        assert!(
            ring.stand > lo[1] && ring.stand < hi[1],
            "{scene}: the garment is hung somewhere down the body and not over \
             its crown: {} against {}..{}",
            ring.stand,
            lo[1],
            hi[1]
        );
        let (radius, stand, at) = every_field_of(ring);
        assert!(radius > 0.0 && stand.is_finite() && at.is_finite());
    }
}
