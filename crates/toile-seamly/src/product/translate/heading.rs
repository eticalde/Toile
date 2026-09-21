use super::super::drawn::Drawing;
use super::{Site, Translator, distance};
use crate::eval::angle;
use crate::eval::env::ANGLE_LINE;
use crate::sym::{Lin, Num, Rat};
use crate::{Error, Expr, Formula, Id, Op};

/// A heading as the file writes it: a drawn line's heading, if it follows
/// one, plus a number of degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Heading<'e> {
    pub(crate) along: Option<&'e str>,
    pub(crate) degrees: Num,
}

impl<'e> Heading<'e> {
    /// The heading `formula` writes, if it is a number or a drawn line's
    /// heading plus or minus one.
    pub(crate) fn of(formula: &'e Formula) -> Option<Heading<'e>> {
        parts(formula.expr())
    }

    /// The same heading turned `degrees` further.
    pub(crate) fn turned(self, degrees: Num) -> Heading<'e> {
        Heading {
            along: self.along,
            degrees: self.degrees.add(degrees),
        }
    }
}

fn parts(expr: &Expr) -> Option<Heading<'_>> {
    Some(match expr {
        Expr::Num(value) => Heading {
            along: None,
            degrees: Num::of_f64(*value),
        },
        Expr::Name(name) if name.starts_with(ANGLE_LINE) => Heading {
            along: Some(name),
            degrees: Num::Exact(Rat::ZERO),
        },
        Expr::Neg(inner) => {
            let inner = parts(inner)?;
            inner.along.is_none().then_some(())?;
            Heading {
                along: None,
                degrees: inner.degrees.neg(),
            }
        }
        Expr::Bin(op @ (Op::Add | Op::Sub), lhs, rhs) => {
            let (lhs, rhs) = (parts(lhs)?, parts(rhs)?);
            let rhs = match op {
                Op::Add => rhs,
                _ if rhs.along.is_some() => return None,
                _ => Heading {
                    along: None,
                    degrees: rhs.degrees.neg(),
                },
            };
            if lhs.along.is_some() && rhs.along.is_some() {
                return None;
            }
            Heading {
                along: lhs.along.or(rhs.along),
                degrees: lhs.degrees.add(rhs.degrees),
            }
        }
        _ => return None,
    })
}

/// How many quarter turns `degrees` is, when it is a whole number of them.
pub(crate) fn quarter_turns(degrees: Num) -> Option<u8> {
    let Num::Exact(degrees) = degrees else {
        return None;
    };
    let quarter = Rat::int(90);
    let turns = degrees.floor_div(quarter)?;
    let whole = Rat::new(turns.checked_mul(90)?, 1)?;
    (whole == degrees).then(|| turns.rem_euclid(4) as u8)
}

/// `(x, y)` turned counter-clockwise on the page by `quarters` right angles.
///
/// The page's y axis grows downward, so a quarter turn takes east to up:
/// `(1, 0)` to `(0, -1)`.
pub(crate) fn turn((x, y): (Lin, Lin), quarters: u8) -> (Lin, Lin) {
    match quarters % 4 {
        0 => (x, y),
        1 => (y, x.neg()),
        2 => (x.neg(), y.neg()),
        _ => (y.neg(), x),
    }
}

impl Translator<'_> {
    /// The unit vector a heading formula points along.
    pub(crate) fn direction(
        &mut self,
        formula: &Formula,
        site: Site<'_>,
    ) -> Result<(Lin, Lin), Error> {
        let heading = Heading::of(formula).ok_or_else(|| Error::Unsupported {
            at: site.at.clone(),
            what: format!(
                "the heading `{}`, which is neither a number nor a drawn line's heading \
                 plus a number",
                formula.source()
            ),
        })?;
        self.toward(heading, site)
    }

    /// The unit vector along `heading`.
    ///
    /// A number of degrees is turned into its cosine and sine once, here, and
    /// exactly when it is a right angle. A drawn line's heading needs no
    /// trigonometry at all: it is the line's own direction, turned by whole
    /// right angles, which is all a formula can turn it by and stay exact.
    pub(crate) fn toward(
        &mut self,
        heading: Heading<'_>,
        site: Site<'_>,
    ) -> Result<(Lin, Lin), Error> {
        let Some(line) = heading.along else {
            return Ok(constant(heading.degrees));
        };
        let quarters = quarter_turns(heading.degrees).ok_or_else(|| Error::Unsupported {
            at: site.at.clone(),
            what: format!("a heading off `{line}` by an angle that is not a whole right angle"),
        })?;
        let Drawing::Segment(from, to) = self.drawn.lookup(line, site.position, site.at)? else {
            return Err(Error::Malformed {
                at: site.at.clone(),
                what: format!("`{line}` names no segment"),
            });
        };
        Ok(turn(self.unit(from, to, site)?, quarters))
    }

    /// The unit vector from the point `from` toward the point `to`.
    pub(crate) fn unit(&mut self, from: Id, to: Id, site: Site<'_>) -> Result<(Lin, Lin), Error> {
        let (a, b) = (self.cite(from)?, self.cite(to)?);
        let (dx, dy) = (b.x.sub(&a.x), b.y.sub(&a.y));
        let length = distance(&dx, &dy);
        let coincide = || Error::Geometry {
            at: site.at.clone(),
            what: "a direction between two points that coincide".to_owned(),
        };
        Ok((
            dx.div(&length).ok_or_else(coincide)?,
            dy.div(&length).ok_or_else(coincide)?,
        ))
    }
}

/// The unit vector `degrees` heads along, exact at a right angle.
fn constant(degrees: Num) -> (Lin, Lin) {
    if let Some(quarters) = quarter_turns(degrees) {
        return turn((Lin::num(Num::ONE), Lin::zero()), quarters);
    }
    let [x, y] = angle::heading(degrees.value());
    (Lin::num(Num::Float(x)), Lin::num(Num::Float(y)))
}
