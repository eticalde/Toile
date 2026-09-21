use super::super::HelperKind;
use super::super::drawn::Drawing;
use super::super::names::ident;
use super::{Site, Translator, distance};
use crate::eval::env::{ANGLE_LINE, CURRENT_LENGTH, LINE, SPLINE};
use crate::sym::{Lin, Num};
use crate::{Error, Expr, Formula, Op};

impl Translator<'_> {
    /// A Seamly formula as a linear form over the measurements and the
    /// variables, `current` standing for `CurrentLength` where there is one.
    pub(crate) fn lin(
        &mut self,
        formula: &Formula,
        site: Site<'_>,
        current: Option<&Lin>,
    ) -> Result<Lin, Error> {
        self.expr(formula.expr(), site, current)
    }

    fn expr(&mut self, expr: &Expr, site: Site<'_>, current: Option<&Lin>) -> Result<Lin, Error> {
        Ok(match expr {
            Expr::Num(value) => Lin::num(Num::of_f64(*value)),
            Expr::Name(name) => self.cited(name, site, current)?,
            Expr::Neg(inner) => self.expr(inner, site, current)?.neg(),
            Expr::Bin(op, lhs, rhs) => {
                let lhs = self.expr(lhs, site, current)?;
                let rhs = self.expr(rhs, site, current)?;
                match op {
                    Op::Add => lhs.add(&rhs),
                    Op::Sub => lhs.sub(&rhs),
                    Op::Mul => lhs.mul(&rhs),
                    Op::Div => lhs.div(&rhs).ok_or_else(|| Error::Geometry {
                        at: site.at.clone(),
                        what: "a division by zero".to_owned(),
                    })?,
                }
            }
        })
    }

    /// What a name in a formula reads, in the order the evaluator looks for
    /// it: a drawing, a variable, a measurement.
    fn cited(&mut self, name: &str, site: Site<'_>, current: Option<&Lin>) -> Result<Lin, Error> {
        if let (CURRENT_LENGTH, Some(length)) = (name, current) {
            return Ok(length.clone());
        }
        if name.starts_with(LINE) || name.starts_with(SPLINE) {
            return self.drawn_length(name, site);
        }
        if name.starts_with(ANGLE_LINE) {
            return Err(Error::Unsupported {
                at: site.at.clone(),
                what: format!("`{name}`, a heading, used as a length"),
            });
        }
        if name.starts_with('#') {
            let toile = self.names.variable(name).ok_or_else(|| Error::Malformed {
                at: site.at.clone(),
                what: format!("`{name}` is no variable of the pattern"),
            })?;
            return Ok(Lin::name(toile));
        }
        Ok(Lin::name(&self.names.measurement(name)?))
    }

    /// A `Line_` or `Spl_` name, read through a variable of the same name so
    /// that the formula that cites it still reads like the author's.
    fn drawn_length(&mut self, name: &str, site: Site<'_>) -> Result<Lin, Error> {
        let helper = ident(name).ok_or_else(|| {
            Error::Product(format!("`{name}` has no spelling a Toile formula can read"))
        })?;
        if let Some(known) = self.helper(&helper) {
            return Ok(known);
        }
        let value = match self.drawn.lookup(name, site.position, site.at)? {
            Drawing::Segment(a, b) => {
                let (a, b) = (self.cite(a)?, self.cite(b)?);
                distance(&b.x.sub(&a.x), &b.y.sub(&a.y))
            }
            Drawing::Spline(spline) => self.spline_length(spline)?,
        };
        self.add_helper(helper, value, HelperKind::Drawn(name.to_owned()))
    }
}
