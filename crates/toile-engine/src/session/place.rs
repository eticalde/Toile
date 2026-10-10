use group::Group;
use heading::Turned;
use ring::ring;

use super::sew::Sewn;
use crate::body::{Collider, bake};
use crate::couture::{Layout, Pin, ShapePipeline};

mod group;
mod heading;
mod ring;
mod stands;
mod strip;

#[cfg(test)]
mod tests;

/// How far the surface is opened at a time where the body needs the room, in
/// metres.
///
/// The field's own cell. A step finer asks the field a question it cannot
/// answer differently, and one coarser hands the garment room it never needed.
const OPENING: f64 = bake::CELL;

/// How tall one band of the clearance profile is, in metres.
///
/// The field's own cell again, and for the same reason: two bands closer
/// together than that would be opened by a reading the field cannot tell apart.
/// It is also what makes the profile's own slope readable — a band of height
/// per band of radius is the steepest wall the surface is allowed to lean at.
const BAND_HEIGHT: f64 = bake::CELL;

/// How far round a product's elastics go, and the line of cloth they run at.
///
/// What holds a garment on is what says where it hangs from, so a product
/// that carries elastics is placed by them: see [`around`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Band {
    /// The cloth the elastics cover, across every piece, in metres.
    pub(super) girth: f64,
    /// The pattern ordinate they run at, in metres.
    pub(super) at: f64,
}

/// Where a document says one of its rings belongs on the body.
///
/// The station by name and not the ring behind it: whether the body on the
/// stand carries that ring is this placement's question, and a reader that
/// resolved it early would leave nothing able to say what had been asked for.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Worn {
    /// The catalogue girth naming the ring, as the document wrote it.
    pub(super) station: String,
    /// The pattern ordinate of the cloth hung from it, in metres.
    pub(super) at: f64,
    /// The pieces a hang from this station is written on, ascending.
    ///
    /// Which ring a piece goes round, and so the one fact that lets a product
    /// be placed on more than one: see [`group::rings`].
    pub(super) on: Vec<usize>,
}

/// How much longer than its own cloth a ring may be let go before a person is
/// told, as a ratio.
///
/// Measured, and set where the two sets of readings part. Everything the tree
/// places today sits under 1.161×, which is the narrow blouse declared at the
/// chest — 86 % of its ring covered — and every garment drafted to the body
/// wearing it reads within a hundredth of 1.0. A garment hung from a ring it
/// does not belong on reads 2.09×: a 40 cm panel declared at the waist of this
/// body comes off an 83.6 cm hoop, 48 % coverage, its two ends half the ring
/// apart. A third of the ring bare is the line between them, and it is the
/// same reading the skirt's band failed at before a hoop per ordinate existed.
const WIDE_OPEN: f64 = 1.5;

/// A declared station the body's own reading would not have chosen.
///
/// Both names, because either alone is half the sentence: the declared ring is
/// where the garment went, and the inferred one is the ring the cloth's own
/// size points at. A person who wrote one of them is owed the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Elsewhere {
    /// The catalogue girth the document declares.
    pub declared: String,
    /// The catalogue girth a garment this size would have been put on.
    pub inferred: String,
}

/// A ring let go much longer than the cloth that goes round it.
///
/// Not a refusal and not a fault of the placement: the opening walk is doing
/// its work, letting the surface out until it clears the person underneath.
/// It is the symptom of one mistake a person makes and cannot see — the ring
/// declared is not the ring the garment belongs on — and a reading is the only
/// form that mistake can reach anybody in.
#[derive(Debug, Clone, PartialEq)]
pub struct Loose {
    /// How far round the ring the garment was let go on goes, in metres.
    pub hoop: f64,
    /// The cloth the strip carries across, in metres.
    pub cloth: f64,
    /// The station this ring was declared at; `None` for one nobody named.
    pub station: Option<String>,
}

impl Loose {
    /// How much longer than its own cloth the ring is, as a ratio.
    pub fn opened(&self) -> f64 {
        self.hoop / self.cloth
    }
}

/// A release: the surfaces the product was let go on, what the body had to say
/// about where the document asked for it, and what was left over.
pub(super) struct Placement {
    /// One per ring the product goes round, the body of the garment first.
    pub(super) rings: Vec<Layout>,
    /// The two rings, when the declaration and the inference name different
    /// ones.
    pub(super) elsewhere: Option<Elsewhere>,
    /// The widest of the rings that came off much longer than its own cloth.
    pub(super) loose: Option<Loose>,
    /// How many pieces of the product no ring could place.
    ///
    /// Counted and not hidden, which is the decision: a product that cannot be
    /// put on the body is let go flat as it always was, and the number of
    /// pieces that happened to reaches a person instead of nothing. What counts
    /// as a piece of the product is [`of_the_product`], the same reading
    /// [`whole`] takes, so a component left off the body is a component this
    /// number names.
    pub(super) adrift: usize,
}

/// Where a sewn product is let go on the body: a ring per declared station.
///
/// One ring does not do the whole of a garment. A shirt's body goes round the
/// chest and its collar round the neck, which is two heights and two girths —
/// and it is also why the one strip walk gave up on them, since the panel
/// between two rings carries three seams and a chain has room for two. How a
/// product is split is [`group::rings`]; what one ring is, is [`ring`].
///
/// `rings` comes back empty when no group's seams chain its pieces into a
/// strip, and a group nobody declared is placed only where it is the whole
/// product. Flat over the person is what the tree has always done with cloth
/// it cannot place, and `adrift` says how much of it there was.
pub(super) fn around(
    sewn: &[Sewn],
    pipes: &[&ShapePipeline],
    collider: &Collider,
    band: Option<Band>,
    (worn, pins): (&[Worn], &[Pin]),
) -> Placement {
    let groups = group::rings(sewn, pipes.len(), worn);
    let turned = heading::spread(sewn, pipes.len(), pins);
    // Every piece this product is made of, so that a group nobody declared can
    // be asked whether it is the whole of it: see [`of_the_product`].
    let product = product_size(sewn, worn, pipes.len());
    let (mut rings, mut elsewhere, mut adrift) = (Vec::new(), None, 0);
    let mut loose: Option<Loose> = None;
    for (k, group) in groups.iter().enumerate() {
        // Only the first, because a band is read across every piece of the
        // product and so describes the garment rather than one of its rings.
        // The garment's body is the ring it is on; an elastic sewn to a collar
        // alone is a reading nothing here can take apart.
        let held = band.filter(|_| k == 0);
        let placed = (group.worn.is_some() || whole(group, sewn, worn, product))
            .then(|| ring(group, sewn, pipes, collider, (held, faced(group, &turned))))
            .flatten();
        match placed {
            Some(rolled) => {
                let read = widened(&rolled, group);
                if read.as_ref().map_or(0.0, Loose::opened)
                    > loose.as_ref().map_or(0.0, Loose::opened)
                {
                    loose = read;
                }
                // The body of the garment's reading and not every ring's: this
                // says where the product went, and a collar strip's own girth
                // pointing at the chest is not that. Set once, on the first
                // ring that places.
                if rings.is_empty() {
                    elsewhere = rolled.elsewhere;
                }
                rings.push(rolled.layout);
            }
            None => {
                adrift += group
                    .pieces
                    .iter()
                    .filter(|&&p| of_the_product(sewn, worn, p))
                    .count();
            }
        }
    }
    Placement {
        rings,
        elsewhere,
        loose,
        adrift,
    }
}

/// Which heading one ring is turned by: the declaration nearest any of its
/// pieces, counted in seams, ties to the one that hangs higher.
///
/// The same ladder the station climbs, which is the rule. A ring with a
/// heading written on one of its own pieces is nought seams from it and takes
/// it, so a shirt whose body and whose collar each declare one gets both; a
/// ring with none follows the garment it is sewn to rather than opening at the
/// middle of its own first panel.
fn faced(group: &Group, turned: &[Option<Turned>]) -> Option<Pin> {
    group
        .pieces
        .iter()
        .filter_map(|&piece| turned.get(piece).copied().flatten())
        .min_by_key(|one| (one.seams, one.rank))
        .map(|one| one.pin)
}

/// Whether a group holds every piece of the product, which is to say whether
/// it is the product.
///
/// What a group nobody declared is placed on, and the guard the one strip walk
/// carried: its walk had to place the whole product or the product went flat. A
/// garment sewn into one chain is that group, so its release is the arithmetic
/// the tree ran before a station could be declared — what the drape goldens and
/// the owner's jeans stand on. A component that declares nothing and reaches
/// nothing that does has no ring of a person to be on, and a ring its own girth
/// happened to match is an ankle.
///
/// Counted over every piece of the product and not the sewn ones alone, which
/// is the correction: a declared piece sewn to nothing was invisible to a count
/// of stitches, so a two-piece bag sewn only to itself beside a declared panel
/// held "every sewn piece" and went round a 19 cm hoop at a calf with `adrift`
/// saying nothing was left over. The cost is the shape read the other way: a
/// tube beside a waistband hung and not yet sewn on goes flat, and is counted.
fn whole(group: &Group, sewn: &[Sewn], worn: &[Worn], product: usize) -> bool {
    group
        .pieces
        .iter()
        .filter(|&&p| of_the_product(sewn, worn, p))
        .count()
        == product
}

/// Whether a piece is part of the product at all: anything sewn to it, or a
/// station it is hung from.
///
/// The two ways a drawing on the table says it is part of a garment. Neither
/// alone is the whole of it: a panel hung from the waist with nothing yet sewn
/// to it is the beginning of a garment and is placed as one, and a piece sewn
/// into a chain is part of whatever that chain is whoever declared it.
///
/// A lone panel nobody sewed and nobody hung is in neither, and that is the
/// decision: it has no partner to be placed against, it is not waiting to be,
/// and every golden in the tree hashes one let go flat beside a garment that
/// was placed. Counting it would have taken that garment off the body.
fn of_the_product(sewn: &[Sewn], worn: &[Worn], piece: usize) -> bool {
    is_sewn(sewn, piece) || worn.iter().any(|one| one.on.contains(&piece))
}

/// How many of the pieces on the stand the product is made of.
fn product_size(sewn: &[Sewn], worn: &[Worn], pieces: usize) -> usize {
    (0..pieces)
        .filter(|&p| of_the_product(sewn, worn, p))
        .count()
}

/// The ring's reading, when it came off much longer than its own cloth.
fn widened(rolled: &ring::Rolled, group: &Group) -> Option<Loose> {
    (rolled.girth > 0.0 && rolled.hoop / rolled.girth > WIDE_OPEN).then(|| Loose {
        hoop: rolled.hoop,
        cloth: rolled.girth,
        station: group.worn.as_ref().map(|worn| worn.station.clone()),
    })
}

/// Whether anything at all is sewn to a piece.
fn is_sewn(sewn: &[Sewn], piece: usize) -> bool {
    sewn.iter().any(|one| one.sides.contains(&piece))
}
