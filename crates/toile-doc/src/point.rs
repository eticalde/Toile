use crate::Binding;

/// A control point of the pattern: two bindings and, if its author wanted one,
/// a name.
///
/// Points live in the document rather than inside a piece, so that a later
/// tool can lift several pieces off one drafting skeleton without the shared
/// corners losing their identity.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    /// The horizontal coordinate, in centimetres, growing to the right.
    pub x: Binding,
    /// The vertical coordinate, in centimetres, growing downward.
    pub y: Binding,
    /// The name its author gave it; unique among the points of a piece.
    pub label: Option<String>,
    /// Whether the drawing shows that name without being asked.
    pub label_visible: bool,
}

/// One of the two coordinates of a point: never a `bool`, never an index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    /// The horizontal coordinate.
    X,
    /// The vertical coordinate.
    Y,
}

impl Point {
    /// An unnamed point at two bindings.
    pub fn at(x: impl Into<Binding>, y: impl Into<Binding>) -> Point {
        Point {
            x: x.into(),
            y: y.into(),
            label: None,
            label_visible: false,
        }
    }

    /// The same point, carrying `label`.
    #[must_use]
    pub fn named(mut self, label: &str) -> Point {
        self.label = Some(label.to_owned());
        self
    }

    /// The binding of one coordinate.
    pub fn binding(&self, axis: Axis) -> &Binding {
        match axis {
            Axis::X => &self.x,
            Axis::Y => &self.y,
        }
    }

    /// The binding of one coordinate, to be written.
    pub fn binding_mut(&mut self, axis: Axis) -> &mut Binding {
        match axis {
            Axis::X => &mut self.x,
            Axis::Y => &mut self.y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_starts_unnamed_and_quiet() {
        let point = Point::at(0.0, 0.0);
        assert_eq!(point.label, None);
        assert!(!point.label_visible);
    }

    #[test]
    fn an_axis_names_the_coordinate_it_reaches() {
        let mut point = Point::at(1.0, 2.0).named("cadera_lat");
        assert_eq!(point.binding(Axis::X), &Binding::Literal(1.0));
        assert_eq!(point.binding(Axis::Y), &Binding::Literal(2.0));
        *point.binding_mut(Axis::Y) = Binding::literal(3.0);
        assert_eq!(point.y, Binding::Literal(3.0));
        assert_eq!(point.label.as_deref(), Some("cadera_lat"));
    }
}
