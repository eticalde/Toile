use toile_engine::draft::{Draft, Seam, SeamKey};
use toile_engine::session::SeamFault;

use crate::seam::{self, Lengths};

/// What a row says when the draft cannot answer for one of the four anchors.
const UNMEASURED: &str = "— / —";
/// What a seam is called when its side A is not anchored on its own piece.
const UNANCHORED: &str = "costura sin anclar";

/// One seam of the document as the panel reports it.
///
/// Every field is read from the draft at call time. A seam carries no name of
/// its own, so it is called by the two nodes its first side runs between —
/// which is the only name the document can answer for.
pub struct Row {
    /// The two nodes side A runs between.
    pub name: String,
    /// What the two sides measure, as the row prints them.
    pub lengths: String,
    /// Whether they meet within the seam's own tolerance, when the document
    /// says how to judge them and both sides could be measured.
    pub meets: Option<bool>,
    /// What to say under the table when they do not.
    pub complaint: Option<String>,
}

/// Every seam of the document, measured along the contours it joins.
pub fn measured(draft: &Draft) -> Vec<Row> {
    draft
        .doc()
        .seams
        .iter()
        .map(|(_, seam)| row(draft, seam))
        .collect()
}

/// The seams the engine could not pair onto the cloth, said in words.
///
/// A seam that refuses is left out of the solver, so the garment draping is a
/// seam short of the one drawn on the table. That is the same kind of thing as
/// two sides that do not close, and it is said in the same place.
pub fn refused(draft: &Draft, faults: &[(SeamKey, SeamFault)]) -> Vec<String> {
    faults
        .iter()
        .filter_map(|(key, why)| {
            let seam = draft.doc().seams.get(*key)?;
            let named = seam::name(draft, &seam.a).unwrap_or_else(|| UNANCHORED.to_owned());
            Some(format!("{named}: {}", seam::why(why)))
        })
        .collect()
}

fn row(draft: &Draft, seam: &Seam) -> Row {
    let name = seam::name(draft, &seam.a).unwrap_or_else(|| UNANCHORED.to_owned());
    let Some(sides) = Lengths::of(draft, seam) else {
        return Row {
            name,
            lengths: UNMEASURED.to_owned(),
            meets: None,
            complaint: None,
        };
    };
    let lengths = format!("{:.1} / {:.1}", sides.a, sides.b);
    let meets = sides.meets(draft, seam);
    let complaint = (meets == Some(false))
        .then(|| format!("{name}: los largos difieren {:.1} cm", sides.delta().abs()));
    Row {
        name,
        lengths,
        meets,
        complaint,
    }
}

#[cfg(test)]
mod tests;
