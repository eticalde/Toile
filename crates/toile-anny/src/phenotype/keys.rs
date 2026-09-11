/// The 26 phenotype variation keys, in Anny's own `PHENOTYPE_VARIATIONS`
/// order: race, then gender, age, muscle, weight, height, proportions,
/// cupsize, firmness.
///
/// A baked row's mask is a bitmask over this exact order — bit `i` set
/// means the row's filename mentioned `KEYS[i]` — so this array is the
/// single source of truth both the baker (turning a filename into a mask)
/// and the evaluator (turning a mask back into which interpolation
/// coefficients to multiply) must agree on.
pub const KEYS: [&str; 26] = [
    "african",
    "asian",
    "caucasian",
    "male",
    "female",
    "newborn",
    "baby",
    "child",
    "young",
    "old",
    "minmuscle",
    "averagemuscle",
    "maxmuscle",
    "minweight",
    "averageweight",
    "maxweight",
    "minheight",
    "maxheight",
    "idealproportions",
    "uncommonproportions",
    "mincup",
    "averagecup",
    "maxcup",
    "minfirmness",
    "averagefirmness",
    "maxfirmness",
];

/// Bit offsets of each feature's first key within [`KEYS`], and how many
/// keys it owns — the boundaries a mask is sliced at.
pub const RACE: (usize, usize) = (0, 3);
/// See [`RACE`].
pub const GENDER: (usize, usize) = (3, 2);
/// See [`RACE`].
pub const AGE: (usize, usize) = (5, 5);
/// See [`RACE`].
pub const MUSCLE: (usize, usize) = (10, 3);
/// See [`RACE`].
pub const WEIGHT: (usize, usize) = (13, 3);
/// See [`RACE`].
pub const HEIGHT: (usize, usize) = (16, 2);
/// See [`RACE`].
pub const PROPORTIONS: (usize, usize) = (18, 2);
/// See [`RACE`].
pub const CUPSIZE: (usize, usize) = (20, 3);
/// See [`RACE`].
pub const FIRMNESS: (usize, usize) = (23, 3);

/// Filename tokens that name no phenotype key but are known to appear in
/// the source data anyway, so the baker can skip them without treating them
/// as a surprise.
///
/// Today that is only `MakeHuman`'s `universal` prefix on the 36
/// sex/age/muscle/weight macrodetail targets, which carries no phenotype
/// meaning of its own (it just marks "not a named race/age combination").
const KNOWN_NON_KEY_TOKENS: [&str; 1] = ["universal"];

/// The 20 `measure-*` lever labels this tier bakes, alphabetically — the
/// fixed order a lever's id (its index here) refers to.
///
/// Each label has an `-incr` and a `-decr` target file (40 files total).
/// Not applied to the mesh yet (see `crate::phenotype::Phenotype`'s docs);
/// baked and exposed for the slice that reads them.
pub const LEVERS: [&str; 20] = [
    "ankle-circ",
    "bust-circ",
    "calf-circ",
    "frontchest-dist",
    "hips-circ",
    "knee-circ",
    "lowerarm-length",
    "lowerleg-height",
    "napetowaist-dist",
    "neck-circ",
    "neck-height",
    "shoulder-dist",
    "thigh-circ",
    "underbust-circ",
    "upperarm-circ",
    "upperarm-length",
    "upperleg-height",
    "waist-circ",
    "waisttohip-dist",
    "wrist-circ",
];

/// Turns a weighted target's filename stem (no directory, no
/// `.target.gz`) into its 26-bit phenotype mask, by splitting on `-` and
/// matching every token against [`KEYS`].
///
/// # Panics
/// If a token is neither a known key nor a [`KNOWN_NON_KEY_TOKENS`] word.
/// The bake must fail loudly on a filename it does not understand rather
/// than silently drop a factor from the row's weight.
pub fn mask_from_stem(stem: &str) -> u32 {
    let mut mask = 0u32;
    for token in stem.split('-') {
        if KNOWN_NON_KEY_TOKENS.contains(&token) {
            continue;
        }
        let Some(bit) = KEYS.iter().position(|&k| k == token) else {
            panic!(
                "unknown phenotype token `{token}` in `{stem}`: not one of \
                 the 26 keys and not a known non-key word"
            );
        };
        mask |= 1 << bit;
    }
    mask
}

/// Turns a lever target's filename stem into its lever id (an index into
/// [`LEVERS`]) and whether it is the `-incr` or the `-decr` half of the
/// pair.
///
/// # Panics
/// If the stem is not `measure-<label>-incr` or `measure-<label>-decr`, or
/// `<label>` is not one of the 20 known levers.
pub fn lever_from_stem(stem: &str) -> (u8, bool) {
    let rest = stem
        .strip_prefix("measure-")
        .unwrap_or_else(|| panic!("lever stem `{stem}` does not start with `measure-`"));
    let (label, incr) = if let Some(label) = rest.strip_suffix("-incr") {
        (label, true)
    } else if let Some(label) = rest.strip_suffix("-decr") {
        (label, false)
    } else {
        panic!("lever stem `{stem}` does not end with `-incr` or `-decr`");
    };
    let id = LEVERS
        .iter()
        .position(|&l| l == label)
        .unwrap_or_else(|| panic!("unknown lever label `{label}` in `{stem}`"));
    (id as u8, incr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_the_worked_examples_from_the_plan() {
        let bit = |name: &str| 1u32 << KEYS.iter().position(|&k| k == name).unwrap();
        assert_eq!(
            mask_from_stem("universal-female-young-maxmuscle-averageweight"),
            bit("female") | bit("young") | bit("maxmuscle") | bit("averageweight")
        );
        assert_eq!(
            mask_from_stem("african-female-young"),
            bit("african") | bit("female") | bit("young")
        );
    }

    #[test]
    #[should_panic(expected = "unknown phenotype token")]
    fn panics_loudly_on_a_surprise_token() {
        mask_from_stem("female-not-a-real-key");
    }

    #[test]
    fn parses_every_lever_pair() {
        for &label in &LEVERS {
            let incr_stem = format!("measure-{label}-incr");
            let decr_stem = format!("measure-{label}-decr");
            let (incr_id, incr) = lever_from_stem(&incr_stem);
            let (decr_id, decr) = lever_from_stem(&decr_stem);
            assert_eq!(incr_id, decr_id, "{label}: incr/decr must share an id");
            assert!(incr && !decr, "{label}: direction bit backwards");
        }
    }

    #[test]
    fn feature_ranges_tile_all_26_keys_with_no_gap() {
        let ranges = [
            RACE,
            GENDER,
            AGE,
            MUSCLE,
            WEIGHT,
            HEIGHT,
            PROPORTIONS,
            CUPSIZE,
            FIRMNESS,
        ];
        let mut next = 0;
        for (start, len) in ranges {
            assert_eq!(start, next, "a gap or overlap before this feature");
            next += len;
        }
        assert_eq!(next, KEYS.len());
    }
}
