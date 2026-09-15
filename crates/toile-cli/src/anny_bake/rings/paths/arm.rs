use super::super::Joints;
use super::super::arc::{Along, Section, length};
use super::super::geom::{add, cross, dist, dot, scale, sub, unit};
use super::super::intersect::Crossing;
use super::super::select::ring_point;
use super::shoulder_point;

/// How much each step tilts the shoulders' plane: the drop, in metres, per
/// metre behind the line through both shoulder points.
const SLOPE_STEP: f64 = 0.01;

/// How many steps the tilt search takes before giving up, well past the
/// point where the line would run down over the shoulder blades.
const SLOPE_STEPS: u32 = 150;

/// How many steps of slope [`taut`] sweeps each half of its fan in. The arc
/// length is flat to a millimetre across a wide band of planes round the
/// shortest, so a finer sweep would move the line without shortening it.
const FAN_STEPS: i32 = 20;

/// How near a section must pass a landmark to count as reaching it: a plane
/// through the point crosses the skin within a fraction of this.
const REACH_M: f64 = 0.005;

/// `brazo` and `hombros`, which both start at the right shoulder point.
pub(super) fn bake(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    j: &Joints,
) -> (Vec<Crossing>, Vec<Crossing>) {
    let right = shoulder_point(positions, j.shoulder_r);
    let left = shoulder_point(positions, j.shoulder_l);
    (
        arm(positions, tris, j, right),
        shoulders(positions, tris, j, right, left),
    )
}

/// `brazo`: from the shoulder point across the outside of the arm to the
/// elbow's point, then down the back of the forearm to the wrist.
///
/// The two lower landmarks are found on the plane the arm bends in, through
/// the shoulder point and the elbow and hand joints: the rest pose bends the
/// arm a little at the elbow, so that plane holds both bones' axes, and its
/// section runs down the arm once on the side of the crook and once on the
/// side of the elbow's point. The trade takes the length over the elbow's
/// point, so that side is kept. Its crossing nearest the elbow joint is the
/// elbow's point, and where it reaches the hand joint's cross-section is the
/// wrist; between the two, the path is that line.
///
/// Above the elbow it is not: that plane holds the shoulder point on top of
/// the arm and the elbow's point behind it, so its line runs over the top of
/// the shoulder and turns down the back of the arm in one corner, which no
/// tape does. That stretch is [`taut`] instead.
fn arm(positions: &[[f64; 3]], tris: &[[u32; 3]], j: &Joints, shoulder: [f64; 3]) -> Vec<Crossing> {
    let bend = cross(sub(j.elbow_r, shoulder), sub(j.hand_r, shoulder));
    let section = Section::through(positions, tris, shoulder, bend, shoulder);
    let top = section.nearest(shoulder);
    let forearm = unit(sub(j.hand_r, j.elbow_r));
    let crook = sub(forearm, unit(sub(j.elbow_r, j.shoulder_r)));
    let beyond = |p: [f64; 3]| dot(sub(p, j.hand_r), forearm);
    let down_to_wrist = |way| {
        let past = section.walk_until(top, way, |_, p| beyond(p) >= 0.0);
        let before = section.next(past, way.reversed());
        let wrist = if beyond(section.at(past)) <= -beyond(section.at(before)) {
            past
        } else {
            before
        };
        section.arc(top, wrist, way)
    };
    let nearest_elbow = |run: &[Crossing]| {
        (0..run.len())
            .min_by(|&a, &b| {
                let d = |i: usize| dist(ring_point(positions, run[i]), j.elbow_r);
                d(a).total_cmp(&d(b))
            })
            .expect("a line down the arm has points")
    };

    let mut outside = [Along::Forward, Along::Backward]
        .into_iter()
        .map(down_to_wrist)
        .filter(|run| {
            let elbow = ring_point(positions, run[nearest_elbow(run)]);
            dot(sub(elbow, j.elbow_r), crook) < 0.0
        });
    let back = outside
        .next()
        .expect("one side of the arm faces away from the crook");
    assert!(
        outside.next().is_none(),
        "only one side faces away from the crook"
    );

    let at_elbow = nearest_elbow(&back);
    let elbow = ring_point(positions, back[at_elbow]);
    let mut path = taut(positions, tris, shoulder, elbow, bend);
    path.extend_from_slice(&back[at_elbow..]);
    path
}

/// The shortest arc from `from` to `to` among the planes that hold the chord
/// between them, the fan starting from the plane normal to `normal`.
///
/// A tape pulled between two points on the skin takes the shortest way over
/// it. The planes through both points turn about the chord like the pages of
/// a book, each cutting the skin in an arc from one point to the other, and
/// the shortest of those arcs is the planar stand-in for the tape. The fan is
/// swept in steps of slope rather than of angle, so the bake needs nothing
/// past `+ - * / sqrt`.
fn taut(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    from: [f64; 3],
    to: [f64; 3],
    normal: [f64; 3],
) -> Vec<Crossing> {
    let along = unit(sub(to, from));
    let u = unit(sub(normal, scale(along, dot(normal, along))));
    let v = cross(along, u);
    let mut best: Option<(Vec<Crossing>, f64)> = None;
    for step in -FAN_STEPS..=FAN_STEPS {
        let s = f64::from(step) / f64::from(FAN_STEPS);
        for page in [add(u, scale(v, s)), add(v, scale(u, s))] {
            for section in Section::all(positions, tris, from, page) {
                let (a, b) = (section.nearest(from), section.nearest(to));
                if dist(section.at(a), from) > REACH_M || dist(section.at(b), to) > REACH_M {
                    continue;
                }
                for way in [Along::Forward, Along::Backward] {
                    let run = section.arc(a, b, way);
                    let len = length(positions, &run);
                    if best.as_ref().is_none_or(|(_, shortest)| len < *shortest) {
                        best = Some((run, len));
                    }
                }
            }
        }
    }
    best.expect("the plane the arm bends in already joins the two points")
        .0
}

/// `hombros`: across the back, from one shoulder point to the other.
///
/// The shoulder points are the tops of the deltoid caps, and on this mesh
/// they stand level with the nape, so a plane through all three is level too
/// and only grazes the caps without ever cutting the skin between them. The
/// plane here holds the line from one shoulder point to the other and tilts
/// back and down from it a step at a time, and the first tilt whose section
/// joins both shoulder points behind the neck is kept: the highest line
/// across the back, under the nape, that still lies on the skin. Tilting
/// further only lowers the line across the shoulder blades and lengthens it.
fn shoulders(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    j: &Joints,
    right: [f64; 3],
    left: [f64; 3],
) -> Vec<Crossing> {
    for step in 1..=SLOPE_STEPS {
        let normal = [0.0, 1.0, -f64::from(step) * SLOPE_STEP];
        for section in Section::all(positions, tris, right, normal) {
            let (from, to) = (section.nearest(right), section.nearest(left));
            if dist(section.at(from), right) > REACH_M || dist(section.at(to), left) > REACH_M {
                continue;
            }
            for way in [Along::Forward, Along::Backward] {
                let path = section.arc(from, to, way);
                let spine = path
                    .iter()
                    .map(|&c| ring_point(positions, c))
                    .min_by(|a, b| a[0].abs().total_cmp(&b[0].abs()));
                if spine.is_some_and(|s| s[2] < j.neck[2]) {
                    return path;
                }
            }
        }
    }
    panic!("no tilt within {SLOPE_STEPS} steps joins the shoulder points behind the neck")
}
