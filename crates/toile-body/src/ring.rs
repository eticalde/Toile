// Determinism contract for every value that reaches the mesh: only
// `+ - * / sqrt` and f64 decimal literals — never powf, sin, cos, tan or hypot.
// The body golden is asserted bit-identical on macOS ARM and Linux x86 at once,
// and the existing goldens already live inside that regime; a transcendental
// here would drift the last bits between the two machines and break the gate.
// This is why roundness is a point-wise blend between an ellipse (n = 2) and a
// rounded square (n = 4, i.e. sqrt only) rather than a fractional exponent,
// and ring directions come from a literal-seeded rotor below.

/// Aspect and roundness of one section, per half: `rho` is half-depth over
/// half-width, `round` 0 is an ellipse and 1 a rounded square.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shape {
    pub rho_front: f64,
    pub rho_back: f64,
    pub round_front: f64,
    pub round_back: f64,
}

/// One cross-section at height `y`, centred on `(cx, cz)`.
///
/// The side columns sit at exactly `cx ± a`; the front (+z) and back (−z)
/// halves carry their own depth and roundness, which is how a bust leads the
/// waist and a seat trails it while the ring centre stays on the side-seam
/// plane.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Ring {
    pub cx: f64,
    pub cz: f64,
    pub y: f64,
    pub a: f64,
    pub depth_front: f64,
    pub depth_back: f64,
    pub round_front: f64,
    pub round_back: f64,
}

/// The `seg` unit directions around a ring as `(cos, sin)` pairs, turned out by
/// a rotor so no runtime trig is called; the seam column is duplicated at the
/// end for a clean wrap, exactly as the sphere generator does.
pub(crate) fn unit_dirs(seg: u32) -> Vec<(f64, f64)> {
    // Rotor step for seg = 32 (2π/32 = 11.25°), as f64 literals; decimal→f64
    // parsing is deterministic. A different seg needs its own cos/sin(2π/seg)
    // pasted here, and moves the body golden on purpose.
    const C: f64 = 0.980_785_280_403_230_4;
    const S: f64 = 0.195_090_322_016_128_25;
    let (mut x, mut y) = (1.0f64, 0.0f64);
    let mut out = Vec::with_capacity(seg as usize + 1);
    for _ in 0..seg {
        out.push((x, y));
        let (nx, ny) = (x * C - y * S, x * S + y * C);
        // Renormalize with sqrt so rounding never accumulates around the ring.
        let inv = 1.0 / (nx * nx + ny * ny).sqrt();
        x = nx * inv;
        y = ny * inv;
    }
    out.push(out[0]);
    out
}

/// Warps one unit direction onto the ring. Each component is blended between
/// itself (an ellipse) and `sign * sqrt(|c|)` (the rounded square) by the
/// half's roundness; at `v = 0` both halves land on exactly `cx ± a`.
pub(crate) fn point(r: &Ring, (u, v): (f64, f64)) -> [f64; 3] {
    let sq = |c: f64| c.signum() * c.abs().sqrt();
    let (b, t) = if v < 0.0 {
        (r.depth_back, r.round_back)
    } else {
        (r.depth_front, r.round_front)
    };
    let fx = u + t * (sq(u) - u);
    let fz = v + t * (sq(v) - v);
    [r.cx + r.a * fx, r.y, r.cz + b * fz]
}

/// The chord-sum perimeter of a ring over the given directions.
pub(crate) fn perimeter(r: &Ring, dirs: &[(f64, f64)]) -> f64 {
    let mut p = 0.0;
    for w in dirs.windows(2) {
        let a = point(r, w[0]);
        let b = point(r, w[1]);
        let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
        p += (dx * dx + dz * dz).sqrt();
    }
    p
}

/// A ring whose perimeter is a measured girth. There is no closed form for
/// a blended superellipse, so the tape is honoured numerically: sum the chords
/// of a unit ring, then scale so its perimeter equals the girth (cm→m by /100).
pub(crate) fn from_girth(
    girth_cm: f64,
    s: Shape,
    cx: f64,
    cz: f64,
    y: f64,
    dirs: &[(f64, f64)],
) -> Ring {
    let unit = from_width(1.0, s, 0.0, 0.0, 0.0);
    let a = (girth_cm / 100.0) / perimeter(&unit, dirs);
    from_width(a, s, cx, cz, y)
}

/// A ring sized by its half-width, for sections that are not a tape (the
/// crotch, the shoulders, the jaw).
pub(crate) fn from_width(a: f64, s: Shape, cx: f64, cz: f64, y: f64) -> Ring {
    Ring {
        cx,
        cz,
        y,
        a,
        depth_front: a * s.rho_front,
        depth_back: a * s.rho_back,
        round_front: s.round_front,
        round_back: s.round_back,
    }
}
