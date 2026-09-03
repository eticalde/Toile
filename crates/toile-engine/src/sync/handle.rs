use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use crossbeam_channel::{RecvTimeoutError, Sender, TryRecvError, unbounded};
use toile_sim::xpbd::{DistanceConstraints, SdfGrid, State};

use super::worker::{Sim, Snapshot};

enum Msg {
    RestUpdate { generation: u64, rests: Vec<f32> },
    Stop,
}

/// A running simulation thread. Dropping it stops the thread and joins it.
pub struct SimHandle {
    tx: Sender<Msg>,
    snapshot: Arc<ArcSwap<Snapshot>>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl SimHandle {
    /// Hot-swaps the rest state. Does not block.
    pub fn send_rests(&self, generation: u64, rests: Vec<f32>) {
        let _ = self.tx.send(Msg::RestUpdate { generation, rests });
    }

    /// The most recent published snapshot. Does not block.
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.snapshot.load_full()
    }

    /// Stops the thread and waits for it.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for SimHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Starts the simulation thread.
///
/// The thread owns the state outright; everything crosses the boundary as a
/// message. Rest updates arrive latest-wins, snapshots leave through arc-swap
/// without locks, and once the cloth converges the thread parks in `recv` at
/// zero CPU until the next edit.
pub fn spawn(
    state: State,
    cons: DistanceConstraints,
    sdf: SdfGrid,
    tris: Vec<u32>,
    dt: f32,
    substeps_per_tick: u32,
) -> SimHandle {
    let (tx, rx) = unbounded::<Msg>();
    let snapshot = Arc::new(ArcSwap::from_pointee(Snapshot::default()));
    let published = snapshot.clone();

    let join = std::thread::spawn(move || {
        let mut sim = Sim::new(state, cons, sdf, tris, dt, substeps_per_tick);

        // Fixed step anchored to the wall clock: a tick represents exactly
        // `substeps_per_tick × dt` of simulated time and is scheduled at that
        // same cadence, so the sim runs neither ahead of nor behind the world.
        let period = Duration::from_secs_f64(f64::from(substeps_per_tick) * f64::from(dt));
        let mut next_tick = Instant::now();

        loop {
            let first = if sim.converged() {
                match rx.recv() {
                    Ok(m) => {
                        next_tick = Instant::now();
                        Some(m)
                    }
                    Err(_) => return,
                }
            } else {
                // Wait out the deadline inside recv_timeout: a message
                // interrupts the wait and is applied at once, but the tick
                // keeps its cadence.
                let now = Instant::now();
                if now < next_tick {
                    match rx.recv_timeout(next_tick - now) {
                        Ok(m) => Some(m),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                } else {
                    None
                }
            };

            match drain(&rx, first, &mut sim) {
                Drained::Continue => {}
                Drained::Stop => return,
            }
            if sim.converged() {
                continue;
            }

            sim.tick();
            next_tick += period;
            // A tick that overran its period re-anchors instead of building
            // up debt it would then try to catch up on.
            if Instant::now() > next_tick + period {
                next_tick = Instant::now();
            }
            published.store(sim.publish());
        }
    });

    SimHandle {
        tx,
        snapshot,
        join: Some(join),
    }
}

enum Drained {
    Continue,
    Stop,
}

/// Empties the mailbox, applying the last rest update to arrive.
fn drain(rx: &crossbeam_channel::Receiver<Msg>, first: Option<Msg>, sim: &mut Sim) -> Drained {
    let mut pending = first;
    loop {
        let m = match pending.take() {
            Some(m) => m,
            None => match rx.try_recv() {
                Ok(m) => m,
                Err(TryRecvError::Empty) => return Drained::Continue,
                Err(TryRecvError::Disconnected) => return Drained::Stop,
            },
        };
        match m {
            Msg::Stop => return Drained::Stop,
            Msg::RestUpdate { generation, rests } => sim.apply_rests(generation, &rests),
        }
    }
}
