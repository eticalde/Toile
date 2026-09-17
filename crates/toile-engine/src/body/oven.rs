use std::thread::JoinHandle;
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use toile_anny::BodyMesh;

use super::Collider;
use super::bake::BakeError;

#[cfg(test)]
mod tests;

/// A body on its way to the oven.
pub struct Job {
    /// What the field it produces will be filed under.
    pub key: u64,
    /// The body to bake.
    pub mesh: BodyMesh,
}

/// A body that came out of the oven.
pub struct Baked {
    /// The key it was sent under.
    pub key: u64,
    /// How long the bake took, in milliseconds.
    pub ms: f64,
    /// The field, or what stopped it from being baked.
    pub field: Result<Collider, BakeError>,
}

/// The bake, on a thread of its own.
///
/// Baking an adult body is about half a second and tens of megabytes of
/// voxels, and a body is re-solved every time a measurement or the phenotype
/// moves. It runs here so that moving one never freezes the window: the mesh
/// the panels draw is solved on the spot, and the field the drape falls on
/// follows when it is ready.
///
/// One body bakes at a time and one waits its turn. A body asked for while
/// one waits takes its place, and the one it displaces is never baked: half a
/// second spent on a field that is already out of date is half a second the
/// body somebody is waiting for does not get. So what comes out is the body
/// last asked for, or one asked for before it, and never a queue of bodies
/// nobody wants any more.
pub struct Oven {
    /// Closing this is what ends the thread, so it is taken away on drop.
    jobs: Option<Sender<Job>>,
    done: Receiver<Baked>,
    join: Option<JoinHandle<()>>,
    /// Whether a body is in the oven with its answer still to come.
    baking: bool,
    /// The body asked for while one was baking, and only ever the last one.
    waiting: Option<Job>,
}

impl Oven {
    /// Starts the bake thread.
    pub fn spawn() -> Oven {
        let (jobs, inbox) = unbounded::<Job>();
        let (outbox, done) = unbounded::<Baked>();
        let join = std::thread::spawn(move || {
            while let Ok(job) = inbox.recv() {
                if outbox.send(bake(job)).is_err() {
                    return;
                }
            }
        });
        Oven {
            jobs: Some(jobs),
            done,
            join: Some(join),
            baking: false,
            waiting: None,
        }
    }

    /// Asks for a body. Does not block.
    ///
    /// One asked for while the oven is busy takes the place of whatever was
    /// waiting rather than queueing behind it.
    pub fn send(&mut self, job: Job) {
        if self.baking {
            self.waiting = Some(job);
        } else {
            self.dispatch(job);
        }
    }

    /// Puts one body in the oven.
    fn dispatch(&mut self, job: Job) {
        if self.jobs.as_ref().is_some_and(|tx| tx.send(job).is_ok()) {
            self.baking = true;
        }
    }

    /// Whether a bake is out and unanswered.
    pub fn busy(&self) -> bool {
        self.baking
    }

    /// The next finished bake, if one is waiting.
    pub fn try_take(&mut self) -> Option<Baked> {
        match self.done.try_recv() {
            Ok(done) => Some(self.took(done)),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    /// The next finished bake, waiting for it if one is out.
    ///
    /// The one place a wait is honest is before the window is on screen: the
    /// first body of a run has nothing to show until its field exists.
    pub fn take(&mut self) -> Option<Baked> {
        if !self.baking {
            return None;
        }
        match self.done.recv() {
            Ok(done) => Some(self.took(done)),
            Err(_) => None,
        }
    }

    /// Books one answer in and sends after it the body that was waiting.
    fn took(&mut self, done: Baked) -> Baked {
        self.baking = false;
        if let Some(next) = self.waiting.take() {
            self.dispatch(next);
        }
        done
    }
}

impl Drop for Oven {
    fn drop(&mut self) {
        // Closing the queue is what ends the thread's loop, and never more
        // than one body is in it, so this waits out the bake already running
        // and not a backlog. The body still waiting its turn goes with this
        // struct, unbaked: quitting must not buy answers nobody is left to
        // read.
        self.jobs = None;
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Bakes one body and times it.
fn bake(job: Job) -> Baked {
    let Job { key, mesh } = job;
    let t = Instant::now();
    let field = Collider::bake(&mesh);
    Baked {
        key,
        ms: t.elapsed().as_secs_f64() * 1000.0,
        field,
    }
}
