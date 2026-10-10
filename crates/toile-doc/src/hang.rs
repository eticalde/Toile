use serde::{Deserialize, Serialize};

use crate::{DocError, EdgeRange, MeasureSet};

mod heading;

pub use heading::{Heading, Sense};

/// A stretch of one piece's contour hung from a station of the measurement
/// catalogue: the line the garment hangs from, said on the body.
///
/// An elastic says how hard a waistband squeezes and says nothing about where
/// on the body that waistband belongs. Measured, a band grips its waist and
/// then travels down the legs keeping its own girth until the body is that
/// size again, and no stiffness or friction this tree can spell stops it. This
/// is the other half of holding a garment on.
///
/// It sits on a stretch of contour for the reason an elastic does — a waistband
/// is a length of one edge, not a join between two pieces and not the whole
/// cloth — and it names the body by a measurement rather than by a coordinate,
/// because a measurement is the one thing about a body the document can hold
/// without knowing which body it will be worn on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hang {
    /// The stretch of contour it holds up.
    ///
    /// An `EdgeRange` for the reason an elastic's is one: its ends are node
    /// keys with a fraction local to the tract leaving them, so a node inserted
    /// inside the stretch moves neither end.
    pub at: EdgeRange,
    /// The catalogue girth naming the ring of the body it hangs from.
    pub station: String,
    /// Which way round that ring the run faces; `None` for a run that says
    /// where it hangs from and nothing about where it points.
    ///
    /// Optional because a height and a heading are two different sentences and
    /// a person may have only the first to say. It is also what keeps every
    /// file already on disk as it was: a hang written before this existed reads
    /// back as a hang with no heading, and saves as the bytes it was read from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<Heading>,
}

impl Hang {
    /// The station a waistband hangs from.
    ///
    /// A constant because it is the station every fixture and every drape scene
    /// in this tree starts at, and a name mistyped by a letter is a garment
    /// hung from nothing at all.
    pub const WAIST: &'static str = "cintura";

    /// A stretch hung from `station`, pointing nowhere in particular.
    pub fn new(at: EdgeRange, station: &str) -> Hang {
        Hang {
            at,
            station: station.to_owned(),
            heading: None,
        }
    }

    /// The same stretch, turned to face `heading`.
    pub fn facing(at: EdgeRange, station: &str, heading: Heading) -> Hang {
        Hang {
            heading: Some(heading),
            ..Hang::new(at, station)
        }
    }

    /// Whether a name is one the body carries a ring for.
    ///
    /// Only a girth is: a length runs down the body and names no height to hold
    /// cloth at, and a name outside the catalogue names nothing on any body.
    pub fn names_a_ring(station: &str) -> bool {
        MeasureSet::GIRTHS.contains(&station)
    }

    /// Refuses a run no one piece answers for, and a station no body carries a
    /// ring for.
    ///
    /// The catalogue guides a measure set and rules here, and the difference is
    /// the point. A person may tape a measurement Toile does not know and keep
    /// it, because the tape is theirs; a garment hung from a name with no ring
    /// behind it hangs from nothing, and whoever read the file would have to
    /// invent a height or drop the hang in silence.
    pub(crate) fn check(&self) -> Result<(), DocError> {
        if self.at.piece().is_none() {
            return Err(DocError::SplitHang);
        }
        if !Hang::names_a_ring(&self.station) {
            return Err(DocError::HangStation(self.station.clone()));
        }
        self.heading.map_or(Ok(()), |heading| heading.check())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PieceKey, PointKey};

    fn range() -> EdgeRange {
        EdgeRange::between(
            PieceKey::new(0, 0),
            PointKey::new(0, 0),
            PointKey::new(1, 0),
        )
    }

    #[test]
    fn a_hang_names_the_stretch_and_the_ring_it_hangs_from() {
        let hang = Hang::new(range(), Hang::WAIST);
        assert_eq!(hang.check(), Ok(()));
        assert_eq!(hang.at, range());
        assert_eq!(hang.station, "cintura");
    }

    /// Every ring the body carries is a station a garment may hang from, so a
    /// girth added to the catalogue is offered here without a second list to
    /// keep in step.
    #[test]
    fn every_catalogue_girth_is_a_station_and_nothing_else_is() {
        for girth in MeasureSet::GIRTHS {
            assert_eq!(Hang::new(range(), girth).check(), Ok(()), "{girth}");
        }
        for name in MeasureSet::LENGTHS.iter().chain(&MeasureSet::WHOLE) {
            assert_eq!(
                Hang::new(range(), name).check(),
                Err(DocError::HangStation((*name).to_owned())),
                "{name}"
            );
        }
    }

    /// A station outside the catalogue is refused with them, and the error
    /// carries the name so whoever typed it is told which one it was.
    #[test]
    fn a_station_the_catalogue_does_not_name_is_refused_by_name() {
        for station in ["", "waist", "CINTURA", "largo_manga"] {
            assert_eq!(
                Hang::new(range(), station).check(),
                Err(DocError::HangStation(station.to_owned())),
                "{station}"
            );
            assert!(!Hang::names_a_ring(station));
        }
    }
}
