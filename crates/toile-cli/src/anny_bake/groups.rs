use std::collections::BTreeMap;

/// A vertex-groups file: group name to inclusive 0-based `[lo, hi]` index
/// ranges. `BTreeMap` rather than the JSON's own object order — a group's
/// name, not its position in the file, is what the bake keys off — and it is
/// what keeps the nearest-joint search in `station` free of `HashMap`
/// iteration order.
pub type Groups = BTreeMap<String, Vec<[usize; 2]>>;

/// Parses `mesh_metadata/basemesh_vertex_groups.json`.
///
/// # Panics
/// If the text is not that shape: a JSON object of name to arrays of
/// two-number arrays. MPFB2 has shipped this one shape since the CC0 release
/// this bake reads from.
pub fn parse(text: &str) -> Groups {
    serde_json::from_str(text).expect("basemesh_vertex_groups.json: name -> [[lo, hi], ...]")
}

/// The single inclusive range a name's ranges must be.
///
/// # Panics
/// If the name is missing or spans more than one range: only used for
/// `body`, which the source data promises is one contiguous block.
pub fn single_range(groups: &Groups, name: &str) -> (usize, usize) {
    let ranges = groups
        .get(name)
        .unwrap_or_else(|| panic!("no vertex group named `{name}`"));
    let [[lo, hi]] = ranges[..] else {
        panic!("`{name}` is not a single contiguous range: {ranges:?}")
    };
    (lo, hi)
}

/// The f64 centroid of a group's vertices, indexed into `vertices`.
fn centroid(ranges: &[[usize; 2]], vertices: &[[f64; 3]]) -> [f64; 3] {
    let mut sum = [0.0f64; 3];
    let mut n = 0usize;
    for &[lo, hi] in ranges {
        for v in &vertices[lo..=hi] {
            sum[0] += v[0];
            sum[1] += v[1];
            sum[2] += v[2];
            n += 1;
        }
    }
    let n = n as f64;
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

/// Every `joint-*` group's name and centroid, sorted by name (the map's own
/// order): the fixed order the station search in `station.rs` walks, so a
/// tie always resolves to the alphabetically first joint rather than to
/// whatever order a hash table happened to hold.
pub fn joint_centroids(groups: &Groups, vertices: &[[f64; 3]]) -> Vec<(String, [f64; 3])> {
    groups
        .iter()
        .filter(|(name, _)| name.starts_with("joint-"))
        .map(|(name, ranges)| (name.clone(), centroid(ranges, vertices)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "body": [[0, 2]],
        "joint-b": [[3, 3]],
        "joint-a": [[4, 5]]
    }"#;

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "exact integer-valued literals over a hand-built fixture"
    )]
    fn reads_ranges_and_sorts_joints_by_name() {
        let groups = parse(SAMPLE);
        assert_eq!(single_range(&groups, "body"), (0, 2));
        let verts = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [20.0, 0.0, 0.0],
            [22.0, 0.0, 0.0],
        ];
        let joints = joint_centroids(&groups, &verts);
        assert_eq!(joints[0].0, "joint-a");
        assert_eq!(joints[0].1, [21.0, 0.0, 0.0]);
        assert_eq!(joints[1].0, "joint-b");
        assert_eq!(joints[1].1, [10.0, 0.0, 0.0]);
    }
}
