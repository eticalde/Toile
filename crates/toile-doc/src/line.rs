/// An internal line on its way into the document.
mod edit;
/// What an internal line is for.
mod kind;
/// One place an internal line runs through.
mod vertex;

pub use edit::{LineEdit, SpanEdit};
pub use kind::LineKind;
use serde::{Deserialize, Serialize};
pub use vertex::{LineVertex, VertexEdit};

use crate::piece::samples_fit;
use crate::{DocError, EdgeAnchor, PieceKey, PointKey, Segment};

/// A run of places on one piece that the pattern draws and does not cut.
///
/// A fold, a topstitch, a pocket mouth, a buttonhole, a mark: what a garment
/// cannot be sewn without and a contour has nowhere to put. It is a run rather
/// than a closed shape because that is what a pattern draws — a line has two
/// ends — and it names one piece because it is cut out with that piece and
/// nothing else.
///
/// The run is a head and the spans that carry it on, rather than a list of
/// places beside a list of spans. Two collections can fall out of step with
/// each other; this one cannot, and a span that reaches nowhere cannot be
/// written down.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalLine {
    /// The piece it is drawn on.
    pub piece: PieceKey,
    /// What the line is for.
    pub kind: LineKind,
    /// The name its author gave it, when it carries one.
    ///
    /// Nothing is looked up by it — a line is addressed by its key — so two
    /// lines may share a name, and seven belt loops that differ only by their
    /// number stay seven lines a person can tell apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Where the run starts.
    pub head: LineVertex,
    /// Every further place, with the span that reaches it.
    pub spans: Vec<LineSpan>,
}

/// One step of an internal line: the span, and the place it reaches.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LineSpan {
    /// Where the step ends.
    pub to: LineVertex,
    /// What runs to it from the place before.
    pub segment: Segment,
    /// How many samples that span is flattened into.
    pub samples: u16,
}

/// One span reduced to what the rules ask of it.
///
/// A line reaching the rules from a file carries a `Segment` and one reaching
/// them from a command carries a `SegmentEdit`; both answer the same three
/// questions, so both are read as this and the rules are written once.
struct Step {
    at: Option<EdgeAnchor>,
    bends: bool,
    samples: u16,
}

impl InternalLine {
    /// The places the line runs through, in order.
    pub fn vertices(&self) -> impl Iterator<Item = LineVertex> {
        std::iter::once(self.head).chain(self.spans.iter().map(|span| span.to))
    }

    /// Every handle the line's curved spans hang on, in span order.
    pub fn handles(&self) -> impl Iterator<Item = PointKey> {
        self.spans
            .iter()
            .filter_map(|span| span.segment.handles())
            .flat_map(|(out, into)| [out, into])
    }

    /// Whether the line names `point`: as a place of its own, or as a handle.
    pub fn cites(&self, point: PointKey) -> bool {
        self.vertices().any(|vertex| vertex.point() == Some(point))
            || self.handles().any(|handle| handle == point)
    }

    /// Whether the line is one a piece can be drawn with.
    pub(crate) fn check(&self) -> Result<(), DocError> {
        checked(self.piece, self.head.anchor(), self.steps())?;
        self.lone_handles()
    }

    fn steps(&self) -> impl Iterator<Item = Step> {
        self.spans.iter().map(|span| Step {
            at: span.to.anchor(),
            bends: span.segment.bends(),
            samples: span.samples,
        })
    }

    /// Refuses a line that hangs two of its spans on one handle.
    ///
    /// Taking the line away removes every handle it hangs on, so a handle
    /// named twice would be removed twice over and the second removal would
    /// fail with the first one already done. No command can build one — an
    /// arena never hands an index out twice — so this is the door a file comes
    /// through.
    fn lone_handles(&self) -> Result<(), DocError> {
        let mut seen: Vec<PointKey> = Vec::new();
        for handle in self.handles() {
            if seen.contains(&handle) {
                return Err(DocError::occupied(handle));
            }
            seen.push(handle);
        }
        Ok(())
    }
}

impl LineSpan {
    /// Whether the span may be flattened at `count`.
    pub fn takes_samples(&self, count: u16) -> bool {
        samples_fit(self.segment.bends(), count)
    }
}

impl LineEdit {
    /// Whether the line this edit draws is one a piece can be drawn with.
    pub(crate) fn check(&self) -> Result<(), DocError> {
        let steps = self.spans.iter().map(|span| Step {
            at: span.to.anchor(),
            bends: span.segment.bends(),
            samples: span.samples,
        });
        checked(self.piece, self.head.anchor(), steps)
    }
}

/// The rules an internal line answers to, whichever side of a command it is
/// on.
///
/// A line through fewer than two places is not a line. A place on the contour
/// has to be on the contour of the piece the line is drawn on, the way a seam
/// side has to start and end on one piece, and at a fraction its tract can
/// answer for. And a span is flattened at a count that span can carry, by the
/// rule the contour's own tracts answer to.
fn checked(
    piece: PieceKey,
    head: Option<EdgeAnchor>,
    steps: impl Iterator<Item = Step>,
) -> Result<(), DocError> {
    let steps: Vec<Step> = steps.collect();
    if steps.is_empty() {
        return Err(DocError::ShortLine);
    }
    drawn_on(piece, head)?;
    for step in steps {
        if !samples_fit(step.bends, step.samples) {
            return Err(DocError::sampling(step.samples));
        }
        drawn_on(piece, step.at)?;
    }
    Ok(())
}

/// One place of a line, held to the piece the line is drawn on.
///
/// A place off the contour answers for nothing here: its point carries two
/// bindings and no piece, so there is no piece for it to disagree with.
fn drawn_on(piece: PieceKey, at: Option<EdgeAnchor>) -> Result<(), DocError> {
    let Some(anchor) = at else {
        return Ok(());
    };
    if anchor.piece != piece {
        return Err(DocError::SplitInternalLine);
    }
    if !anchor.is_valid() {
        return Err(DocError::AnchorFraction);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EdgeAnchor, SAMPLES};

    fn piece() -> PieceKey {
        PieceKey::new(0, 0)
    }

    fn free(index: u32) -> LineVertex {
        LineVertex::free(PointKey::new(index, 0))
    }

    fn on(t: f64) -> LineVertex {
        LineVertex::Contour(EdgeAnchor {
            piece: piece(),
            from: PointKey::new(1, 0),
            t,
        })
    }

    fn line(head: LineVertex, spans: Vec<LineSpan>) -> InternalLine {
        InternalLine {
            piece: piece(),
            kind: LineKind::Stitch,
            label: None,
            head,
            spans,
        }
    }

    fn straight(to: LineVertex) -> LineSpan {
        LineSpan {
            to,
            segment: Segment::Line,
            samples: 1,
        }
    }

    fn curved(to: LineVertex, out: u32, into: u32, samples: u16) -> LineSpan {
        LineSpan {
            to,
            segment: Segment::Cubic {
                out: PointKey::new(out, 0),
                into: PointKey::new(into, 0),
            },
            samples,
        }
    }

    #[test]
    fn a_line_that_runs_nowhere_is_not_a_line() {
        assert_eq!(line(free(1), Vec::new()).check(), Err(DocError::ShortLine));
        assert_eq!(line(free(1), vec![straight(free(2))]).check(), Ok(()));
    }

    #[test]
    fn a_place_on_another_piece_s_contour_is_refused() {
        let elsewhere = LineVertex::Contour(EdgeAnchor::at_node(
            PieceKey::new(9, 0),
            PointKey::new(1, 0),
        ));
        let drawn = line(free(1), vec![straight(elsewhere)]);
        assert_eq!(drawn.check(), Err(DocError::SplitInternalLine));
        let head = line(elsewhere, vec![straight(free(1))]);
        assert_eq!(head.check(), Err(DocError::SplitInternalLine));
    }

    #[test]
    fn a_fraction_no_tract_can_answer_for_is_refused_at_either_end() {
        for t in [-0.5, 1.5, f64::NAN] {
            let drawn = line(on(t), vec![straight(free(2))]);
            assert_eq!(drawn.check(), Err(DocError::AnchorFraction), "{t}");
        }
        assert_eq!(line(on(1.0), vec![straight(free(2))]).check(), Ok(()));
    }

    #[test]
    fn a_span_is_flattened_by_the_rule_a_tract_of_the_contour_answers_to() {
        let bent = line(free(1), vec![curved(free(2), 3, 4, 1)]);
        assert_eq!(bent.check(), Err(DocError::sampling(1)));
        let fine = line(free(1), vec![curved(free(2), 3, 4, SAMPLES.0)]);
        assert_eq!(fine.check(), Ok(()));
        let over = line(free(1), vec![straight(free(2))]);
        assert!(over.spans[0].takes_samples(1));
        assert!(!over.spans[0].takes_samples(SAMPLES.1 + 1));
    }

    #[test]
    fn a_line_hanging_two_spans_on_one_handle_is_refused() {
        let twice = line(
            free(1),
            vec![curved(free(2), 5, 6, 8), curved(free(3), 5, 7, 8)],
        );
        assert_eq!(twice.check(), Err(DocError::occupied(PointKey::new(5, 0))));
        let once = line(
            free(1),
            vec![curved(free(2), 5, 6, 8), curved(free(3), 7, 8, 8)],
        );
        assert_eq!(once.check(), Ok(()));
        assert_eq!(once.handles().count(), 4);
        assert!(once.cites(PointKey::new(5, 0)), "a handle it hangs on");
        assert!(once.cites(PointKey::new(2, 0)), "a place of its own");
        assert!(!once.cites(PointKey::new(9, 0)));
    }
}
