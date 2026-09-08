use super::Ctx;
use crate::ring::{Ring, Shape, from_width};

// Neck and head sections. They ride a little ahead of the side-seam plane
// (`cz` > 0) — the neck leans forward from the shoulders — and the jaw's
// front depth is the chin, so no ring has to tilt.
const NECK: Shape = Shape {
    rho_front: 1.05,
    rho_back: 1.05,
    round_front: 0.10,
    round_back: 0.10,
};
const JAW: Shape = Shape {
    rho_front: 1.30,
    rho_back: 1.00,
    round_front: 0.35,
    round_back: 0.15,
};
const CHEEK: Shape = Shape {
    rho_front: 1.25,
    rho_back: 1.25,
    round_front: 0.25,
    round_back: 0.25,
};
const HEAD: Shape = Shape {
    rho_front: 1.20,
    rho_back: 1.30,
    round_front: 0.25,
    round_back: 0.25,
};

/// The neck-base ring, which the shoulders are sized against.
pub(crate) fn neck_base(c: &Ctx) -> Ring {
    c.girth_ring(c.m.neck, NECK, 0.010, c.lm.neck_base)
}

/// The five stations above the shoulders, bottom to top: neck base, neck
/// top, jaw, cheek, and the head at its widest.
pub(crate) fn stations(c: &Ctx) -> [Ring; 5] {
    let m = c.m;
    let lm = c.lm;
    let head_max = c.girth_ring(m.head, HEAD, 0.010, lm.head_max);
    [
        neck_base(c),
        c.girth_ring(0.95 * m.neck, NECK, 0.015, lm.neck_top),
        // The jaw is narrower than the skull by a fixed share; its long front
        // depth carries the chin.
        from_width(0.74 * head_max.a, JAW, 0.0, 0.015, lm.jaw),
        c.girth_ring(0.92 * m.head, CHEEK, 0.015, lm.cheek),
        head_max,
    ]
}
