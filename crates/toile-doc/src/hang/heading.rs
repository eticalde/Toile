use serde::{Deserialize, Serialize};

use crate::DocError;

/// Which way round the body a hung run travels from the point it is pinned at.
///
/// Two words and not a `±1`, because a heading already carries one signed
/// number and the two would read alike on the page and in the file: −45 is a
/// heading and −1 is a sense. Nor is it the angle said twice. An angle pins one
/// line of cloth and leaves the rest of the garment free to go round either
/// way, which is how a buttoned blouse comes out mirrored with every panel
/// running against the turn — measured, 3 of 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sense {
    /// Toward the left of whoever wears the garment.
    Leftward,
    /// Toward their right.
    Rightward,
}

/// Which way round the body a hung run faces: which of its own points is
/// pinned, the turn it is pinned at, and which way the run goes on from there.
///
/// What it replaces is the document's storage order. With nothing declared the
/// turn opens at the middle of whichever panel the walk reaches first, so the
/// piece a file happens to hold first puts its own middle at the front:
/// measured on one three-panel blouse, 44.6° round with a front stored first
/// and 180.0° — the garment back to front — with the back.
///
/// An angle and not metres of cloth, which is the whole of why the surface
/// survives it: a turn rotates the surface rigidly and every meridian stays a
/// meridian. A shift measured in cloth would come to a different angle on
/// every hoop — 1.0400 m of it at the chest against 1.2570 m at the hip — and
/// the centre front would spiral downwards.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Heading {
    /// Which point of the run itself is pinned: 0 its head, 1 its tail.
    pub pin: f64,
    /// Degrees from the centre front, growing toward the left of whoever wears
    /// it, in (-180, 180].
    pub turn: f64,
    /// Which way the run travels from the pin.
    pub sense: Sense,
}

impl Heading {
    /// Where the pin sits when nobody moves it: the head of the run.
    ///
    /// The head and not the middle, and it was the middle that lost. A run's
    /// middle moves with the width of the cloth either side of it — one back
    /// read 46.0° as drawn, 48.8° with ease added to the fronts and 44.1° with
    /// the same ease added to itself — so the same declaration means a
    /// different place on the body every time a panel is redrafted. Its head
    /// says the sentence of the trade, "this edge is the centre front", and on
    /// a piece drawn against a fold the head of the run *is* the crease.
    pub const HEAD: f64 = 0.0;

    /// The four places of the trade, and the turn each one is.
    ///
    /// Stops over the continuous datum rather than a replacement for it: what
    /// the file holds is a number, and these are the four a person steps
    /// through to reach the ones they mean. Taking them off the body's own ring
    /// instead was measured and dropped — quartering the upper chest girth puts
    /// the centre front at +37.4° and the centre back at −145.5°, so a person
    /// pressing "centro espalda" would get neither the back of this body nor
    /// the same place on the next one.
    pub const QUARTERS: [(&'static str, f64); 4] = [
        ("centro delantero", 0.0),
        ("costado izquierdo", 90.0),
        ("centro espalda", 180.0),
        ("costado derecho", -90.0),
    ];

    /// A run whose head is pinned at `turn` degrees and runs on `sense`-ward.
    pub fn facing(turn: f64, sense: Sense) -> Heading {
        Heading {
            pin: Heading::HEAD,
            turn,
            sense,
        }
    }

    /// The turn in radians, which is what a placement rolls cloth by.
    pub fn radians(&self) -> f64 {
        self.turn.to_radians()
    }

    /// `1.0` when the run goes toward the wearer's left, `-1.0` when it goes to
    /// their right.
    ///
    /// The sign the angle grows in: a turn read from the centre front grows
    /// leftward, so a run declared leftward walks the cloth the way the turn
    /// walks and one declared rightward walks it against.
    pub fn leftward(&self) -> f64 {
        match self.sense {
            Sense::Leftward => 1.0,
            Sense::Rightward => -1.0,
        }
    }

    /// Refuses a pin that names no point of its run, and a turn outside the one
    /// lap there is.
    ///
    /// Both reach a placement as an angle to roll the cloth by, so neither has
    /// anywhere to fall back to: a pin of 2 names cloth the run does not cover
    /// and a turn of 400 names a place three quarters round from where it
    /// reads.
    pub(crate) fn check(&self) -> Result<(), DocError> {
        if !(0.0..=1.0).contains(&self.pin) {
            return Err(DocError::HangPin);
        }
        if !(self.turn > -180.0 && self.turn <= 180.0) {
            return Err(DocError::HangTurn);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heading_pins_the_head_of_its_run_at_the_turn_it_is_given() {
        let front = Heading::facing(0.0, Sense::Leftward);
        assert_eq!(front.pin, Heading::HEAD);
        assert_eq!(front.check(), Ok(()));
        assert_eq!(front.radians(), 0.0);
        assert_eq!(front.leftward(), 1.0);
        assert_eq!(Heading::facing(0.0, Sense::Rightward).leftward(), -1.0);
    }

    /// Every one of the four names a person presses is a turn the document
    /// takes, so a stop cannot offer a heading the file refuses.
    #[test]
    fn each_of_the_four_places_of_the_trade_is_a_turn_the_document_accepts() {
        for (name, turn) in Heading::QUARTERS {
            let heading = Heading::facing(turn, Sense::Leftward);
            assert_eq!(heading.check(), Ok(()), "{name}");
        }
        let turns: Vec<f64> = Heading::QUARTERS.iter().map(|&(_, turn)| turn).collect();
        assert_eq!(turns, [0.0, 90.0, 180.0, -90.0], "front, left, back, right");
    }

    /// The centre back is 180 and not −180, which is the one place the
    /// half-open range decides: both spell the same turn, and a file with
    /// two spellings for one place has two documents for one garment.
    #[test]
    fn the_lap_is_half_open_so_the_centre_back_has_one_spelling() {
        assert_eq!(Heading::facing(180.0, Sense::Leftward).check(), Ok(()));
        for turn in [-180.0, 180.001, -180.001, 360.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                Heading::facing(turn, Sense::Leftward).check(),
                Err(DocError::HangTurn),
                "{turn}"
            );
        }
    }

    #[test]
    fn a_pin_outside_its_own_run_is_refused() {
        for pin in [0.0, 0.5, 1.0] {
            let heading = Heading {
                pin,
                ..Heading::facing(0.0, Sense::Leftward)
            };
            assert_eq!(heading.check(), Ok(()), "{pin}");
        }
        for pin in [-0.001, 1.001, f64::NAN, f64::NEG_INFINITY] {
            let heading = Heading {
                pin,
                ..Heading::facing(0.0, Sense::Leftward)
            };
            assert_eq!(heading.check(), Err(DocError::HangPin), "{pin}");
        }
    }
}
