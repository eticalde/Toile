// Determinism contract for every value that reaches the mesh: only
// `+ - * / sqrt` and f64 decimal literals — never powf, sin, cos, tan or hypot.
// The body golden is asserted bit-identical on macOS ARM and Linux x86 at once,
// and the existing goldens already live inside that regime; a transcendental
// here would drift the last bits between the two machines and break the gate.
// This is why the superellipse is limited to n in {2, 4} (2/n in {1.0, 0.5},
// i.e. sqrt only) and ring directions come from a literal-seeded rotor below.

/// One superelliptic cross-section: centre `(cx, cz)`, half-width `a` on x,
/// half-depth `b` on z, exponent `n`, at height `y`. Only n in {2, 4} are used.
pub(crate) struct Ring {
    pub cx: f64,
    pub cz: f64,
    pub y: f64,
    pub a: f64,
    pub b: f64,
    pub n: f64,
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

/// Warps one unit direction onto the superellipse. n = 2 is a plain ellipse;
/// n = 4 raises each component to 2/n = 0.5, i.e. `sign * sqrt(|c|)`.
pub(crate) fn point(r: &Ring, dir: (f64, f64)) -> [f32; 3] {
    // `< 3.0` splits the only two exponents in play (2 and 4) without an
    // equality compare on a float; interpolation keeps n exactly on 2 or 4.
    let f = |c: f64| {
        if r.n < 3.0 {
            c
        } else {
            c.signum() * c.abs().sqrt()
        }
    };
    [
        (r.cx + r.a * f(dir.0)) as f32,
        r.y as f32,
        (r.cz + r.b * f(dir.1)) as f32,
    ]
}

/// Turns a measured girth into the `(a, b)` axes for aspect ρ = b/a and
/// exponent n. A superellipse has no closed-form perimeter, so the tape is
/// honoured numerically: sum the chords of a unit ring, then scale so its
/// perimeter equals the girth (cm→m by /100).
pub(crate) fn axes_for_girth(c_cm: f64, rho: f64, n: f64, dirs: &[(f64, f64)]) -> (f64, f64) {
    let unit = Ring {
        cx: 0.0,
        cz: 0.0,
        y: 0.0,
        a: 1.0,
        b: rho,
        n,
    };
    let mut p = 0.0;
    for w in dirs.windows(2) {
        let a = point(&unit, w[0]);
        let b = point(&unit, w[1]);
        let (dx, dz) = (f64::from(b[0] - a[0]), f64::from(b[2] - a[2]));
        p += (dx * dx + dz * dz).sqrt();
    }
    let a = (c_cm / 100.0) / p;
    (a, a * rho)
}
