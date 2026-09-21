use std::collections::BTreeMap;

use crate::{Error, Id, Place};

/// What an id names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Point,
    Line,
    Spline,
    SplinePath,
    Arc,
    Copy,
    InternalPath,
    Piece,
}

impl Kind {
    fn noun(self) -> &'static str {
        match self {
            Self::Point => "a point",
            Self::Line => "a line",
            Self::Spline => "a spline",
            Self::SplinePath => "a spline path",
            Self::Arc => "an arc",
            Self::Copy => "a modeling copy",
            Self::InternalPath => "an internal path",
            Self::Piece => "a piece",
        }
    }
}

/// Every id read so far. The file is read in order and every reference is
/// checked here, so a graph that parses cites nothing below its citer and
/// nothing of the wrong kind.
#[derive(Debug, Default)]
pub(crate) struct Index {
    kinds: BTreeMap<Id, Kind>,
    copies: BTreeMap<Id, (Id, Kind)>,
}

fn malformed(at: &Place, what: String) -> Error {
    Error::Malformed {
        at: at.clone(),
        what,
    }
}

impl Index {
    pub(crate) fn insert(&mut self, id: Id, kind: Kind, at: &Place) -> Result<(), Error> {
        match self.kinds.insert(id, kind) {
            Some(_) => Err(malformed(at, format!("id {id} is used twice"))),
            None => Ok(()),
        }
    }

    /// `id`, if it names `kind` above this point.
    pub(crate) fn expect(&self, id: Id, kind: Kind, at: &Place) -> Result<Id, Error> {
        match self.kinds.get(&id) {
            Some(&found) if found == kind => Ok(id),
            Some(&found) => Err(malformed(
                at,
                format!("id {id} is {}, not {}", found.noun(), kind.noun()),
            )),
            None => Err(malformed(at, format!("id {id} is not defined above"))),
        }
    }

    /// Records `copy` as a modeling copy of `original`, which must be `kind`.
    pub(crate) fn add_copy(
        &mut self,
        copy: Id,
        original: Id,
        kind: Kind,
        at: &Place,
    ) -> Result<(), Error> {
        self.expect(original, kind, at)?;
        self.insert(copy, Kind::Copy, at)?;
        self.copies.insert(copy, (original, kind));
        Ok(())
    }

    /// The construction object `copy` stands for, if it copies `kind`.
    pub(crate) fn original(&self, copy: Id, kind: Kind, at: &Place) -> Result<Id, Error> {
        match self.copies.get(&copy) {
            Some(&(original, found)) if found == kind => Ok(original),
            Some(&(_, found)) => Err(malformed(
                at,
                format!("id {copy} copies {}, not {}", found.noun(), kind.noun()),
            )),
            None => Err(malformed(
                at,
                format!("id {copy} is not a modeling copy defined above"),
            )),
        }
    }
}
