#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{
    Axis, Binding, Command, Doc, Grain, History, MannequinKey, MeasureSet, Placement, PointKey,
    VariableKey, block,
};

/// The seed. Fixed, so a failure names a sequence anyone can replay.
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// How many gestures the sweep makes.
const GESTURES: u32 = 400;

/// The names a gesture can carry into the status bar.
const LABELS: [&str; 4] = [
    "mover punto",
    "escribir fórmula",
    "renombrar",
    "cambiar medida",
];

/// The sources a drawn formula is written from.
const SOURCES: [&str; 6] = [
    "cintura / 4 + 1",
    "cadera / 4 + holgura_cadera",
    "raya - ancho_bajo / 2",
    "tiro - extension_tiro",
    "-extension_tiro",
    "min(cintura, cadera) / 3",
];

/// The names a drawn label is written from; they collide on purpose.
const NAMES: [&str; 4] = ["A", "B", "cadera_lat", "punto_medio"];

/// The lines a drawn label of a piece is written from.
const LINES: [&str; 4] = [
    "cortar 2 espejadas",
    "cortar 1 al doblez c.b.",
    "cortar 2 + entretela",
    "al bies - repetir",
];

struct Draw {
    state: u64,
}

impl Draw {
    fn new() -> Draw {
        Draw { state: SEED }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn below(&mut self, count: u64) -> usize {
        (self.next() % count) as usize
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.below(from.len() as u64)]
    }

    /// A number a drag would leave behind: quantised to a tenth of a
    /// millimetre, which is what the gesture rounds a delta to.
    fn number(&mut self) -> f64 {
        self.below(4_000) as f64 / 100.0 - 20.0
    }

    fn binding(&mut self) -> Binding {
        if self.below(2) == 0 {
            Binding::literal(self.number())
        } else {
            Binding::parse(self.pick(&SOURCES)).expect("the sources parse")
        }
    }
}

/// One edit the interface could emit, drawn from the ones that are built.
fn command(draw: &mut Draw, doc: &Doc) -> Command {
    let points: Vec<PointKey> = doc.points.keys().collect();
    let variables: Vec<VariableKey> = doc.variables.keys().collect();
    let bodies: Vec<MannequinKey> = doc.mannequins.keys().collect();
    let point = draw.pick(&points);
    let piece = draw.pick(&doc.piece_keys());
    match draw.below(14) {
        0 => Command::MovePoint {
            point,
            to: [draw.binding(), draw.binding()],
        },
        1 => Command::SetBinding {
            point,
            axis: if draw.below(2) == 0 { Axis::X } else { Axis::Y },
            to: draw.binding(),
        },
        2 => Command::SetVariable {
            variable: draw.pick(&variables),
            to: draw.binding(),
        },
        3 => Command::SetMeasure {
            mannequin: draw.pick(&bodies),
            name: draw.pick(&MeasureSet::CATALOGUE).to_owned(),
            to: draw.number(),
        },
        4 => Command::ResolveWith {
            mannequin: draw.pick(&bodies),
        },
        5 => Command::RenamePiece {
            piece,
            to: draw.pick(&NAMES).to_owned(),
        },
        6 => Command::SetGrain {
            piece,
            to: Grain::Angle(draw.number()),
        },
        7 => Command::LabelPoint {
            point,
            to: (draw.below(4) > 0).then(|| draw.pick(&NAMES).to_owned()),
        },
        8 => Command::ShowLabel {
            point,
            to: draw.below(2) == 0,
        },
        9 => Command::PlacePiece {
            piece,
            to: (draw.below(3) > 0).then(|| Placement::new(draw.number(), draw.number())),
        },
        // Drawn from the numbers a person types, zero and the margins of a
        // real pattern among them, so a refused edit is part of the sweep.
        10 => Command::SetSeamAllowance {
            piece,
            to: (draw.below(4) > 0).then(|| draw.below(4) as f64 / 2.0),
        },
        11 => Command::SetQuantity {
            piece,
            to: draw.below(4) as u32,
        },
        12 => Command::SetLetter {
            piece,
            to: (draw.below(4) > 0).then(|| draw.pick(&NAMES).to_owned()),
        },
        _ => Command::SetLabels {
            piece,
            to: (0..draw.below(3))
                .map(|_| draw.pick(&LINES).to_owned())
                .collect(),
        },
    }
}

/// Plays `gestures` gestures of one to four edits, counting the ones the
/// document took.
///
/// A refused edit — a name another point already shows — changes nothing and
/// is recorded nowhere, so drawing one is a draw like any other.
fn play(history: &mut History, doc: &mut Doc, draw: &mut Draw, gestures: u32) -> u32 {
    let mut taken = 0;
    for gesture in 0..gestures {
        history.begin(draw.pick(&LABELS));
        for _ in 0..=draw.below(4) {
            let edit = command(draw, doc);
            taken += u32::from(history.edit(doc, edit).is_ok());
        }
        history.end();
        if gesture % 37 == 0 {
            let saved = doc.to_canonical_json();
            let reread = Doc::from_json(&saved).expect("what the writer wrote, the reader reads");
            assert_eq!(
                reread.to_canonical_json(),
                saved,
                "after {gesture} gestures"
            );
        }
    }
    taken
}

/// The document is the formulas its author wrote, not the numbers they came
/// out as. A resolved value written back into a binding would survive an undo
/// that put the formula back, and the bytes are where that shows.
#[test]
fn a_random_command_sequence_fully_undone_yields_the_same_bytes() {
    let mut doc = block::trouser_front();
    let before = doc.to_canonical_json();
    let mut history = History::new();
    let mut draw = Draw::new();

    let taken = play(&mut history, &mut doc, &mut draw, GESTURES);
    assert!(taken > 700, "the sweep has to reach the document: {taken}");
    assert_ne!(doc.to_canonical_json(), before);

    while history.depth() > 0 {
        history.undo(&mut doc).expect("every entry undoes");
    }
    assert_eq!(doc.to_canonical_json(), before);
}

#[test]
fn redoing_everything_undone_yields_the_bytes_undo_took_away() {
    let mut doc = block::trouser_front();
    let mut history = History::new();
    let mut draw = Draw::new();

    play(&mut history, &mut doc, &mut draw, 60);
    let edited = doc.to_canonical_json();
    while history.depth() > 0 {
        history.undo(&mut doc).expect("every entry undoes");
    }
    while history.redo_depth() > 0 {
        history.redo(&mut doc).expect("every entry redoes");
    }
    assert_eq!(doc.to_canonical_json(), edited);
}

#[test]
fn a_document_saved_and_loaded_edits_into_the_same_bytes_as_the_original() {
    let mut here = block::trouser_front();
    let mut there =
        Doc::from_json(&here.to_canonical_json()).expect("the block writes and reads back");
    let mut draw = Draw::new();
    let mut other = Draw::new();
    let mut history = History::new();
    let mut carried = History::new();

    play(&mut history, &mut here, &mut draw, 40);
    play(&mut carried, &mut there, &mut other, 40);
    assert_eq!(here.to_canonical_json(), there.to_canonical_json());
}
