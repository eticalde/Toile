use toile_anny::{BodyMesh, Station};

use self::grid::{Grid, Span};
use self::pierce::Point;

mod grid;
mod pierce;

/// How deep a slab off the underside of a body counts as its soles, in
/// metres.
pub const SOLE_DEPTH: f32 = 0.03;

/// Where on a body two sheets of its skin pass through each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// Under a foot: both triangles tagged as foot, and wholly inside the
    /// slab the body stands on. The default body carries these, they cost it
    /// voxels buried in a sole, and no garment is ever held there.
    Sole,
    /// On the body, at the station each of the two triangles belongs to. The
    /// two differ where one part of the body runs into another, an arm into
    /// the chest beside it.
    Body([Station; 2]),
    /// On a mesh that carries no stations, where only the point says where.
    Unplaced,
}

/// One pair of triangles that pass through each other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    /// The two triangles, by their place in the index list, lower first.
    pub triangles: [u32; 2],
    /// Halfway between the two triangles' centroids, in metres.
    pub at: [f32; 3],
    /// Where on the body that is.
    pub place: Place,
}

/// Every place a mesh passes through itself, in rising order of triangle.
///
/// A report and never a verdict. The sign of a baked field is only exact for
/// a surface that does not cross itself, and nothing the bake counts can see
/// whether one does; this can, and says where. It refuses nothing, because
/// the body the app opens with crosses itself under both soles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Crossings {
    found: Vec<Crossing>,
}

impl Crossings {
    /// Every crossing, the soles included.
    pub fn all(&self) -> &[Crossing] {
        &self.found
    }

    /// The crossings anywhere but under the soles: the ones that sit where a
    /// garment can be.
    pub fn exposed(&self) -> impl Iterator<Item = &Crossing> {
        self.found.iter().filter(|c| c.place != Place::Sole)
    }

    /// The stations the exposed crossings touch, each once, in the body's
    /// own station order.
    pub fn stations(&self) -> Vec<Station> {
        let touched = |s: &Station| {
            self.exposed()
                .any(|c| matches!(c.place, Place::Body(pair) if pair.contains(s)))
        };
        Station::ALL.into_iter().filter(touched).collect()
    }
}

/// Finds every pair of triangles of a mesh that pass through each other.
///
/// Two triangles that share a vertex are never a pair: neighbours always
/// meet along what they share, and that is the surface being a surface. A
/// fold sharp enough to push a triangle through its own neighbour is
/// therefore not seen.
///
/// The arithmetic is `f64` over the mesh's `f32` and is not exact, so a pair
/// that only just touches may read either way; it reads the same way on
/// every run. A mesh with no triangle, or with a position that is not a
/// number, is not looked at and reports nothing.
pub fn crossings(mesh: &BodyMesh) -> Crossings {
    let tris = mesh.indices.as_chunks::<3>().0;
    if tris.is_empty() || !mesh.positions.iter().all(|f| f.is_finite()) {
        return Crossings::default();
    }
    let corners = |t: u32| tris[t as usize].map(|v| at(&mesh.positions, v));
    let boxes: Vec<Span> = (0..tris.len() as u32).map(|t| span(corners(t))).collect();

    let mut pairs: Vec<[u32; 2]> = Vec::new();
    Grid::over(&boxes).pairs(&boxes, |a, b| {
        let (ta, tb) = (tris[a as usize], tris[b as usize]);
        if !ta.iter().any(|v| tb.contains(v)) && pierce::crosses(corners(a), corners(b)) {
            pairs.push([a, b]);
        }
    });
    // The grid hands pairs over cell by cell, and the cells come from the
    // mesh's own size. Sorted, the report does not move with them.
    pairs.sort_unstable();

    let floor = mesh
        .positions
        .as_chunks::<3>()
        .0
        .iter()
        .fold(f32::INFINITY, |lo, p| lo.min(p[1]));
    let found = pairs
        .into_iter()
        .map(|triangles| {
            let [p, q] = triangles.map(corners);
            Crossing {
                triangles,
                at: [0, 1, 2]
                    .map(|c| ((0..3).map(|k| p[k][c] + q[k][c]).sum::<f64>() / 6.0) as f32),
                place: place(mesh, triangles.map(|t| tris[t as usize]), floor),
            }
        })
        .collect();
    Crossings { found }
}

/// Where a crossing between two triangles is, as far as the mesh can say.
fn place(mesh: &BodyMesh, tris: [[u32; 3]; 2], floor: f32) -> Place {
    let Some(stations) = stationed(mesh, tris) else {
        return Place::Unplaced;
    };
    let low = tris
        .iter()
        .flatten()
        .all(|&v| mesh.positions[v as usize * 3 + 1] <= floor + SOLE_DEPTH);
    if low && stations == [Station::Ankle; 2] {
        Place::Sole
    } else {
        Place::Body(stations)
    }
}

/// The station of each of two triangles; `None` for a mesh whose vertices
/// carry no readable tag.
fn stationed(mesh: &BodyMesh, tris: [[u32; 3]; 2]) -> Option<[Station; 2]> {
    let tag = |v: u32| {
        let tag = *mesh.stations.get(v as usize)?;
        Station::ALL.get(usize::from(tag)).copied()
    };
    // A triangle astride two stations belongs to the one two of its corners
    // agree on, and to its first corner's when all three differ.
    let of = |t: [u32; 3]| {
        let [a, b, c] = [tag(t[0])?, tag(t[1])?, tag(t[2])?];
        Some(if b == c { b } else { a })
    };
    Some([of(tris[0])?, of(tris[1])?])
}

/// One vertex as a `f64` point.
fn at(positions: &[f32], v: u32) -> Point {
    let i = v as usize * 3;
    [0, 1, 2].map(|k| f64::from(positions[i + k]))
}

/// The box around one triangle.
fn span(p: [Point; 3]) -> Span {
    (
        [0, 1, 2].map(|c| p[0][c].min(p[1][c]).min(p[2][c])),
        [0, 1, 2].map(|c| p[0][c].max(p[1][c]).max(p[2][c])),
    )
}
