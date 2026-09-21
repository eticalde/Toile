use super::{Lin, Num, Shape, render};

impl Lin {
    /// The weight and the rest, when the form is one weighted atom; else one
    /// and the whole form.
    fn split(&self) -> (Num, Lin) {
        match self.single() {
            Some((atom, weight)) => (weight, Lin::atom(atom.shape.clone(), atom.source.clone())),
            None => (Num::ONE, self.clone()),
        }
    }

    pub(crate) fn mul(&self, other: &Lin) -> Lin {
        if let Some(by) = self.constant() {
            return other.scale(by).with_frozen(self, other);
        }
        if let Some(by) = other.constant() {
            return self.scale(by).with_frozen(self, other);
        }
        let ((ka, a), (kb, b)) = (self.split(), other.split());
        let product = if a == b {
            let base = a.source();
            Lin::atom(Shape::Square(base), format!("{}^2", render::factor(&a)))
        } else {
            let mut factors = [render::factor(&a), render::factor(&b)];
            factors.sort();
            Lin::atom(Shape::Product, factors.join(" * "))
        };
        product.scale(ka.mul(kb)).with_frozen(self, other)
    }

    /// The quotient; `None` for a divisor that is exactly zero.
    pub(crate) fn div(&self, other: &Lin) -> Option<Lin> {
        if let Some(by) = other.constant() {
            let inverse = Num::ONE.div(by)?;
            return Some(self.scale(inverse).with_frozen(self, other));
        }
        if let Some(ratio) = self.ratio_to(other) {
            return Some(Lin::num(ratio).with_frozen(self, other));
        }
        let ((ka, a), (kb, b)) = (self.split(), other.split());
        let top = match a.single() {
            Some((atom, _)) if atom.shape == Shape::Product => atom.source.clone(),
            _ => render::factor(&a),
        };
        let quotient = Lin::atom(Shape::Product, format!("{top} / {}", render::factor(&b)));
        Some(quotient.scale(ka.div(kb)?).with_frozen(self, other))
    }

    /// The number this form is of `other`, when it is `other` scaled.
    fn ratio_to(&self, other: &Lin) -> Option<Num> {
        let (first, weight) = other.terms.first()?;
        let ratio = self
            .terms
            .first()
            .filter(|(atom, _)| atom == first)?
            .1
            .div(*weight)?;
        let terms_agree = self.terms.len() == other.terms.len()
            && self
                .terms
                .iter()
                .zip(&other.terms)
                .all(|((a, wa), (b, wb))| a == b && *wa == wb.mul(ratio));
        let constants_agree = Num::Exact(self.exact) == Num::Exact(other.exact).mul(ratio)
            && Num::Float(self.float) == Num::Float(other.float).mul(ratio);
        (terms_agree && constants_agree).then_some(ratio)
    }

    /// The root; `None` for a negative number. The root of a weighted square
    /// is the absolute value of what was squared.
    pub(crate) fn sqrt(&self) -> Option<Lin> {
        if let Some(value) = self.constant() {
            return Some(Lin::num(value.sqrt()?).with_frozen(self, self));
        }
        if let Some((atom, weight)) = self.single()
            && let Shape::Square(base) = &atom.shape
        {
            let root = Lin::atom(Shape::Call, format!("abs({base})"));
            return Some(root.scale(weight.sqrt()?).with_frozen(self, self));
        }
        Some(Lin::atom(Shape::Call, format!("sqrt({})", self.source())).with_frozen(self, self))
    }

    pub(crate) fn abs(&self) -> Lin {
        if let Some(value) = self.constant() {
            return Lin::num(value.abs()).with_frozen(self, self);
        }
        let (weight, rest) = self.split();
        Lin::atom(Shape::Call, format!("abs({})", rest.source()))
            .scale(weight.abs())
            .with_frozen(self, self)
    }

    pub(crate) fn square(&self) -> Lin {
        self.mul(self)
    }
}
