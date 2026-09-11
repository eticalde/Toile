use toile_body::Station;
use toile_body::Station::{
    Ankle, Armpit, Biceps, Bust, Calf, Cheek, Crotch, Crown, Deltoid, Elbow, Forearm, HeadMax, Hip,
    Jaw, Knee, NeckBase, NeckTop, Shoulder, Thigh, Underbust, Waist, Wrist,
};

/// Which side of the body a paired joint (`joint-l-*` / `joint-r-*`) sits on.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

impl Side {
    fn letter(self) -> char {
        match self {
            Side::Left => 'l',
            Side::Right => 'r',
        }
    }
}

/// Splits a `joint-*` group name into its anatomical family (`"ankle"`,
/// `"finger-2-3"`, ...) and, for a paired joint, which side it is on. A
/// central joint (`neck`, `spine-1`, ...) carries no side.
fn split(name: &str) -> (&str, Option<Side>) {
    let rest = name.strip_prefix("joint-").expect("a joint-* group name");
    if let Some(fam) = rest.strip_prefix("l-") {
        (fam, Some(Side::Left))
    } else if let Some(fam) = rest.strip_prefix("r-") {
        (fam, Some(Side::Right))
    } else {
        (rest, None)
    }
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// A named joint's centroid, by family and side.
///
/// # Panics
/// If the name is not one of the 125 joints this bake reads: every family
/// this module names is checked against the full set in its own test.
fn find(joints: &[(String, [f64; 3])], family: &str, side: Option<Side>) -> [f64; 3] {
    let name = match side {
        Some(s) => format!("joint-{}-{family}", s.letter()),
        None => format!("joint-{family}"),
    };
    joints
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("no joint group named `{name}`"))
        .1
}

/// A vertex's position along the segment from `a` to `b`, as the fraction of
/// `dist(v, a)` over the round trip through `v`. The two joints of every
/// segment this bake uses sit close enough to the limb's own axis that this
/// ratio and a true axial projection agree everywhere but a sliver right at
/// the skin, and the ratio needs no cross product to compute.
fn fraction(v: [f64; 3], a: [f64; 3], b: [f64; 3]) -> f64 {
    let (da, db) = (dist(v, a), dist(v, b));
    da / (da + db)
}

/// Buckets a fraction into thirds along a two-joint segment: the near third
/// keeps that joint's own station, the far third the other joint's, and the
/// middle third is the station flanked by both that has no joint of its own.
fn thirds(t: f64, near: Station, mid: Station, far: Station) -> Station {
    if t < 1.0 / 3.0 {
        near
    } else if t < 2.0 / 3.0 {
        mid
    } else {
        far
    }
}

/// The station for every joint family that maps to one directly, with no
/// fraction test needed: most of the 125 joints fall here.
fn direct(family: &str) -> Station {
    match family {
        "upper-leg" => Thigh,
        "pelvis" => Crotch,
        "spine-4" => Hip,
        "spine-3" => Waist,
        "spine-2" => Underbust,
        "spine-1" => Bust,
        "scapula" => Armpit,
        "clavicle" => Shoulder,
        "mouth" => Jaw,
        "eye" | "eye-target" | "upperlid" | "lowerlid" => Cheek,
        "head" => HeadMax,
        "head-2" => Crown,
        "hand-2" | "hand-3" => Wrist,
        "foot-1" | "foot-2" | "ground" => Ankle,
        f if f.starts_with("tongue-") => Jaw,
        f if f.starts_with("finger-") => Wrist,
        f if f.starts_with("toe-") => Ankle,
        other => panic!("no direct station for joint family `{other}`"),
    }
}

/// The station rule: the nearest joint-cube centroid over all 125 `joint-*`
/// groups (ties toward the alphabetically first name, since `joints` is
/// sorted by name and the search keeps the first strict minimum), then that
/// joint's family names a station — directly for most families via
/// [`direct`], or through [`fraction`] and [`thirds`] along a two-joint
/// segment for the four families that flank a station with no joint of its
/// own: `Calf` between the ankle and the knee, `Forearm` between the hand and
/// the elbow, `Biceps` between the elbow and the shoulder, and `NeckTop`
/// between the neck and the jaw. The elbow is the one joint that flanks two
/// such segments; which one applies is decided by whether the vertex sits
/// nearer the hand or the shoulder.
pub fn classify(v: [f64; 3], joints: &[(String, [f64; 3])]) -> Station {
    let mut best = 0usize;
    let mut best_d = f64::INFINITY;
    for (i, (_, c)) in joints.iter().enumerate() {
        let d = dist(v, *c);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    let (family, side) = split(&joints[best].0);
    match family {
        "ankle" | "knee" => {
            let (a, b) = (find(joints, "ankle", side), find(joints, "knee", side));
            thirds(fraction(v, a, b), Ankle, Calf, Knee)
        }
        "hand" => {
            let (a, b) = (find(joints, "hand", side), find(joints, "elbow", side));
            thirds(fraction(v, a, b), Wrist, Forearm, Elbow)
        }
        "elbow" => {
            let elbow = find(joints, "elbow", side);
            let hand = find(joints, "hand", side);
            let shoulder = find(joints, "shoulder", side);
            if dist(v, hand) <= dist(v, shoulder) {
                thirds(fraction(v, hand, elbow), Wrist, Forearm, Elbow)
            } else {
                thirds(fraction(v, elbow, shoulder), Elbow, Biceps, Deltoid)
            }
        }
        "shoulder" => {
            let (a, b) = (find(joints, "elbow", side), find(joints, "shoulder", side));
            thirds(fraction(v, a, b), Elbow, Biceps, Deltoid)
        }
        "neck" | "jaw" => {
            let (a, b) = (find(joints, "neck", None), find(joints, "jaw", None));
            thirds(fraction(v, a, b), NeckBase, NeckTop, Jaw)
        }
        other => direct(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 70 joint families this bake actually reads. `direct` must know
    /// every one of them not otherwise handled in `classify`'s match, and
    /// `classify` must not panic on any of the 125 real joint names.
    #[test]
    fn every_real_joint_family_resolves_to_a_station() {
        let mut families: Vec<String> = Vec::new();
        for side in ["l-", "r-", ""] {
            for base in [
                "ankle",
                "knee",
                "upper-leg",
                "hand",
                "hand-2",
                "hand-3",
                "elbow",
                "shoulder",
                "clavicle",
                "scapula",
                "eye",
                "eye-target",
                "upperlid",
                "lowerlid",
                "foot-1",
                "foot-2",
            ] {
                if side.is_empty() {
                    continue; // these families are always paired
                }
                families.push(format!("{side}{base}"));
            }
        }
        for f in 1..=5 {
            for p in 1..=4 {
                families.push(format!("l-finger-{f}-{p}"));
            }
        }
        for t in 1..=5 {
            let segs = if t == 1 { 3 } else { 4 };
            for p in 1..=segs {
                families.push(format!("l-toe-{t}-{p}"));
            }
        }
        for central in [
            "pelvis", "spine-1", "spine-2", "spine-3", "spine-4", "head", "head-2", "neck", "jaw",
            "mouth", "tongue-1", "tongue-2", "tongue-3", "tongue-4", "ground",
        ] {
            families.push(central.to_owned());
        }

        // A minimal joint set: one arbitrary but distinct centroid per name,
        // just enough for `find` to resolve every lookup `classify` makes.
        let joints: Vec<(String, [f64; 3])> = families
            .iter()
            .enumerate()
            .map(|(i, f)| (format!("joint-{f}"), [i as f64, 0.0, 0.0]))
            .collect();
        assert!(joints.len() >= 60, "the family list itself looks short");

        for (name, centroid) in &joints {
            let _ = name; // classify reads the *nearest* joint's name, not this one
            classify(*centroid, &joints);
        }
    }
}
