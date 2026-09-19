use super::*;

/// The thickness is the mesh's own, and a mesh with no edges has none.
#[test]
fn the_thickness_comes_off_the_mesh() {
    let (state, tris) = sheet(8, 8, [0.0, 0.0, 0.0]);
    let cons = stretch(&state, &tris);
    let layers = Layers::of(&tris, &cons, &Seams::default(), state.len());
    assert_eq!(layers.thickness(), mean_rest(&cons) * THICKNESS_OF_EDGE);
    assert!(layers.thickness() > 0.0 && layers.thickness() < SPACING);

    let bare = Layers::of(
        &tris,
        &DistanceConstraints::default(),
        &Seams::default(),
        64,
    );
    assert_eq!(bare.thickness(), 0.0, "nothing to read an edge off");
}
