/// The math behind [`Phenotype`]: `torch.linspace`-compatible anchors and
/// the interpolation coefficients over them.
mod interp;
/// The 26 phenotype keys and the 20 lever labels, and the filename parsers
/// that turn a baked target's stem into a mask or a lever id.
mod keys;

use interp::{coefficients, linspace, race_weights};
pub use keys::{KEYS, LEVERS, lever_from_stem, mask_from_stem};

/// Years-to-parameter control points for [`age_param_from_years`]: a young
/// adult, a mid-life adult, and an old adult, each paired with the Anny
/// `age` parameter that reproduces `MakeHuman`'s own age-by-height curve at
/// that year.
///
/// Not derived from anything in this bake — read off `MakeHuman`'s own
/// age-to-height table, the one part of Anny's population priors that is
/// free of the year/parameter unit mismatch its other priors carry.
const AGE_TABLE: [(f64, f64); 3] = [(18.0, 0.77), (64.0, 0.83), (110.0, 1.0)];

/// The six phenotype inputs this slice exposes, in Anny's own units: `0.0`
/// to `1.0` for every field except `age`.
///
/// Three of Anny's nine phenotype dimensions are not exposed here — `race`,
/// `cupsize` and `firmness` — because no control in this slice's UI would
/// mean anything for them yet; the evaluator treats them as Anny's own
/// neutral defaults (an equal third for race, "average" for the other two)
/// regardless of what a caller sets. A future slice can add them without
/// changing this struct's shape, the same way the per-part `measure-*`
/// levers are already baked but not yet applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Phenotype {
    /// `0.0` is Anny's `male` anchor, `1.0` its `female` anchor; values in
    /// between blend the two.
    pub gender: f64,
    /// Anny's own age parameter, not years — see [`age_param_from_years`]
    /// for the conversion a caller thinking in years should apply first.
    /// Evaluating a mesh floors this below the `young` anchor regardless
    /// (see [`phens26`]'s doc): this tier bakes only the adult age rows.
    pub age: f64,
    /// `0.0` is `minmuscle`, `1.0` is `maxmuscle`.
    pub muscle: f64,
    /// `0.0` is `minweight`, `1.0` is `maxweight` — the build control.
    pub weight: f64,
    /// `0.0` is `minheight`, `1.0` is `maxheight`. This is Anny's own
    /// parameter, not centimetres; measure the generated mesh's
    /// bounding-box height for the number a person expects (see
    /// `toile_engine::body::stature_cm`).
    pub height: f64,
    /// `0.0` is `idealproportions`, `1.0` is `uncommonproportions`.
    pub proportions: f64,
}

impl Default for Phenotype {
    /// Every field at Anny's own neutral 0.5 except `age`, which defaults
    /// to a 25-year-old adult (`age_param_from_years(25.0)`, about 0.779):
    /// Anny's own all-0.5 body is a 1.63 m twelve-year-old, and this tier's
    /// bake does not even ship the child rows that value would need.
    fn default() -> Self {
        Self {
            gender: 0.5,
            age: age_param_from_years(25.0),
            muscle: 0.5,
            weight: 0.5,
            height: 0.5,
            proportions: 0.5,
        }
    }
}

/// Converts an age in years to Anny's own `[0, 1]`-ish age parameter, along
/// the piecewise-linear table in [`AGE_TABLE`].
///
/// Years below 18 clamp to the 18-year point: this v1 tier is adults-only
/// (see [`Phenotype::age`]'s doc), so there is no meaningful parameter to
/// interpolate toward for a child. Years above 110 likewise clamp to the
/// oldest point.
pub fn age_param_from_years(years: f64) -> f64 {
    if years <= AGE_TABLE[0].0 {
        return AGE_TABLE[0].1;
    }
    if years >= AGE_TABLE[2].0 {
        return AGE_TABLE[2].1;
    }
    for pair in AGE_TABLE.windows(2) {
        let (y0, p0) = pair[0];
        let (y1, p1) = pair[1];
        if years <= y1 {
            let t = (years - y0) / (y1 - y0);
            return p0 + t * (p1 - p0);
        }
    }
    unreachable!("the loop above covers every year between the table's ends")
}

/// The `young` anchor along the age axis: the floor [`phens26`] clamps a
/// raw age parameter to, since this tier bakes no row below it. Exposed to
/// the mesh evaluator's own clamp comment and to tests that need to name
/// the floor without hardcoding the anchor's value.
pub(crate) fn adult_age_floor() -> f64 {
    linspace(-1.0 / 3.0, 1.0, 5)[3]
}

/// Writes one feature's interpolation coefficients into their slice of the
/// 26-key array.
fn write_feature(out: &mut [f64; 26], start: usize, anchors: &[f64], v: f64) {
    let coeffs = coefficients(v, anchors);
    out[start..start + coeffs.len()].copy_from_slice(&coeffs);
}

/// Expands a [`Phenotype`] into the 26 per-key coefficients a baked row's
/// mask is weighed against — one entry per [`KEYS`] position, in that same
/// order.
///
/// The `age` field is floored at the `young` anchor before interpolating:
/// this tier's bake ships only the `young`/`old` adult rows, so a
/// parameter below `young` would silently lose every age-dependent delta
/// (no row exists to supply it) rather than gracefully degrade toward a
/// child that is not there. The floor is the same anchor Anny itself would
/// interpolate from, so it never alters an already-adult input.
pub(crate) fn phens26(p: &Phenotype) -> [f64; 26] {
    let mut out = [0.0f64; 26];

    // Race, cup size and firmness are not exposed on `Phenotype` (see its
    // doc): Anny's own neutral defaults are used unconditionally.
    let race = race_weights(0.5, 0.5, 0.5);
    out[keys::RACE.0..keys::RACE.0 + keys::RACE.1].copy_from_slice(&race);

    write_feature(&mut out, keys::GENDER.0, &linspace(0.0, 1.0, 2), p.gender);

    let age_anchors = linspace(-1.0 / 3.0, 1.0, 5);
    let age = p.age.max(adult_age_floor());
    write_feature(&mut out, keys::AGE.0, &age_anchors, age);

    write_feature(&mut out, keys::MUSCLE.0, &linspace(0.0, 1.0, 3), p.muscle);
    write_feature(&mut out, keys::WEIGHT.0, &linspace(0.0, 1.0, 3), p.weight);
    write_feature(&mut out, keys::HEIGHT.0, &linspace(0.0, 1.0, 2), p.height);
    write_feature(
        &mut out,
        keys::PROPORTIONS.0,
        &linspace(0.0, 1.0, 2),
        p.proportions,
    );
    write_feature(&mut out, keys::CUPSIZE.0, &linspace(0.0, 1.0, 3), 0.5);
    write_feature(&mut out, keys::FIRMNESS.0, &linspace(0.0, 1.0, 3), 0.5);

    out
}

/// A baked row's weight for one phenotype: the product, over every key the
/// row's filename mentioned (its `mask`, bit `i` for `KEYS[i]`), of that
/// key's coefficient in `phens` — every other key contributes a factor of
/// one. Mirrors Anny's `prod(phens * mask + (1 - mask))` exactly:
/// multiplying by the literal `1.0` for an unmasked key cannot change the
/// running product's bits, so walking only the masked keys in [`KEYS`]
/// order gives the identical result.
pub(crate) fn row_weight(mask: u32, phens: &[f64; 26]) -> f64 {
    let mut w = 1.0;
    for (key, &phen) in phens.iter().enumerate() {
        if mask & (1 << key) != 0 {
            w *= phen;
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "hits the table's own literal control points exactly"
    )]
    fn age_years_hit_the_table_exactly() {
        assert_eq!(age_param_from_years(18.0), 0.77);
        assert_eq!(age_param_from_years(64.0), 0.83);
        assert_eq!(age_param_from_years(110.0), 1.0);
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "clamped output lands on the table's own literal endpoints"
    )]
    fn age_years_clamp_below_eighteen_and_above_a_hundred_ten() {
        assert_eq!(age_param_from_years(0.0), 0.77);
        assert_eq!(age_param_from_years(200.0), 1.0);
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "both sides call the same conversion, so they must match bit for bit"
    )]
    fn twenty_five_years_is_the_documented_default() {
        let years25 = age_param_from_years(25.0);
        assert!(
            (years25 - 0.779).abs() < 1.0e-3,
            "expected about 0.779, got {years25}"
        );
        assert_eq!(Phenotype::default().age, years25);
    }

    #[test]
    fn the_default_phenotype_is_already_an_adult() {
        let age_anchors = linspace(-1.0 / 3.0, 1.0, 5);
        assert!(Phenotype::default().age >= age_anchors[3]);
    }

    #[test]
    fn phens26_sums_to_one_within_every_interpolated_feature() {
        let p = Phenotype {
            gender: 0.3,
            ..Phenotype::default()
        };
        let phens = phens26(&p);
        for (start, len) in [
            keys::GENDER,
            keys::AGE,
            keys::MUSCLE,
            keys::WEIGHT,
            keys::HEIGHT,
            keys::PROPORTIONS,
            keys::CUPSIZE,
            keys::FIRMNESS,
        ] {
            let sum: f64 = phens[start..start + len].iter().sum();
            assert!((sum - 1.0).abs() < 1.0e-12, "feature at {start} sums {sum}");
        }
        let race_sum: f64 = phens[keys::RACE.0..keys::RACE.0 + keys::RACE.1]
            .iter()
            .sum();
        assert!((race_sum - 1.0).abs() < 1.0e-12);
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "exact literal inputs multiply to an exact literal result"
    )]
    fn row_weight_multiplies_only_the_masked_keys() {
        let mut phens = [0.0f64; 26];
        phens[3] = 0.25; // male
        phens[8] = 0.5; // young
        let mask = (1 << 3) | (1 << 8);
        assert_eq!(row_weight(mask, &phens), 0.125);
        assert_eq!(row_weight(0, &phens), 1.0);
    }
}
