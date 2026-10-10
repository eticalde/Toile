use super::range::stretch;
use crate::{Applied, ChangeClass, Command, Doc, DocError, Hang, HangKey, Heading, Identity};

/// Hangs a stretch of contour from a station of the body.
///
/// Two hangs over one stretch are not refused, for the reason two elastics over
/// one stretch are not: the document holds keys and names and measures nothing,
/// so what a stretch covers is a question for whoever walks the resolved
/// contour. What two of them come to on the cloth is settled where the cloth is
/// — a vertex both of them name is pulled once toward each, in the order the
/// arena hands them over, which is why that order is a key order and not a
/// hash's.
pub(crate) fn add_hang(
    doc: &mut Doc,
    identity: Identity<Hang>,
    hang: Hang,
) -> Result<Applied, DocError> {
    hang.check()?;
    stretch(doc, hang.at)?;
    let touched = vec![hang.at.head.piece];
    let key = match identity {
        Identity::New => doc.hangs.insert(hang),
        Identity::Restored(key) => {
            doc.hangs.restore(key, hang)?;
            key
        }
    };
    Ok(Applied {
        inverse: Command::RemoveHang { hang: key },
        touched,
        class: ChangeClass::Shape,
    })
}

/// Lets the stretch off the ring it hung from.
///
/// The inverse carries it back under its own key, so a gesture that takes one
/// off and an undo that puts it back leave the document it started from.
pub(crate) fn remove_hang(doc: &mut Doc, hang: HangKey) -> Result<Applied, DocError> {
    let held = doc.hangs.remove(hang)?;
    let touched = vec![held.at.head.piece];
    Ok(Applied {
        inverse: Command::AddHang {
            identity: Identity::Restored(hang),
            hang: held,
        },
        touched,
        class: ChangeClass::Shape,
    })
}

/// Writes which of the body's rings a stretch hangs from.
pub(crate) fn set_station(doc: &mut Doc, hang: HangKey, to: String) -> Result<Applied, DocError> {
    // Before the arena is touched, so a station nothing on the body answers to
    // leaves the document exactly as it was.
    if !Hang::names_a_ring(&to) {
        return Err(DocError::HangStation(to));
    }
    let held = doc
        .hangs
        .get_mut(hang)
        .ok_or_else(|| DocError::stale(hang))?;
    let from = std::mem::replace(&mut held.station, to);
    let piece = held.at.head.piece;
    Ok(Applied {
        inverse: Command::SetHangStation { hang, to: from },
        touched: vec![piece],
        class: ChangeClass::Shape,
    })
}

/// Writes which way round the body a hung stretch faces, or takes the heading
/// off.
///
/// One edit for both, because the heading is one field and `None` is the value
/// it had before anybody declared one: a second command to clear it would be a
/// second inverse to keep in step, and the undo of a declaration is the value
/// that was there.
pub(crate) fn set_heading(
    doc: &mut Doc,
    hang: HangKey,
    to: Option<Heading>,
) -> Result<Applied, DocError> {
    // Before the arena is touched, so a turn outside the one lap there is
    // leaves the document exactly as it was — stamp included, which is what a
    // refused edit on a version 8 file turns on.
    if let Some(heading) = to {
        heading.check()?;
    }
    let held = doc
        .hangs
        .get_mut(hang)
        .ok_or_else(|| DocError::stale(hang))?;
    let from = std::mem::replace(&mut held.heading, to);
    let piece = held.at.head.piece;
    Ok(Applied {
        inverse: Command::SetHangHeading { hang, to: from },
        touched: vec![piece],
        class: ChangeClass::Shape,
    })
}
