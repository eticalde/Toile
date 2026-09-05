#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use std::collections::BTreeMap;

use toile_doc::formula::Formula;

/// How many sources the sweep draws.
const DRAWS: u32 = 80_000;

/// How deep a drawn expression may nest.
const DEPTH: u32 = 3;

/// The seed. Fixed, so a failure names a source anyone can reproduce.
const SEED: u64 = 0x2545_F491_4F6C_DD1D;

fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// A source drawn from the grammar itself, at most `depth` deep.
///
/// Drawing from the grammar rather than from the alphabet is what reaches the
/// conditional: a random string of tokens practically never spells one, and
/// the conditional is the branch of the rewrite with the most to get wrong.
fn expression(state: &mut u64, depth: u32) -> String {
    let shape = xorshift(state) % if depth == 0 { 3 } else { 10 };
    match shape {
        0 => "a".to_owned(),
        1 => "b".to_owned(),
        2 => format!("{}", xorshift(state) % 40),
        3 => format!("({})", expression(state, depth - 1)),
        4 => format!("-{}", expression(state, depth - 1)),
        5 => format!(
            "{} {} {}",
            expression(state, depth - 1),
            ["+", "-", "*", "/", "^"][(xorshift(state) % 5) as usize],
            expression(state, depth - 1)
        ),
        6 => format!(
            "min({}, {})",
            expression(state, depth - 1),
            expression(state, depth - 1)
        ),
        7 => format!("abs({})", expression(state, depth - 1)),
        _ => format!(
            "{} {} {} ? {} : {}",
            expression(state, depth - 1),
            ["<", ">", "<=", ">=", "==", "!="][(xorshift(state) % 6) as usize],
            expression(state, depth - 1),
            expression(state, depth - 1),
            expression(state, depth - 1)
        ),
    }
}

/// The rewrite walks its token slice by raw index and tokenizes on an
/// `expect`, so what it must never do is panic on a source the parser took.
/// And what it writes has to be the same expression plus the delta: the
/// adjustment absorbs the move, the rest of the expression is untouched.
#[test]
fn a_nudge_of_anything_the_parser_took_still_parses_and_still_adds_up() {
    let env: BTreeMap<String, f64> = [("a", 3.0), ("b", 7.0)]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect();
    let mut state = SEED;
    let (mut taken, mut conditional, mut valued) = (0u32, 0u32, 0u32);
    for _ in 0..DRAWS {
        let src = expression(&mut state, DEPTH);
        let Ok(formula) = Formula::parse(&src) else {
            continue;
        };
        taken += 1;
        conditional += u32::from(src.contains('?'));
        for delta in [0.6f64, -0.6, 0.0, 12.5, -12.5] {
            let out = formula.nudged_source(delta, 0.1);
            let Ok(moved) = Formula::parse(&out) else {
                panic!("{src:?} nudged by {delta} wrote {out:?}, which no longer parses");
            };
            let (Ok(here), Ok(there)) = (formula.eval(&env), moved.eval(&env)) else {
                continue;
            };
            valued += 1;
            assert!(
                (there - here - delta).abs() < 1.0e-9 * here.abs().max(1.0),
                "{src:?} nudged by {delta} wrote {out:?}: {here} became {there}"
            );
        }
    }
    assert!(
        taken > 10_000 && conditional > 1_000 && valued > 50_000,
        "the sweep has to reach the grammar: {taken} taken, {conditional} conditional, {valued} valued"
    );
}
