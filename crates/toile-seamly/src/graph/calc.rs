use roxmltree::Node;

use super::index::{Index, Kind};
use crate::xml::{Attrs, elements, unsupported};
use crate::{Arc, Construction, Error, Id, Object, ObjectKind, PathPoint, Place, Spline};

/// Reads one element of a `<calculation>` and records its id.
pub(super) fn object(node: Node<'_, '_>, index: &mut Index) -> Result<Object, Error> {
    let mut attrs = Attrs::new(node);
    let at = attrs.at();
    let (kind, recorded) = match (node.tag_name().name(), node.attribute("type")) {
        ("point", Some(rule)) => (point(node, &mut attrs, rule, index, &at)?, Kind::Point),
        ("line", None) => {
            let kind = ObjectKind::Line {
                first: point_ref(&mut attrs, "firstPoint", index, &at)?,
                second: point_ref(&mut attrs, "secondPoint", index, &at)?,
                line_type: attrs.req("lineType")?.to_owned(),
            };
            (kind, Kind::Line)
        }
        ("spline", Some("simpleInteractive")) => {
            let spline = spline(&mut attrs, index, &at)?;
            (ObjectKind::Spline(spline), Kind::Spline)
        }
        ("spline", Some("pathInteractive")) => (
            ObjectKind::SplinePath(path(node, index, &at)?),
            Kind::SplinePath,
        ),
        ("arc", Some("simple")) => {
            let arc = Arc {
                center: point_ref(&mut attrs, "center", index, &at)?,
                radius: attrs.formula("radius")?,
                angle1: attrs.formula("angle1")?,
                angle2: attrs.formula("angle2")?,
            };
            (ObjectKind::Arc(arc), Kind::Arc)
        }
        _ => return Err(unsupported(node, "this construction")),
    };
    let id = attrs.id("id")?;
    attrs.ignore(&["type"]);
    attrs.finish()?;
    index.insert(id, recorded, &at)?;
    Ok(Object { id, at, kind })
}

fn point_ref(
    attrs: &mut Attrs<'_, '_>,
    name: &'static str,
    index: &Index,
    at: &Place,
) -> Result<Id, Error> {
    index.expect(attrs.id(name)?, Kind::Point, at)
}

fn point(
    node: Node<'_, '_>,
    attrs: &mut Attrs<'_, '_>,
    rule: &str,
    index: &Index,
    at: &Place,
) -> Result<ObjectKind, Error> {
    let construction = match rule {
        "single" => Construction::Single {
            x: attrs.formula("x")?,
            y: attrs.formula("y")?,
        },
        "endLine" => Construction::EndLine {
            base: point_ref(attrs, "basePoint", index, at)?,
            length: attrs.formula("length")?,
            angle: attrs.formula("angle")?,
        },
        "alongLine" => Construction::AlongLine {
            first: point_ref(attrs, "firstPoint", index, at)?,
            second: point_ref(attrs, "secondPoint", index, at)?,
            length: attrs.formula("length")?,
        },
        "lineIntersect" => Construction::LineIntersect {
            line1: [
                point_ref(attrs, "p1Line1", index, at)?,
                point_ref(attrs, "p2Line1", index, at)?,
            ],
            line2: [
                point_ref(attrs, "p1Line2", index, at)?,
                point_ref(attrs, "p2Line2", index, at)?,
            ],
        },
        "cutSpline" => Construction::CutSpline {
            spline: index.expect(attrs.id("spline")?, Kind::Spline, at)?,
            length: attrs.formula("length")?,
        },
        _ => return Err(unsupported(node, format!("the point type `{rule}`"))),
    };
    Ok(ObjectKind::Point {
        name: attrs.req("name")?.to_owned(),
        construction,
        line_type: attrs.opt("lineType").map(str::to_owned),
    })
}

fn spline(attrs: &mut Attrs<'_, '_>, index: &Index, at: &Place) -> Result<Spline, Error> {
    let written_length = match attrs.opt("length") {
        Some(_) => Some(attrs.number("length")?),
        None => None,
    };
    Ok(Spline {
        start: point_ref(attrs, "point1", index, at)?,
        end: point_ref(attrs, "point4", index, at)?,
        angle1: attrs.formula("angle1")?,
        length1: attrs.formula("length1")?,
        angle2: attrs.formula("angle2")?,
        length2: attrs.formula("length2")?,
        written_length,
    })
}

fn path(node: Node<'_, '_>, index: &Index, at: &Place) -> Result<Vec<PathPoint>, Error> {
    let mut points = Vec::new();
    for child in elements(node) {
        if child.tag_name().name() != "pathPoint" {
            return Err(unsupported(child, "this element inside a spline path"));
        }
        let mut attrs = Attrs::new(child);
        let point = PathPoint {
            point: point_ref(&mut attrs, "pSpline", index, at)?,
            angle1: attrs.formula("angle1")?,
            length1: attrs.formula("length1")?,
            angle2: attrs.formula("angle2")?,
            length2: attrs.formula("length2")?,
        };
        attrs.finish()?;
        points.push(point);
    }
    if points.len() < 2 {
        return Err(Error::Malformed {
            at: at.clone(),
            what: "a spline path needs at least two points".to_owned(),
        });
    }
    Ok(points)
}
