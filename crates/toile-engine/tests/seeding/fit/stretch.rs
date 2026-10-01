use super::{apart, let_go, meshed};
use crate::extremes::wide_hipped;
use crate::skirt::Cut;
use crate::watch::reference;

/// What rolling costs the cloth, edge by edge, on both bodies.
///
/// The numbers `Layout`'s own doc carries, and the reason they are in it: the
/// line a garment hangs by holds its length, and the cloth that crosses a
/// change of girth does not. Both are properties of a cone and neither is
/// adjustable, so what this test is for is noticing the day one of them moves.
///
/// The band is read over the very pairs the elastic holds, which is the run
/// that has to close first; the tail is the worst one per cent of every edge of
/// every piece, which is where a change of girth lands.
#[test]
#[ignore = "release-only: two real bodies baked"]
fn rolling_holds_the_line_a_garment_hangs_by_and_stretches_the_cloth_at_the_hip() {
    for (scene, mesh, cut) in [
        ("reference adult", reference(), Cut::REFERENCE),
        ("wide-hipped body", wide_hipped(), Cut::WIDE_HIPPED),
    ] {
        let (session, _) = let_go(&mesh, cut);
        let ring = session.layout().expect("the seams place the skirt");
        let pipes = meshed(&session);
        let bases: Vec<u32> = session
            .pieces()
            .iter()
            .map(|&key| session.offset(key).expect("every piece drapes"))
            .collect();

        let mut ratios: Vec<f64> = Vec::new();
        let (mut whole_rolled, mut whole_flat) = (0.0f64, 0.0f64);
        for (at, pipe) in pipes.iter().enumerate() {
            for &(a, b) in &pipe.edges {
                let (pa, pb) = (pipe.pos2d[a as usize], pipe.pos2d[b as usize]);
                let flat = (pb[0] - pa[0]).hypot(pb[1] - pa[1]);
                let (Some(qa), Some(qb)) = (ring.point(at, pa), ring.point(at, pb)) else {
                    continue;
                };
                let rolled = f64::from(apart(qa, qb));
                whole_rolled += rolled;
                whole_flat += flat;
                if flat > 1.0e-9 {
                    ratios.push(rolled / flat);
                }
            }
        }
        ratios.sort_by(f64::total_cmp);
        let tail = ratios[ratios.len() * 99 / 100];
        let whole = whole_rolled / whole_flat;
        let shortened = ratios.iter().filter(|&&r| r < 1.0).count();

        // The held run, over the pairs the elastic itself names, so this is the
        // length of the very line that has to shut.
        let (mut band_rolled, mut band_flat) = (0.0f64, 0.0f64);
        for ((a, b), _) in session.held_cloth() {
            let mut each = [None, None];
            for (at, pipe) in pipes.iter().enumerate() {
                let n = pipe.pos2d.len() as u32;
                for (k, v) in [a, b].into_iter().enumerate() {
                    if v >= bases[at] && v < bases[at] + n {
                        each[k] = Some((at, pipe.pos2d[(v - bases[at]) as usize]));
                    }
                }
            }
            let (Some((at, pa)), Some((_, pb))) = (each[0], each[1]) else {
                continue;
            };
            band_flat += (pb[0] - pa[0]).hypot(pb[1] - pa[1]);
            let (Some(qa), Some(qb)) = (ring.point(at, pa), ring.point(at, pb)) else {
                continue;
            };
            band_rolled += f64::from(apart(qa, qb));
        }
        let band = band_rolled / band_flat;
        println!(
            "{scene}: {} edges · the whole cloth {:+.3} % of flat, worst 1 % at {tail:.4}×, \
             worst edge {:.4}× · {shortened} came out shorter, the worst at {:.4}× · the held \
             run {:+.4} % over {} pairs · girth {:.4} m at the crest to {:.4} m at the hip, \
             {:.0} cm apart",
            ratios.len(),
            100.0 * (whole - 1.0),
            ratios[ratios.len() - 1],
            ratios[0],
            100.0 * (band - 1.0),
            session.held_cloth().len(),
            ring.round.girth(ring.crest),
            ring.round.girth(-cut.hip_drop / 100.0),
            cut.hip_drop
        );
        // Under one per cent, which is where the two things that cost it land:
        // a released edge is the chord of its own arc, and the mesh's boundary
        // cuts the piece's corners so the very top line is read a little short
        // of the drawing. Neither is a stretch the cloth has to carry.
        assert!(
            (band - 1.0).abs() < 0.01,
            "{scene}: the line the garment hangs by came out {:+.4} % of flat",
            100.0 * (band - 1.0)
        );
        assert!(
            whole > 1.0 && whole < 1.10,
            "{scene}: the whole cloth came out {:+.3} % of flat",
            100.0 * (whole - 1.0)
        );
        // And the tail, which `Layout`'s own doc quotes and nothing held. A sum
        // over sixty thousand edges is the wrong instrument for it: the two
        // things that stretch the cloth are the profile's slope and the shear
        // of a line the changing width is not a meridian of, and either
        // could double while the mean moved a per cent. Measured,
        // 1.2894× and 1.6680×.
        assert!(
            tail < 1.75,
            "{scene}: the worst one per cent of edges came out {tail:.4}×, where \
             the doc says 1.289× and 1.668×"
        );
        // A third of the cloth comes out shorter than flat, because a flare
        // carries more of it than the path it has to run. That is not a defect
        // — it bunches — but the doc owes the number and this is where
        // it lives.
        assert!(
            ratios[0] > 0.30 && shortened * 4 > ratios.len(),
            "{scene}: {shortened} of {} edges came out shorter, the worst at \
             {:.4}×",
            ratios.len(),
            ratios[0]
        );
    }
}
