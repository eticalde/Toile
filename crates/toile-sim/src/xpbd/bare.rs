use super::color::ColoredConstraints;
use super::contact::Grip;
use super::layers::Layers;
use super::sdf::SdfGrid;
use super::solver::GRAVITY;
use super::stage::Stage;
use super::state::Seams;

#[cfg(test)]
mod tests;

/// A pass [`super::substep`] runs that the coloured paths do not.
///
/// They were written to measure what colour partitioning and SIMD buy on the
/// stretch solve, and nothing beyond that solve and the body was ever written
/// for them. Naming the pass rather than answering "no" is what lets a caller
/// see which half of its scene it would have lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dropped {
    /// The post-solve clamp on over-elongated edges.
    StrainLimit,
    /// The extra sweeps over the edges an elastic holds, whose indices name
    /// the set before the colouring permuted it.
    Held,
    /// The inter-piece seam attachments.
    Seams,
    /// Cloth parted from its own folds.
    Layers,
    /// The ground plane cloth may not fall through.
    Floor,
    /// Contacts that read the push instead of taking a fixed share of it.
    Grip,
    /// A pull other than the constant the coloured integrator applies.
    Gravity,
}

/// A scene the coloured paths solve whole.
///
/// [`Bare::of`] is the only way to make one and it is handed every value a
/// missing pass could hide in, while [`super::color_constraints`] is the only
/// way to make the set inside it.
///
/// It never leaves the substep that earns it, and that is the whole of its
/// worth. An earlier design handed one in from outside; a review defeated it
/// in forty lines, because a proof is a value: earn it over a body hanging in
/// the void, put a floor under that same body, and the value is still in hand
/// while the cloth falls 27 m through the ground. A check answers for the
/// scene it read and for no later one.
#[derive(Debug)]
pub(super) struct Bare<'a> {
    pub(super) cons: &'a ColoredConstraints,
    pub(super) sdf: &'a SdfGrid,
}

impl<'a> Bare<'a> {
    /// The scene, if the coloured paths run all of it.
    ///
    /// Takes what [`super::substep`] takes and in the same order, minus the
    /// state and the step, so the two signatures read against each other.
    ///
    /// # Errors
    /// [`Dropped`] names the first pass the coloured paths would skip.
    pub(super) fn of(
        cons: &'a ColoredConstraints,
        seams: &Seams,
        stage: &Stage<'a>,
        layers: Option<&Layers>,
    ) -> Result<Bare<'a>, Dropped> {
        // Taken apart field by field, so that a fifth thing put on the stage
        // has to be answered for on this line. Read through `stage.` instead,
        // and the coloured paths would drop it without a word.
        let &Stage {
            sdf,
            floor,
            gravity,
            grip,
        } = stage;
        if !seams.is_empty() {
            return Err(Dropped::Seams);
        }
        if layers.is_some() {
            return Err(Dropped::Layers);
        }
        if floor.level().is_some() {
            return Err(Dropped::Floor);
        }
        if grip != Grip::slipping() {
            return Err(Dropped::Grip);
        }
        // Asked on the bits: the coloured integrator adds this very constant,
        // so a pull that is not it to the last bit is a pull it would not give.
        if gravity.to_bits() != GRAVITY.to_bits() {
            return Err(Dropped::Gravity);
        }
        Ok(Bare { cons, sdf })
    }
}
