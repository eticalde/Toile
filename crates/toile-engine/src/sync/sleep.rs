use toile_sim::xpbd::State;

/// How far a vertex travels across one tick and still counts as moving, in
/// metres: two microns, which at sixty ticks a second is 0.12 mm/s.
///
/// Set just above what the solver itself never stops doing. A garment lying
/// still on the ground is pushed down by gravity and back out by contact every
/// substep, and that leaves its vertices jittering by up to 1.6 microns a tick
/// for as long as it is simulated; a threshold under that never sleeps, and one
/// much over it calls the slow creep of a heap still finding its folds rest.
const MOVED: f32 = 2.0e-6;

/// The most vertices in a thousand that may be moving in a tick called quiet.
///
/// Not zero, because a vertex pinched in a seam can flutter by half a
/// millimetre a tick for ever, and a sewn skirt carries a handful of those —
/// under one in a thousand. Not one in a hundred either, because that is what
/// a hem still falling amounts to while the rest of the garment lies still.
const MOVING_PER_MILLE: usize = 5;

/// Consecutive quiet ticks required before sleeping: six seconds.
///
/// A drape comes to rest in slips. A fold holds, gives way, and the cloth
/// round it lies still until the next one goes; measured on a trouser front
/// and a bodice heaped on the ground, those lulls last up to 3.7 seconds while
/// the drape is still a centimetre from where it ends. Sleeping late costs a
/// few seconds of one core, and sleeping early shows a garment frozen halfway.
const QUIET_TICKS_TO_SLEEP: u32 = 360;

/// Decides when a drape has stopped moving enough to stop being simulated.
///
/// By how far the vertices got across a whole tick, never by how fast they
/// are going at the end of one: kinetic damping zeroes every velocity at each
/// energy peak, so a tick that ends on one reads as perfectly still. And by
/// counting the vertices that moved rather than averaging over all of them,
/// because a settled garment outvotes the hem that is still falling.
///
/// Sleep is a pause and never a verdict. Whatever message arrives next wakes
/// the drape at once, and nothing may read a sleeping drape as a finished one.
#[derive(Debug, Default)]
pub struct Sleep {
    px: Vec<f32>,
    py: Vec<f32>,
    pz: Vec<f32>,
    quiet_ticks: u32,
    asleep: bool,
}

impl Sleep {
    /// Whether the drape has been quiet long enough to stop being simulated.
    pub fn asleep(&self) -> bool {
        self.asleep
    }

    /// Puts the drape back in motion and forgets how quiet it had been.
    pub fn wake(&mut self) {
        self.quiet_ticks = 0;
        self.asleep = false;
    }

    /// Remembers where every vertex is as a tick begins.
    ///
    /// Into buffers kept from the tick before, so this allocates only when a
    /// swap has made the state longer than it has ever been.
    pub fn mark(&mut self, state: &State) {
        for (kept, now) in [
            (&mut self.px, &state.px),
            (&mut self.py, &state.py),
            (&mut self.pz, &state.pz),
        ] {
            kept.clear();
            kept.extend_from_slice(now);
        }
    }

    /// Judges the tick that began at the last [`Sleep::mark`].
    pub fn judge(&mut self, state: &State) {
        if quiet(self.moving(state), state.len()) {
            self.quiet_ticks += 1;
        } else {
            self.quiet_ticks = 0;
        }
        // Everything that asks whether the drape sleeps reads this flag, so
        // simulating without ever stopping is this one line answering false.
        self.asleep = self.quiet_ticks >= QUIET_TICKS_TO_SLEEP;
    }

    /// How many vertices ended the tick further than [`MOVED`] from where they
    /// began it.
    fn moving(&self, state: &State) -> usize {
        let limit = MOVED * MOVED;
        let mut moving = 0;
        for i in 0..state.len().min(self.px.len()) {
            let dx = state.px[i] - self.px[i];
            let dy = state.py[i] - self.py[i];
            let dz = state.pz[i] - self.pz[i];
            // Asked as "is it still" so that a position gone to NaN, which
            // compares false with everything, counts as moving.
            let still = dx * dx + dy * dy + dz * dz <= limit;
            if !still {
                moving += 1;
            }
        }
        moving
    }
}

/// Whether `moving` vertices out of `all` are few enough to call a tick quiet.
fn quiet(moving: usize, all: usize) -> bool {
    moving * 1000 <= all * MOVING_PER_MILLE
}

#[cfg(test)]
mod tests;
