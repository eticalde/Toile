use std::f64::consts::TAU;

use toile_engine::couture::Layout;
use toile_engine::session::Session;

use super::outline::Outline;
use crate::fit::meshed;

/// How much of the band the tall piece's own edge may swing across, in radians.
///
/// Not a tolerance so much as a name for zero: that edge is one side of the
/// piece all the way up, so on a surface carried straight up it is a meridian
/// and the swing is the arithmetic's own noise. Below the band, where the strip
/// is whole, the same edge holds to 0.0016 rad across the five centimetres
/// under it, so a hundredth of a radian is still well inside what the cloth's
/// own shape does where the reading is not in question.
const A_MERIDIAN: f64 = 0.01;

/// What the surface does above the height the shortest piece stops at.
///
/// The block ships a back that rises above its front, so between the two tops
/// there is cloth of one piece and none of the other. Two readings are owed
/// there and they are not the same one: how much cloth is at that height, which
/// is the back's alone, and how far round the surface goes, which is the last
/// hoop the strip agreed on, carried straight up.
///
/// Both are printed, because reading one for the other is how a hoop came to be
/// sized off a width that is not there — and because the crest's own pair is no
/// coverage: a back ending in a tab puts the crest at the tab's tip, where
/// 0.00000 m of cloth sits on a 0.44075 m hoop.
///
/// # Panics
/// If the block stops exercising this, if the girth there is not the cloth that
/// is there, if the hoop pinches inside the strip's own last one, or if the
/// piece left up there is sheared round rather than carried up.
pub fn above_the_strip(session: &Session, ring: &Layout) {
    let outlines: Vec<Outline> = meshed(session).iter().map(Outline::of).collect();
    let whole = outlines.iter().map(|o| o.span().1).fold(f64::MAX, f64::min);
    let top = outlines.iter().map(|o| o.span().1).fold(f64::MIN, f64::max);
    let band = top - whole;
    println!(
        "the strip is whole to {whole:.5} and one piece reaches {top:.5}, a band of {:.1} mm: \
         girth {:.5} m at the top against {:.5} m on the strip · hoop {:.5} m against {:.5} m",
        band * 1000.0,
        ring.round.girth(top),
        ring.round.girth(whole),
        TAU * ring.round.radius(top),
        TAU * ring.round.radius(whole)
    );
    assert!(
        band > 0.020,
        "no piece of the block reaches above the others any more, so nothing here \
         is being read: {:.1} mm",
        band * 1000.0
    );

    // The coverage, read where there is cloth to read it on. At the crest the
    // ratio is nought over the hoop above; on the topmost line the whole strip
    // reaches it is a figure, and the room the body asked for is the whole of
    // what separates the two numbers in it.
    let (cloth, hoop) = (ring.round.girth(whole), TAU * ring.round.radius(whole));
    println!(
        "the crest is at {top:.5} and carries {:.5} m on a {:.5} m hoop — no \
         coverage to read; on the topmost whole line it is {cloth:.5} m on \
         {hoop:.5} m, {:.1} %, the {:.5} m between them being the room the body \
         asked for",
        ring.round.girth(top),
        TAU * ring.round.radius(top),
        100.0 * cloth / hoop,
        hoop - cloth
    );
    // And what the other reading of the band would have cost, which is why the
    // surface carries the last whole hoop up instead of following the cloth
    // that is left: half a millimetre above the front's top edge only the
    // back is there, so a hoop sized off that alone steps down by the
    // front's whole width between two rows of vertices. Both radii off
    // the cloth alone, so the room the body asked for is in neither.
    let (on_it, above) = (cloth / TAU, ring.round.girth(whole + 0.0005) / TAU);
    println!(
        "a hoop following the cloth that is left would fall from {on_it:.5} m of \
         radius on that line to {above:.5} m half a millimetre above it, \
         {:.1} mm of cliff",
        (on_it - above) * 1000.0
    );

    // The girth is the cloth that is there, against this suite's own reading of
    // which pieces reach that high and how wide they are on the way.
    for k in 0..=10 {
        let y = whole + band * f64::from(k) / 10.0;
        let there: f64 = outlines.iter().map(|o| o.width(y)).sum();
        assert!(
            (ring.round.girth(y) - there).abs() < 1.0e-12,
            "at {y} the surface says it carries {:.5} m of cloth and the pieces \
             that reach that high carry {there:.5} m",
            ring.round.girth(y)
        );
    }
    // And the hoop is the strip's own last one, carried straight up: no cliff
    // for the cloth to fold at. Read against the cloth alone, because the room
    // the body asked for is a separate table that only ever adds.
    let flat = ring.round.girth(whole) / TAU;
    for k in 0..=10 {
        let y = whole + band * f64::from(k) / 10.0;
        assert!(
            ring.round.radius(y) - flat >= -1.0e-12,
            "at {y} the surface pinches {:.5} m inside the last hoop the strip \
             agreed on",
            flat - ring.round.radius(y)
        );
    }

    // The line the shear showed on: the low edge of the tall piece at its very
    // top is one side of it all the way down the band, so carried up the
    // surface it is a meridian. Read every piece's width at its own
    // ordinate while the hoop holds still and it swings 1.604 rad across this
    // band instead.
    let (tall, _) = outlines
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.span().1.total_cmp(&b.1.span().1))
        .expect("the block has pieces");
    let edge = outlines[tall]
        .scan(top)
        .expect("the tall piece is there at its own top")
        .0;
    let mut swing = 0.0f64;
    let mut first = None;
    for k in 0..=20 {
        let y = whole + band * f64::from(k) / 20.0;
        let Some(q) = ring.point(tall, [edge, y]) else {
            continue;
        };
        let turn = f64::from(q[2] - ring.axis[1]).atan2(f64::from(q[0] - ring.axis[0]));
        let from = *first.get_or_insert(turn);
        let apart = (turn - from).abs();
        swing = swing.max(apart.min(TAU - apart));
    }
    println!("the tall piece's own edge swings {swing:.6} rad across the band");
    assert!(
        swing < A_MERIDIAN,
        "the piece left above the strip is sheared round the surface rather than \
         carried up it: its own edge swings {swing:.4} rad over {:.1} mm of \
         pattern",
        band * 1000.0
    );
}
