mod basis;

use toile_engine::body::bake::Crossings;
use toile_engine::body::cache::{self, Cache};
use toile_engine::body::oven::{Baked, Job, Made, Order, Oven};
use toile_engine::body::{self, Collider};
use toile_engine::draft::{BodyMesh, Doc, MeasureSet};
use toile_engine::session::Session;

use self::basis::{Basis, tape, tape_of};

#[cfg(test)]
mod tests;

/// What the field on the stand cost, for the status bar.
pub struct Cost {
    /// Milliseconds the bake took; zero when the cache answered.
    pub ms: f64,
    /// Whether it came off the disk rather than out of the oven.
    pub cached: bool,
}

/// Whether a control is in hand, which says whether the value the document
/// carries is the one that was meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hand {
    /// A control is being held, and will write another value next frame.
    Holding,
    /// Nothing is held, so what the document says is what was asked for.
    Free,
}

/// The body the product on the table is being fitted to.
///
/// Owned by the program rather than by a tab, because two things need it and
/// neither can own it: the Probador draws it behind the cloth, and the solver
/// collides against it whichever tab is in front. The mannequin tab keeps its
/// own view of it — what that tab shows has to answer to the hand on the
/// slider, not to whatever was last baked — but not its own body: the tape is
/// read from the one place it is written, so the two cannot drift apart.
pub struct Fitting {
    basis: Option<Basis>,
    mesh: Option<BodyMesh>,
    /// The body on the stand: the field the drape is falling on right now.
    ///
    /// Held here because a session about to be built needs it before it has
    /// anything to ask — see [`Fitting::body_for`].
    held: Collider,
    /// What the body on the stand is filed under, so a bake that lands for a
    /// body the fitting has already moved past can be told from this one.
    key: u64,
    /// Counts the bodies solved, so a view can tell whether the one it drew
    /// is still the one on the stand.
    solves: u64,
    oven: Oven,
    cache: Option<Cache>,
    /// What the last field cost, and where it came from.
    pub cost: Option<Cost>,
    /// Why the last body could not be baked, if one could not.
    pub refused: Option<String>,
    /// Where the body on the stand passes through itself, once the oven has
    /// looked it over. Never a reason to refuse it.
    pub crossings: Option<Crossings>,
}

impl Fitting {
    /// The first body of the run, and the program's hold on every one after.
    ///
    /// This is the one wait the fitting ever makes, and the drape is why. A
    /// session seeds its garment the moment it is built, at a height the body
    /// it falls on decides, and nothing ever re-drops it: a bodice let go over
    /// the demo sphere while the real body baked would hang below that body's
    /// shoulders for the rest of the run. The window is already built by now,
    /// but it stays hidden until the first frame is painted, so what this
    /// costs is a later launch rather than a frozen window — about half a
    /// second, once per body, and nothing once the field is on the disk. Every
    /// later body is baked off this thread; see [`Fitting::settle`].
    pub fn open(loose: &MeasureSet) -> Fitting {
        let mut fitting = Fitting::empty();
        // Written down as the body now on the stand, so the first frame does
        // not solve it a second time and read its field off the disk again.
        fitting.moved_to(loose);
        if let Some(ready) = fitting.begin(loose).or_else(|| fitting.wait()) {
            fitting.held = ready;
        }
        fitting
    }

    /// A fitting with nothing on the stand but the demo sphere, before any
    /// real body has been asked for.
    fn empty() -> Fitting {
        Fitting {
            basis: None,
            mesh: None,
            held: Collider::demo(),
            key: 0,
            solves: 0,
            oven: Oven::spawn(),
            #[cfg(not(test))]
            cache: crate::config::cache_dir().map(Cache::at),
            #[cfg(test)]
            cache: None,
            cost: None,
            refused: None,
            crossings: None,
        }
    }

    /// The body last solved, for whoever draws it.
    pub fn mesh(&self) -> Option<&BodyMesh> {
        self.mesh.as_ref()
    }

    /// How many bodies have been solved. A view that drew one compares this
    /// to know whether what it holds is still the body on the stand.
    pub fn solves(&self) -> u64 {
        self.solves
    }

    /// The body on the stand, for a session that is about to be built over it.
    pub fn body(&self) -> Collider {
        self.held.clone()
    }

    /// The body to seed a drape over, for a product about to take the table.
    ///
    /// A session seeds its drape the moment it is built, at a height the body
    /// it falls on decides, so the body has to be in hand before the document
    /// is. A bake is half a second and nobody opening a file may be made to
    /// wait for one, so the answer is the new body's own field when the cache
    /// still had it and the body already on the stand when it did not — a real
    /// person either way, never the demo sphere, so the garment is never let
    /// go below the shoulders of the body underneath it. The bake this starts
    /// lands through [`Fitting::settle`], exactly as a measurement moved
    /// mid-drape does.
    pub fn body_for(&mut self, doc: Option<&Doc>, loose: &MeasureSet) -> Collider {
        let tape = tape_of(doc, loose);
        if self.moved_to(&tape)
            && let Some(ready) = self.begin(&tape)
        {
            self.held = ready;
        }
        self.body()
    }

    /// Brings the body in line with the product on the table.
    ///
    /// The mesh is solved here and now, because it costs milliseconds and the
    /// view needs it this frame. The field is not: it costs half a second, so
    /// it goes to the oven, and the drape carries on against the body it has
    /// until it comes back. Nothing here blocks.
    ///
    /// A bake still lands while a control is held, so a body asked for before
    /// the hand came down is not left in the oven until it lifts.
    ///
    /// Answers whether anything moved, which is what asks for another frame.
    pub fn settle(&mut self, session: &mut Session, loose: &MeasureSet, hand: Hand) -> bool {
        let mut moved = self.resolve(session, loose, hand);
        while let Some(done) = self.oven.try_take() {
            moved |= self.land(done, session);
        }
        moved
    }

    /// Solves the product's body again when what it is solved from changed,
    /// unless a control is still in hand.
    ///
    /// A drag writes a value a frame, and chasing it costs a mesh solve on
    /// this thread and a field of tens of megabytes on the other, for a body
    /// superseded before anyone could see it. The mannequin tab holds its own
    /// view back on this very rule, so waiting for the hand is also what keeps
    /// the body behind the cloth and the body on the stand the same one.
    fn resolve(&mut self, session: &mut Session, loose: &MeasureSet, hand: Hand) -> bool {
        if hand == Hand::Holding {
            return false;
        }
        let tape = tape(session, loose);
        if !self.moved_to(&tape) {
            return false;
        }
        if let Some(ready) = self.begin(&tape) {
            self.wear(ready, session);
        }
        true
    }

    /// Whether `tape` asks for a body other than the one already asked for,
    /// remembering it when it does.
    fn moved_to(&mut self, tape: &MeasureSet) -> bool {
        let basis = Basis::of(tape);
        if self.basis.as_ref() == Some(&basis) {
            return false;
        }
        self.basis = Some(basis);
        true
    }

    /// Solves `tape` into a body and looks for its field.
    ///
    /// Answers with the field when the cache had it; otherwise the body is in
    /// the oven and the answer comes later.
    fn begin(&mut self, tape: &MeasureSet) -> Option<Collider> {
        let solved = body::solve_anny(
            tape,
            &body::phenotype_of(&tape.phenotype.unwrap_or_default()),
        );
        let mesh = body::body_mesh(&solved.phenotype, &solved.levers);
        let key = cache::key(tape);
        self.key = key;
        self.solves += 1;
        self.refused = None;
        self.crossings = None;
        let found = self.cache.as_ref().and_then(|cache| cache.load(key, &mesh));
        self.mesh = Some(mesh);
        if found.is_some() {
            self.cost = Some(Cost {
                ms: 0.0,
                cached: true,
            });
            // A field off the disk is voxels and nothing else, so the body it
            // came back for has still never been looked over.
            self.order(Order::Inspect);
        } else {
            self.order(Order::Bake);
        }
        found
    }

    /// Sends the body last solved to the oven.
    fn order(&mut self, order: Order) {
        if let Some(mesh) = self.mesh.clone() {
            let key = self.key;
            self.oven.send(Job { key, mesh, order });
        }
    }

    /// Waits for the bake that is out, for the one caller allowed to wait.
    fn wait(&mut self) -> Option<Collider> {
        let done = self.oven.take()?;
        self.keep(done)
    }

    /// Puts a body on the stand and under the drape at once, so that what the
    /// session collides against is what this holds.
    fn wear(&mut self, collider: Collider, session: &mut Session) {
        self.held = collider.clone();
        session.set_collider(collider);
    }

    /// Takes a finished bake in and puts it under the drape.
    ///
    /// Whatever lands for the body on the stand is worth a frame: a field, a
    /// refusal, or only word of where the body crosses itself.
    fn land(&mut self, done: Baked, session: &mut Session) -> bool {
        let current = done.key == self.key;
        if let Some(collider) = self.keep(done) {
            self.wear(collider, session);
        }
        current
    }

    /// Files a finished bake and hands it back, unless it is a body the
    /// fitting has already moved past or one that could not be baked.
    fn keep(&mut self, done: Baked) -> Option<Collider> {
        // The change that replaced this body sent a bake of its own, and that
        // one is the answer; taking this would fit the body as it no longer is.
        if done.key != self.key {
            return None;
        }
        match done.made {
            Made::Crossings(found) => {
                self.crossings = Some(found);
                None
            }
            Made::Field(Ok(collider)) => {
                if let Some(cache) = self.cache.as_ref() {
                    let _ = cache.store(done.key, &collider);
                }
                self.cost = Some(Cost {
                    ms: done.ms,
                    cached: false,
                });
                // Asked for only now, with the field already on its way to
                // the drape: nobody waiting for a body waits for this.
                self.order(Order::Inspect);
                Some(collider)
            }
            Made::Field(Err(why)) => {
                self.refused = Some(format!("el cuerpo no se pudo hornear: {why}"));
                None
            }
        }
    }
}
