use super::{sew, slots, three_places};
use crate::dart::wedge_seat;
use crate::{
    Applied, ChangeClass, Command, Dart, DartKey, Doc, DocError, DrawnWedge, Identity, PointKey,
    SeamKey,
};

/// Declares a dart over a wedge the contour already has.
///
/// The cut's twin for a pattern that was drafted with its wedge in it: the
/// three nodes are already drawn, under their own keys and with their own
/// bindings, so this brings only the seam that shuts them and the record that
/// says the three are a dart. Nothing is placed, nothing is inserted, and
/// nothing about the three nodes is written — which is the whole of the point,
/// because a node the draft states as a formula stops being one the moment
/// anything rewrites it.
///
/// A topology change, priced with the seam it brings rather than with a node it
/// does not: the drawing does not move and the contour keeps every node it had.
///
/// The keys the dart names are taken off the wedge and never off the `Dart` the
/// command carries — only the fold is read from there, exactly as in the cut.
pub(crate) fn declare_dart(
    doc: &mut Doc,
    identity: Identity<Dart>,
    dart: Dart,
    wedge: DrawnWedge,
) -> Result<Applied, DocError> {
    fits(doc, identity, &wedge, dart.seam)?;
    let (apex, legs) = (wedge.apex(), wedge.legs());
    let seam = sew(doc, identity, dart.seam, wedge.piece, apex, legs)?;
    let made = Dart {
        apex,
        legs,
        seam,
        fold: dart.fold,
    };
    let key = match identity {
        Identity::New => doc.darts.insert(made),
        Identity::Restored(key) => {
            doc.darts.restore(key, made)?;
            key
        }
    };
    Ok(Applied {
        inverse: Command::UndeclareDart { dart: key },
        touched: vec![wedge.piece],
        class: ChangeClass::Topology,
    })
}

/// Takes a declared dart off: the record and the thread, and not the wedge.
///
/// The three nodes stay where they are, with the bindings they were drawn with,
/// because the declaration never touched them and this is its inverse. What
/// is left is the contour the pattern was drafted with — a wedge somebody drew,
/// with nothing yet saying it is sewn shut.
///
/// The wedge is held against the contour before anything goes, so that the
/// declaration this hands back is one that can be applied again: a document
/// whose wedge no longer stands together is named here rather than on the redo.
pub(crate) fn undeclare_dart(doc: &mut Doc, dart: DartKey) -> Result<Applied, DocError> {
    let held = *doc.darts.get(dart).ok_or_else(|| DocError::stale(dart))?;
    let sewn = *doc
        .seams
        .get(held.seam)
        .ok_or_else(|| DocError::stale(held.seam))?;
    // The seam is where the dart says which contour its wedge is on: the record
    // names points, and a point on its own does not name the piece it is on.
    let piece = sewn.a.piece().ok_or(DocError::SplitSeamSide)?;
    let nodes = [held.legs.0, held.apex, held.legs.1];
    wedge_seat(
        doc.pieces
            .get(piece)
            .ok_or_else(|| DocError::stale(piece))?,
        nodes,
    )?;
    doc.darts.remove(dart)?;
    doc.seams.remove(held.seam)?;
    Ok(Applied {
        inverse: Command::DeclareDart {
            identity: Identity::Restored(dart),
            dart: held,
            wedge: DrawnWedge { piece, nodes },
        },
        touched: vec![piece],
        class: ChangeClass::Topology,
    })
}

/// Checks everything the declaration claims, before anything is written.
///
/// Where the cut asks for three vacant point keys, this asks the opposite: the
/// three have to be live nodes of the one contour, standing together in the
/// order a dart names them. The dart's own key and its seam's are asked for
/// only on the way back, which is the only time this edit is told them.
fn fits(
    doc: &Doc,
    identity: Identity<Dart>,
    wedge: &DrawnWedge,
    seam: SeamKey,
) -> Result<(), DocError> {
    let held = doc
        .pieces
        .get(wedge.piece)
        .ok_or_else(|| DocError::stale(wedge.piece))?;
    wedge_seat(held, wedge.nodes)?;
    let mut places = Vec::with_capacity(wedge.nodes.len());
    for key in wedge.nodes {
        places.push(doc.points.get(key).ok_or_else(|| DocError::stale(key))?);
    }
    three_places([places[0], places[1], places[2]])?;
    undarted(doc, wedge.nodes)?;
    slots(doc, identity, seam)
}

/// Refuses a wedge any dart of the document already names a node of.
///
/// Two darts over one node are two seams pulling one place and two mouths
/// crossing on one printed sheet, and the same three nodes declared twice is
/// that case at its plainest. The whole document is walked rather than the one
/// piece, because a point can be drawn by two pieces at once.
fn undarted(doc: &Doc, nodes: [PointKey; 3]) -> Result<(), DocError> {
    for (_, dart) in doc.darts.iter() {
        let named = [dart.legs.0, dart.apex, dart.legs.1];
        if nodes.iter().any(|node| named.contains(node)) {
            return Err(DocError::AlreadyDarted);
        }
    }
    Ok(())
}
