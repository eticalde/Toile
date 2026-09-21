use super::{Coords, Site, Translator, distance};
use crate::sym::{Lin, Num};
use crate::{Construction, Error, Formula, Id};

impl Translator<'_> {
    pub(super) fn construct(
        &mut self,
        id: Id,
        construction: &Construction,
        site: Site<'_>,
    ) -> Result<Coords, Error> {
        match construction {
            Construction::Single { x, y } => Ok(Coords {
                x: self.lin(x, site, None)?,
                y: self.lin(y, site, None)?,
            }),
            Construction::EndLine {
                base,
                length,
                angle,
            } => {
                let base = self.cite(*base)?;
                let length = self.lin(length, site, None)?;
                let (ux, uy) = self.direction(angle, site)?;
                Ok(Coords {
                    x: base.x.add(&length.mul(&ux)),
                    y: base.y.add(&length.mul(&uy)),
                })
            }
            Construction::AlongLine {
                first,
                second,
                length,
            } => self.along(id, *first, *second, length, site),
            Construction::LineIntersect { line1, line2 } => self.intersect(*line1, *line2, site),
            Construction::CutSpline { spline, .. } => self.cut(id, *spline),
        }
    }

    /// `length` from `first` toward `second`.
    ///
    /// The fraction of the line the length covers is written out when the
    /// line's own length cancels from it, as a length of `CurrentLength / 2`
    /// makes it do. Along an axis, whichever way the line runs is read off the
    /// imported body when the formulas cannot say, and the point is recorded
    /// as one whose direction was fixed; anywhere else the direction is the
    /// line's own, divided by its length, and nothing is assumed.
    fn along(
        &mut self,
        id: Id,
        first: Id,
        second: Id,
        length: &Formula,
        site: Site<'_>,
    ) -> Result<Coords, Error> {
        let (a, b) = (self.cite(first)?, self.cite(second)?);
        let (dx, dy) = (b.x.sub(&a.x), b.y.sub(&a.y));
        let full = distance(&dx, &dy);
        let length = self.lin(length, site, Some(&full))?;
        let geometry = |what: &str| Error::Geometry {
            at: site.at.clone(),
            what: what.to_owned(),
        };
        let fraction = length
            .div(&full)
            .ok_or_else(|| geometry("its two points coincide and give no direction"))?;
        if fraction.constant().is_some() {
            return Ok(Coords {
                x: a.x.add(&fraction.mul(&dx)),
                y: a.y.add(&fraction.mul(&dy)),
            });
        }
        if dx.is_zero() || dy.is_zero() {
            let (run, axis) = if dy.is_zero() { (&dx, 0) } else { (&dy, 1) };
            let sign = if let Some(value) = run.constant() {
                value.value()
            } else {
                self.directions.push(id);
                self.reference_point(second)?[axis] - self.reference_point(first)?[axis]
            };
            if !(sign.is_finite() && sign.abs() > 0.0) {
                return Err(geometry("its two points coincide and give no direction"));
            }
            let step = length.scale(Num::of_f64(sign.signum()));
            return Ok(if axis == 0 {
                Coords {
                    x: a.x.add(&step),
                    y: a.y,
                }
            } else {
                Coords {
                    x: a.x,
                    y: a.y.add(&step),
                }
            });
        }
        // Written the way it reads best: a fixed length times the line's
        // direction, a fixed run times the length over the line's, or the
        // length times the run over the line's length.
        let over = |run: &Lin| {
            let written = if length.constant().is_some() {
                run.div(&full).map(|direction| direction.mul(&length))
            } else if run.constant().is_some() {
                length.div(&full).map(|part| part.mul(run))
            } else {
                length.mul(run).div(&full)
            };
            written.ok_or_else(|| geometry("its two points coincide and give no direction"))
        };
        Ok(Coords {
            x: a.x.add(&over(&dx)?),
            y: a.y.add(&over(&dy)?),
        })
    }

    /// Where the line through `line1` crosses the line through `line2`, as
    /// the first line's point plus the part of its direction that reaches the
    /// second.
    fn intersect(
        &mut self,
        line1: [Id; 2],
        line2: [Id; 2],
        site: Site<'_>,
    ) -> Result<Coords, Error> {
        let (p1, p2) = (self.cite(line1[0])?, self.cite(line1[1])?);
        let (p3, p4) = (self.cite(line2[0])?, self.cite(line2[1])?);
        let (vx, vy) = (p2.x.sub(&p1.x), p2.y.sub(&p1.y));
        let (wx, wy) = (p4.x.sub(&p3.x), p4.y.sub(&p3.y));
        let (rx, ry) = (p3.x.sub(&p1.x), p3.y.sub(&p1.y));
        let across = vx.mul(&wy).sub(&vy.mul(&wx));
        let reach = rx.mul(&wy).sub(&ry.mul(&wx));
        let part = |v: &Lin| {
            reach.mul(v).div(&across).ok_or_else(|| Error::Geometry {
                at: site.at.clone(),
                what: "its two lines are parallel".to_owned(),
            })
        };
        Ok(Coords {
            x: p1.x.add(&part(&vx)?),
            y: p1.y.add(&part(&vy)?),
        })
    }

    /// Where the imported body puts the point `id`.
    pub(crate) fn reference_point(&self, id: Id) -> Result<crate::Xy, Error> {
        self.reference
            .points
            .get(&id)
            .map(|located| located.at)
            .ok_or_else(|| Error::Product(format!("point {id} has no position")))
    }
}
