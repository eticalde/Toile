use super::*;

/// A rectangle, which carries the same cloth at every height and so rolls
/// onto one radius: the surface where the old cylinder and this one agree.
fn pipeline(w: f64, h: f64) -> ShapePipeline {
    let rectangle = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
    ShapePipeline::build(&rectangle, 16, 0.01).expect("the rectangle is finite")
}

/// A piece flared ten to one: 0.10 m of cloth at the head, 1.00 m at the
/// foot, and only 0.20 m of pattern between the two.
fn flare() -> ShapePipeline {
    let contour = [[0.0, -0.20], [1.00, -0.20], [0.10, 0.0], [0.0, 0.0]];
    ShapePipeline::build(&contour, 128, 0.001).expect("the flare is finite")
}

/// A piece the pattern brings to a point: 0.40 m of cloth at its foot,
/// narrowing to nothing 0.49 m above it.
///
/// A gore, which is an ordinary pattern shape: a cone skirt or a cape is cut as
/// a ring of these, every one of them coming to a point at the same height.
fn spire() -> ShapePipeline {
    let contour = [[0.0, 0.0], [0.40, 0.0], [0.20, 0.49]];
    ShapePipeline::build(&contour, 96, 0.001).expect("the spire is finite")
}

/// The strip walked over `order`, let go at `stand` with the crest at 0.20.
fn layout(order: &[(usize, f64)], pipes: &[&ShapePipeline], closed: bool, stand: f32) -> Layout {
    let round = Round::over(order, pipes, closed, 0.005, None).expect("the pieces have widths");
    Layout::on(round, [0.0, 0.0], stand, 0.20, pipes.len())
}

/// Along a hoop, rolling preserves length: two points at one ordinate come
/// out as far apart along the arc as they are across the cloth.
#[test]
fn an_arc_carries_exactly_as_much_cloth_as_it_is_long() {
    let pipe = pipeline(0.30, 0.20);
    let place = layout(&[(0, 1.0)], &[&pipe], false, 1.0);
    let (near, far) = (
        place.point(0, [0.0, 0.10]).expect("placed"),
        place.point(0, [0.10, 0.10]).expect("placed"),
    );
    let radius = place.round.radius(0.10);
    let turn = (0.10f64 / radius) * 0.5;
    let chord = 2.0 * radius * turn.sin();
    let got = f64::from((far[0] - near[0]).hypot(far[2] - near[2]));
    // Read back out of the `f32` the release is written in, so the
    // tolerance is that type's own and not the arithmetic's.
    assert!((got - chord).abs() < 1.0e-7, "{got} against {chord}");
}

/// And down a meridian it does not, which is the number the doc above owes.
///
/// The seam edge of a tube runs straight down the pattern and comes out
/// running down the surface's own profile, which leans out wherever the
/// girth climbs. A piece flared ten to one over 0.20 m of pattern leans a
/// long way: the edge comes out about 1.7× its flat length, and the ratio
/// is the profile's own slope and nothing adjustable.
#[test]
fn down_a_meridian_the_cloth_is_stretched_by_how_fast_the_girth_climbs() {
    let pipe = flare();
    let place = layout(&[(0, 1.0), (1, -1.0)], &[&pipe, &pipe], true, 1.0);
    let (head, foot) = ([0.0, 0.0], [0.0, -0.20]);
    let (a, b) = (
        place.point(0, head).expect("placed"),
        place.point(0, foot).expect("placed"),
    );
    let across = f64::from((b[0] - a[0]).hypot(b[2] - a[2]));
    let rolled = across.hypot(f64::from(b[1] - a[1]));
    let lean = (place.round.radius(-0.20) - place.round.radius(0.0)) / 0.20;
    assert!(
        (rolled / 0.20 - lean.hypot(1.0)).abs() < 0.01,
        "the edge came out {rolled} against 0.20 flat, where the profile's \
         slope of {lean} accounts for {}",
        0.20 * lean.hypot(1.0)
    );
    assert!(rolled > 0.20 * 1.6, "and that is a long way: {rolled}");
}

/// The sense is the mirror: the same piece walked the other way lands the
/// same points at the same distance from the axis, turned the other way.
#[test]
fn the_two_senses_are_mirror_images() {
    let pipe = pipeline(0.30, 0.20);
    let with = layout(&[(0, 1.0)], &[&pipe], false, 1.0);
    let against = layout(&[(0, -1.0)], &[&pipe], false, 1.0);
    let (a, b) = (
        with.point(0, [0.10, 0.05]).expect("placed"),
        against.point(0, [0.10, 0.05]).expect("placed"),
    );
    assert!((a[0] + b[0]).abs() < 1.0e-7, "the turn flipped");
    assert!((a[2] - b[2]).abs() < 1.0e-7, "and nothing else did");
    assert!((a[1] - b[1]).abs() < 1.0e-7);
}

/// The crest is what the surface's height names: the cloth at that
/// ordinate starts there, and every other line hangs below it by the
/// distance the pattern puts between them.
#[test]
fn the_pattern_line_the_crest_names_is_the_one_let_go_at_the_ring() {
    let pipe = pipeline(0.30, 0.20);
    let place = layout(&[(0, 1.0)], &[&pipe], false, 1.5);
    let top = place.point(0, [0.0, 0.20]).expect("placed");
    let below = place.point(0, [0.0, -0.30]).expect("placed");
    assert!((top[1] - 1.5).abs() < 1.0e-7);
    assert!((below[1] - 1.0).abs() < 1.0e-7);
}

/// A product whose pieces come to a point is placed to the point, and not
/// dropped off the body.
///
/// Two gores sewn into a cone, which is how a cape or a circular skirt is cut.
/// At their shared apex the hoop closes with the cloth, so that one ordinate
/// has no width at all — and `wrap_into` places a piece or none of it, so one
/// vertex answering nothing left both of them at the flat release, 1.2 m above
/// the body. What a closed hoop should do is the question the refusal never
/// asked: it has one place on it, and every vertex of that ordinate belongs
/// there.
#[test]
fn a_product_that_comes_to_a_point_is_placed_to_its_apex() {
    let pipe = spire();
    let place = layout(&[(0, 1.0), (1, -1.0)], &[&pipe, &pipe], true, 1.0);
    let mut state = State::new(2 * pipe.pos2d.len());
    for at in 0..2 {
        assert!(
            place.wrap_into(&mut state, at * pipe.pos2d.len(), &pipe, at),
            "gore {at} is placed, apex and all"
        );
    }
    let apex = pipe
        .pos2d
        .iter()
        .copied()
        .fold([0.0, f64::MIN], |hi, p| if p[1] > hi[1] { p } else { hi });
    let out = |p: [f64; 2]| {
        let q = place.point(0, p).expect("the gore is placed");
        f64::from((q[0] - place.axis[0]).hypot(q[2] - place.axis[1]))
    };
    println!(
        "the cone's apex at {:.5} lands {:.4e} m from the axis on a hoop of \
         {:.4e} m, and the cloth a millimetre below it {:.6} m out",
        apex[1],
        out(apex),
        place.round.radius(apex[1]),
        out([apex[0], apex[1] - 0.001])
    );
    assert!(
        place.round.radius(apex[1]) <= 1.0e-12,
        "the hoop at the apex has closed, which is the reading being answered: {}",
        place.round.radius(apex[1])
    );
    // Every vertex, each at the radius its own ordinate names. A placement that
    // answered the apex and lost the cloth around it would pass the line above.
    for (i, &p) in pipe.pos2d.iter().enumerate() {
        let q = place
            .point(0, p)
            .expect("every vertex of the gore is placed");
        assert!(
            q.iter().all(|c| c.is_finite()),
            "vertex {i} at ordinate {} came out at {q:?}",
            p[1]
        );
        let asked = place.round.radius(p[1]);
        assert!(
            (out(p) - asked).abs() < 1.0e-6,
            "vertex {i} at ordinate {} sits {} out where its own hoop asks for \
             {asked}",
            p[1],
            out(p)
        );
    }
    // And the crest may be the point, which is where a cone's own topmost
    // ordinate puts it: with nothing holding a product up it hangs by its
    // highest line, and for this one that line is the apex. Read as a refusal
    // there, every wrap came out empty and the walk placed no piece at all.
    let round = Round::over(&[(0, 1.0), (1, -1.0)], &[&pipe, &pipe], true, 0.005, None)
        .expect("both gores have widths");
    let hangs = Layout::on(round, [0.0, 0.0], 1.0, apex[1], 2);
    assert!(
        hangs.wraps.iter().all(Option::is_some),
        "a product hung by its own apex is still placed: {:?}",
        hangs.wraps
    );
}

/// A piece the seams do not place is left for the flat release, and says
/// so rather than landing somewhere arbitrary.
#[test]
fn a_piece_without_a_wrap_is_not_placed() {
    let pipe = pipeline(0.30, 0.20);
    let place = layout(&[(1, 1.0)], &[&pipe, &pipe], false, 1.0);
    let mut state = State::new(pipe.pos2d.len());
    assert!(place.wraps[0].is_none(), "the walk never crossed it");
    assert!(place.wraps[1].is_some(), "and did cross the other");
    assert!(!place.wrap_into(&mut state, 0, &pipe, 0));
    assert!(
        !place.wrap_into(&mut state, 0, &pipe, 7),
        "nor is one the layout has never heard of"
    );
    assert_eq!(place.point(0, [0.0, 0.0]), None);
}
