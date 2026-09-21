use std::collections::BTreeMap;

use super::angle::{bearing, toward};
use super::env::{ANGLE_LINE, Env, LINE, SPLINE};
use super::{CircleArc, Cubic, Cut, Evaluation, Located, SplineLength, Xy};
use crate::{
    Arc, Construction, Error, Formula, FormulaError, Id, Measurements, ObjectKind, PathPoint,
    Pattern, Place, Spline,
};

pub(super) fn evaluate(
    pattern: &Pattern,
    measurements: &Measurements,
    lengths: SplineLength,
) -> Result<Evaluation, Error> {
    let mut eval = Evaluation {
        points: BTreeMap::new(),
        splines: BTreeMap::new(),
        paths: BTreeMap::new(),
        arcs: BTreeMap::new(),
        cuts: BTreeMap::new(),
        env: Env::new(measurements),
    };
    for variable in &pattern.variables {
        let value = eval
            .env
            .eval(&variable.formula, None)
            .map_err(|source| formula_error(&variable.at, "formula", &variable.formula, source))?;
        if !eval.env.set_variable(&variable.name, value) {
            return Err(Error::Malformed {
                at: variable.at.clone(),
                what: format!("`{}` hides the measurement of that name", variable.name),
            });
        }
    }
    for object in pattern.objects() {
        let mut step = Step {
            eval: &mut eval,
            at: &object.at,
            lengths,
        };
        match &object.kind {
            ObjectKind::Point {
                name,
                construction,
                line_type,
            } => step.point(object.id, name, construction, line_type.as_deref())?,
            ObjectKind::Line { first, second, .. } => step.draw(*first, *second)?,
            ObjectKind::Spline(spline) => step.spline(object.id, spline)?,
            ObjectKind::SplinePath(points) => step.path(object.id, points)?,
            ObjectKind::Arc(arc) => step.arc(object.id, arc)?,
        }
    }
    Ok(eval)
}

fn formula_error(
    at: &Place,
    attribute: &'static str,
    formula: &Formula,
    source: FormulaError,
) -> Error {
    Error::Formula {
        at: at.clone(),
        attribute,
        formula: formula.source().to_owned(),
        source,
    }
}

fn distance(a: Xy, b: Xy) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    (dx * dx + dy * dy).sqrt()
}

/// One object's evaluation, with the element to blame if it fails.
struct Step<'a> {
    eval: &'a mut Evaluation,
    at: &'a Place,
    lengths: SplineLength,
}

impl Step<'_> {
    fn value(&self, formula: &Formula, attribute: &'static str) -> Result<f64, Error> {
        self.value_along(formula, attribute, None)
    }

    fn value_along(
        &self,
        formula: &Formula,
        attribute: &'static str,
        current: Option<f64>,
    ) -> Result<f64, Error> {
        self.eval
            .env
            .eval(formula, current)
            .map_err(|source| formula_error(self.at, attribute, formula, source))
    }

    fn geometry(&self, what: impl Into<String>) -> Error {
        Error::Geometry {
            at: self.at.clone(),
            what: what.into(),
        }
    }

    fn located(&self, id: Id) -> Result<Located, Error> {
        self.eval
            .points
            .get(&id)
            .cloned()
            .ok_or_else(|| self.geometry(format!("point {id} has no position above this")))
    }

    fn xy(&self, id: Id) -> Result<Xy, Error> {
        Ok(self.located(id)?.at)
    }

    fn point(
        &mut self,
        id: Id,
        name: &str,
        construction: &Construction,
        line_type: Option<&str>,
    ) -> Result<(), Error> {
        let (at, drawn_from) = match construction {
            Construction::Single { x, y } => ([self.value(x, "x")?, self.value(y, "y")?], None),
            Construction::EndLine {
                base,
                length,
                angle,
            } => {
                let origin = self.xy(*base)?;
                let length = self.value(length, "length")?;
                (
                    toward(origin, length, self.value(angle, "angle")?),
                    Some(*base),
                )
            }
            Construction::AlongLine {
                first,
                second,
                length,
            } => (self.along(*first, *second, length)?, None),
            Construction::LineIntersect { line1, line2 } => (self.intersect(*line1, *line2)?, None),
            Construction::CutSpline { spline, length } => (self.cut(id, *spline, length)?, None),
        };
        self.eval.points.insert(
            id,
            Located {
                name: name.to_owned(),
                at,
            },
        );
        // An end-line point strokes a segment from its base, which names a
        // `Line_` like a drawn line does; a hidden stroke draws nothing.
        if let Some(base) = drawn_from
            && line_type.is_some_and(|stroke| stroke != "none")
        {
            self.draw(base, id)?;
        }
        Ok(())
    }

    fn along(&self, first: Id, second: Id, length: &Formula) -> Result<Xy, Error> {
        let (a, b) = (self.xy(first)?, self.xy(second)?);
        let full = distance(a, b);
        if full == 0.0 {
            return Err(self.geometry("its two points coincide and give no direction"));
        }
        let fraction = self.value_along(length, "length", Some(full))? / full;
        Ok([
            a[0] + fraction * (b[0] - a[0]),
            a[1] + fraction * (b[1] - a[1]),
        ])
    }

    fn intersect(&self, [p1, p2]: [Id; 2], [p3, p4]: [Id; 2]) -> Result<Xy, Error> {
        let [x1, y1] = self.xy(p1)?;
        let [x2, y2] = self.xy(p2)?;
        let [x3, y3] = self.xy(p3)?;
        let [x4, y4] = self.xy(p4)?;
        let den = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
        if den == 0.0 {
            return Err(self.geometry("its two lines are parallel"));
        }
        let (a, b) = (x1 * y2 - y1 * x2, x3 * y4 - y3 * x4);
        Ok([
            (a * (x3 - x4) - (x1 - x2) * b) / den,
            (a * (y3 - y4) - (y1 - y2) * b) / den,
        ])
    }

    fn cut(&mut self, id: Id, spline: Id, length: &Formula) -> Result<Xy, Error> {
        let cubic = *self
            .eval
            .splines
            .get(&spline)
            .ok_or_else(|| self.geometry(format!("spline {spline} has no curve above this")))?;
        let length = self.value(length, "length")?;
        let t = cubic.t_at_length(length).ok_or_else(|| {
            self.geometry(format!(
                "a cut {length} cm along a spline {} cm long",
                cubic.length()
            ))
        })?;
        self.eval.cuts.insert(id, Cut { spline, length, t });
        Ok(cubic.point(t))
    }

    fn draw(&mut self, first: Id, second: Id) -> Result<(), Error> {
        let (a, b) = (self.located(first)?, self.located(second)?);
        let length = distance(a.at, b.at);
        let heading = bearing(a.at, b.at);
        let env = &mut self.eval.env;
        env.define(format!("{LINE}{}_{}", a.name, b.name), length);
        env.define(format!("{LINE}{}_{}", b.name, a.name), length);
        env.define(format!("{ANGLE_LINE}{}_{}", a.name, b.name), heading);
        let back = (heading + 180.0).rem_euclid(360.0);
        env.define(format!("{ANGLE_LINE}{}_{}", b.name, a.name), back);
        Ok(())
    }

    fn spline(&mut self, id: Id, spline: &Spline) -> Result<(), Error> {
        let (start, end) = (self.located(spline.start)?, self.located(spline.end)?);
        let leaving = toward(
            start.at,
            self.value(&spline.length1, "length1")?,
            self.value(&spline.angle1, "angle1")?,
        );
        let arriving = toward(
            end.at,
            self.value(&spline.length2, "length2")?,
            self.value(&spline.angle2, "angle2")?,
        );
        let cubic = Cubic([start.at, leaving, arriving, end.at]);
        let length = match self.lengths {
            SplineLength::ArcLength => cubic.length(),
            SplineLength::Written => spline
                .written_length
                .ok_or_else(|| self.geometry("the file writes no length on this spline"))?,
        };
        let name = format!("{SPLINE}{}_{}", start.name, end.name);
        self.eval.env.define(name, length);
        self.eval.splines.insert(id, cubic);
        Ok(())
    }

    fn path(&mut self, id: Id, points: &[PathPoint]) -> Result<(), Error> {
        let mut through = Vec::with_capacity(points.len());
        for point in points {
            let at = self.xy(point.point)?;
            let arriving = toward(
                at,
                self.value(&point.length1, "length1")?,
                self.value(&point.angle1, "angle1")?,
            );
            let leaving = toward(
                at,
                self.value(&point.length2, "length2")?,
                self.value(&point.angle2, "angle2")?,
            );
            through.push((at, arriving, leaving));
        }
        // The first point's arriving handle and the last one's leaving handle
        // shape no segment; they are evaluated only so their errors surface.
        let segments = through
            .windows(2)
            .map(|pair| Cubic([pair[0].0, pair[0].2, pair[1].1, pair[1].0]))
            .collect();
        self.eval.paths.insert(id, segments);
        Ok(())
    }

    fn arc(&mut self, id: Id, arc: &Arc) -> Result<(), Error> {
        let center = self.xy(arc.center)?;
        let radius = self.value(&arc.radius, "radius")?;
        if radius <= 0.0 {
            return Err(self.geometry(format!("a radius of {radius} cm")));
        }
        let angle1 = self.value(&arc.angle1, "angle1")?;
        let angle2 = self.value(&arc.angle2, "angle2")?;
        let arc = CircleArc {
            center,
            radius,
            angle1,
            angle2,
            start: toward(center, radius, angle1),
            end: toward(center, radius, angle2),
        };
        self.eval.arcs.insert(id, arc);
        Ok(())
    }
}
