use toile_engine::draft::{Axis, Binding};

use super::Drag;

/// The question a release asks when the drag rewrote a formula.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    /// One row per coordinate the drag rewrote.
    pub rows: Vec<AskRow>,
}

/// One coordinate's formula, before and after the drag.
#[derive(Debug, Clone, PartialEq)]
pub struct AskRow {
    /// The coordinate, as the modal names it: the node and the axis when the
    /// gesture holds several points, the axis alone when it holds one.
    pub axis: String,
    /// What its author wrote.
    pub before: String,
    /// What the drag made of it.
    pub after: String,
}

impl Ask {
    /// The question a release asks, when the drag rewrote a formula at all.
    pub fn of(drag: &Drag, step: f64, except: Option<Axis>) -> Option<Ask> {
        let rows = drag.rewrites(step, except);
        (!rows.is_empty()).then_some(Ask { rows })
    }
}

impl Drag {
    /// What the drag makes of every coordinate written as a formula.
    ///
    /// A coordinate whose source comes out unchanged is not a rewrite and
    /// leaves no row, so a drag too small to show asks nothing. `except` is
    /// the coordinate of the anchor the precision box has already written by
    /// hand, which is not the drag's doing and asks nothing either.
    pub fn rewrites(&self, step: f64, except: Option<Axis>) -> Vec<AskRow> {
        let many = self.nodes.len() > 1;
        let anchor = self.anchor().point;
        self.carried()
            .flat_map(|(held, delta)| {
                [(Axis::X, 0), (Axis::Y, 1)]
                    .into_iter()
                    .map(move |pair| (held, delta, pair))
            })
            .filter(|&(held, _, (axis, _))| !(held.point == anchor && except == Some(axis)))
            .filter_map(|(held, delta, (axis, k))| {
                let Binding::Formula(formula) = &held.origin[k] else {
                    return None;
                };
                let after = formula.nudged_source(delta[k], step);
                (after != formula.source()).then(|| AskRow {
                    axis: if many {
                        format!("{} · {}", held.name, name(axis))
                    } else {
                        name(axis).to_owned()
                    },
                    before: formula.source().to_owned(),
                    after,
                })
            })
            .collect()
    }
}

/// How the panels name a coordinate.
pub fn name(axis: Axis) -> &'static str {
    match axis {
        Axis::X => "X",
        Axis::Y => "Y",
    }
}
