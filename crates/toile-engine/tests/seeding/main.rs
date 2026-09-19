#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// What a waistband is worth on the body, and what it is still not worth.
mod grip;
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
use watch::{LANDED, REST, at_rest, buried, footing, parks_by, reference, span, touching};

/// How far above the ground the whole garment stands once it has parked, in
/// metres, for a scene that came to rest on the body rather than on the floor.
const ON_THE_BODY: f32 = 1.00;

/// What a seeded scene is held to: the garment came down, it came to a stop,
/// none of it went through the ground, and none of it is buried where the
/// field could never push it out again.
///
/// Measured at rest, which is a thing that exists only since the floor landed.
/// Before it, a garment let go over a body fell for as long as anything
/// integrated it, so these scenes had to be read at a fixed substep instead.
///
/// Burial is the sharp one: the bug these tests were written for, where a
/// garment let go at some other body's height starts inside this one and no
/// contact solve can carry it out.
///
/// Where the garment's weight came to rest is what keeps the other three from
/// being satisfied by one that never met the body at all. Both scenes now rest
/// on the body rather than round its feet: 3,235 of the startup bodice's
/// 12,540 particles sit within a cell of the skin, where under the fixed share
/// of the motion it was 16.
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

/// A drape over a body now parks on the body, and not on the ground.
///
/// This is what a contact that reads the push bought. Under the fixed share of
/// the substep's motion a panel let go over an adult came down on the crown,
/// hung while friction lost to gravity, slid off and heaped on the plane the
/// body stands on — at rest, 16 of the bodice's 12,540 particles were still
/// within a cell of the skin. The same bodice now comes down onto the
/// shoulders and stays: it parks at about 1,070 substeps with its cloth
/// between 0.53 m and 1.06 m, the lowest of it 1.36 m clear of the floor.
///
/// The sphere half is not scenery: it is the physics reference, it has no
/// floor, it asks for no grip at all, and it still parks on its own — which is
/// what says the budget below measures a drape that settles rather than one
/// nothing watches.
#[test]
#[ignore = "release-only: a real body baked and a whole drape run"]
fn a_drape_over_a_body_parks_on_the_body_and_not_on_the_ground() {
    // The sphere half is not scenery: it is the physics reference, it has no
    // floor, and it still parks on its own — which is what says the budget
    // below measures a drape that settles rather than one nothing watches.
    assert!(
        parks_by(&Session::demo_bodice(), REST),
        "the demo bodice stopped parking on the sphere inside {REST} substeps"
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
        high - floor > ON_THE_BODY,
        "the garment is worn on the body rather than lying on the floor: its \
         highest point {high} stands {} above the ground",
        high - floor
    );
}
