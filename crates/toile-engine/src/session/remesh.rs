use std::thread::JoinHandle;
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use toile_mesh::cdt::MeshError;

use crate::couture::{self, COMPLIANCE, MeshSwap, ShapePipeline};
use crate::draft::PieceKey;

/// A rebuild on its way to the mesher.
pub struct Job {
    /// The piece being re-meshed.
    pub piece: PieceKey,
    /// The topology count its contour was resolved at.
    pub topology: u64,
    /// The contour to mesh, in metres.
    pub contour: Vec<[f64; 2]>,
    /// Rest positions of the mesh this one replaces.
    pub old_pos2d: Vec<[f64; 2]>,
    /// Triangles of the mesh this one replaces.
    pub old_tris: Vec<u32>,
}

/// A rebuild that finished.
pub struct Remeshed {
    /// The piece it was built for.
    pub piece: PieceKey,
    /// The topology count it was built at. A count the draft has since left
    /// behind marks a rebuild that a later edit already superseded.
    pub topology: u64,
    /// How long the rebuild took, in milliseconds.
    pub ms: f64,
    /// The new mesh, or what stopped the mesher from building it.
    pub built: Result<Rebuilt, MeshError>,
}

/// The new mesh, and the message that carries the drape onto it.
pub struct Rebuilt {
    /// The contour it was built from.
    pub contour: Vec<[f64; 2]>,
    /// The pipeline that compiles that contour's shape edits.
    pub pipeline: ShapePipeline,
    /// What the sim thread needs to inherit the drape.
    pub swap: Box<MeshSwap>,
}

/// The mesher, on a thread of its own.
///
/// A topology edit is a rebuild from nothing — triangulation, classification
/// and the interpolation matrix — and it costs two orders of magnitude more
/// than a shape edit. It runs here so the interface never waits on it and the
/// solver goes on integrating the mesh it has until the swap arrives.
///
/// Jobs are served in order and answered in order. A second edit arriving on
/// top of the first does not cancel it: the answer is dropped on the way in,
/// by the topology count it was built at.
pub struct Remesher {
    /// Closing this is what ends the thread, so it is taken away on drop.
    jobs: Option<Sender<Job>>,
    done: Receiver<Remeshed>,
    join: Option<JoinHandle<()>>,
    in_flight: usize,
}

impl Remesher {
    /// Starts the mesher thread.
    pub fn spawn() -> Remesher {
        let (jobs, inbox) = unbounded::<Job>();
        let (outbox, done) = unbounded::<Remeshed>();
        let join = std::thread::spawn(move || {
            while let Ok(job) = inbox.recv() {
                if outbox.send(build(job)).is_err() {
                    return;
                }
            }
        });
        Remesher {
            jobs: Some(jobs),
            done,
            join: Some(join),
            in_flight: 0,
        }
    }

    /// Queues a rebuild. Does not block.
    pub fn send(&mut self, job: Job) {
        if self.jobs.as_ref().is_some_and(|tx| tx.send(job).is_ok()) {
            self.in_flight += 1;
        }
    }

    /// Whether a rebuild is out and unanswered.
    pub fn busy(&self) -> bool {
        self.in_flight > 0
    }

    /// The next finished rebuild, if one is waiting.
    pub fn try_take(&mut self) -> Option<Remeshed> {
        match self.done.try_recv() {
            Ok(done) => {
                self.in_flight -= 1;
                Some(done)
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    /// The next finished rebuild, waiting for it if one is out.
    ///
    /// The interface never calls this: it is how a test or a benchmark asks
    /// for the number the person would have waited for.
    pub fn take(&mut self) -> Option<Remeshed> {
        if !self.busy() {
            return None;
        }
        match self.done.recv() {
            Ok(done) => {
                self.in_flight -= 1;
                Some(done)
            }
            Err(_) => None,
        }
    }
}

impl Drop for Remesher {
    fn drop(&mut self) {
        // Closing the queue is what ends the thread's loop; the join is only
        // so that a rebuild in flight finishes before its memory goes.
        self.jobs = None;
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Meshes a contour and compiles the message that carries the drape onto it.
fn build(job: Job) -> Remeshed {
    let t = Instant::now();
    let (samples, max_area) = couture::for_contour(&job.contour);
    let built = ShapePipeline::build(&job.contour, samples, max_area).map(|pipeline| {
        let swap = MeshSwap::new(&job.old_pos2d, &job.old_tris, &pipeline, COMPLIANCE);
        Rebuilt {
            contour: job.contour,
            pipeline,
            swap: Box::new(swap),
        }
    });
    Remeshed {
        piece: job.piece,
        topology: job.topology,
        ms: t.elapsed().as_secs_f64() * 1000.0,
        built,
    }
}
