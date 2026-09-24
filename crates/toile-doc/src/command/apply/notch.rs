use super::range::anchored;
use crate::{Applied, ChangeClass, Command, Doc, DocError, Identity, Notch, NotchKey, PieceKey};

/// Marks a contour, and the contour it answers to along with it.
///
/// A twin is born in the very edit that makes the notch it answers to, so the
/// two name each other from the moment either exists and neither can be left
/// pointing at a mark that is not there. Nothing moves until the whole plan is
/// known to fit: a pair half written would leave a document no inverse
/// describes.
pub(crate) fn add_notch(
    doc: &mut Doc,
    identity: Identity<Notch>,
    notch: Notch,
    mate: Option<(Identity<Notch>, Notch)>,
) -> Result<Applied, DocError> {
    anchored(doc, notch.at)?;
    let mut wanted = vec![identity];
    if let Some((twin, held)) = &mate {
        anchored(doc, held.at)?;
        wanted.push(*twin);
    }
    fits(doc, &wanted)?;
    let key = put(doc, identity, notch)?;
    let mut touched = vec![notch.at.piece];
    if let Some((twin, held)) = mate {
        touched.push(held.at.piece);
        let other = put(
            doc,
            twin,
            Notch {
                mate: Some(key),
                ..held
            },
        )?;
        doc.notches
            .get_mut(key)
            .ok_or_else(|| DocError::stale(key))?
            .mate = Some(other);
    }
    Ok(Applied {
        inverse: Command::RemoveNotch { notch: key },
        touched: ordered(touched),
        class: ChangeClass::Topology,
    })
}

/// Takes a notch off a contour, and the notch it answers to with it.
///
/// A pair arrives in one edit and goes in one, so no half of it is ever left
/// naming a mark that has gone. The inverse carries both back under their own
/// keys, naming each other again, so a removal and an undo leave the document
/// they started from.
pub(crate) fn remove_notch(doc: &mut Doc, notch: NotchKey) -> Result<Applied, DocError> {
    let held = *doc
        .notches
        .get(notch)
        .ok_or_else(|| DocError::stale(notch))?;
    // Followed once and no further: a pair written by this module names each
    // other, so the mark at the end of the link is the whole of the pair.
    let twin = held
        .mate
        .and_then(|key| Some((key, *doc.notches.get(key)?)));
    doc.notches.remove(notch)?;
    let mut touched = vec![held.at.piece];
    let mate = match twin {
        Some((key, other)) => {
            touched.push(other.at.piece);
            doc.notches.remove(key)?;
            Some((Identity::Restored(key), other))
        }
        None => None,
    };
    Ok(Applied {
        inverse: Command::AddNotch {
            identity: Identity::Restored(notch),
            notch: held,
            mate,
        },
        touched: ordered(touched),
        class: ChangeClass::Topology,
    })
}

/// The pieces an edit changed, in key order and each named once.
fn ordered(mut touched: Vec<PieceKey>) -> Vec<PieceKey> {
    touched.sort_unstable();
    touched.dedup();
    touched
}

/// Writes one notch under the key its identity asks for.
fn put(doc: &mut Doc, identity: Identity<Notch>, notch: Notch) -> Result<NotchKey, DocError> {
    match identity {
        Identity::New => Ok(doc.notches.insert(notch)),
        Identity::Restored(key) => {
            doc.notches.restore(key, notch)?;
            Ok(key)
        }
    }
}

/// Checks every key the edit claims, before anything moves.
///
/// A restored key has to name an open slot, and the two halves of a pair may
/// not ask for the same one: two notches landing on one key is the plan that
/// cannot fit however the arena is arranged.
fn fits(doc: &Doc, wanted: &[Identity<Notch>]) -> Result<(), DocError> {
    let mut taken: Vec<NotchKey> = Vec::new();
    for identity in wanted {
        let Identity::Restored(key) = *identity else {
            continue;
        };
        if taken.contains(&key) {
            return Err(DocError::occupied(key));
        }
        if !doc.notches.is_vacant(key) {
            return Err(match doc.notches.get(key) {
                Some(_) => DocError::occupied(key),
                None => DocError::stale(key),
            });
        }
        taken.push(key);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EdgeAnchor, PointKey, block};

    /// The block, its front, and two nodes of it to cut marks at.
    fn front() -> (Doc, PieceKey, [PointKey; 2]) {
        let doc = block::trouser_front();
        let piece = doc.piece_named(block::FRONT).expect("the block draws one");
        let node = |label: &str| {
            doc.shows_label(piece, label)
                .unwrap_or_else(|| panic!("the block names {label}"))
        };
        let nodes = [node("cintura_cf"), node("cadera_lat")];
        (doc, piece, nodes)
    }

    fn at(piece: PieceKey, node: PointKey, t: f64) -> EdgeAnchor {
        EdgeAnchor {
            piece,
            from: node,
            t,
        }
    }

    #[test]
    fn a_lone_notch_is_cut_and_undone_under_its_own_key() {
        let (mut doc, piece, nodes) = front();
        let place = at(piece, nodes[0], 0.5);
        let applied = Command::AddNotch {
            identity: Identity::New,
            notch: Notch::lone(place),
            mate: None,
        }
        .apply(&mut doc)
        .expect("the middle of a tract is a place on the contour");
        assert_eq!(doc.notches.len(), 1);
        assert_eq!(applied.touched, vec![piece]);
        assert_eq!(applied.class, ChangeClass::Topology);

        let Command::RemoveNotch { notch } = applied.inverse else {
            panic!("the inverse takes the mark off the contour");
        };
        let back = Command::RemoveNotch { notch }
            .apply(&mut doc)
            .expect("the key is live");
        assert_eq!(doc.notches.len(), 0);
        back.inverse
            .apply(&mut doc)
            .expect("the slot is open again");
        assert_eq!(doc.notches.get(notch).map(|held| held.at), Some(place));
    }

    /// A pair names itself in both directions the moment it exists, and goes in
    /// one edit, so no undo can leave half of it behind.
    #[test]
    fn a_pair_names_itself_both_ways_and_goes_together() {
        let (mut doc, piece, nodes) = front();
        let places = [at(piece, nodes[0], 0.25), at(piece, nodes[1], 0.75)];
        let applied = Command::AddNotch {
            identity: Identity::New,
            notch: Notch::lone(places[0]),
            mate: Some((Identity::New, Notch::lone(places[1]))),
        }
        .apply(&mut doc)
        .expect("both places sit on the contour");
        let keys: Vec<NotchKey> = doc.notches.keys().collect();
        assert_eq!(keys.len(), 2);
        let held = |key: NotchKey| *doc.notches.get(key).expect("the key is live");
        assert_eq!(held(keys[0]).mate, Some(keys[1]));
        assert_eq!(held(keys[1]).mate, Some(keys[0]));

        let back = applied.inverse.apply(&mut doc).expect("the key is live");
        assert_eq!(doc.notches.len(), 0, "neither half is left behind");
        back.inverse.apply(&mut doc).expect("both slots are open");
        assert_eq!(doc.notches.keys().collect::<Vec<_>>(), keys);
        let held = |key: NotchKey| *doc.notches.get(key).expect("the key is live");
        assert_eq!(
            held(keys[0]).mate,
            Some(keys[1]),
            "and they find each other"
        );
        assert_eq!(held(keys[1]).mate, Some(keys[0]));
    }

    /// The anchors answer to the one door every place on a contour comes
    /// through, and a pair whose second half is refused writes nothing at all.
    #[test]
    fn a_place_no_contour_answers_for_is_refused_for_either_half() {
        let (mut doc, piece, nodes) = front();
        let good = at(piece, nodes[0], 0.5);
        let past_the_end = at(piece, nodes[1], 1.5);
        let edits = [
            (Notch::lone(past_the_end), None),
            (
                Notch::lone(good),
                Some((Identity::New, Notch::lone(past_the_end))),
            ),
        ];
        for (notch, mate) in edits {
            let refused = Command::AddNotch {
                identity: Identity::New,
                notch,
                mate,
            }
            .apply(&mut doc);
            assert_eq!(refused, Err(DocError::AnchorFraction));
            assert_eq!(doc.notches.len(), 0, "nothing moved");
        }
    }

    /// Two halves asking for one slot is a plan no arena can arrange, and it is
    /// refused before either is written.
    #[test]
    fn a_pair_that_wants_one_key_twice_is_refused_before_anything_moves() {
        let (mut doc, piece, nodes) = front();
        let key = doc.notches.insert(Notch::lone(at(piece, nodes[0], 0.0)));
        doc.notches.remove(key).expect("the key is live");
        let refused = Command::AddNotch {
            identity: Identity::Restored(key),
            notch: Notch::lone(at(piece, nodes[0], 0.0)),
            mate: Some((
                Identity::Restored(key),
                Notch::lone(at(piece, nodes[1], 0.0)),
            )),
        }
        .apply(&mut doc);
        assert_eq!(refused, Err(DocError::occupied(key)));
        assert_eq!(doc.notches.len(), 0, "the freed slot is still free");
    }
}
