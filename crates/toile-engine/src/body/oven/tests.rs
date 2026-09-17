use std::time::{Duration, Instant};

use toile_anny::BodyMesh;

use super::{Job, Oven};

/// How long a test waits on the bake thread before calling it stuck.
const PATIENCE: Duration = Duration::from_secs(20);

/// A body the bake refuses, so that what these tests measure is which bodies
/// go in rather than what comes out of them: every job still answers, under
/// its own key, in microseconds instead of half a second.
fn nothing() -> BodyMesh {
    BodyMesh {
        positions: Vec::new(),
        normals: Vec::new(),
        stations: Vec::new(),
        indices: Vec::new(),
    }
}

/// A body displaced while it waited for the oven is never baked.
///
/// This is the shape of a drag: a value a frame, each one a body, and every
/// answer but the last thrown away. Baking them all cost half a second each
/// before anyone could throw them away, and the queue was served in order, so
/// the body the hand had reached came out last of all.
#[test]
fn a_body_displaced_before_it_goes_in_is_never_baked() {
    let mut oven = Oven::spawn();
    for key in 0..3 {
        oven.send(Job {
            key,
            mesh: nothing(),
        });
    }
    let first = oven.take().expect("the first body went straight in");
    assert_eq!(first.key, 0, "the body that found the oven free");
    assert!(
        first.field.is_err(),
        "the fixture is a body the bake refuses"
    );
    let last = oven.take().expect("the body that waited went in after it");
    assert_eq!(
        last.key, 2,
        "body 1 was displaced while it waited, and baking it would have cost \
         half a second for a field nobody would ever have read"
    );
    assert!(
        oven.take().is_none(),
        "two bakes for the three bodies asked for"
    );
    assert!(!oven.busy());
}

/// The body that waited goes in when the answer before it is collected, which
/// is the once-a-frame collection the fitting makes.
#[test]
fn the_body_that_waited_goes_in_when_the_answer_before_it_is_taken() {
    let mut oven = Oven::spawn();
    for key in [7, 9] {
        oven.send(Job {
            key,
            mesh: nothing(),
        });
    }
    let mut seen = Vec::new();
    let deadline = Instant::now() + PATIENCE;
    while seen.len() < 2 && Instant::now() < deadline {
        if let Some(done) = oven.try_take() {
            seen.push(done.key);
        }
    }
    assert_eq!(seen, [7, 9], "in the order they were asked for");
    assert!(!oven.busy(), "and the oven is empty");
}
