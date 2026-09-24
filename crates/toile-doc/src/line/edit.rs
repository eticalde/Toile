use super::{LineKind, LineVertex};
use crate::{PieceKey, SegmentEdit};

/// An internal line on its way into the document, its handles included.
///
/// It stands to `InternalLine` as `SegmentEdit` stands to `Segment`, and for
/// the same reason: a span that bends hangs on two handles that are points of
/// the document, so the edit that draws the line creates them and the edit
/// that takes it away removes them. The handles travel in the command rather
/// than their keys alone, which is what lets undo give the very same keys back
/// with whatever they had grown into.
#[derive(Debug, Clone, PartialEq)]
pub struct LineEdit {
    /// The piece the line is drawn on.
    pub piece: PieceKey,
    /// What the line is for.
    pub kind: LineKind,
    /// The name its author gave it.
    pub label: Option<String>,
    /// Where the run starts.
    pub head: LineVertex,
    /// Every further place, with the span that reaches it.
    pub spans: Vec<SpanEdit>,
}

/// One step of an internal line on its way in.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanEdit {
    /// Where the step ends.
    pub to: LineVertex,
    /// What runs to it, with the handles a curve brings.
    pub segment: SegmentEdit,
    /// How many samples that span is flattened into.
    pub samples: u16,
}

impl LineEdit {
    /// A line of `kind` on `piece`, starting at `head` and running nowhere
    /// yet.
    pub fn new(piece: PieceKey, kind: LineKind, head: LineVertex) -> LineEdit {
        LineEdit {
            piece,
            kind,
            label: None,
            head,
            spans: Vec::new(),
        }
    }

    /// The same line carried on to `to` by a straight span.
    #[must_use]
    pub fn to(mut self, to: LineVertex) -> LineEdit {
        self.spans.push(SpanEdit {
            to,
            segment: SegmentEdit::Line,
            samples: 1,
        });
        self
    }

    /// The same line carried on to `to` by a span that bends, flattened at
    /// `samples`.
    #[must_use]
    pub fn curving(mut self, to: LineVertex, segment: SegmentEdit, samples: u16) -> LineEdit {
        self.spans.push(SpanEdit {
            to,
            segment,
            samples,
        });
        self
    }

    /// The same line, carrying the name its author gave it.
    #[must_use]
    pub fn named(mut self, label: &str) -> LineEdit {
        self.label = Some(label.to_owned());
        self
    }

    /// The places the line runs through, in order.
    pub fn vertices(&self) -> impl Iterator<Item = LineVertex> {
        std::iter::once(self.head).chain(self.spans.iter().map(|span| span.to))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, PointKey};

    fn vertex(index: u32) -> LineVertex {
        LineVertex::free(PointKey::new(index, 0))
    }

    #[test]
    fn a_line_is_built_one_place_at_a_time_and_reads_back_in_order() {
        let edit = LineEdit::new(PieceKey::new(0, 0), LineKind::Placement, vertex(1))
            .to(vertex(2))
            .to(vertex(3))
            .named("pasacintos 3");
        assert_eq!(edit.label.as_deref(), Some("pasacintos 3"));
        assert_eq!(
            edit.vertices().collect::<Vec<_>>(),
            [vertex(1), vertex(2), vertex(3)]
        );
        assert!(edit.spans.iter().all(|span| span.samples == 1));
    }

    #[test]
    fn a_span_that_bends_carries_the_two_handles_it_hangs_on() {
        let curve = SegmentEdit::cubic(Point::at(1.0, 2.0), Point::at(3.0, 4.0));
        let edit = LineEdit::new(PieceKey::new(0, 0), LineKind::Slit, vertex(1)).curving(
            vertex(2),
            curve,
            12,
        );
        let span = edit.spans.first().expect("the line runs one span");
        assert!(span.segment.bends());
        assert_eq!(span.samples, 12);
    }
}
