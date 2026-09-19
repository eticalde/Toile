#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// The same habits over a product of two sewn pieces.
mod product;
/// The tube skirt an elastic is proved on.
mod skirt;
/// The vocabulary every scene here is measured in.
mod watch;
/// What holds a garment on, measured against the same garment with nothing
/// holding it.
mod worn;

use toile_engine::body::{Collider, bake};
use toile_engine::draft::block;
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;
use watch::{LANDED, LONG, at_rest, buried, footing, parks_by, reference, span, touching};

/// How close to the ground the whole garment lies once it has parked, in
/// metres, for a scene that came to rest on the floor rather than on the body.
const ON_THE_GROUND: f32 = 0.15;

/// What a seeded scene is held to: the garment came down, it came to a stop,
/// none of it went through the ground, and none of it is buried where the
/// field could never push it out again.
///
/// Measured at rest, which is a thing that exists only since the floor landed.
/// Before it, a garment let go over a body slid off and fell for as long as
/// anything integrated it — hundreds of metres inside the minute — so these
/// scenes had to be read at a fixed substep instead.
///
/// Burial is the sharp one. The others say the drape behaved; this one is the
/// bug the tests were written for, where a garment let go at some other body's
/// height starts inside this one and no contact solve can carry it out.
///
/// Where the heap came to rest is what keeps the other three from being
/// satisfied by a garment that never met the body at all: one let go beside
/// the person comes down, stops above the ground, and is buried in nothing.
fn rests_clear_of_the_body(scene: &str, session: &Session, sdf: &SdfGrid) {
    let (lo, hi) = session.collider().extent();
    let release = session.collider().release_height();
    let floor = session
        .collider()
        .ground()
        .expect("a baked body stands on a plane");
    let (points, at) = at_rest(session);
    let (low, high) = span(&points);
    let (deep, all) = buried(sdf, &points);
    let on_skin = touching(sdf, &points);
    println!(
        "{scene}: parked at {at} substeps · cloth {low:.4}..{high:.4} · body {:.4}..{:.4} \
         · floor {floor:.4} · released {release:.3} · {on_skin} of {all} on the skin, {deep} buried",
        lo[1], hi[1]
    );
    assert!(
        high < release - LANDED,
        "{scene}: the garment never came down from {release}: {high}"
    );
    assert!(
        low >= floor,
        "{scene}: the garment went through the ground: {low} under {floor}"
    );
    assert_eq!(
        deep, 0,
        "{scene}: {deep} of {all} particles are buried past the band"
    );
    // The miss the other three cannot see. Cloth resting on the floor a long
    // way from the person satisfies every one of them, and counting what is
    // buried says least of all out there, since nothing far from a body is
    // ever inside one. These two scenes settle a third of a metre and half a
    // metre inside the footprint; the same panel let go two metres to the side
    // lands well over a metre outside it.
    let (cx, cz) = footing(&points);
    assert!(
        cx >= lo[0] && cx <= hi[0] && cz >= lo[2] && cz <= hi[2],
        "{scene}: the garment came to rest beside the body instead of on it: \
         its weight lies at x {cx} z {cz}, outside x {}..{} z {}..{}",
        lo[0],
        hi[0],
        lo[2],
        hi[2]
    );
}

/// Opening a document lets its garment go over the body the document names,
/// and none of it ends up buried under that body's skin.
///
/// The failure this pins is not a near miss. A session seeded at some other
/// body's height starts a whole panel inside a person: let go at the sphere's
/// height over an adult body, 1,299 of the trouser front's 10,513 particles
/// begin out past the band, where the field is saturated flat and its
/// gradient is exactly zero. The contact solve has no normal to push along,
/// so they never come out.
#[test]
#[ignore = "release-only: a real body baked and a whole drape run"]
fn opening_a_document_does_not_bury_the_garment_in_the_body() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    // The field a second time: a `Collider` answers whether a point is under
    // the skin, and what is measured here is how far under, which only the
    // grid itself carries.
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    let session = Session::from_doc(block::trouser_front(), body).expect("the block drapes");
    rests_clear_of_the_body("opened document", &session, &sdf);
}

/// The scene the app starts on, measured the same way, which is what said the
/// numbers above belonged to opening a document rather than to the body.
#[test]
#[ignore = "release-only: a real body baked and a whole drape run"]
fn the_startup_scene_does_not_bury_the_garment_in_the_body() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    let session = Session::demo_bodice_over(body);
    rests_clear_of_the_body("startup scene", &session, &sdf);
}

/// A drape over a body now parks — on the ground, and not on the body.
///
/// This is what the floor bought and what it did not. Before it, a panel let
/// go over an adult came down on the crown, hung while friction lost to
/// gravity, slid off and then fell for ever, so nothing over a body ever went
/// quiet. Now the same panel lands on the plane the body stands on and stops
/// there: the startup bodice parks at about 39,300 substeps with every one of
/// its particles within a couple of centimetres of the floor.
///
/// What it did not buy is a garment that stays on. Nothing holds a bodice at
/// the shoulders of a body it was dropped over, so at rest it is a heap around
/// the feet — 7 of its 12,540 particles within a cell of the skin. An elastic
/// exists now and places a garment where it belongs, and it does not change
/// this reading: measured in `worn`, a waistband grips and then creeps down at
/// a rate its own tension cannot alter. Whoever gives friction that tension to
/// read should tighten the last assertion here, because it is the one that
/// says the cloth is on the floor.
#[test]
#[ignore = "release-only: a real body baked and a whole drape run"]
fn a_drape_over_a_body_parks_on_the_ground_and_not_yet_on_the_body() {
    // The sphere half is the contrast, and it says sleep is not handed out to
    // whatever is slow. A bodice balanced on a ball with no floor under it is
    // never at rest: it slides sideways at a few millimetres a second and
    // gathering, and leaves the ball at about 23,000 substeps to fall for
    // ever. A mean of the energy called that slide parked at 2,280.
    assert!(
        !parks_by(&Session::demo_bodice(), LONG),
        "the demo bodice went to sleep on the sphere inside {LONG} substeps. If \
         something now holds it there, this is the assertion to invert"
    );

    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let floor = body.ground().expect("a baked body stands on a plane");
    let session = Session::demo_bodice_over(body);
    let (points, at) = at_rest(&session);
    let (low, high) = span(&points);
    println!("over a body: parked at {at} substeps · cloth {low:.4}..{high:.4} · floor {floor:.4}");
    assert!(
        low >= floor,
        "it came to rest on the ground, not through it: {low} under {floor}"
    );
    assert!(
        high - floor < ON_THE_GROUND,
        "the whole garment is lying on the floor rather than worn on the body: \
         its highest point {high} stands {} above the ground. If a garment now \
         stays on, this is the assertion to tighten and the two tests above \
         should measure what is worn rather than what came to rest",
        high - floor
    );
}
