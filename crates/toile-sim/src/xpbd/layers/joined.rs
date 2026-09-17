use crate::xpbd::state::Seams;

/// The vertex pairs that may never read as one layer meeting another.
///
/// A triangle's own corners, the ring of vertices sharing a triangle with
/// them, and whatever a seam has drawn onto them all sit inside any thickness
/// worth having. Counted as contacts they would have a garment shoving itself
/// apart along every edge it owns, and fighting every seam while it closes.
///
/// One sorted run per vertex, so asking costs a binary search over a handful
/// of entries and allocates nothing per query.
pub(super) struct Joined {
    /// Where each vertex's run begins, with a final sentinel.
    start: Vec<u32>,
    /// The partners, ascending within each run.
    with: Vec<u32>,
}

impl Joined {
    /// Every pair the mesh and the sewing already hold together.
    pub(super) fn of(tris: &[u32], seams: &Seams, n_verts: usize) -> Joined {
        let mut pairs = ring_pairs(tris, n_verts);
        if !seams.is_empty() {
            let ring = rows(pairs.clone(), n_verts);
            for k in 0..seams.len() {
                let (a, b) = (seams.a[k], seams.b[k]);
                if a as usize >= n_verts || b as usize >= n_verts {
                    continue;
                }
                pairs.push((a, b));
                pairs.push((b, a));
                // A sewn pair is pulled onto one point, so the triangles round
                // either end reach across to the other. Excluding the partner
                // alone would leave a closing seam pushing its own two layers
                // apart, which is the one place in a garment where cloth is
                // meant to lie on cloth.
                for (x, y) in [(a, b), (b, a)] {
                    for &near in ring.with_vertex(y) {
                        pairs.push((x, near));
                        pairs.push((near, x));
                    }
                }
            }
        }
        rows(pairs, n_verts)
    }

    /// The vertices joined to `v`, ascending.
    pub(super) fn with_vertex(&self, v: u32) -> &[u32] {
        let at = v as usize;
        &self.with[self.start[at] as usize..self.start[at + 1] as usize]
    }

    /// Whether the mesh or the sewing already holds these two together.
    pub(super) fn holds(&self, v: u32, w: u32) -> bool {
        self.with_vertex(v).binary_search(&w).is_ok()
    }
}

/// Both directions of every pair of corners sharing a triangle.
fn ring_pairs(tris: &[u32], n_verts: usize) -> Vec<(u32, u32)> {
    let mut pairs = Vec::with_capacity(tris.len() * 2);
    for t in tris.as_chunks::<3>().0 {
        if t.iter().any(|&v| v as usize >= n_verts) {
            continue;
        }
        for &x in t {
            for &y in t {
                if x != y {
                    pairs.push((x, y));
                }
            }
        }
    }
    pairs
}

/// Sorted pairs into one run per vertex.
///
/// The sort is unstable, which is safe here for the reason it is not safe on a
/// key with ties: the key is the whole element, so two entries that compare
/// equal are the same entry and the dedup below removes them either way.
fn rows(mut pairs: Vec<(u32, u32)>, n_verts: usize) -> Joined {
    pairs.sort_unstable();
    pairs.dedup();
    let mut start = vec![0u32; n_verts + 1];
    for &(x, _) in &pairs {
        start[x as usize + 1] += 1;
    }
    for i in 0..n_verts {
        start[i + 1] += start[i];
    }
    Joined {
        start,
        with: pairs.into_iter().map(|(_, y)| y).collect(),
    }
}
