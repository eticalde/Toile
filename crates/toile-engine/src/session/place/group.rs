use super::super::sew::Sewn;
use super::Worn;

#[cfg(test)]
mod tests;

/// One ring's worth of a product: the pieces that go on it, the seams that
/// chain them, and what a person declared about where it belongs.
///
/// A garment need not go round one ring. A shirt's body goes round the chest
/// and its collar round the neck, and those are two heights, two girths and two
/// surfaces — which is also why the one strip walk gave up on them: the panel
/// between two of them carries three seams, and a chain has room for two.
pub(super) struct Group {
    /// The pieces on this ring, ascending.
    pub(super) pieces: Vec<usize>,
    /// Which of the product's seams chain them, as indices into the sewn list.
    pub(super) seams: Vec<usize>,
    /// The station this ring was declared at; `None` for one nobody named.
    pub(super) worn: Option<Worn>,
}

/// Splits a sewn product into the rings it goes round.
///
/// `worn` is every station the document declared, the highest-hung first; what
/// is declared is read and only what is not is derived, by [`claims`]. A seam
/// between two different rings chains neither of them, which is exactly what
/// lets the panel it hangs off carry a third.
///
/// With nothing declared this is one group per component the seams chain, each
/// with no station: for a garment sewn into one chain that is the single strip
/// the walk has always been handed, bit for bit the placement of the tree
/// before this existed — what the drape goldens and the owner's jeans stand on.
/// A product all of whose pieces hang from one station is that group, named.
///
/// Two components are two groups, and [`super::around`] places an undeclared
/// one only where it holds the whole product — every piece anybody sewed or
/// hung, by [`super::of_the_product`]. A bag sewn to nothing else has no reason
/// to be anywhere; a panel nobody sewed or hung is no part of the product.
pub(super) fn rings(sewn: &[Sewn], pieces: usize, worn: &[Worn]) -> Vec<Group> {
    let claim = claims(sewn, pieces, worn);
    let mut of: Vec<usize> = (0..pieces).collect();
    for one in sewn {
        let [a, b] = one.sides;
        if a < pieces && b < pieces && claim[a] == claim[b] {
            join(&mut of, a, b);
        }
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut roots: Vec<usize> = Vec::new();
    for (piece, claimed) in claim.iter().enumerate() {
        let root = root(&of, piece);
        let at = if let Some(at) = roots.iter().position(|&r| r == root) {
            at
        } else {
            roots.push(root);
            groups.push(Group {
                pieces: Vec::new(),
                seams: Vec::new(),
                worn: claimed.map(|k| worn[k].clone()),
            });
            roots.len() - 1
        };
        groups[at].pieces.push(piece);
    }
    for (k, one) in sewn.iter().enumerate() {
        let [a, b] = one.sides;
        if a < pieces && b < pieces && root(&of, a) == root(&of, b) {
            let at = roots.iter().position(|&r| r == root(&of, a));
            if let Some(at) = at.and_then(|at| groups.get_mut(at)) {
                at.seams.push(k);
            }
        }
    }
    // Largest first, so the ring the body of a garment is on is the one a
    // client asking for "the ring" is handed and the one the elastic's band
    // is read against. Never the order the document stores its pieces in:
    // ties are broken by the seams a group carries and then by the station
    // a person typed, both of which are the garment and not the file.
    groups.sort_by(|a, b| {
        b.pieces
            .len()
            .cmp(&a.pieces.len())
            .then(b.seams.len().cmp(&a.seams.len()))
            .then(named(a).cmp(named(b)))
    });
    groups
}

/// The station a group declares, for ordering; the empty name for one that
/// declares none, which sorts it before every real station.
fn named(group: &Group) -> &str {
    group.worn.as_ref().map_or("", |worn| worn.station.as_str())
}

/// Which declared station each piece belongs to, as an index into `worn`.
///
/// A piece a hang names takes that hang's station outright. A piece nobody
/// named takes the station of the nearest piece that was, counted in seams,
/// because the nearest declaration is the one a person would point at: an
/// undeclared panel sewn to a declared one is part of that garment, not a
/// second one. Ties go to the higher-hung station, which is the rule and the
/// reason [`super::super::Session::declared`] already picks the topmost line.
///
/// Walked in rings of equal distance rather than piece by piece, so what
/// decides a tie is how many seams away a declaration is and never where the
/// file keeps the piece.
fn claims(sewn: &[Sewn], pieces: usize, worn: &[Worn]) -> Vec<Option<usize>> {
    let mut claim = vec![None; pieces];
    for (k, one) in worn.iter().enumerate() {
        for &piece in &one.on {
            if claim.get(piece).is_some_and(Option::is_none) {
                claim[piece] = Some(k);
            }
        }
    }
    let mut front: Vec<usize> = (0..pieces).filter(|&p| claim[p].is_some()).collect();
    while !front.is_empty() {
        let mut next: Vec<(usize, usize)> = Vec::new();
        for &piece in &front {
            let Some(k) = claim[piece] else { continue };
            for one in sewn {
                let Some(side) = one.sides.iter().position(|&p| p == piece) else {
                    continue;
                };
                let other = one.sides[1 - side];
                if claim.get(other).is_some_and(Option::is_none) {
                    next.push((other, k));
                }
            }
        }
        // Written after the whole ring is read, so two declarations an equal
        // number of seams away are settled by which hangs higher and not by
        // which piece the loop reached first.
        next.sort_unstable();
        front.clear();
        for (piece, k) in next {
            if claim[piece].is_none() {
                claim[piece] = Some(k);
                front.push(piece);
            }
        }
    }
    claim
}

/// The representative of a piece's group.
fn root(of: &[usize], mut piece: usize) -> usize {
    while of[piece] != piece {
        piece = of[piece];
    }
    piece
}

/// Puts two pieces in one group, lowest representative first so the answer does
/// not depend on which of the two was asked about.
fn join(of: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (root(of, a), root(of, b));
    if ra < rb {
        of[rb] = ra;
    } else {
        of[ra] = rb;
    }
}
