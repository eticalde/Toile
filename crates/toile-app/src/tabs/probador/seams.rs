use toile_engine::draft::{Draft, EdgeRange, Seam, SeamKind};

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

fn row(draft: &Draft, seam: &Seam) -> Row {
    let name = name(draft, &seam.a).unwrap_or_else(|| UNANCHORED.to_owned());
    let sides = side_cm(draft, &seam.a).zip(side_cm(draft, &seam.b));
    let Some((a, b)) = sides else {
        return Row {
            name,
            lengths: UNMEASURED.to_owned(),
            meets: None,
            complaint: None,
        };
    };
    let lengths = format!("{a:.1} / {b:.1}");
    let meets = meets(draft, seam, (a, b));
    let complaint = (meets == Some(false))
        .then(|| format!("{name}: los largos difieren {:.1} cm", (a - b).abs()));
    Row {
        name,
        lengths,
        meets,
        complaint,
    }
}

/// What one side measures along its piece's flattened contour, in centimetres.
///
/// The walk is the one the contour runs, head to tail, over the same fractions
/// the solver pairs the two sides on; a side that passes the closure is as long
/// as that walk and never as short as the gap it leaves.
fn side_cm(draft: &Draft, range: &EdgeRange) -> Option<f64> {
    let piece = range.piece()?;
    let head = draft.anchor_fraction(&range.head)?;
    let tail = draft.anchor_fraction(&range.tail)?;
    Some((tail - head).rem_euclid(1.0) * draft.perimeter_cm(piece))
}

/// The two nodes a side runs between, under the names its piece shows.
fn name(draft: &Draft, range: &EdgeRange) -> Option<String> {
    let doc = draft.doc();
    let piece = range.piece()?;
    let head = doc.label_of(piece, range.head.from)?;
    let tail = doc.label_of(piece, range.tail.from)?;
    Some(format!("{head} → {tail}"))
}

/// Whether the two lengths agree, by the seam's own rule.
///
/// `None` for a gathered seam: it is judged on a ratio of a long side to a
/// short one, and the document does not say which of the two is which. A mark
/// drawn from a guess about that is exactly the verdict this table must not
/// render.
fn meets(draft: &Draft, seam: &Seam, sides: (f64, f64)) -> Option<bool> {
    if matches!(seam.kind, SeamKind::Gathered { .. }) {
        return None;
    }
    let tolerance = seam
        .tolerance
        .or_else(|| draft.env().value(seam.tolerance_variable()))
        .unwrap_or(Seam::DEFAULT_TOLERANCE_CM);
    let excess = (sides.0 - sides.1).abs() - seam.expected_cm();
    Some(excess.abs() <= tolerance)
}

#[cfg(test)]
mod tests;
