#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(
    clippy::float_cmp,
    reason = "a release height derived from one literal lands on it exactly"
)]

use std::time::{Duration, Instant};

use toile_engine::body::{CLEARANCE, Collider};
use toile_engine::draft::BodyMesh;
use toile_engine::session::Session;
use toile_engine::{couture, demo};

/// How long a test waits on another thread before calling it stuck.
const PATIENCE: Duration = Duration::from_secs(20);

/// Half the side of the cube that stands in for a body here.
const HALF: f32 = 0.1;

/// A closed, outward-wound cube: a body small enough to bake in a debug
/// build, which a real one is not.
fn cube() -> BodyMesh {
    let mut positions = Vec::new();
    for k in [-HALF, HALF] {
        for j in [-HALF, HALF] {
            for i in [-HALF, HALF] {
                positions.extend_from_slice(&[i, j, k]);
            }
        }
    }
    // Corner `v` carries bit 0 for +x, bit 1 for +y, bit 2 for +z, so each
    // face below is named by its four corners in counter-clockwise order seen
    // from outside the cube.
    let faces: [[u32; 4]; 6] = [
        [0, 2, 3, 1], // -z
        [4, 5, 7, 6], // +z
        [0, 1, 5, 4], // -y
        [2, 6, 7, 3], // +y
        [0, 4, 6, 2], // -x
        [1, 3, 7, 5], // +x
    ];
    let mut indices = Vec::new();
    for [a, b, c, d] in faces {
        indices.extend_from_slice(&[a, b, c, a, c, d]);
    }
    BodyMesh {
        normals: vec![0.0; positions.len()],
        stations: vec![0; positions.len() / 3],
        positions,
        indices,
    }
}

/// Waits until the sim thread has run past `mark` substeps.
fn substeps_past(session: &Session, mark: u64) -> u64 {
    let deadline = Instant::now() + PATIENCE;
    loop {
        let now = session.snapshot().substeps;
        if now > mark || Instant::now() > deadline {
            return now;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Waits until the sim thread has published a frame past generation `mark`.
///
/// Substeps are no answer to whether a message landed: the thread drains its
/// mailbox between ticks, so a frame published in the moment between the
/// message being sent and being drained carries a later substep count and the
/// generation from before it.
fn generation_past(session: &Session, mark: u64) -> u64 {
    let deadline = Instant::now() + PATIENCE;
    loop {
        let now = session.snapshot().generation;
        if now > mark || Instant::now() > deadline {
            return now;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// The demo scene is the physics reference and still falls on the sphere: the
/// body it carries is the ball, and the height it is let go from is the very
/// constant the drape golden was taken at.
#[test]
fn the_demo_scene_still_falls_on_the_sphere() {
    let session = Session::demo_bodice();
    let (lo, hi) = session.collider().extent();
    assert_eq!(hi[1], demo::AVATAR_RADIUS);
    assert_eq!(lo[1], -demo::AVATAR_RADIUS);
    assert_eq!(session.collider().release_height(), couture::DROP_HEIGHT);
    assert_eq!(session.collider().dims(), [256, 256, 256]);
}

/// A garment is let go over the body it will fall on, not at a constant: the
/// rule is the body's own top plus the gap the demo scene has over its ball.
#[test]
fn the_release_height_is_the_bodys_own() {
    let body = Collider::bake(&cube()).expect("a cube is closed and orientable");
    let (_, hi) = body.extent();
    assert_eq!(hi[1], HALF);
    assert_eq!(body.release_height(), HALF + CLEARANCE);
    assert!(
        body.release_height() != couture::DROP_HEIGHT,
        "a body decides its own, rather than inheriting the sphere's"
    );
    // The same rule read against the sphere gives the sphere's own literal to
    // the centimetre, which is what makes it one rule and not two.
    let sphere = Collider::demo();
    let (_, top) = sphere.extent();
    assert!((sphere.release_height() - (top[1] + CLEARANCE)).abs() < 1.0e-6);
}

/// A body swapped under a running drape leaves the sim running: the thread
/// takes the message, goes on integrating, and publishes frames against the
/// new body rather than stopping or refusing it.
#[test]
fn a_body_swapped_mid_drape_leaves_the_sim_running() {
    let mut session = Session::demo_bodice();
    let before = substeps_past(&session, 0);
    assert!(before > 0, "the sim thread is running");
    let generation = session.snapshot().generation;

    session.set_collider(Collider::bake(&cube()).expect("the cube bakes"));
    assert!(session.simulating(), "the thread is still there");

    let woke = generation_past(&session, generation);
    assert!(
        woke > generation,
        "the body reached the solver: {woke} against {generation}"
    );
    // Only now is the drape known to be running against the new body, so this
    // is the point from which it has to go on advancing.
    let at = session.snapshot().substeps;
    let after = substeps_past(&session, at);
    assert!(
        after > at,
        "and the solver kept integrating past it: {after} against {at}"
    );
    assert!(after > before, "which is past where it was: {after}");

    let snap = session.snapshot();
    assert_eq!(snap.refused, None, "the body was taken, not refused");
    assert!(
        snap.positions.iter().all(|f| f.is_finite()),
        "no particle was flung anywhere unrepresentable"
    );
}

/// A table with nothing draping has no thread to tell, and takes the body all
/// the same: the next piece drawn on it is let go over that body.
#[test]
fn a_blank_table_takes_a_body_for_the_piece_not_drawn_yet() {
    let mut session = Session::blank(Collider::demo());
    assert!(!session.simulating());
    let body = Collider::bake(&cube()).expect("the cube bakes");
    let release = body.release_height();
    session.set_collider(body);
    assert_eq!(session.collider().release_height(), release);
}
