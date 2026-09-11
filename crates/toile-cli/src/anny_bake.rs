mod groups;
mod obj;
mod quad;
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
    let positions: Vec<f32> = body_verts
        .iter()
        .flat_map(|v| {
            [
                ((v[0] - centre[0]) * 0.1) as f32,
                ((v[1] - centre[1]) * 0.1) as f32,
                ((v[2] - centre[2]) * 0.1) as f32,
            ]
        })
        .collect();

    let indices = quad::triangulate(&obj.group_quads, &obj.vertices);

    let joints = groups::joint_centroids(&groups, &obj.vertices);
    assert_eq!(joints.len(), 125, "expected the 125 joint-cube groups");
    let stations: Vec<u8> = body_verts
        .iter()
        .map(|&v| station::classify(v, &joints).tag())
        .collect();

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
    }
}
