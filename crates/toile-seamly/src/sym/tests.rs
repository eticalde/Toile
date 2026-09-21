use super::*;

fn n(value: f64) -> Lin {
    Lin::num(Num::of_f64(value))
}

fn a() -> Lin {
    Lin::name("a")
}

fn b() -> Lin {
    Lin::name("b")
}

#[test]
fn an_offset_added_and_taken_away_leaves_nothing_behind() {
    let there = n(36.0).add(&a().scale(Num::of_f64(0.5))).add(&n(4.0));
    let back = there.sub(&n(36.0));
    assert_eq!(back.source(), "4 + a / 2");
    assert!(there.sub(&there).is_zero());
    assert!(back.is_linear());
}

#[test]
fn decimals_add_up_to_the_decimal_a_person_would_write() {
    let sum = n(0.79375).add(&n(29.9829)).add(&b()).sub(&a());
    assert_eq!(sum.source(), "30.77665 + b - a");
    assert_eq!(n(3.0).mul(&n(1.2)).source(), "3.6");
    assert_eq!(n(-2.5).source(), "-2.5");
    assert_eq!(Lin::zero().source(), "0");
}

#[test]
fn a_weight_that_is_the_reciprocal_of_a_whole_number_reads_as_a_division() {
    let third = a().div(&n(3.0)).expect("three is not zero");
    assert_eq!(third.source(), "a / 3");
    let two_thirds = third.scale(Num::of_f64(2.0));
    assert_eq!(two_thirds.source(), "2 * a / 3");
    assert_eq!(a().scale(Num::of_f64(-0.25)).source(), "-a / 4");
}

#[test]
fn the_root_of_a_square_is_an_absolute_value_and_a_ratio_of_likes_a_number() {
    let dx = a().sub(&n(1.0));
    let distance = dx.square().add(&Lin::zero()).sqrt().expect("a square");
    assert_eq!(distance.source(), "abs(a - 1)");
    let half = distance.scale(Num::of_f64(0.5));
    assert_eq!(half.div(&distance), Some(n(0.5)));
    let both = dx
        .square()
        .add(&b().square())
        .sqrt()
        .expect("a sum of squares");
    assert_eq!(both.source(), "sqrt((a - 1)^2 + b^2)");
}

#[test]
fn a_product_is_the_same_atom_whichever_way_round_it_is_written() {
    let ab = a().add(&n(1.0)).mul(&b());
    let ba = b().mul(&a().add(&n(1.0)));
    assert!(ab.sub(&ba).is_zero());
    let quotient = ab.div(&b().add(&n(2.0))).expect("not a zero divisor");
    assert_eq!(quotient.source(), "(1 + a) * b / (2 + b)");
    assert!(!quotient.is_linear());
}

#[test]
fn a_divisor_of_exactly_zero_is_refused() {
    assert_eq!(a().div(&Lin::zero()), None);
    assert_eq!(n(-4.0).sqrt(), None);
}

#[test]
fn a_float_enters_the_constant_without_disturbing_the_exact_part() {
    let handle = n(30.77665).add(&Lin::num(Num::Float(std::f64::consts::FRAC_1_SQRT_2)));
    assert_eq!(handle.source(), "30.77665 + 0.7071067811865476");
    let scaled = handle.scale(Num::of_f64(2.0));
    assert_eq!(scaled.source(), "61.5533 + 1.4142135623730951");
}

#[test]
fn a_frozen_quantity_marks_everything_computed_from_it() {
    let frozen = n(0.04).frozen_by(Frozen::SplineExcess(39));
    let length = a().add(&frozen);
    let far = length
        .mul(&b())
        .div(&a().square().sqrt().expect("a root"))
        .expect("a divisor");
    assert!(far.frozen().contains(&Frozen::SplineExcess(39)));
    assert!(a().frozen().is_empty());
    assert!(frozen.scale(Num::of_f64(0.0)).frozen().is_empty());
}
