#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// The analytic sphere the demo scene collides against must keep baking the
/// same 16.7 M voxels, and keep being cubic.
///
/// The eternal drape golden stands on this field, so this is the narrower
/// statement underneath it: not "the cloth lands where it did" but "the field
/// it lands on is the same field". A grid refactor that moved a bit here
/// would have to move the drape too, and this says which of the two broke.
#[test]
#[ignore = "release-only golden: cargo test --release -- --ignored"]
fn the_analytic_sphere_field_hashes_to_a_fixed_value() {
    let sdf = toile_engine::demo::avatar_sdf();
    assert_eq!(sdf.dims, [256, 256, 256], "the demo sphere is a cubic grid");
    assert_eq!(
        toile_engine::golden::sphere_sdf_hash(),
        0xd28b_fb0f_0ebb_32fa,
        "the analytic sphere bake changed bits: the sphere constructor, the \
         demo's grid extent or the arithmetic under them moved. This field is \
         the project's collision fixture and does not move; a bake path that \
         needed changing belongs on a new path with its own golden"
    );
}
