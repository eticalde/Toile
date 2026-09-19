use std::time::{Duration, Instant};

use toile_anny::BodyMesh;
use toile_sim::xpbd::SdfGrid;

use super::{Job, Made, Order, Oven};
use crate::body::bake::tests::crossed_cubes;
use crate::body::bake::{self, Crossings};
use crate::body::{NO_LEVERS, Phenotype, body_mesh};

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
            order: Order::Bake,
        });
    }
    let first = oven.take().expect("the first body went straight in");
    assert_eq!(first.key, 0, "the body that found the oven free");
    assert!(
        matches!(first.made, Made::Field(Err(_))),
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
            order: Order::Bake,
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

/// Sends one body through the oven and waits for what comes of it.
fn through(mesh: &BodyMesh, order: Order) -> Made {
    let mut oven = Oven::spawn();
    oven.send(Job {
        key: 1,
        mesh: mesh.clone(),
        order,
    });
    oven.take().expect("the oven was free").made
}

/// Whether two fields are one field: the same lattice, and every voxel the
/// same bits.
fn same_bits(a: &SdfGrid, b: &SdfGrid) -> bool {
    let bits = |g: &SdfGrid| -> Vec<u32> {
        let placed = g.origin.iter().chain([&g.cell]);
        placed.chain(&g.data).map(|f| f.to_bits()).collect()
    };
    a.dims == b.dims && bits(a) == bits(b)
}

/// Bakes a body through the oven, has the oven look it over, and holds the
/// field to the one a bake that never heard of crossings gives: to the bit.
///
/// The second half is true by construction, and this is here to keep it so.
/// A crossing is read off the mesh under an order of its own, and the order
/// that bakes never sees one.
fn baked_and_inspected(mesh: &BodyMesh) -> Crossings {
    let plain = bake::sdf(mesh).expect("the fixture earns the sign");
    let Made::Field(field) = through(mesh, Order::Bake) else {
        panic!("a bake was ordered");
    };
    let body = field.expect("a body that crosses itself is never refused");
    assert!(same_bits(body.field(), &plain), "the field moved");
    let Made::Crossings(found) = through(mesh, Order::Inspect) else {
        panic!("a looking-over was ordered");
    };
    found
}

/// A body that passes through itself is baked like any other, and the oven
/// says where it does. Refusing it would refuse the body the app opens with.
#[test]
fn a_body_that_crosses_itself_still_bakes_and_its_field_is_untouched() {
    let found = baked_and_inspected(&crossed_cubes());
    assert!(!found.all().is_empty(), "the cubes do cross");
}

/// The same, on the body that matters: the default phenotype bakes with its
/// 52 sole crossings, into the field it always had.
#[test]
#[ignore = "release-only: this bakes millions of voxels, twice"]
fn the_default_body_bakes_with_its_crossings_and_the_field_it_always_had() {
    let mesh = body_mesh(&Phenotype::default(), &NO_LEVERS);
    let found = baked_and_inspected(&mesh);
    assert_eq!(found.all().len(), 52);
    assert_eq!(found.exposed().count(), 0);
}
