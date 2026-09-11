use std::fs;
use std::io::Read;

use flate2::read::GzDecoder;

/// Only body vertices carry a delta this bake ever reads — see
/// `targets::BODY_VERTEX_COUNT`'s doc for why.
const BODY_VERTEX_COUNT: usize = 13_380;

/// The height (`y`, in `positions`' own space) the breast targets
/// themselves say the bust apex sits at, on the neutral template.
///
/// The neutral template carries no breast at all, so a "fullest section"
/// scan over it can only ever find the ribcage — the underlying problem
/// `RingId::Bust` had before this. But `breast-point-incr.target.gz`
/// displaces exactly the vertices that make up the breast, by an amount
/// that peaks at the nipple and falls off toward the chest wall; the
/// displacement-weighted mean height of those vertices is therefore a
/// principled estimate of the bust line, read directly off the CC0 data
/// rather than guessed. This is the only place this bake reads that file:
/// it locates a cutting height and is never baked as a row (breast size
/// itself still comes from the ordinary weighted breast rows).
///
/// # Panics
/// If `targets/breast/breast-point-incr.target.gz` under `root` is
/// missing or not valid gzip: that would mean a different (or incomplete)
/// checkout of the CC0 data was pointed at this bake.
pub(super) fn weighted_mean_height(root: &str, positions: &[[f64; 3]]) -> f64 {
    let path = format!("{root}/targets/breast/breast-point-incr.target.gz");
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("no se pudo leer «{path}»: {e}"));
    let mut gz = GzDecoder::new(&bytes[..]);
    let mut text = String::new();
    gz.read_to_string(&mut text)
        .unwrap_or_else(|e| panic!("«{path}» no es gzip válido: {e}"));
    weighted_mean_from_text(&text, positions)
}

/// The pure weighting math behind [`weighted_mean_height`], apart from the
/// file I/O and decompression so it can be tested on a literal fixture.
///
/// # Panics
/// If no line names a body vertex at all (every delta falls outside
/// [`BODY_VERTEX_COUNT`]): that would mean a target file this bake did not
/// expect was pointed at this function.
fn weighted_mean_from_text(text: &str, positions: &[[f64; 3]]) -> f64 {
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let Some(idx_tok) = words.next() else {
            continue;
        };
        let idx: usize = idx_tok
            .parse()
            .unwrap_or_else(|_| panic!("index is not numeric: {idx_tok}"));
        if idx >= BODY_VERTEX_COUNT {
            continue;
        }
        let comps: Vec<f64> = words
            .map(|w| w.parse().unwrap_or_else(|_| panic!("not a number: {w}")))
            .collect();
        let [dx, dy, dz] = comps[..] else {
            panic!("vertex {idx}: expected 3 components, got {}", comps.len())
        };
        let weight = (dx * dx + dy * dy + dz * dz).sqrt();
        weighted_sum += weight * positions[idx][1];
        weight_total += weight;
    }
    assert!(
        weight_total > 0.0,
        "no line named a body vertex with a nonzero delta"
    );
    weighted_sum / weight_total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_by_displacement_magnitude_not_by_count() {
        // Vertex 0 moves a little, vertex 1 moves ten times as much, so the
        // mean should land nine-tenths of the way from vertex 0's height
        // toward vertex 1's.
        let positions = vec![[0.0, 0.0, 0.0], [0.0, 10.0, 0.0]];
        let text = "0 0.1 0.0 0.0\n1 1.0 0.0 0.0\n";
        let mean = weighted_mean_from_text(text, &positions);
        assert!((mean - 9.090_909_09).abs() < 1.0e-6, "got {mean}");
    }

    #[test]
    fn ignores_a_delta_past_the_body_vertex_range() {
        let positions = vec![[0.0, 5.0, 0.0]];
        let text = "0 1.0 0.0 0.0\n20000 9.0 0.0 0.0\n";
        let mean = weighted_mean_from_text(text, &positions);
        assert!((mean - 5.0).abs() < 1.0e-9, "got {mean}");
    }

    #[test]
    #[should_panic(expected = "no line named a body vertex")]
    fn panics_if_every_delta_is_outside_the_body_range() {
        let positions = vec![[0.0, 0.0, 0.0]];
        weighted_mean_from_text("20000 1.0 0.0 0.0\n", &positions);
    }
}
