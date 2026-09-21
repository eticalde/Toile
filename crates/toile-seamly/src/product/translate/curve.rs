use super::heading::{Heading, turn};
use super::{Coords, Site, Translator, distance};
use crate::eval::angle;
use crate::sym::{Frozen, Lin, Num, Rat};
use crate::{Error, Formula, Id, ObjectKind};

/// One span of an arc as a cubic, in the arc's own direction.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Span {
    pub(crate) start: Coords,
    pub(crate) out: Coords,
    pub(crate) into: Coords,
    pub(crate) end: Coords,
}

/// `at` moved `length` along the unit vector `(ux, uy)`.
fn step(at: &Coords, length: &Lin, (ux, uy): &(Lin, Lin)) -> Coords {
    Coords {
        x: at.x.add(&length.mul(ux)),
        y: at.y.add(&length.mul(uy)),
    }
}

impl Translator<'_> {
    /// A handle: `length` from its end point, heading `angle`.
    fn handle(
        &mut self,
        at: &Coords,
        length: &Formula,
        angle: &Formula,
        site: Site<'_>,
    ) -> Result<Coords, Error> {
        let length = self.lin(length, site, None)?;
        let direction = self.direction(angle, site)?;
        Ok(step(at, &length, &direction))
    }

    /// The four control points of the spline `id`.
    ///
    /// Its handles keep the heading and the length the file gives them, from
    /// whichever point they leave, as Seamly keeps them when a body changes.
    pub(crate) fn spline(&mut self, id: Id) -> Result<[Coords; 4], Error> {
        let (position, object) = self.object(id)?;
        let ObjectKind::Spline(spline) = &object.kind else {
            return Err(Error::Product(format!("object {id} is not a spline")));
        };
        let site = Site {
            position,
            at: &object.at,
        };
        let start = self.cite(spline.start)?;
        let end = self.cite(spline.end)?;
        let out = self.handle(&start, &spline.length1, &spline.angle1, site)?;
        let into = self.handle(&end, &spline.length2, &spline.angle2, site)?;
        Ok([start, out, into, end])
    }

    /// The segments of the spline path `id`, four control points each, in
    /// path order.
    pub(crate) fn path(&mut self, id: Id) -> Result<Vec<[Coords; 4]>, Error> {
        let (position, object) = self.object(id)?;
        let ObjectKind::SplinePath(points) = &object.kind else {
            return Err(Error::Product(format!("object {id} is not a spline path")));
        };
        let site = Site {
            position,
            at: &object.at,
        };
        let mut segments = Vec::with_capacity(points.len().saturating_sub(1));
        let mut previous: Option<(Coords, Coords)> = None;
        for (index, point) in points.iter().enumerate() {
            let at = self.cite(point.point)?;
            if let Some((from, leaving)) = previous.take() {
                let arriving = self.handle(&at, &point.length1, &point.angle1, site)?;
                segments.push([from, leaving, arriving, at.clone()]);
            }
            if index + 1 < points.len() {
                let leaving = self.handle(&at, &point.length2, &point.angle2, site)?;
                previous = Some((at, leaving));
            }
        }
        Ok(segments)
    }

    /// The arc `id` as cubic spans of equal sweep, none wider than a right
    /// angle, from its first heading counter-clockwise to its second.
    pub(crate) fn arc(&mut self, id: Id) -> Result<Vec<Span>, Error> {
        let (position, object) = self.object(id)?;
        let ObjectKind::Arc(arc) = &object.kind else {
            return Err(Error::Product(format!("object {id} is not an arc")));
        };
        let site = Site {
            position,
            at: &object.at,
        };
        let unsupported = |what: &str| Error::Unsupported {
            at: site.at.clone(),
            what: what.to_owned(),
        };
        let center = self.cite(arc.center)?;
        let radius = self.lin(&arc.radius, site, None)?;
        let (Some(first), Some(last)) = (Heading::of(&arc.angle1), Heading::of(&arc.angle2)) else {
            return Err(unsupported(
                "an arc whose headings are not numbers or drawn headings",
            ));
        };
        if first.along != last.along {
            return Err(unsupported("an arc whose sweep changes with the body"));
        }
        let sweep = sweep(first.degrees, last.degrees)
            .ok_or_else(|| unsupported("an arc that sweeps a whole circle or nothing"))?;
        let count = spans(sweep);
        let each = sweep
            .div(Num::Exact(Rat::int(count)))
            .expect("an arc is cut into at least one span");
        let reach = radius.scale(Num::Float(angle::arc_handle_ratio(each.value())));
        let mut out = Vec::new();
        for index in 0..count {
            let from = first.turned(each.mul(Num::Exact(Rat::int(index))));
            let to = first.turned(each.mul(Num::Exact(Rat::int(index + 1))));
            let (outward, inward) = (self.toward(from, site)?, self.toward(to, site)?);
            let start = step(&center, &radius, &outward);
            let end = step(&center, &radius, &inward);
            let leaving = turn(outward, 1);
            let (ax, ay) = turn(inward, 1);
            out.push(Span {
                out: step(&start, &reach, &leaving),
                into: step(&end, &reach, &(ax.neg(), ay.neg())),
                start,
                end,
            });
        }
        Ok(out)
    }

    /// The length a `Spl_` name gives the spline `id`.
    ///
    /// A spline whose handles are both zero long is its own chord, and the
    /// chord is a square root of the body. A curved one is longer by an
    /// amount no formula can follow: that excess is taken from the imported
    /// body and frozen, while the chord it is added to keeps following.
    pub(crate) fn spline_length(&mut self, id: Id) -> Result<Lin, Error> {
        let [start, out, into, end] = self.spline(id)?;
        let chord = distance(&end.x.sub(&start.x), &end.y.sub(&start.y));
        if out == start && into == end {
            return Ok(chord);
        }
        let cubic = self
            .reference
            .splines
            .get(&id)
            .ok_or_else(|| Error::Product(format!("spline {id} was not evaluated")))?;
        let [p0, _, _, p3] = cubic.0;
        let (dx, dy) = (p3[0] - p0[0], p3[1] - p0[1]);
        let excess = cubic.length() - (dx * dx + dy * dy).sqrt();
        let frozen = Frozen::SplineExcess(id);
        self.frozen.insert(frozen, excess);
        Ok(chord.add(&Lin::num(Num::Float(excess)).frozen_by(frozen)))
    }

    /// A cut point: the spline evaluated at the parameter the imported body
    /// puts the cut at.
    ///
    /// The parameter is frozen and the control points are not, so the point
    /// stays on the curve as the body changes; what it stops doing is landing
    /// at the arc length its formula asks for.
    pub(super) fn cut(&mut self, id: Id, spline: Id) -> Result<Coords, Error> {
        let points = self.spline(spline)?;
        let t = self
            .reference
            .cuts
            .get(&id)
            .map(|cut| cut.t)
            .ok_or_else(|| Error::Product(format!("cut point {id} was not evaluated")))?;
        let frozen = Frozen::CutParameter(id);
        self.frozen.insert(frozen, t);
        let u = 1.0 - t;
        let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
        let blend = |axis: fn(&Coords) -> &Lin| {
            weights
                .iter()
                .zip(&points)
                .fold(Lin::zero(), |sum, (&weight, point)| {
                    sum.add(&axis(point).scale(Num::Float(weight)))
                })
                .frozen_by(frozen)
        };
        Ok(Coords {
            x: blend(|c| &c.x),
            y: blend(|c| &c.y),
        })
    }
}

/// How far counter-clockwise the second heading is from the first, in
/// `(0, 360)`; `None` when they coincide.
fn sweep(first: Num, last: Num) -> Option<Num> {
    let turn = Num::Exact(Rat::int(360));
    let apart = last.add(first.neg());
    let wrapped = match apart {
        Num::Exact(value) => {
            let turns = value.floor_div(Rat::int(360))?;
            apart.add(turn.mul(Num::Exact(Rat::new(-turns, 1)?)))
        }
        Num::Float(value) => Num::Float(value.rem_euclid(360.0)),
    };
    (!wrapped.is_zero()).then_some(wrapped)
}

/// How many equal spans keep each within a right angle.
fn spans(sweep: Num) -> i64 {
    match sweep {
        Num::Exact(value) => {
            let quarter = Rat::int(90);
            let whole = value.floor_div(quarter).unwrap_or(0);
            let exact = Rat::new(whole * 90, 1) == Some(value);
            (if exact { whole } else { whole + 1 }).max(1) as i64
        }
        Num::Float(value) => ((value / 90.0).ceil() as i64).max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sweep_is_counted_counter_clockwise_and_wraps() {
        let n = Num::of_f64;
        assert_eq!(sweep(n(270.0), n(0.0)), Some(n(90.0)));
        assert_eq!(sweep(n(180.0), n(0.0)), Some(n(180.0)));
        assert_eq!(sweep(n(0.0), n(180.0)), Some(n(180.0)));
        assert_eq!(sweep(n(90.0), n(90.0)), None);
    }

    #[test]
    fn no_span_is_wider_than_a_right_angle() {
        let n = Num::of_f64;
        assert_eq!(spans(n(90.0)), 1);
        assert_eq!(spans(n(90.5)), 2);
        assert_eq!(spans(n(180.0)), 2);
        assert_eq!(spans(n(45.0)), 1);
        assert_eq!(spans(n(359.0)), 4);
    }
}
