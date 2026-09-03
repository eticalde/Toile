/// A raw pointer into the state buffers, shareable across threads.
///
/// The type system cannot express what makes this sound; the caller's index
/// discipline does. It exists because graph colouring already guarantees that
/// no two constraints in a colour touch the same vertex, which makes the
/// writes disjoint — but that guarantee lives in [`super::color`], not here.
#[derive(Clone, Copy)]
pub(super) struct Ptr(pub(super) *mut f32);

// SAFETY: the pointer carries no ownership and no interior mutability of its
// own. Every use below is guarded by the disjoint-index invariant documented
// on `at`, which the colouring establishes and the callers uphold.
unsafe impl Send for Ptr {}
// SAFETY: as above — sharing the pointer is sound; dereferencing it is what
// carries the obligation, and that is `at`'s contract.
unsafe impl Sync for Ptr {}

impl Ptr {
    /// Access goes through a method on `self` so that closures capture the
    /// whole struct, which is `Sync`, rather than the bare `*mut f32` field.
    /// Rust 2021's disjoint capture would otherwise capture the field and
    /// break the `Sync` bound.
    ///
    /// # Safety
    /// `i` must be in bounds for the buffer this pointer came from, and no
    /// other thread may touch index `i` for the duration of the phase.
    #[inline(always)]
    pub(super) unsafe fn at(self, i: usize) -> *mut f32 {
        // SAFETY: the caller guarantees `i` is in bounds.
        unsafe { self.0.add(i) }
    }
}

/// The nine position and velocity buffers, as thread-shareable pointers.
///
/// Bundling them keeps the parallel phases readable and confines the pointer
/// construction to one place.
#[derive(Clone, Copy)]
pub(super) struct Buffers {
    pub px: Ptr,
    pub py: Ptr,
    pub pz: Ptr,
    pub vx: Ptr,
    pub vy: Ptr,
    pub vz: Ptr,
    pub qx: Ptr,
    pub qy: Ptr,
    pub qz: Ptr,
}

impl Buffers {
    pub(super) fn of(state: &mut super::state::State) -> Self {
        Self {
            px: Ptr(state.px.as_mut_ptr()),
            py: Ptr(state.py.as_mut_ptr()),
            pz: Ptr(state.pz.as_mut_ptr()),
            vx: Ptr(state.vx.as_mut_ptr()),
            vy: Ptr(state.vy.as_mut_ptr()),
            vz: Ptr(state.vz.as_mut_ptr()),
            qx: Ptr(state.qx.as_mut_ptr()),
            qy: Ptr(state.qy.as_mut_ptr()),
            qz: Ptr(state.qz.as_mut_ptr()),
        }
    }

    /// Reads position and previous position at `i`.
    ///
    /// # Safety
    /// `i` in bounds, and no other thread touching `i` during this phase.
    #[inline(always)]
    pub(super) unsafe fn pq(self, i: usize) -> ([f32; 3], [f32; 3]) {
        // SAFETY: delegated to the caller's contract.
        unsafe {
            (
                [*self.px.at(i), *self.py.at(i), *self.pz.at(i)],
                [*self.qx.at(i), *self.qy.at(i), *self.qz.at(i)],
            )
        }
    }

    /// Writes position and previous position at `i`.
    ///
    /// # Safety
    /// `i` in bounds, and no other thread touching `i` during this phase.
    #[inline(always)]
    pub(super) unsafe fn set_pq(self, i: usize, p: [f32; 3], q: [f32; 3]) {
        // SAFETY: delegated to the caller's contract.
        unsafe {
            *self.px.at(i) = p[0];
            *self.py.at(i) = p[1];
            *self.pz.at(i) = p[2];
            *self.qx.at(i) = q[0];
            *self.qy.at(i) = q[1];
            *self.qz.at(i) = q[2];
        }
    }
}
