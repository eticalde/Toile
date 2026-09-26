/// Cutting a dart out of a contour, and closing it back up.
mod cut;
/// Declaring a dart over a wedge the contour already has, and taking it off.
mod declare;

pub(crate) use cut::{add_dart, remove_dart};
pub(crate) use declare::{declare_dart, undeclare_dart};

use crate::{Arena, Dart, Doc, DocError, Identity, Key, PieceKey, Point, PointKey, SeamKey};

/// Issues the seam that closes the wedge, or gives its key back.
///
/// The seam is made here on the way back as well as on the way out, rather than
/// carried in the inverse, because a dart's record has nowhere to carry one and
/// no edit writes a dart's seam anything other than this.
fn sew(
    doc: &mut Doc,
    identity: Identity<Dart>,
    key: SeamKey,
    piece: PieceKey,
    apex: PointKey,
    legs: (PointKey, PointKey),
) -> Result<SeamKey, DocError> {
    let sewn = Dart::shutting_seam(piece, apex, legs);
    match identity {
        Identity::New => Ok(doc.seams.insert(sewn)),
        Identity::Restored(_) => {
            doc.seams.restore(key, sewn)?;
            Ok(key)
        }
    }
}

/// Checks that the wedge's three nodes are written to three places.
///
/// Two of them at one place is a wedge with nothing taken out of it: the
/// contour doubles back on a point and the side of the seam that reaches there
/// has no length for the solver to pair. The bindings are what is compared,
/// because they are what the document holds — resolving them would need a body,
/// and the document is the same document whichever body it is read against.
fn three_places(places: [&Point; 3]) -> Result<(), DocError> {
    let same = |a: &Point, b: &Point| a.x == b.x && a.y == b.y;
    let [first, apex, second] = places;
    if same(first, apex) || same(apex, second) || same(first, second) {
        return Err(DocError::FlatWedge);
    }
    Ok(())
}

/// One key an entry on its way back in asks for.
fn open<T>(arena: &Arena<T>, key: Key<T>) -> Result<(), DocError> {
    if arena.is_vacant(key) {
        return Ok(());
    }
    Err(match arena.get(key) {
        Some(_) => DocError::occupied(key),
        None => DocError::stale(key),
    })
}

/// Checks the dart's own key and its seam's, which are asked for only on the
/// way back — the only time either edit is told them.
fn slots(doc: &Doc, identity: Identity<Dart>, seam: SeamKey) -> Result<(), DocError> {
    if let Identity::Restored(key) = identity {
        open(&doc.darts, key)?;
        open(&doc.seams, seam)?;
    }
    Ok(())
}
