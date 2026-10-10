use eframe::egui::{self, Id};
use toile_engine::draft::{Command, HangKey, Heading, Sense};

use super::super::super::entry;
use super::super::super::wire::Verb;
use super::super::elastic::press;
use super::BOX_W;
use crate::theme::Theme;
use crate::widgets::{PAD, cycle_named, footer_note, plain_note};

const NONE: &str = "Sin rumbo, la prenda se suelta mirando a donde apunte la pieza que el \
                    documento guarde primero. Dar rumbo es decir «este canto es el centro \
                    delantero», y entonces el orden del archivo deja de decidirlo.";
const HELD: &str = "La cabeza del tramo se clava ahí, y la prenda gira entera con ella. El \
                    sentido dice hacia qué lado del cuerpo sigue el tramo desde ese punto: sin \
                    él, un ángulo solo deja que la prenda salga espejada.";
const MUTE: &str = "Este tramo no dice hacia qué lado del cuerpo sigue la prenda, así que no hay \
                    rumbo que darle. Un canto vertical sí lo dice —la tela está de un lado y la \
                    prenda sigue hacia ahí—, y este tramo tiene tanta tela a un lado como al \
                    otro, o no llega a dos puntos de la tela.";
const DEAD: &str = "El documento guarda un rumbo aquí, y la colocación no lo usa: este tramo no \
                    dice hacia qué lado sigue la prenda. Quitarlo deja dicho lo que pasa.";

const TURN: &str = "cambiar el rumbo del colgado";
const SENSE: &str = "cambiar el sentido del colgado";
const PUT_ON: &str = "dar rumbo al colgado";
const TAKE_OFF: &str = "quitarle el rumbo al colgado";

/// Which way round the body the chosen tract faces, and the ways to change it.
///
/// Nothing is written by looking, by the rule the rest of this panel follows: a
/// hang with no heading offers the press that gives it one and writes nothing
/// until it, and a document opened and read keeps its bytes and its version.
///
/// `mute` is the engine's answer about this tract: a tract no heading could
/// turn gets the reason and no press, by the rule the greyed tiles of
/// `tools.rs` follow. The reading is the placement's own and not one taken
/// here, so the panel cannot come to disagree with what the cloth does.
pub(super) fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    (key, heading): (HangKey, Option<Heading>),
    mute: bool,
    verbs: &mut Vec<Verb>,
) {
    match (heading, mute) {
        // A heading the document holds over a tract that cannot be turned: the
        // boxes would step a declaration nothing reads, so what is drawn is
        // the reason and the press that takes it off.
        (Some(_), true) => {
            take_off(ui, theme, key, verbs);
            footer_note(ui, theme, DEAD);
        }
        (Some(heading), false) => faced(ui, theme, (key, heading), verbs),
        (None, true) => plain_note(ui, theme, MUTE),
        (None, false) => offer(ui, theme, key, verbs),
    }
}

/// What a hang with no heading shows: what one would do, and the press.
///
/// It goes on at the centre front running toward the wearer's left, which is
/// the sentence a person is most often about to write — the edge they chose is
/// the opening of the garment — and the two boxes under it reach every other.
fn offer(ui: &mut egui::Ui, theme: &Theme, key: HangKey, verbs: &mut Vec<Verb>) {
    if press(ui, theme, put_on_id(key), "Dar rumbo") {
        verbs.extend(entry(
            PUT_ON,
            Command::SetHangHeading {
                hang: key,
                to: Some(Heading::facing(0.0, Sense::Leftward)),
            },
        ));
    }
    footer_note(ui, theme, NONE);
}

/// The heading the hang already carries: where it is pinned, which way it runs
/// on, and the press that takes it off.
fn faced(
    ui: &mut egui::Ui,
    theme: &Theme,
    (key, heading): (HangKey, Heading),
    verbs: &mut Vec<Verb>,
) {
    stepped(ui, theme, key, heading, verbs);
    take_off(ui, theme, key, verbs);
    footer_note(ui, theme, HELD);
}

/// The press that takes a heading off, and what it writes.
fn take_off(ui: &mut egui::Ui, theme: &Theme, key: HangKey, verbs: &mut Vec<Verb>) {
    if press(ui, theme, take_off_id(key), "Quitar el rumbo") {
        verbs.extend(entry(
            TAKE_OFF,
            Command::SetHangHeading {
                hang: key,
                to: None,
            },
        ));
    }
}

/// The two boxes, and what each press writes.
fn stepped(
    ui: &mut egui::Ui,
    theme: &Theme,
    key: HangKey,
    heading: Heading,
    verbs: &mut Vec<Verb>,
) {
    if step(ui, theme, turn_id(key), ("rumbo", shown(heading.turn))) {
        verbs.extend(entry(
            TURN,
            Command::SetHangHeading {
                hang: key,
                to: Some(Heading {
                    turn: next(heading.turn),
                    ..heading
                }),
            },
        ));
    }
    if step(ui, theme, sense_id(key), ("sentido", side(heading.sense))) {
        verbs.extend(entry(
            SENSE,
            Command::SetHangHeading {
                hang: key,
                to: Some(Heading {
                    sense: other(heading.sense),
                    ..heading
                }),
            },
        ));
    }
}

/// One box that steps to the next value, and whether it was pressed.
fn step(ui: &mut egui::Ui, theme: &Theme, id: Id, (caption, value): (&str, String)) -> bool {
    ui.add_space(6.0);
    let pressed = ui
        .horizontal(|ui| {
            ui.add_space(PAD);
            cycle_named(ui, theme, id, caption, &value, BOX_W).clicked()
        })
        .inner;
    ui.add_space(4.0);
    pressed
}

/// A turn as a person reads it: the name of the trade where it is one of the
/// four, and the degrees it really is where it is not.
///
/// The four names are stops over the continuous datum, not a replacement for
/// it: a file may hold any turn, and a turn with no name is shown as the number
/// it is rather than rounded to the nearest name it is not.
fn shown(turn: f64) -> String {
    Heading::QUARTERS
        .iter()
        .find(|&&(_, at)| at.to_bits() == turn.to_bits())
        .map_or_else(|| format!("{turn:.1}°"), |&(name, _)| name.to_owned())
}

/// The turn a press steps to: the next of the four, and the first for a turn
/// that is none of them.
fn next(turn: f64) -> f64 {
    let at = Heading::QUARTERS
        .iter()
        .position(|&(_, one)| one.to_bits() == turn.to_bits());
    let along = at.map_or(0, |at| (at + 1) % Heading::QUARTERS.len());
    Heading::QUARTERS[along].1
}

/// A sense in the words of the body it is about.
fn side(sense: Sense) -> String {
    match sense {
        Sense::Leftward => "hacia la izquierda".to_owned(),
        Sense::Rightward => "hacia la derecha".to_owned(),
    }
}

/// The sense a press steps to, which is the other one: there are two.
fn other(sense: Sense) -> Sense {
    match sense {
        Sense::Leftward => Sense::Rightward,
        Sense::Rightward => Sense::Leftward,
    }
}

/// The identity of the press that gives a hang a heading.
pub(in crate::tabs::patronaje) fn put_on_id(at: HangKey) -> Id {
    Id::new(("patronaje-hang-heading-on", at))
}

/// The identity of the box that steps the turn.
pub(in crate::tabs::patronaje) fn turn_id(at: HangKey) -> Id {
    Id::new(("patronaje-hang-turn", at))
}

/// The identity of the box that steps the sense.
pub(in crate::tabs::patronaje) fn sense_id(at: HangKey) -> Id {
    Id::new(("patronaje-hang-sense", at))
}

/// The identity of the press that takes a heading off.
pub(in crate::tabs::patronaje) fn take_off_id(at: HangKey) -> Id {
    Id::new(("patronaje-hang-heading-off", at))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stepping the turn reaches all four names of the trade and closes, so
    /// every stop a person can press is one the document takes.
    #[test]
    fn the_step_reaches_every_one_of_the_four_names_and_closes_the_ring() {
        let mut turn = Heading::QUARTERS[0].1;
        let mut seen = vec![shown(turn)];
        for _ in 0..Heading::QUARTERS.len() {
            turn = next(turn);
            if turn.to_bits() == Heading::QUARTERS[0].1.to_bits() {
                break;
            }
            seen.push(shown(turn));
        }
        assert_eq!(turn, Heading::QUARTERS[0].1, "the ring closes");
        assert_eq!(
            seen,
            [
                "centro delantero",
                "costado izquierdo",
                "centro espalda",
                "costado derecho"
            ]
        );
    }

    /// A turn that is none of the four is shown as the number it is and steps
    /// to the first name, rather than being rounded to a name it is not.
    #[test]
    fn a_turn_with_no_name_is_shown_as_degrees_and_steps_to_the_first_name() {
        assert_eq!(shown(-37.5), "-37.5°");
        assert_eq!(next(-37.5), 0.0);
        assert_eq!(shown(0.0), "centro delantero");
    }

    /// There are two senses, so one press reaches the other and a second comes
    /// back.
    #[test]
    fn stepping_the_sense_twice_comes_back() {
        assert_eq!(other(Sense::Leftward), Sense::Rightward);
        assert_eq!(other(other(Sense::Leftward)), Sense::Leftward);
        assert_eq!(side(Sense::Leftward), "hacia la izquierda");
    }
}
