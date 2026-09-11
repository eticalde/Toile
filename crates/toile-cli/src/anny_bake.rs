mod groups;
mod obj;
mod quad;
/// Bakes the seventeen anatomical rings `toile_anny::measure` reads a
/// generated body against: cut once here, against the neutral template,
/// then walked (never re-cut) at runtime as the phenotype morphs the mesh.
///
/// Several placements depart from where their name would literally point,
/// because this mesh's topology does not allow it, or because the literal
/// joint turns out not to be the anatomically honest choice — logged here
/// rather than in a scattered set of comments.
///
/// `pecho_alto` cannot sit at the scapula/clavicle joint height the
/// catalogue names for "armpit level": by there, the mesh has already
/// fused the arm's surface into the torso's (no gap remains to cut
/// through), so a horizontal cut traces up over the shoulder and down the
/// arm instead of around the chest — `rings::bands::highest_unfused_trunk_y`
/// finds the highest cut that still separates them, the honest reading of
/// "as high as a tape can go."
///
/// `tobillo` is not literally at the ankle joint either: that joint sits
/// at the boundary into the foot, where the cross-section is already an
/// elongated foot shape rather than a round ankle, so `rings::legs::ANKLE_T`
/// backs off slightly toward the knee, to the narrowest point actually on
/// the leg. `muneca` similarly backs off from the hand joint
/// (`rings::arms::WRIST_T`): the forearm tapers smoothly all the way into
/// the hand with no distinct wrist-bone pinch on this mesh, so a cut right
/// at the joint barely responds to the build and gender morphs at all.
///
/// `crotch` is not the pelvis joint: that joint sits about 10 cm above
/// where the legs actually separate. `rings::bands::fork_y` finds the
/// fork itself — the lowest horizontal cut that still encloses both legs
/// as one loop — and `cadera`'s band is constrained to start strictly
/// above it, so its "fullest section" cannot be both thighs still pressed
/// together rather than the seat.
///
/// For the same fusion reason as `pecho_alto`, the two `Shoulder*` girth
/// landmark rings (used only for `hombros`) reuse
/// `rings::bands::limb_fullest`'s own discovered offset rather than sitting
/// exactly at the shoulder joint. `brazo`'s own starting point does not
/// reuse that offset, though: a length landmark can sit anywhere on the
/// surface nearest the joint, so `rings::arms::bake` also stores the single
/// body vertex nearest the true shoulder joint (`RingId::ShoulderJoint`) —
/// not a cut, since no plane there can separate arm from torso, but a
/// legitimate single-point landmark that follows the mesh the same way a
/// ring's points do.
mod rings;
mod station;
mod targets;

use targets::ParsedTarget;
use toile_anny::asset::{self, Baked, Delta, Row};

/// Where the asset lands, relative to this crate's own manifest: the baker
/// always writes to the tree it ships from, never wherever `cargo run`
/// happened to be invoked from.
const OUT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../toile-anny/assets/body.bin");

/// Runs `toile anny-bake RUTA/a/mpfb2`: turns the CC0 MakeHuman/MPFB2 source
/// data into `toile-anny`'s embedded asset.
///
/// `RUTA` is the path to an `mpfb2` directory (it must contain `3dobjs/` and
/// `mesh_metadata/`); the naver/anny repository's
/// `src/anny/data/mpfb2` is exactly that directory, but this bake never
/// downloads or hardcodes a path to it — the caller always says where their
/// checkout lives.
pub fn run(args: &[String]) {
    let Some(root) = args.first() else {
        eprintln!("uso: toile anny-bake RUTA/a/mpfb2");
        return;
    };
    let obj_path = format!("{root}/3dobjs/base.obj");
    let groups_path = format!("{root}/mesh_metadata/basemesh_vertex_groups.json");
    let obj_text = match std::fs::read_to_string(&obj_path) {
        Ok(text) => text,
        Err(why) => return eprintln!("no se pudo leer «{obj_path}»: {why}"),
    };
    let groups_text = match std::fs::read_to_string(&groups_path) {
        Ok(text) => text,
        Err(why) => return eprintln!("no se pudo leer «{groups_path}»: {why}"),
    };

    let parsed_targets = targets::discover_and_read(root);
    let baked = bake(&obj_text, &groups_text, parsed_targets);
    let bytes = asset::encode(&baked);
    match std::fs::write(OUT, &bytes) {
        Ok(()) => println!(
            "{OUT}: {} bytes, {} vértices, {} triángulos, {} filas, {} deltas",
            bytes.len(),
            baked.positions.len() / 3,
            baked.indices.len() / 3,
            baked.rows.len(),
            baked.deltas.len()
        ),
        Err(why) => eprintln!("no se pudo escribir «{OUT}»: {why}"),
    }
}

/// The bake itself, kept apart from file I/O so a test can drive it on a
/// crafted fixture without touching the disk. `parsed_targets` is already
/// read, quantized and in its fixed bake order (see
/// `targets::discover_and_read`); building the row table here is then just
/// concatenating each target's deltas and recording where they landed.
fn bake(obj_text: &str, groups_text: &str, parsed_targets: Vec<ParsedTarget>) -> Baked {
    let groups = groups::parse(groups_text);
    let body_range = groups::single_range(&groups, "body");
    assert_eq!(
        body_range,
        (0, 13_379),
        "the body group is expected to be the mesh's first 13,380 vertices"
    );

    let obj = obj::parse(obj_text, "body");
    assert_eq!(
        obj.group_quads.len(),
        13_378,
        "the body group is expected to be all quads"
    );

    let body_verts = &obj.vertices[body_range.0..=body_range.1];
    let centre = bbox_centre(body_verts);
    let to_body_space = |v: [f64; 3]| {
        [
            (v[0] - centre[0]) * 0.1,
            (v[1] - centre[1]) * 0.1,
            (v[2] - centre[2]) * 0.1,
        ]
    };
    let positions: Vec<f32> = body_verts
        .iter()
        .flat_map(|&v| to_body_space(v).map(|c| c as f32))
        .collect();

    let indices = quad::triangulate(&obj.group_quads, &obj.vertices);

    let joints = groups::joint_centroids(&groups, &obj.vertices);
    assert_eq!(joints.len(), 125, "expected the 125 joint-cube groups");
    let stations: Vec<u8> = body_verts
        .iter()
        .map(|&v| station::classify(v, &joints).tag())
        .collect();

    // The rings are cut against the exact f32 bytes the asset ships (widened
    // back to f64 only for the intersection math), not the raw f64 OBJ
    // coordinates: that is what `toile_anny::measure` will actually walk at
    // runtime, and the two must agree bit for bit on which side of a plane
    // a vertex sits.
    let ring_positions: Vec<[f64; 3]> = positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| c.map(f64::from))
        .collect();
    let ring_tris: Vec<[u32; 3]> = indices.as_chunks::<3>().0.to_vec();
    let ring_joints: Vec<(String, [f64; 3])> = joints
        .iter()
        .map(|(name, v)| (name.clone(), to_body_space(*v)))
        .collect();
    let (ring_ranges, ring_points) = rings::bake(&ring_positions, &ring_tris, &ring_joints);

    let mut rows = Vec::with_capacity(parsed_targets.len());
    let mut deltas: Vec<Delta> = Vec::new();
    for target in parsed_targets {
        rows.push(Row {
            kind: target.kind,
            offset: deltas.len() as u32,
            length: target.deltas.len() as u32,
        });
        deltas.extend(target.deltas);
    }

    Baked {
        positions,
        indices,
        stations,
        rows,
        deltas,
        ring_ranges,
        ring_points,
    }
}

/// The bounding-box centre of a set of points, per axis.
fn bbox_centre(points: &[[f64; 3]]) -> [f64; 3] {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in points {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    [
        f64::midpoint(lo[0], hi[0]),
        f64::midpoint(lo[1], hi[1]),
        f64::midpoint(lo[2], hi[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one true bake, run against the checked-out study clone rather
    /// than a fixture, gated behind an environment variable so the ordinary
    /// suite (and CI, which has no such clone) never depends on it. Point
    /// `ANNY_MPFB2` at a `.../data/mpfb2` directory and run with
    /// `--ignored` to exercise the real pipeline end to end.
    #[test]
    #[ignore = "needs a local checkout of naver/anny's data/mpfb2, via ANNY_MPFB2"]
    fn bakes_the_real_source_data_without_panicking() {
        let Ok(root) = std::env::var("ANNY_MPFB2") else {
            return;
        };
        let obj_text = std::fs::read_to_string(format!("{root}/3dobjs/base.obj")).unwrap();
        let groups_text =
            std::fs::read_to_string(format!("{root}/mesh_metadata/basemesh_vertex_groups.json"))
                .unwrap();
        let parsed_targets = targets::discover_and_read(&root);
        assert_eq!(
            parsed_targets.len(),
            376,
            "48 + 72 + 72 + 144 weighted rows, plus 40 levers"
        );
        let lever_rows = parsed_targets
            .iter()
            .filter(|t| matches!(t.kind, toile_anny::asset::RowKind::Lever { .. }))
            .count();
        assert_eq!(lever_rows, 40);
        let total_deltas: usize = parsed_targets.iter().map(|t| t.deltas.len()).sum();
        assert_eq!(total_deltas, 2_124_560);

        let baked = bake(&obj_text, &groups_text, parsed_targets);
        assert_eq!(baked.positions.len(), 13_380 * 3);
        assert_eq!(baked.indices.len(), 26_756 * 3);
        assert_eq!(baked.stations.len(), 13_380);
        assert_eq!(baked.rows.len(), 376);
        assert_eq!(baked.deltas.len(), 2_124_560);
        assert_eq!(baked.ring_ranges.len(), toile_anny::asset::RingId::COUNT);
        assert!(
            baked.ring_ranges.iter().all(|r| r.length > 0),
            "every ring must have at least one point"
        );
        assert_eq!(
            baked.ring_points.len() as u32,
            baked.ring_ranges.iter().map(|r| r.length).sum::<u32>(),
            "the flat point array must hold exactly the ranges' own lengths"
        );
    }
}
