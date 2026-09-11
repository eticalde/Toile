use std::sync::OnceLock;

use crate::asset::{self, Baked, Row, RowKind};
use crate::normals::vertex_normals;
use crate::phenotype::{self, Phenotype};

#[cfg(test)]
mod tests;

/// The bytes `assets/body.bin` carries, embedded so the crate has no runtime
/// file to find and no path to get wrong. Regenerate it with
/// `cargo run -p toile-cli -- anny-bake RUTA/a/mpfb2`.
pub(crate) const ASSET: &[u8] = include_bytes!("../../assets/body.bin");

/// The asset decoded once per process rather than once per mesh: the row
/// table and 2,124,560 deltas are the bulk of its ~17 MB, and every field
/// but the positions is identical from one phenotype to the next, so
/// parsing them again on every slider tick would turn "rebuild before the
/// next paint" into a stutter. Only the first call after startup pays that
/// cost; see the crate's tests for the measured split.
static DECODED: OnceLock<Baked> = OnceLock::new();

/// The shipped asset, decoded once. `pub(crate)` so [`crate::measure`] and
/// [`crate::solve`] can read the row and ring tables off the same cached
/// [`Baked`] value this module builds meshes from, rather than paying to
/// decode the ~17 MB payload a second time.
pub(crate) fn decoded() -> &'static Baked {
    DECODED.get_or_init(|| {
        asset::decode(ASSET).expect("the shipped asset decodes: it is baked and tested here")
    })
}

/// A triangulated body surface in the renderer's terms.
///
/// Metres, y-up, centred on the bounding-box centre of the source body,
/// CCW-outward winding, unit outward normals — the same shape as
/// `toile_body::BodyMesh`, so `toile_engine::body` can move the fields
/// straight across without either crate depending on the other.
pub struct BodyMesh {
    /// Vertex positions as xyz triples.
    pub positions: Vec<f32>,
    /// Unit outward normals, one xyz triple per position.
    pub normals: Vec<f32>,
    /// CCW triangle indices into `positions`.
    pub indices: Vec<u32>,
    /// The `Station` tag of every vertex, matching `toile_body::Station`'s
    /// numbering byte for byte (the baker writes it from that very enum), so
    /// a client can light the region a measurement is read at exactly as it
    /// does for the procedural dummy.
    pub stations: Vec<u8>,
}

impl BodyMesh {
    /// The number of vertices in the mesh.
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }
}

/// A lever row's weight at `t`: `max(t, 0)` for the `-incr` half, `max(-t,
/// 0)` for the `-decr` half — only one of the two is ever nonzero for a
/// given `t`.
pub(crate) fn lever_weight(t: f64, incr: bool) -> f64 {
    if incr { t.max(0.0) } else { (-t).max(0.0) }
}

/// Applies every nonzero-weight row's deltas onto `positions` (already
/// widened to `f64` for the accumulation) for which `skip` says no,
/// walking the asset's rows in their fixed baked order — [`accumulate`] is
/// this with a `skip` that never fires; [`crate::solve`] uses the general
/// form to build a baseline that holds one row (or every height-tagged
/// row) out, so its secant loop only ever touches what is actually
/// changing.
///
/// Skipping a row whose weight is exactly `0.0` is not just an optimization:
/// most of the 336 weighted rows are zero at any one phenotype (a row for
/// `newborn` contributes nothing once age is floored into the adult range),
/// so this keeps a rebuild to the handful of rows actually in play rather
/// than touching all 2.1M deltas every time.
pub(crate) fn accumulate_except(
    baked: &Baked,
    phens: &[f64; 26],
    levers: &[f64; 20],
    positions: &mut [f64],
    skip: impl Fn(&Row) -> bool,
) {
    for row in &baked.rows {
        if skip(row) {
            continue;
        }
        let weight = match row.kind {
            RowKind::Weighted { mask } => phenotype::row_weight(mask, phens),
            RowKind::Lever { lever, incr } => lever_weight(levers[lever as usize], incr),
        };
        if weight == 0.0 {
            continue;
        }
        let start = row.offset as usize;
        let end = start + row.length as usize;
        for delta in &baked.deltas[start..end] {
            let base = delta.vertex as usize * 3;
            positions[base] += weight * (f64::from(delta.dx) / 10_000.0);
            positions[base + 1] += weight * (f64::from(delta.dy) / 10_000.0);
            positions[base + 2] += weight * (f64::from(delta.dz) / 10_000.0);
        }
    }
}

/// [`accumulate_except`] with a `skip` that never fires: every weighted row
/// and every lever row contributes.
fn accumulate(baked: &Baked, phens: &[f64; 26], levers: &[f64; 20], positions: &mut [f64]) {
    accumulate_except(baked, phens, levers, positions, |_| false);
}

/// The template's positions, widened to `f64` once so a caller can
/// accumulate onto them without repeating the narrow-to-`f32` step this
/// module itself does at the end of [`body_mesh`]. Exposed to
/// [`crate::solve`], which builds several such accumulations per solve.
pub(crate) fn template_positions_f64(baked: &Baked) -> Vec<f64> {
    baked.positions.iter().map(|&f| f64::from(f)).collect()
}

/// Builds the Anny body for one phenotype and lever vector: the embedded
/// template plus every baked row's weighted delta, summed in the asset's
/// fixed row order.
///
/// `levers` is in [`phenotype::LEVERS`] order, each in `[-1, 1]`; all zero
/// reproduces the phenotype-only body the second slice shipped.
///
/// The accumulation runs in `f64` even though the template and the result
/// are `f32`, widening once at the start and narrowing once at the end,
/// so many small weighted additions round only twice rather than once per
/// row — still nothing but `+ - * /` on the path, so the result stays
/// bit-identical across architectures the same way the neutral template
/// already was.
///
/// # Panics
/// If the embedded bytes fail to decode. That would mean `assets/body.bin`
/// was hand-edited or corrupted in transit: the file this crate ships is
/// always produced by the baker and is tested against this same reader.
pub fn body_mesh(phenotype: &Phenotype, levers: &[f64; 20]) -> BodyMesh {
    let baked = decoded();
    let phens = phenotype::phens26(phenotype);

    let mut positions = template_positions_f64(baked);
    accumulate(baked, &phens, levers, &mut positions);
    let positions: Vec<f32> = positions.iter().map(|&d| d as f32).collect();

    let normals = vertex_normals(&positions, &baked.indices);
    BodyMesh {
        positions,
        normals,
        indices: baked.indices.clone(),
        stations: baked.stations.clone(),
    }
}
