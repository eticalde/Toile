use toile_engine::draft::MeasureSet;

use crate::tabs::UNNAMED;

/// The body no history keeps: the one the tab shapes while no product is open,
/// carrying its own way back from the press that replaces it.
///
/// The body and that way back are one thing, so the field holding the body is
/// private to this module and no sibling can reach around [`Loose::write`].
/// Nothing saves this body: every write into it is a write the step back
/// discards, and one that arrived by another route would be discarded in
/// silence by a panel that had promised otherwise.
#[derive(Debug)]
pub(super) struct Loose {
    set: MeasureSet,
    replaced: Option<Replaced>,
}

/// The way back from the press that put another body in the loose one's place.
///
/// With no product on the table there is no history to enter, so the one
/// gesture that throws a whole body away — a name, its tape and its shape —
/// carries its own single step back. It is not a history: nothing steps
/// forward from here, because the way forward is the button that was pressed,
/// still under the pointer.
#[derive(Debug)]
struct Replaced {
    /// The loose body as it stood before the press.
    was: MeasureSet,
    /// Whether the body that took its place has been written into since, which
    /// is what the step back would throw away in turn.
    touched: bool,
}

impl Loose {
    /// The body on the stand, with no press behind it yet.
    pub(super) fn new(set: MeasureSet) -> Loose {
        Loose {
            set,
            replaced: None,
        }
    }

    /// The body, to read.
    pub(super) fn body(&self) -> &MeasureSet {
        &self.set
    }

    /// The body, opened for writing, noting that whatever the last press put
    /// here has been written into.
    ///
    /// The step back discards every write alike, whatever it wrote, so the
    /// sentence offering it has to warn of every write alike.
    pub(super) fn write(&mut self) -> &mut MeasureSet {
        if let Some(back) = self.replaced.as_mut() {
            back.touched = true;
        }
        &mut self.set
    }

    /// Puts `set` where the body stood, keeping the way back from it.
    ///
    /// Asking first was the other shape this could take, and it is the worse
    /// one: a question cannot tell a body somebody measured from the untouched
    /// default, so it would have to be asked over both, and a question that is
    /// right to wave through is a question that gets waved through. Kept
    /// reversible instead, the press costs one keystroke to take back, and the
    /// panel can name the keystroke.
    pub(super) fn replace(&mut self, set: MeasureSet) {
        let was = std::mem::replace(&mut self.set, set);
        self.replaced = Some(Replaced {
            was,
            touched: false,
        });
    }

    /// Takes the one step back the last press left, if it left one.
    pub(super) fn back(&mut self) {
        if let Some(back) = self.replaced.take() {
            self.set = back.was;
        }
    }

    /// Drops the way back without taking it.
    pub(super) fn forget(&mut self) {
        self.replaced = None;
    }

    /// What the panel says of the press that last replaced the body, and of the
    /// step back still open from it.
    ///
    /// The names are read now rather than kept from the press: whatever the
    /// two bodies are called on this frame is what the sentence has to name.
    pub(super) fn said(&self) -> Option<String> {
        let back = self.replaced.as_ref()?;
        let (was, now) = (named(&back.was), named(&self.set));
        let cost = if back.touched {
            ", y descarta lo que hayas cambiado desde entonces"
        } else {
            ""
        };
        Some(format!(
            "«{now}» sustituye a «{was}», que no se guardaba en ningún sitio. Cmd+Z devuelve a \
             «{was}»{cost}."
        ))
    }
}

/// A body's name as a sentence has to say it: a body nobody named is named the
/// way every panel names it, not left as a gap between two quotes.
fn named(set: &MeasureSet) -> &str {
    let name = set.name.trim();
    if name.is_empty() { UNNAMED } else { name }
}
