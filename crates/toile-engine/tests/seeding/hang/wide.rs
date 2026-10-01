use toile_engine::body::{Collider, bake, measured_anny};
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::draft::Doc;
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

use super::off_its_ring;
use crate::extremes::{GATHERED, MET, hips_a_little_wide, wide_hipped};
use crate::fit::report;
use crate::skirt::{Cut, cut_to, hang_from_the_waist};
use crate::watch::{REST, SETTLED, at_substep, buried, rest_by, span, through_the_drape};

/// The wide-hipped skirt's side seam, hung from the waist and not.
///
/// `extremes.rs` records that this seam now meets, and asserts that it does.
/// This is the same reading with the anchor on, kept because the anchor was the
/// remedy decision 12 held in reserve for it — and because what closed the seam
/// was not the anchor. Every height of the garment is let go on a hoop of that
/// height's own cloth, so the arc a seam has to walk is the arc the cloth gives
/// it; measured, the anchor moves the gap by a fifth of a millimetre.
///
/// And the anchor's own margin on this body, which until now nothing watched:
/// the whole suite read it on the reference adult, and how far an anchor has to
/// pull is a thing the body's shape decides.
#[test]
#[ignore = "release-only: an extreme body baked and two whole drapes run"]
fn hanging_the_wide_hipped_skirt_from_the_waist_leaves_its_side_seam_shut() {
    let mesh = wide_hipped();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");

    let bare = worn(
        "unhung, wide-hipped",
        cut(false),
        &body,
        &sdf,
        Stops::Quietly,
    );
    let hung = worn("hung, wide-hipped", cut(true), &body, &sdf, Stops::Never);
    assert!(
        bare.0 < MET,
        "the wide-hipped side seam rests {:.1} mm open, against the {:.1} mm a \
         met seam is held to",
        bare.0 * 1000.0,
        MET * 1000.0
    );
    // And the anchor costs some of that, which is worth a number rather than a
    // silence: measured, 8.4 mm against the unhung 1.7 mm. The waistline is
    // held at its ring while the cloth below it hangs, and the side seam
    // pays the difference. It was 119.6 mm here, so this is the same
    // reading fourteen times better and still not met.
    assert!(
        hung.0 < 0.010,
        "and hung it rests {:.1} mm open, against the {:.1} mm unhung",
        hung.0 * 1000.0,
        bare.0 * 1000.0
    );
    assert_eq!(bare.1, None, "the control hangs from nothing at all");
    let (mean, worst) = hung.1.expect("the hung skirt names a ring");
    println!(
        "the wide-hipped waistline stands {:.2} mm off its ring, the run averaging {:.2} mm",
        worst * 1000.0,
        mean * 1000.0
    );
    // And the anchor is doing something here, which how far the run ended from
    // its ring cannot say: measured, both seam readings above pass with the
    // anchor's dose taken to zero, because what shuts this side seam is the
    // hoop and not the hang. What the dose does move is where the garment
    // ends — 0.3333 held against 0.2672 unhung, 66 mm of it — and with the
    // dose off the two read the same to four decimals.
    assert!(
        hung.2 > bare.2,
        "the hung skirt stands no higher than the one held by its band alone: \
         {:.4} against {:.4}",
        hung.2,
        bare.2
    );
}

/// The same anchor on the body between that one and the reference adult, read
/// and not capped.
///
/// `hips-circ` at 0.6 where the wide-hipped body has 1.0, on the same waist and
/// thigh levers: measured, a 105.4 cm hip over a 61.3 cm waist, which is what a
/// person gets by typing those two numbers. It is here because it is the body
/// the old cap broke on: the reference adult settles 2.6 mm off its ring and
/// the wide-hipped body 4.8, both under the anchor's 5.0 mm dose, and this one
/// settles at 8.9. An interior point of what the sliders draw, not an edge of
/// it — a suite reading only the two ends went on reporting a bound the middle
/// breaks.
#[test]
#[ignore = "release-only: a slider body baked and two whole drapes run"]
fn the_anchor_ends_further_from_its_ring_on_the_body_between_the_two() {
    let mesh = hips_a_little_wide();
    let tape = measured_anny(&mesh);
    let of = |name: &str| {
        tape.get(name)
            .unwrap_or_else(|| panic!("the tape reports `{name}`"))
    };
    println!(
        "hips 0.6 body: waist {:.1} cm · hip {:.1} cm · drop {:.1} cm · thigh {:.1} cm",
        of("cintura"),
        of("cadera"),
        of("cadera") - of("cintura"),
        of("muslo")
    );
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");

    let bare = worn("unhung, hips 0.6", cut(false), &body, &sdf, Stops::Quietly);
    let hung = worn("hung, hips 0.6", cut(true), &body, &sdf, Stops::Never);
    assert_eq!(bare.1, None, "the control hangs from nothing at all");
    let (mean, worst) = hung.1.expect("the hung skirt names a ring");
    println!(
        "the hips 0.6 waistline stands {:.2} mm off its ring, the run averaging {:.2} mm",
        worst * 1000.0,
        mean * 1000.0
    );
    // And the garment is held up, which is the one thing the anchor is for and
    // the one thing a reading with no cap can still hold it to. Higher, with no
    // margin named: on this body the skirt is long enough that both scenes end
    // on the floor, so what the anchor buys is the top alone and it buys 74 mm
    // of it — against the reference adult's whole metre, where the
    // control's hem reaches the ground and the hung one's does not.
    println!(
        "hips 0.6: hung, the cloth ends {:.4} against {:.4} unhung — {:.1} mm of \
         garment the anchor is holding up",
        hung.2,
        bare.2,
        (hung.2 - bare.2) * 1000.0
    );
    assert!(
        hung.2 > bare.2,
        "the hung skirt stands no higher than the one held by its band alone: \
         {:.4} against {:.4}",
        hung.2,
        bare.2
    );
}

/// The wide-hipped skirt, hung from the waist or not.
fn cut(hung: bool) -> Doc {
    let mut doc = cut_to(Cut::WIDE_HIPPED, Some((GATHERED, HOLDS_ITS_RATIO)));
    if hung {
        hang_from_the_waist(&mut doc);
    }
    doc
}

/// Whether a scene's drape goes quiet inside the suite's own rest budget, which
/// decides where the scene is read.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Stops {
    /// It goes quiet, and is read wherever it did.
    Quietly,
    /// It does not, and is read at [`SETTLED`] instead.
    Never,
}

/// Drapes one wide-hipped scene and hands back how far its worst seam stands
/// open and how far its hung run ended from the ring it names.
///
/// The two scenes stop differently. Unhung, the skirt slides off the hip and
/// heaps on the floor, and a heap goes quiet — measured, inside 14,880
/// substeps. Hung, it goes *still* without ever going *quiet*: its worst sewn
/// pair and its worst hung vertex read 8.36 mm and −4.81 mm at substep 12,000
/// and the same to the hundredth of a millimetre at 120,000, with the frame
/// saying asleep false the whole way and [`REST`] reached without a verdict.
/// Nothing about that is the placement's, so the hanging scene is read at
/// [`SETTLED`], where the reference scenes are read, and the sleep is left to
/// whoever owns it.
fn worn(
    scene: &str,
    doc: Doc,
    body: &Collider,
    sdf: &SdfGrid,
    stops: Stops,
) -> (f32, Option<(f64, f64)>, f32) {
    let session = Session::from_doc(doc, body.clone()).expect("the skirt drapes");
    assert!(
        session.seam_faults().is_empty(),
        "{scene}: the skirt's seams all pair: {:?}",
        session.seam_faults()
    );
    let hung = session.hung_cloth();
    let watched = through_the_drape(&session, sdf);
    assert_eq!(
        watched.worst.0, 0,
        "{scene}: {} particles were driven past the band at substep {}",
        watched.worst.0, watched.worst.1
    );
    let (points, when) = match stops {
        Stops::Quietly => {
            let (points, at) = crate::watch::at_rest(&session);
            (points, format!("at rest by {at}"))
        }
        Stops::Never => (at_substep(&session, SETTLED), format!("at {SETTLED}")),
    };
    let (low, high) = span(&points);
    let floor = body.ground().expect("a baked body stands on a plane");
    let (deep, all) = buried(sdf, &points);
    let off = off_its_ring(&hung, &points);
    println!(
        "{scene} {when}: cloth {low:.4}..{high:.4} over a floor at {floor:.4} · {} vertices \
         hung, {} · {deep} of {all} buried",
        hung.len(),
        off.map_or_else(
            || "nothing held".to_owned(),
            |(mean, worst)| format!(
                "standing {:.1} mm off its ring, worst vertex {:.1} mm",
                mean * 1000.0,
                worst * 1000.0
            )
        )
    );
    assert_eq!(
        deep, 0,
        "{scene}: {deep} of {all} particles are buried past the band"
    );
    assert!(
        low >= floor,
        "{scene}: the garment went through the ground: {low} under {floor}"
    );
    let worst = report(scene, &when, &session.sewn_pairs(), &points)
        .expect("the skirt is sewn into a tube")
        .0;
    // And whether it ever stops, reported and not waited on for the scene that
    // does not: a pendulum held at the waist is the shape of that, and no rest
    // budget this suite could carry would reach the end of one.
    if stops == Stops::Quietly {
        println!(
            "{scene}: {}",
            match rest_by(&session, REST) {
                Some((_, at)) => format!("asleep by {at}"),
                None => format!("still moving at {REST}"),
            }
        );
    }
    (worst, off, high)
}
