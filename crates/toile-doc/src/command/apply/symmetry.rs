use super::range::stretch;
use crate::{Applied, ChangeClass, Command, Doc, DocError, Identity, Symmetry, SymmetryKey};

/// Draws a piece against an axis, so that the cloth is the drawing and its
/// mirror.
///
/// A topology change and not a shape one: the cloth gains every point of the
/// mirrored half, so the mesh built from the half cannot be warm-started at
/// the whole and has to be built again.
pub(crate) fn add_symmetry(
    doc: &mut Doc,
    identity: Identity<Symmetry>,
    symmetry: Symmetry,
) -> Result<Applied, DocError> {
    symmetry.check()?;
    stretch(doc, symmetry.axis)?;
    let piece = symmetry.piece().ok_or(DocError::SplitSymmetry)?;
    if doc.symmetry_of(piece).is_some() {
        return Err(DocError::AlreadySymmetric);
    }
    let key = match identity {
        Identity::New => doc.symmetries.insert(symmetry),
        Identity::Restored(key) => {
            doc.symmetries.restore(key, symmetry)?;
            key
        }
    };
    Ok(Applied {
        inverse: Command::RemoveSymmetry { symmetry: key },
        touched: vec![piece],
        class: ChangeClass::Topology,
    })
}

/// Drops the axis, so that the cloth is the drawing again.
///
/// The inverse carries the whole axis back under its own key, so unfolding a
/// piece and undoing it leave the document that was there before.
pub(crate) fn remove_symmetry(doc: &mut Doc, symmetry: SymmetryKey) -> Result<Applied, DocError> {
    let held = doc.symmetries.remove(symmetry)?;
    Ok(Applied {
        inverse: Command::AddSymmetry {
            identity: Identity::Restored(symmetry),
            symmetry: held,
        },
        touched: held.piece().into_iter().collect(),
        class: ChangeClass::Topology,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EdgeAnchor, EdgeRange, PieceKey, PointKey, SymmetryKind, block};

    /// The block, its front, and the waist as a stretch to fold it on.
    fn front() -> (Doc, PieceKey, EdgeRange) {
        let doc = block::trouser_front();
        let piece = doc.piece_named(block::FRONT).expect("the block draws one");
        let node = |label: &str| {
            doc.shows_label(piece, label)
                .unwrap_or_else(|| panic!("the block names {label}"))
        };
        let axis = EdgeRange::between(piece, node("cintura_cf"), node("cintura_lat"));
        (doc, piece, axis)
    }

    #[test]
    fn a_fold_is_written_and_undone_under_its_own_key() {
        let (mut doc, piece, axis) = front();
        let applied = Command::AddSymmetry {
            identity: Identity::New,
            symmetry: Symmetry::fold(axis),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the front");
        assert_eq!(applied.touched, vec![piece]);
        assert_eq!(applied.class, ChangeClass::Topology);
        assert_eq!(doc.fold_of(piece), Some(axis));

        let back = applied.inverse.apply(&mut doc).expect("the key is live");
        assert_eq!(doc.fold_of(piece), None);
        back.inverse
            .apply(&mut doc)
            .expect("the slot is open again");
        assert_eq!(doc.fold_of(piece), Some(axis));
        assert_eq!(doc.symmetries.len(), 1, "and there is still only one");
    }

    #[test]
    fn a_piece_carries_one_axis_and_no_second() {
        let (mut doc, piece, axis) = front();
        let fold = |axis| Command::AddSymmetry {
            identity: Identity::New,
            symmetry: Symmetry::fold(axis),
        };
        fold(axis).apply(&mut doc).expect("the first one fits");
        let other = EdgeRange {
            head: axis.tail,
            tail: axis.head,
        };
        assert_eq!(
            fold(other).apply(&mut doc),
            Err(DocError::AlreadySymmetric),
            "a second reflection is a quarter piece, not this"
        );
        assert_eq!(
            doc.symmetry_of(piece).map(|(_, held)| held.axis),
            Some(axis)
        );
    }

    /// The axis comes through the one door every stretch of contour comes
    /// through, so a fraction off the end of its tract, a node no contour runs
    /// through and a mirror nothing can cut are all refused before anything is
    /// written.
    #[test]
    fn an_axis_the_contour_cannot_answer_for_writes_nothing() {
        let (mut doc, piece, axis) = front();
        let past_the_end = EdgeRange {
            head: EdgeAnchor {
                t: 1.5,
                ..axis.head
            },
            ..axis
        };
        let elsewhere = EdgeRange {
            tail: EdgeAnchor {
                piece: PieceKey::new(9, 0),
                ..axis.tail
            },
            ..axis
        };
        let stray = EdgeRange::between(piece, axis.head.from, PointKey::new(99, 0));
        for (symmetry, why) in [
            (Symmetry::fold(past_the_end), DocError::AnchorFraction),
            (Symmetry::fold(elsewhere), DocError::SplitSymmetry),
            (Symmetry::fold(stray), DocError::stale(PointKey::new(99, 0))),
            (
                Symmetry {
                    axis,
                    kind: SymmetryKind::Mirror,
                },
                DocError::NotYetImplemented,
            ),
        ] {
            let refused = Command::AddSymmetry {
                identity: Identity::New,
                symmetry,
            }
            .apply(&mut doc);
            assert_eq!(refused, Err(why));
            assert!(doc.symmetries.is_empty(), "nothing moved");
        }
    }
}
