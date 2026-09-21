use std::collections::BTreeMap;

use crate::eval::env::{ANGLE_LINE, LINE, SPLINE};
use crate::{Construction, Error, Id, ObjectKind, Pattern, Place};

/// What a drawn name stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Drawing {
    /// A segment, from its first point to its second: `Line_a_b` measures it
    /// and `AngleLine_a_b` heads along it.
    Segment(Id, Id),
    /// A spline, which `Spl_a_b` measures.
    Spline(Id),
}

/// Every `Line_`, `AngleLine_` and `Spl_` name the file's drawings define,
/// with where in the file each definition is made.
///
/// A formula may cite only what is drawn above it, so a name is looked up as
/// of the object that cites it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Drawn {
    names: BTreeMap<String, Vec<(usize, Drawing)>>,
}

impl Drawn {
    pub(crate) fn new(pattern: &Pattern) -> Drawn {
        let names: BTreeMap<Id, &str> = pattern
            .objects()
            .filter_map(|object| Some((object.id, object.name()?)))
            .collect();
        let mut drawn = Drawn::default();
        for (position, object) in pattern.objects().enumerate() {
            match &object.kind {
                ObjectKind::Line { first, second, .. } => {
                    drawn.segment(&names, position, *first, *second);
                }
                // An end-line point strokes a segment from its base, and a
                // hidden stroke draws nothing: the evaluator's rule.
                ObjectKind::Point {
                    construction: Construction::EndLine { base, .. },
                    line_type: Some(stroke),
                    ..
                } if stroke != "none" => drawn.segment(&names, position, *base, object.id),
                ObjectKind::Spline(spline) => {
                    if let (Some(a), Some(b)) = (names.get(&spline.start), names.get(&spline.end)) {
                        drawn.define(
                            format!("{SPLINE}{a}_{b}"),
                            position,
                            Drawing::Spline(object.id),
                        );
                    }
                }
                _ => {}
            }
        }
        drawn
    }

    fn segment(&mut self, names: &BTreeMap<Id, &str>, position: usize, first: Id, second: Id) {
        let (Some(a), Some(b)) = (names.get(&first), names.get(&second)) else {
            return;
        };
        let (there, back) = (
            Drawing::Segment(first, second),
            Drawing::Segment(second, first),
        );
        self.define(format!("{LINE}{a}_{b}"), position, there);
        self.define(format!("{LINE}{b}_{a}"), position, back);
        self.define(format!("{ANGLE_LINE}{a}_{b}"), position, there);
        self.define(format!("{ANGLE_LINE}{b}_{a}"), position, back);
    }

    fn define(&mut self, name: String, position: usize, drawing: Drawing) {
        self.names
            .entry(name)
            .or_default()
            .push((position, drawing));
    }

    /// What `name` stands for, as the object at `position` sees it.
    ///
    /// # Errors
    /// A name nothing above draws, or one two drawings above define as
    /// different things.
    pub(crate) fn lookup(&self, name: &str, position: usize, at: &Place) -> Result<Drawing, Error> {
        let mut above = self
            .names
            .get(name)
            .into_iter()
            .flatten()
            .filter(|(defined, _)| *defined < position)
            .map(|&(_, drawing)| drawing);
        let first = above.next().ok_or_else(|| Error::Malformed {
            at: at.clone(),
            what: format!("nothing drawn above defines `{name}`"),
        })?;
        // A segment drawn twice between the same two points is one segment.
        if above.any(|other| other != first) {
            return Err(Error::Malformed {
                at: at.clone(),
                what: format!("`{name}` names two different drawings"),
            });
        }
        Ok(first)
    }
}
