use std::collections::{BTreeMap, BTreeSet};

use super::HelperKind;
use super::drawn::Drawn;
use super::names::{Names, ident};
use crate::sym::{Frozen, Lin};
use crate::{Error, Evaluation, Id, Object, ObjectKind, Pattern, Place};

/// How the file places a point, as formulas.
mod construct;
/// Splines, spline paths and arcs, as cubic control points.
mod curve;
/// A Seamly formula as a linear form.
mod formula;
/// Headings as unit vectors, exact wherever the drafting squares off.
mod heading;

/// A point's two coordinates.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Coords {
    pub(crate) x: Lin,
    pub(crate) y: Lin,
}

/// A variable the translation adds to the pattern's own.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Helper {
    pub(crate) name: String,
    pub(crate) value: Lin,
    pub(crate) stands_for: HelperKind,
}

/// Where in the file a formula is read: the object citing a name sees only
/// what is drawn above it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Site<'a> {
    pub(crate) position: usize,
    pub(crate) at: &'a Place,
}

/// Turns the construction graph into formulas over the measurements and the
/// variables, one point at a time and only the points asked for.
///
/// A point whose coordinates square off is written out in full wherever it is
/// needed. One that needs a square root and that another construction builds
/// on gets its coordinates as two variables named after it, so the formulas
/// that build on it read its name instead of repeating it.
pub(crate) struct Translator<'p> {
    pub(crate) pattern: &'p Pattern,
    /// The pattern evaluated for the imported body, with integrated spline
    /// lengths: where every frozen number and every sign is read from.
    pub(crate) reference: &'p Evaluation,
    pub(crate) names: Names,
    objects: BTreeMap<Id, (usize, &'p Object)>,
    drawn: Drawn,
    definitions: BTreeMap<Id, Coords>,
    hoisted: BTreeSet<Id>,
    pub(crate) helpers: Vec<Helper>,
    /// Along-line points whose direction was read off the imported body,
    /// because the line it runs on could turn round for another.
    pub(crate) directions: Vec<Id>,
    /// Every frozen quantity, and the number it was frozen at.
    pub(crate) frozen: BTreeMap<Frozen, f64>,
}

impl<'p> Translator<'p> {
    pub(crate) fn new(pattern: &'p Pattern, reference: &'p Evaluation, names: Names) -> Self {
        Translator {
            pattern,
            reference,
            names,
            objects: pattern
                .objects()
                .enumerate()
                .map(|(position, object)| (object.id, (position, object)))
                .collect(),
            drawn: Drawn::new(pattern),
            definitions: BTreeMap::new(),
            hoisted: BTreeSet::new(),
            helpers: Vec::new(),
            directions: Vec::new(),
            frozen: BTreeMap::new(),
        }
    }

    /// The construction object `id`, and where in the file it sits.
    pub(crate) fn object(&self, id: Id) -> Result<(usize, &'p Object), Error> {
        self.objects
            .get(&id)
            .copied()
            .ok_or_else(|| Error::Product(format!("no construction object has id {id}")))
    }

    /// The name of the point `id`.
    pub(crate) fn point_name(&self, id: Id) -> Result<&'p str, Error> {
        self.object(id)?
            .1
            .name()
            .ok_or_else(|| Error::Product(format!("object {id} is not a point")))
    }

    /// A point's coordinates written out in full: how the file places it.
    pub(crate) fn define(&mut self, id: Id) -> Result<Coords, Error> {
        if let Some(known) = self.definitions.get(&id) {
            return Ok(known.clone());
        }
        let (position, object) = self.object(id)?;
        let ObjectKind::Point { construction, .. } = &object.kind else {
            return Err(Error::Product(format!("object {id} is not a point")));
        };
        let site = Site {
            position,
            at: &object.at,
        };
        let coords = self.construct(id, construction, site)?;
        self.definitions.insert(id, coords.clone());
        Ok(coords)
    }

    /// A point's coordinates as another construction reads them: in full
    /// when they square off, through the point's own variables otherwise.
    pub(crate) fn cite(&mut self, id: Id) -> Result<Coords, Error> {
        let coords = self.define(id)?;
        if coords.x.is_linear() && coords.y.is_linear() {
            return Ok(coords);
        }
        let point = self.point_name(id)?;
        let name = ident(point).ok_or_else(|| {
            Error::Product(format!(
                "the point `{point}` has no spelling a Toile formula can read"
            ))
        })?;
        let (x, y) = (format!("{name}_x"), format!("{name}_y"));
        if self.hoisted.insert(id) {
            let stands_for = HelperKind::Coordinate(point.to_owned());
            self.add_helper(x.clone(), coords.x.clone(), stands_for.clone())?;
            self.add_helper(y.clone(), coords.y.clone(), stands_for)?;
        }
        Ok(Coords {
            x: named_like(&x, &coords.x),
            y: named_like(&y, &coords.y),
        })
    }

    /// A contour node's coordinates, once every construction has been read:
    /// through its variables if something else needed them, in full if not.
    pub(crate) fn node(&mut self, id: Id) -> Result<Coords, Error> {
        if self.hoisted.contains(&id) {
            self.cite(id)
        } else {
            self.define(id)
        }
    }

    /// The variable `name`, added with `value` unless it already is.
    pub(crate) fn add_helper(
        &mut self,
        name: String,
        value: Lin,
        stands_for: HelperKind,
    ) -> Result<Lin, Error> {
        if let Some(known) = self.helpers.iter().find(|helper| helper.name == name) {
            return Ok(named_like(&name, &known.value));
        }
        if self.names.variable_spelled(&name) || self.names.measurement_spelled(&name) {
            return Err(Error::Product(format!(
                "the translation needs a variable `{name}`, which the pattern already names"
            )));
        }
        let read = named_like(&name, &value);
        self.helpers.push(Helper {
            name,
            value,
            stands_for,
        });
        Ok(read)
    }

    /// How many construction points have been written out as formulas.
    pub(crate) fn defined(&self) -> usize {
        self.definitions.len()
    }

    /// The variable `name`, if the translation has already added it.
    pub(crate) fn helper(&self, name: &str) -> Option<Lin> {
        self.helpers
            .iter()
            .find(|helper| helper.name == name)
            .map(|helper| named_like(name, &helper.value))
    }
}

/// A variable read by its name, depending on whatever its value depends on.
fn named_like(name: &str, value: &Lin) -> Lin {
    value
        .frozen()
        .iter()
        .fold(Lin::name(name), |lin, &frozen| lin.frozen_by(frozen))
}

/// The distance a difference of two points covers.
///
/// Along an axis it is the absolute value of the one difference, not the
/// root of its square: the same number, written as a draftsman would.
pub(crate) fn distance(dx: &Lin, dy: &Lin) -> Lin {
    if dy.is_zero() {
        dx.abs()
    } else if dx.is_zero() {
        dy.abs()
    } else {
        dx.square()
            .add(&dy.square())
            .sqrt()
            .expect("a sum of squares is never negative")
    }
}
