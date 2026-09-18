use super::range::stretch;
use crate::{Applied, ChangeClass, Command, Doc, DocError, Elastic, ElasticKey, Identity};

/// Puts an elastic on a stretch of contour.
///
/// Two elastics over one stretch are not refused. The document holds keys and
/// fractions and measures nothing, so what a stretch covers is a question for
/// whoever walks the resolved contour — which is where a stretch that cannot be
/// turned into cloth is already faulted, and where two seams over one stretch
/// are already allowed. The file still says exactly one thing: entries carry
/// their ids and an arena is read in index order.
pub(crate) fn add_elastic(
    doc: &mut Doc,
    identity: Identity<Elastic>,
    elastic: Elastic,
) -> Result<Applied, DocError> {
    if elastic.at.piece().is_none() {
        return Err(DocError::SplitElastic);
    }
    elastic.check()?;
    stretch(doc, elastic.at)?;
    let key = match identity {
        Identity::New => doc.elastics.insert(elastic),
        Identity::Restored(key) => {
            doc.elastics.restore(key, elastic)?;
            key
        }
    };
    Ok(Applied {
        inverse: Command::RemoveElastic { elastic: key },
        touched: vec![elastic.at.head.piece],
        class: ChangeClass::Shape,
    })
}

/// Takes the elastic off the stretch it held.
///
/// The inverse carries it back under its own key, so a gesture that takes one
/// off and an undo that puts it back leave the document it started from.
pub(crate) fn remove_elastic(doc: &mut Doc, elastic: ElasticKey) -> Result<Applied, DocError> {
    let held = doc.elastics.remove(elastic)?;
    Ok(Applied {
        inverse: Command::AddElastic {
            identity: Identity::Restored(elastic),
            elastic: held,
        },
        touched: vec![held.at.head.piece],
        class: ChangeClass::Shape,
    })
}

/// Writes the ratio an elastic holds its stretch to.
pub(crate) fn set_ratio(doc: &mut Doc, elastic: ElasticKey, to: f64) -> Result<Applied, DocError> {
    let held = doc
        .elastics
        .get_mut(elastic)
        .ok_or_else(|| DocError::stale(elastic))?;
    // Checked as the elastic the edit would leave behind, so a slider dragged
    // past what an elastic carries is refused by the one rule that knows.
    Elastic { ratio: to, ..*held }.check()?;
    let from = std::mem::replace(&mut held.ratio, to);
    let piece = held.at.head.piece;
    Ok(Applied {
        inverse: Command::SetElasticRatio { elastic, to: from },
        touched: vec![piece],
        class: ChangeClass::Shape,
    })
}

/// Writes how hard an elastic holds its stretch.
pub(crate) fn set_strength(
    doc: &mut Doc,
    elastic: ElasticKey,
    to: f64,
) -> Result<Applied, DocError> {
    let held = doc
        .elastics
        .get_mut(elastic)
        .ok_or_else(|| DocError::stale(elastic))?;
    Elastic {
        strength: to,
        ..*held
    }
    .check()?;
    let from = std::mem::replace(&mut held.strength, to);
    let piece = held.at.head.piece;
    Ok(Applied {
        inverse: Command::SetElasticStrength { elastic, to: from },
        touched: vec![piece],
        class: ChangeClass::Shape,
    })
}
