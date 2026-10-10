use super::*;

/// One seam, as the grouping reads it: which two pieces it joins. Where across
/// them it runs is the walk's question and no part of this one.
fn seam(sides: [usize; 2]) -> Sewn {
    Sewn {
        sides,
        at: [0.0, 0.0],
        ends: [[0.0, 0.0], [0.0, 0.0]],
        a: Vec::new(),
        b: Vec::new(),
        dart: false,
    }
}

/// A station hung from the pieces named, at the pattern ordinate `at`.
fn worn(station: &str, at: f64, on: &[usize]) -> Worn {
    Worn {
        station: station.to_owned(),
        at,
        on: on.to_vec(),
    }
}

/// The pieces of each group, in the order the grouping hands them back.
fn pieces(groups: &[Group]) -> Vec<Vec<usize>> {
    groups.iter().map(|one| one.pieces.clone()).collect()
}

/// The station each group declares, in the same order.
fn stations(groups: &[Group]) -> Vec<&str> {
    groups.iter().map(named).collect()
}

/// A product nobody hung is one ring holding everything, which is the whole of
/// the placement the tree ran before a station could be declared.
///
/// The reduction the owner's jeans and every drape golden stand on: one group,
/// every seam in it, no station, so the walk is handed exactly what it was
/// handed before.
#[test]
fn a_product_nobody_hung_is_one_ring_with_every_seam_on_it() {
    let sewn = [seam([0, 1]), seam([1, 2])];
    let groups = rings(&sewn, 3, &[]);
    assert_eq!(pieces(&groups), [vec![0, 1, 2]]);
    assert_eq!(groups[0].seams, [0, 1]);
    assert_eq!(stations(&groups), [""]);
}

/// And a product every piece of which hangs from one station is that same ring
/// with a name on it.
#[test]
fn one_station_over_the_whole_product_is_still_one_ring() {
    let sewn = [seam([0, 1]), seam([1, 2])];
    let groups = rings(&sewn, 3, &[worn("cintura", 0.0, &[0, 1, 2])]);
    assert_eq!(pieces(&groups), [vec![0, 1, 2]]);
    assert_eq!(stations(&groups), ["cintura"]);
}

/// A piece nobody named joins the ring of the piece it is sewn to.
///
/// What keeps every skirt in the tree where it was: a hang written on one panel
/// of two places both of them, because the second is that garment and not
/// another one. Split them and neither has a chain left to walk.
#[test]
fn an_undeclared_piece_joins_the_ring_it_is_sewn_to() {
    let sewn = [seam([0, 1])];
    let groups = rings(&sewn, 2, &[worn("cintura", 0.0, &[0])]);
    assert_eq!(pieces(&groups), [vec![0, 1]]);
    assert_eq!(stations(&groups), ["cintura"]);
}

/// Two stations are two rings, and the seams between them chain neither.
///
/// The defect this exists to end, in the smallest shape that carries it: piece
/// 1 is the back of a shirt and carries three seams — two side seams and the
/// collar's. Two of them are its own ring's and the third is the collar's, so
/// the chain it is walked in has room for it.
#[test]
fn a_collar_on_its_own_station_takes_its_seams_off_the_body_of_the_shirt() {
    let sewn = [
        seam([0, 1]),
        seam([1, 2]),
        seam([0, 3]),
        seam([1, 3]),
        seam([2, 3]),
    ];
    let declared = [
        worn("cabeza", 0.0, &[3]),
        worn("pecho_alto", -0.01, &[0, 1, 2]),
    ];
    let groups = rings(&sewn, 4, &declared);
    assert_eq!(pieces(&groups), [vec![0, 1, 2], vec![3]]);
    assert_eq!(stations(&groups), ["pecho_alto", "cabeza"]);
    // The body of the shirt keeps its two side seams and nothing else, so the
    // panel the collar is sewn to is walked across two of them.
    assert_eq!(groups[0].seams, [0, 1]);
    assert!(groups[1].seams.is_empty(), "a collar strip has no chain");
}

/// The body of the garment comes first, so a client asking for "the ring" is
/// handed the ring most of the cloth is on.
///
/// By the pieces a ring carries and never by the order the document stores
/// them: the collar is written first here and still comes second.
#[test]
fn the_ring_the_body_of_the_garment_is_on_comes_first() {
    let sewn = [seam([0, 1]), seam([1, 2]), seam([0, 2])];
    let declared = [worn("cabeza", 0.0, &[0]), worn("cintura", -0.5, &[1, 2])];
    let groups = rings(&sewn, 3, &declared);
    assert_eq!(pieces(&groups), [vec![1, 2], vec![0]]);
    assert_eq!(stations(&groups), ["cintura", "cabeza"]);
}

/// A piece two stations away from both takes the nearer one, counted in seams.
///
/// Not the higher: the nearest declaration is the one a person would point at,
/// and piece 2 here is one seam from the hip and two from the chest. The chest
/// hangs higher and still does not claim it.
#[test]
fn an_undeclared_piece_takes_the_nearest_station_and_not_the_highest() {
    let sewn = [seam([0, 1]), seam([1, 2]), seam([2, 3])];
    let declared = [worn("pecho_alto", 0.0, &[0]), worn("cadera", -0.4, &[3])];
    let groups = rings(&sewn, 4, &declared);
    // Piece 1 is one seam from the chest, piece 2 one seam from the hip, so the
    // seam between them is the one that chains neither ring. The two rings
    // carry the same cloth, so which comes first falls to the station a person
    // typed — the one tie-break left, and never the order of the file.
    assert_eq!(pieces(&groups), [vec![2, 3], vec![0, 1]]);
    assert_eq!(stations(&groups), ["cadera", "pecho_alto"]);
    assert_eq!(groups[0].seams, [2], "the hip keeps its own seam");
    assert_eq!(groups[1].seams, [0], "and the chest keeps its own");
}

/// A piece nothing is sewn to is a ring of its own, which is what leaves the
/// lone panel every golden hashes let go flat.
#[test]
fn a_piece_nothing_is_sewn_to_stands_alone() {
    let sewn = [seam([0, 1])];
    let groups = rings(&sewn, 3, &[]);
    assert_eq!(pieces(&groups), [vec![0, 1], vec![2]]);
    assert!(groups[1].seams.is_empty());
}
