use super::spawn::{drape_piece, spawn_sim};
use super::{Draped, PieceSlot, Session, SessionError};
use crate::body::Collider;
use crate::demo;
use crate::draft::{Doc, Draft, MeasureSet};

impl Session {
    /// The demo bodice, draping over the avatar on its own thread.
    ///
    /// The sphere is the physics reference, and it is let go at the height the
    /// drape golden was taken at: [`Collider::demo`] carries that very
    /// constant, so nothing here chooses a number.
    pub fn demo_bodice() -> Session {
        Session::demo_bodice_over(Collider::demo())
    }

    /// The demo bodice, let go over `collider` rather than over the sphere.
    ///
    /// What the desktop app opens on once it has fitted a body. The scene the
    /// drape golden pins is [`Session::demo_bodice`] and only that: the panel
    /// is the same panel, but here the body decides the height it falls from,
    /// so this drapes differently and is meant to.
    pub fn demo_bodice_over(collider: Collider) -> Session {
        let contour = demo::bodice_contour();
        let slot = PieceSlot::new(demo::pipeline(&contour), 0);
        let mut session = build(None, collider);
        session.draping = vec![Draped::new(None, slot, contour)];
        session.start();
        session
    }

    /// A blank document over `body`: a table with nothing drawn, ready for the
    /// first piece.
    ///
    /// The document still carries a mannequin, since every coordinate resolves
    /// against one; it simply has no pieces yet. Nothing drapes until one is
    /// drawn, so there is no mesh and no sim thread until then — but the body
    /// is taken now all the same, because the piece drawn later is let go at a
    /// height only the body it falls on decides.
    ///
    /// # Panics
    /// Never in practice: a document with no pieces has nothing to resolve, so
    /// the draft cannot fail to build.
    pub fn blank(body: Collider) -> Session {
        let doc = Doc::new(MeasureSet::default());
        let draft = Draft::from_doc(doc).expect("an empty document resolves");
        build(Some(draft), body)
    }

    /// A document on the table over `body`, draping every piece of it that can
    /// be draped.
    ///
    /// The body arrives with the document because this seeds the drape: a
    /// garment let go at some other body's height starts inside this one,
    /// metres deep in a field that is saturated flat, where no contact solve
    /// can carry it back out. Whoever opens a document knows the body it
    /// resolves against, so no later frame has to correct this one.
    ///
    /// A document with no pieces — or one whose pieces are still too partial to
    /// mesh, as an autosave taken mid-drawing leaves them — opens as a blank
    /// table wrapping it, the same state a fresh product starts in, so every
    /// saved product reopens rather than being refused.
    ///
    /// # Errors
    /// `SessionError` when the document itself does not resolve into a draft.
    pub fn from_doc(doc: Doc, body: Collider) -> Result<Session, SessionError> {
        let mut session = build(Some(Draft::from_doc(doc)?), body);
        session.reseed();
        Ok(session)
    }

    /// Meshes every piece the document holds and lets the whole product go.
    ///
    /// Called when the set of pieces on the stand changes — one drawn, one
    /// removed, one that has only now become meshable. The pieces already
    /// draping are let go again with it: a piece arriving or leaving moves
    /// where every piece after it begins in the combined state, and re-dropping
    /// is the honest way to say so until the solver can take a piece in on its
    /// own. A rebuild and a drag do not go through here, and those are the two
    /// that happen while somebody is watching the cloth.
    pub(super) fn reseed(&mut self) {
        // Any rebuild in flight was compiled against a set of meshes that is
        // about to stop existing, so the mesher goes with them.
        self.remesher = None;
        self.draping = match self.draft.as_ref() {
            Some(draft) => meshed(draft),
            None => Vec::new(),
        };
        self.start();
    }

    /// Lays the pieces out as one solver state and starts the thread that
    /// integrates it. A stand with nothing on it gets no thread.
    ///
    /// The seams are paired and the product placed here and nowhere else in
    /// the opening: this is the one moment the cloth has no position yet, so
    /// it is the one moment a placement can be chosen without throwing a drape
    /// away.
    fn start(&mut self) {
        self.relist();
        self.generation = 0;
        self.mesh_generation = 0;
        let seams = self.resew();
        let hung = self.hung();
        let around = self.layout();
        let cons = self.constraints();
        self.handle = (!self.draping.is_empty()).then(|| {
            spawn_sim(
                &self.pipelines(),
                self.tris.clone(),
                cons,
                (seams, hung),
                around.as_ref(),
                &self.collider,
            )
        });
    }
}

/// Every piece of the document that meshes, in the order the document holds
/// them.
///
/// Best-effort on purpose. A piece being drawn lands empty and gains its
/// vertices one command at a time, so it cannot mesh until it has enough of
/// them; leaving it off the stand until the next vertex is what keeps the
/// whole draw one smooth gesture.
fn meshed(draft: &Draft) -> Vec<Draped> {
    draft
        .doc()
        .piece_keys()
        .into_iter()
        .filter_map(|piece| {
            let (slot, contour) = drape_piece(draft, piece).ok()?;
            Some(Draped::new(Some(piece), slot, contour))
        })
        .collect()
}

/// Fills the session's fields; the caller lays the pieces out and starts the
/// thread.
fn build(draft: Option<Draft>, collider: Collider) -> Session {
    Session {
        draping: Vec::new(),
        tris: Vec::new(),
        draft,
        handle: None,
        collider,
        remesher: None,
        faults: Vec::new(),
        generation: 0,
        mesh_generation: 0,
        revision: 0,
        last_derive_ms: 0.0,
        last_remesh_ms: 0.0,
    }
}
