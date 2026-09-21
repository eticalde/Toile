use super::{Atom, Lin, Num, Rat, Shape};

impl Lin {
    /// The form as the source of a Toile formula, in the order a person
    /// writes an offset: a positive constant, the names added and then those
    /// taken away, a negative constant, whatever the form could not open up,
    /// and last whatever was computed at import. So `knh - entrepierna -
    /// hipup`, and a handle reads as its point plus how far it reaches.
    ///
    /// An exact weight prints as its decimal, or as a division when it is the
    /// reciprocal of a whole number, so that a quarter of a waist reads
    /// `cintura / 4`. A float prints as the shortest decimal that reads back
    /// to it, so the formula evaluates to the number that was computed.
    pub(crate) fn source(&self) -> String {
        let mut parts: Vec<(bool, String)> = Vec::new();
        let constant =
            (!self.exact.is_zero()).then(|| (self.exact.is_negative(), rat(self.exact.abs())));
        let signed =
            |(atom, weight): &(Atom, Num)| (weight.is_negative(), term(atom, weight.abs()));
        let names = self
            .terms
            .iter()
            .filter(|(atom, _)| atom.shape == Shape::Name);
        if let Some(positive @ (false, _)) = &constant {
            parts.push(positive.clone());
        }
        parts.extend(names.clone().filter(|(_, w)| !w.is_negative()).map(signed));
        parts.extend(names.filter(|(_, w)| w.is_negative()).map(signed));
        if let Some(negative @ (true, _)) = constant {
            parts.push(negative);
        }
        parts.extend(
            self.terms
                .iter()
                .filter(|(atom, _)| atom.shape != Shape::Name)
                .map(signed),
        );
        if self.has_float() {
            parts.push((self.float < 0.0, format!("{}", self.float.abs())));
        }
        let mut out = String::new();
        for (index, (negative, magnitude)) in parts.iter().enumerate() {
            match (index, negative) {
                (0, true) => out.push('-'),
                (0, false) => {}
                (_, true) => out.push_str(" - "),
                (_, false) => out.push_str(" + "),
            }
            out.push_str(magnitude);
        }
        if out.is_empty() {
            out.push('0');
        }
        out
    }
}

/// The form as one operand of a product: bare when it is one plain atom,
/// parenthesised otherwise.
pub(super) fn factor(lin: &Lin) -> String {
    match lin.single() {
        Some((atom, weight)) if weight.is_one() && atom.shape != Shape::Product => {
            atom.source.clone()
        }
        _ => format!("({})", lin.source()),
    }
}

/// A non-negative exact number.
fn rat(value: Rat) -> String {
    value.decimal().unwrap_or_else(|| {
        let (num, den) = value.parts();
        format!("{num} / {den}")
    })
}

/// A weighted atom, the weight non-negative.
fn term(atom: &Atom, weight: Num) -> String {
    if weight.is_one() {
        return atom.source.clone();
    }
    let atom = &atom.source;
    match weight {
        Num::Exact(value) => {
            if let Some(whole) = value.reciprocal_of_whole() {
                format!("{atom} / {whole}")
            } else if let Some(decimal) = value.decimal() {
                format!("{decimal} * {atom}")
            } else {
                let (num, den) = value.parts();
                format!("{num} * {atom} / {den}")
            }
        }
        Num::Float(value) => format!("{value} * {atom}"),
    }
}
