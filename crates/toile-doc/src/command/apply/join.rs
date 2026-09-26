use super::range::stretch;
use crate::{
    Applied, ChangeClass, Command, Doc, DocError, EdgeRange, Identity, PieceKey, Seam, SeamKey,
};

/// Sews two stretches of contour together.
///
/// Every anchor is checked before the seam lands, by the one rule every stretch
/// answers to.
pub(crate) fn add_seam(
    doc: &mut Doc,
    identity: Identity<Seam>,
    seam: Seam,
) -> Result<Applied, DocError> {
    side(doc, seam.a)?;
    side(doc, seam.b)?;
    let touched = touched(&seam);
    let key = match identity {
        Identity::New => doc.seams.insert(seam),
        Identity::Restored(key) => {
            doc.seams.restore(key, seam)?;
            key
        }
    };
    Ok(Applied {
        inverse: Command::RemoveSeam { seam: key },
        touched,
        class: ChangeClass::Topology,
    })
}

/// Unpicks a seam.
///
/// The inverse carries the seam back under its own key, so undoing the unpick
/// leaves anything that named the seam naming it still.
pub(crate) fn remove_seam(doc: &mut Doc, seam: SeamKey) -> Result<Applied, DocError> {
    // A dart is a wedge and the seam that shuts it, so a seam taken out from
    // under one leaves a dart pointing at nothing — which `json::check`
    // refuses to load. The editor has to refuse what the loader refuses, or a
    // product can be saved in a state it can never be opened in. Taking the
    // dart out takes its seam with it, which is the way through.
    //
    // This is also the only door out of a sewn seam, so it is where turning one
    // round is refused too: the document has no edit that rewrites a seam, and
    // every hand that changes one takes it out and puts it back under its key.
    if doc.dart_closed_by(seam).is_some() {
        return Err(DocError::SeamClosesADart);
    }
    let held = doc.seams.remove(seam)?;
    let touched = touched(&held);
    Ok(Applied {
        inverse: Command::AddSeam {
            identity: Identity::Restored(seam),
            seam: held,
        },
        touched,
        class: ChangeClass::Topology,
    })
}

/// One side of a seam, checked end to end.
fn side(doc: &Doc, range: EdgeRange) -> Result<(), DocError> {
    if range.piece().is_none() {
        return Err(DocError::SplitSeamSide);
    }
    stretch(doc, range)
}

/// The pieces the seam sews, in key order.
fn touched(seam: &Seam) -> Vec<PieceKey> {
    let mut pieces = vec![seam.a.head.piece, seam.b.head.piece];
    pieces.sort_unstable();
    pieces.dedup();
    pieces
}
