use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use toile_anny::asset::{Delta, RowKind};
use toile_anny::phenotype::{lever_from_stem, mask_from_stem};

/// Only body vertices (the group the bake keeps; see the `body` range
/// check in the parent module) carry a delta the mesh ever applies —
/// anything at or past this index names helper geometry the asset never
/// ships, and its delta line is simply discarded.
const BODY_VERTEX_COUNT: usize = 13_380;

/// One target file's parsed contribution: what row it becomes, and its
/// quantized, range-checked deltas.
pub struct ParsedTarget {
    /// Which row this becomes: a weighted macrodetail/breast target, or one
    /// half of a `measure-*` lever pair.
    pub kind: RowKind,
    /// This file's deltas on body vertices, in the file's own line order.
    pub deltas: Vec<Delta>,
}

/// Which of the two shapes a wanted target file has — the rest of
/// `targets/` (hands, head, expression, the non-adult macrodetails, ...)
/// is not read at all.
enum Category {
    /// A macrodetails or breast target whose weight is a product of
    /// phenotype coefficients.
    Weighted,
    /// A `measure-*` lever half.
    Lever,
}

/// Whether `relative` (a target's path under `.../mpfb2/targets/`, with
/// forward slashes) is one this adult tier reads: all of `macrodetails/`
/// (including its `height/` and `proportions/` subdirectories) and
/// `breast/` restricted to filenames containing `-young` or `-old`, plus
/// every `measure-*` lever wherever it lives.
fn categorize(relative: &str) -> Option<Category> {
    let file_name = relative.rsplit('/').next().unwrap_or(relative);
    let stem = file_name.strip_suffix(".target.gz")?;
    if stem.starts_with("measure-") {
        return Some(Category::Lever);
    }
    let is_adult = stem.contains("-young") || stem.contains("-old");
    if !is_adult {
        return None;
    }
    if relative.starts_with("macrodetails/") {
        return Some(Category::Weighted);
    }
    if relative.starts_with("breast/") && (stem.starts_with("female") || stem.starts_with("male")) {
        return Some(Category::Weighted);
    }
    None
}

/// Recursively lists every file under `dir`, sorting each directory's own
/// entries before descending. A plain `read_dir` order is unspecified and
/// OS-dependent; the bake's row order — and therefore its golden hash —
/// must depend only on the source data, never on the filesystem it happens
/// to run on, so [`discover_and_read`] also re-sorts the final list by
/// full relative path rather than trusting this alone.
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("no se pudo leer «{}»: {e}", dir.display()))
        .map(|entry| entry.expect("directory entry is readable").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Discovers, reads and quantizes every target this bake wants, from
/// `root/targets`, in the fixed order of their relative path strings.
pub fn discover_and_read(root: &str) -> Vec<ParsedTarget> {
    let targets_dir = Path::new(root).join("targets");
    let mut paths = Vec::new();
    walk(&targets_dir, &mut paths);

    let mut wanted: Vec<(String, PathBuf, Category)> = paths
        .into_iter()
        .filter_map(|path| {
            let relative = path
                .strip_prefix(&targets_dir)
                .expect("walk only yields paths under targets_dir")
                .to_string_lossy()
                .replace('\\', "/");
            let category = categorize(&relative)?;
            Some((relative, path, category))
        })
        .collect();
    wanted.sort_by(|a, b| a.0.cmp(&b.0));

    wanted
        .into_iter()
        .map(|(relative, path, category)| {
            let file_name = relative.rsplit('/').next().unwrap_or(&relative);
            let stem = file_name
                .strip_suffix(".target.gz")
                .expect("categorize only keeps .target.gz files");
            let kind = match category {
                Category::Weighted => RowKind::Weighted {
                    mask: mask_from_stem(stem),
                },
                Category::Lever => {
                    let (lever, incr) = lever_from_stem(stem);
                    RowKind::Lever { lever, incr }
                }
            };
            let deltas = read_target(&path, &relative);
            ParsedTarget { kind, deltas }
        })
        .collect()
}

/// Reads and decompresses one `.target.gz`, keeping only its body-vertex
/// lines as quantized [`Delta`]s. `label` is the file's relative path, used
/// only to name it in a panic message.
fn read_target(path: &Path, label: &str) -> Vec<Delta> {
    let bytes =
        fs::read(path).unwrap_or_else(|e| panic!("no se pudo leer «{}»: {e}", path.display()));
    let mut gz = GzDecoder::new(&bytes[..]);
    let mut text = String::new();
    gz.read_to_string(&mut text)
        .unwrap_or_else(|e| panic!("«{label}» no es gzip válido: {e}"));

    let mut deltas = Vec::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let Some(idx_tok) = words.next() else {
            continue;
        };
        let idx: usize = idx_tok
            .parse()
            .unwrap_or_else(|_| panic!("«{label}»: índice no numérico: {idx_tok}"));
        if idx >= BODY_VERTEX_COUNT {
            continue;
        }
        let comps: Vec<f64> = words.map(|w| parse_f64(label, w)).collect();
        let [dx, dy, dz] = comps[..] else {
            panic!(
                "«{label}» vértice {idx}: se esperaban 3 componentes, hubo {}",
                comps.len()
            )
        };
        deltas.push(Delta {
            vertex: idx as u16,
            dx: quantize(label, idx, "x", dx),
            dy: quantize(label, idx, "y", dy),
            dz: quantize(label, idx, "z", dz),
        });
    }
    deltas
}

fn parse_f64(label: &str, tok: &str) -> f64 {
    tok.parse()
        .unwrap_or_else(|_| panic!("«{label}»: not a number: {tok}"))
}

/// Quantizes one axis of a delta as thousandths of a decimetre, asserting
/// the losslessness `toile_anny::asset::Delta`'s doc promises: the source
/// text carries at most three decimal places, so `round(value * 1000)`
/// must be exact, and it must fit `i16`. A file that violates either is a
/// surprise in the source data, not a bug this bake should paper over, so
/// it fails the bake loudly, naming the file and the vertex.
fn quantize(label: &str, vertex: usize, axis: &str, value: f64) -> i16 {
    let scaled = value * 1000.0;
    let rounded = scaled.round();
    assert!(
        (scaled - rounded).abs() < 1.0e-6,
        "«{label}» vertex {vertex} axis {axis}: {value} is not an exact multiple of 0.001"
    );
    assert!(
        rounded.abs() <= f64::from(i16::MAX),
        "«{label}» vertex {vertex} axis {axis}: {value} does not fit i16 after quantizing"
    );
    rounded as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorizes_the_real_data_shapes_correctly() {
        assert!(matches!(
            categorize("macrodetails/african-female-young.target.gz"),
            Some(Category::Weighted)
        ));
        assert!(matches!(
            categorize(
                "macrodetails/height/female-young-averagemuscle-averageweight-maxheight.target.gz"
            ),
            Some(Category::Weighted)
        ));
        assert!(matches!(
            categorize(
                "macrodetails/proportions/female-old-maxmuscle-minweight-idealproportions.target.gz"
            ),
            Some(Category::Weighted)
        ));
        assert!(matches!(
            categorize("breast/female-young-maxmuscle-maxweight-mincup-averagefirmness.target.gz"),
            Some(Category::Weighted)
        ));
        assert!(matches!(
            categorize("torso/measure-waist-circ-incr.target.gz"),
            Some(Category::Lever)
        ));
        assert!(matches!(
            categorize("legs/measure-thigh-circ-decr.target.gz"),
            Some(Category::Lever)
        ));
    }

    #[test]
    fn rejects_non_adult_and_unrelated_files() {
        assert!(categorize("macrodetails/african-female-baby.target.gz").is_none());
        assert!(categorize("macrodetails/african-female-child.target.gz").is_none());
        assert!(categorize("breast/breast-dist-decr.target.gz").is_none());
        assert!(categorize("hands/hand-scale-vert-incr.target.gz").is_none());
        assert!(categorize("macrodetails/height/README.md").is_none());
    }

    #[test]
    fn quantize_round_trips_a_typical_thousandth() {
        assert_eq!(quantize("t", 0, "x", 0.026), 26);
        assert_eq!(quantize("t", 0, "x", -0.166), -166);
        assert_eq!(quantize("t", 0, "x", 0.0), 0);
    }

    #[test]
    #[should_panic(expected = "is not an exact multiple of 0.001")]
    fn quantize_panics_on_a_lossy_value() {
        quantize("t", 0, "x", 0.026_1);
    }

    #[test]
    #[should_panic(expected = "does not fit i16")]
    fn quantize_panics_out_of_range() {
        quantize("t", 0, "x", 40.0);
    }
}
