use super::spawn::{drape_piece, spawn_sim};
use super::{Drafted, PieceSlot, Session, SessionError};
use crate::body::Collider;
use crate::draft::{Doc, Draft, MeasureSet};
use crate::sync::SimHandle;
use crate::{couture, demo};

impl Session {
    /// The demo bodice, draping over the avatar on its own thread.
    pub fn demo_bodice() -> Session {
        let contour = demo::bodice_contour();
        let pipeline = demo::pipeline(&contour);
        let state = demo::drop_state(&pipeline);
        let slot = PieceSlot::new(pipeline, 0);
        let collider = Collider::demo();
        let handle = spawn_sim(&slot, state, &collider);
        build(Some(slot), contour, None, Some(handle), collider)
    }

    /// The demo bodice, let go over `collider` rather than over the sphere.
    ///
    /// What the desktop app opens on once it has fitted a body. The scene the
    /// drape golden pins is [`Session::demo_bodice`] and only that: the panel
    /// is the same panel, but here the body decides the height it falls from,
    /// so this drapes differently and is meant to.
    pub fn demo_bodice_over(collider: Collider) -> Session {
        let contour = demo::bodice_contour();
        let pipeline = demo::pipeline(&contour);
        let state = couture::drop_state(&pipeline, collider.release_height());
        let slot = PieceSlot::new(pipeline, 0);
        let handle = spawn_sim(&slot, state, &collider);
        build(Some(slot), contour, None, Some(handle), collider)
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
        blank_with(draft, body)
    }

    /// A document on the table over `body`, draping its first piece if one can
    /// be draped.
    ///
    /// The body arrives with the document because this seeds the drape: a
    /// garment let go at some other body's height starts inside this one,
    /// metres deep in a field that is saturated flat, where no contact solve
    /// can carry it back out. Whoever opens a document knows the body it
    /// resolves against, so no later frame has to correct this one.
    ///
    /// A document with no pieces — or one whose first piece is still too
    /// partial to mesh, as an autosave taken mid-drawing leaves it — opens as a
    /// blank table wrapping it, the same state a fresh product starts in, so
    /// every saved product reopens rather than being refused. Drawing on takes
    /// the piece up again once its contour can be meshed.
    ///
    /// # Errors
    /// `SessionError` when the document itself does not resolve into a draft.
    pub fn from_doc(doc: Doc, body: Collider) -> Result<Session, SessionError> {
        let draft = Draft::from_doc(doc)?;
        let Some(piece) = draft.doc().piece_keys().first().copied() else {
            return Ok(blank_with(draft, body));
        };
        let Ok((slot, contour, state)) = drape_piece(&draft, piece, body.release_height()) else {
            return Ok(blank_with(draft, body));
        };
        let handle = spawn_sim(&slot, state, &body);
        let drafted = Drafted {
            draft,
            piece: Some(piece),
        };
        Ok(build(
            Some(slot),
            contour,
            Some(drafted),
            Some(handle),
            body,
        ))
    }
}

/// A blank table wrapping a document already resolved into a draft: no piece
/// drapes, no mesh, no sim thread, until one is drawn or taken up.
fn blank_with(draft: Draft, body: Collider) -> Session {
    let drafted = Drafted { draft, piece: None };
    build(None, Vec::new(), Some(drafted), None, body)
}

/// Fills the session's fields; the caller decides whether a piece drapes.
fn build(
    slot: Option<PieceSlot>,
    contour: Vec<[f64; 2]>,
    drafted: Option<Drafted>,
    handle: Option<SimHandle>,
    collider: Collider,
) -> Session {
    Session {
        slot,
        contour,
        drafted,
        handle,
        collider,
        remesher: None,
        moved_while_meshing: false,
        generation: 0,
        mesh_generation: 0,
        revision: 0,
        last_derive_ms: 0.0,
        last_remesh_ms: 0.0,
    }
}
