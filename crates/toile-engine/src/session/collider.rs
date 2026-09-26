use super::Session;
use super::spawn::scene_of;
use crate::body::Collider;

impl Session {
    /// The body the drape falls on.
    pub fn collider(&self) -> &Collider {
        &self.collider
    }

    /// Puts the drape on another body, without restarting it.
    ///
    /// A body is re-solved whenever a measurement or the phenotype moves, so
    /// this arrives in the middle of a drape by design. The garment keeps the
    /// position and the motion it had; cloth the new body has swallowed is
    /// carried back out to its skin on the sim thread, which is the only place
    /// that holds the state. Nothing is re-dropped, because a re-drop is the
    /// one thing that would throw away the drape the person is looking at.
    ///
    /// The release height travels with the body, so a piece drawn after this
    /// is let go over this body rather than over the one before it. So does
    /// the ground: a body of another height stands on another plane, and the
    /// drape has to land on the one under the body it is now falling on. A
    /// table with nothing draping has no thread to tell, and takes the body
    /// anyway for whenever a piece is drawn on it.
    pub fn set_collider(&mut self, collider: Collider) {
        self.generation += 1;
        // Against the body arriving and not the one leaving: a ring's height
        // is the new person's, and holding the garment at the old one's would
        // be the very fault this message exists to avoid.
        let hung = self.hung_on(&collider);
        if let Some(handle) = self.handle.as_ref() {
            handle.send_collider(self.generation, scene_of(&collider, hung));
        }
        self.collider = collider;
    }
}
