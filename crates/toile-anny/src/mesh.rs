use crate::asset::{self, Baked};
use crate::normals::vertex_normals;

/// The bytes `assets/body.bin` carries, embedded so the crate has no runtime
/// file to find and no path to get wrong. Regenerate it with
/// `cargo run -p toile-cli -- anny-bake RUTA/a/mpfb2`.
const ASSET: &[u8] = include_bytes!("../assets/body.bin");

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

/// Decodes the embedded asset and computes its normals.
///
/// The neutral Anny body: sex, age and phenotype are not inputs yet — every
/// call returns the same bake. A future slice adds the levers; this one
/// proves the data pipeline, the asset format and the render path.
///
/// # Panics
/// If the embedded bytes fail to decode. That would mean `assets/body.bin`
/// was hand-edited or corrupted in transit: the file this crate ships is
/// always produced by the baker and is tested against this same reader.
pub fn body_mesh() -> BodyMesh {
    let Baked {
        positions,
        indices,
        stations,
    } = asset::decode(ASSET).expect("the shipped asset decodes: it is baked and tested here");
    let normals = vertex_normals(&positions, &indices);
    BodyMesh {
        positions,
        normals,
        indices,
        stations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERTEX_COUNT: usize = 13_380;
    const TRIANGLE_COUNT: usize = 26_756;

    #[test]
    fn the_shipped_asset_loads_at_the_expected_size() {
        let m = body_mesh();
        assert_eq!(m.vertex_count(), VERTEX_COUNT);
        assert_eq!(m.positions.len(), VERTEX_COUNT * 3);
        assert_eq!(m.normals.len(), VERTEX_COUNT * 3);
        assert_eq!(m.indices.len(), TRIANGLE_COUNT * 3);
        assert_eq!(m.stations.len(), VERTEX_COUNT);
    }

    #[test]
    fn every_index_is_in_range_and_every_position_is_finite() {
        let m = body_mesh();
        assert!(m.indices.iter().all(|&i| (i as usize) < m.vertex_count()));
        assert!(m.positions.iter().all(|f| f.is_finite()));
    }

    #[test]
    fn every_station_tag_is_below_the_count() {
        let m = body_mesh();
        for &tag in &m.stations {
            assert!(
                u32::from(tag) < u32::from(toile_body::Station::COUNT),
                "tag {tag} is not a real station"
            );
        }
    }

    #[test]
    fn all_twenty_two_stations_are_carried_by_some_vertex() {
        let m = body_mesh();
        for station in toile_body::Station::ALL {
            assert!(
                m.stations.contains(&station.tag()),
                "no vertex is tagged {station:?}"
            );
        }
    }

    #[test]
    fn every_normal_is_unit_length() {
        let m = body_mesh();
        for n in m.normals.as_chunks::<3>().0 {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1.0e-4, "normal length {len}");
        }
    }

    #[test]
    fn normals_point_outward() {
        // toile_body's per-part version dots each vertex's normal against the
        // vector from its own ring's centroid, which works because a ring is
        // star-shaped around it. The whole body is not star-shaped around any
        // one point — an outstretched arm's surface does not point away from
        // the torso's centroid — so that test measured only 58% outward here.
        // A closed, consistently-wound mesh has a much stronger and exact
        // property instead: the divergence theorem says the signed volume
        // enclosed, summed one tetrahedron per triangle from any apex, is
        // positive exactly when the winding is CCW-outward. It comes out to
        // about 55 litres for this body, a plausible adult volume.
        let m = body_mesh();
        let at = |i: u32| {
            let i = i as usize * 3;
            [
                f64::from(m.positions[i]),
                f64::from(m.positions[i + 1]),
                f64::from(m.positions[i + 2]),
            ]
        };
        let signed_volume_x6: f64 = m
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let (a, b, c) = (at(t[0]), at(t[1]), at(t[2]));
                let cross = [
                    b[1] * c[2] - b[2] * c[1],
                    b[2] * c[0] - b[0] * c[2],
                    b[0] * c[1] - b[1] * c[0],
                ];
                a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]
            })
            .sum();
        let litres = signed_volume_x6 / 6.0 * 1000.0;
        assert!(
            (10.0..200.0).contains(&litres),
            "enclosed volume {litres} L is not a plausible body"
        );
    }
}
