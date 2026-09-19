use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use crossbeam_channel::{RecvTimeoutError, Sender, TryRecvError, unbounded};
use toile_sim::xpbd::{DistanceConstraints, Floor, Grip, SdfGrid, Seams, State};

use super::report::Snapshot;
use super::worker::Sim;
use crate::couture::MeshSwap;

/// What the cloth collides with: the body's field, the ground it stands on,
/// and how the two hold what touches them.
///
/// The three travel together because they move together. A body put on the
/// stand brings its own floor with it — a shorter person stands lower — and a
/// message carrying the field alone would leave the drape resting on the
/// plane the body before it stood on. The grip goes with them for the same
/// reason: a person's skin holds a garment and the demo sphere does not, so
/// swapping one body for the other has to swap what its surface does.
#[derive(Clone)]
pub struct Scene {
    /// The body's field, shared rather than copied.
    pub sdf: Arc<SdfGrid>,
    /// The plane under it, if this body stands on one.
    pub floor: Floor,
    /// How that body and that plane hold cloth pressed against them.
    pub grip: Grip,
}

enum Msg {
    /// The product recompiled: what every edge rests at, how hard it is held
    /// there, which edges an elastic holds, and what is sewn to what. They
    /// travel together because a shape edit moves all four.
    RestUpdate {
        generation: u64,
        rests: Vec<f32>,
        compliance: Vec<f32>,
        /// The edges an elastic holds, and the sweeps they get.
        held: (Vec<u32>, u32),
        seams: Seams,
    },
    /// A piece re-meshed elsewhere, with the drape to carry onto it.
    ///
    /// Boxed because it is an order of magnitude wider than a rest update, and
    /// an enum as wide as its widest variant would put a whole mesh on the
    /// stack of every message the sim thread ever receives.
    MeshSwap {
        generation: u64,
        swap: Box<MeshSwap>,
    },
    /// A body re-solved elsewhere, for the drape to carry on against.
    ///
    /// Shared rather than boxed, for the same reason `MeshSwap` is boxed: a
    /// baked body is tens of megabytes of voxels, and they must not widen
    /// every message the thread receives. An `Arc` is one pointer either way,
    /// and it lets the session keep the very field it handed over, so seeding
    /// a newly drawn piece against this body copies nothing at all.
    Collider {
        generation: u64,
        scene: Scene,
    },
    Stop,
}

/// A running simulation thread. Dropping it stops the thread and joins it.
pub struct SimHandle {
    tx: Sender<Msg>,
    snapshot: Arc<ArcSwap<Snapshot>>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl SimHandle {
    /// Hot-swaps the rest state and the sewing. Does not block.
    pub fn send_rests(
        &self,
        generation: u64,
        rests: Vec<f32>,
        compliance: Vec<f32>,
        held: (Vec<u32>, u32),
        seams: Seams,
    ) {
        let _ = self.tx.send(Msg::RestUpdate {
            generation,
            rests,
            compliance,
            held,
            seams,
        });
    }

    /// Hands the sim a mesh built elsewhere. Does not block.
    ///
    /// The solver goes on integrating the mesh it has until the message
    /// reaches the top of the mailbox, which is what keeps a topology edit off
    /// the interface thread and out of the drape.
    pub fn send_swap(&self, generation: u64, swap: Box<MeshSwap>) {
        let _ = self.tx.send(Msg::MeshSwap { generation, swap });
    }

    /// Hands the sim the body to collide against from now on. Does not block.
    ///
    /// The drape carries on against the body it has until the message reaches
    /// the top of the mailbox, which is what keeps a re-solved body off the
    /// interface thread.
    pub fn send_collider(&self, generation: u64, scene: Scene) {
        let _ = self.tx.send(Msg::Collider { generation, scene });
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
    seams: Seams,
    scene: Scene,
    tris: Vec<u32>,
    dt: f32,
    substeps_per_tick: u32,
) -> SimHandle {
    let (tx, rx) = unbounded::<Msg>();
    let snapshot = Arc::new(ArcSwap::from_pointee(Snapshot::default()));
    let published = snapshot.clone();

    let join = std::thread::spawn(move || {
        let mut sim = Sim::new(state, cons, seams, scene, tris, dt, substeps_per_tick);

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

/// Empties the mailbox in order, keeping whatever the messages leave behind.
///
/// This runs between ticks and never inside one, which is what makes a mesh
/// swap safe: the state it carries onto the new mesh is always a whole
/// substep's worth. A message the sim refuses is recorded rather than retried
/// — it was compiled against a mesh that no longer exists, and the edit that
/// replaced it has a message of its own further down the mailbox.
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
        let taken = match m {
            Msg::Stop => return Drained::Stop,
            Msg::RestUpdate {
                generation,
                rests,
                compliance,
                held,
                seams,
            } => sim.apply_rests(generation, &rests, &compliance, held, seams),
            Msg::MeshSwap { generation, swap } => sim.apply_swap(generation, swap),
            Msg::Collider { generation, scene } => sim.apply_collider(generation, scene),
        };
        if let Err(why) = taken {
            sim.refuse(why);
        }
    }
}
