/// A person's lower-body measurements, in centimetres.
///
/// English field names by STD-001; each maps to one catalogue name the
/// Maniquies tab edits (the Spanish-name mapping is done in
/// `toile_engine::body`, where the names stay data rather than identifiers).
pub struct BodyMeasures {
    /// Estatura: floor to the crown of the head.
    pub height: f64,
    /// Cintura: waist girth.
    pub waist: f64,
    /// Cadera: hip girth.
    pub hip: f64,
    /// Muslo: thigh girth.
    pub thigh: f64,
    /// Rodilla: knee girth.
    pub knee: f64,
    /// Tobillo: ankle girth.
    pub ankle: f64,
    /// Tiro: waist to crotch, which anchors the crotch height.
    pub rise: f64,
    /// Largo lateral: waist to ankle down the outside.
    pub outseam: f64,
    /// Entrepierna: crotch to ankle; a documented cross-check, not an anchor.
    pub inseam: f64,
    /// Altura de cadera: the drop from waist to hip.
    pub hip_drop: f64,
}

impl Default for BodyMeasures {
    /// Public-domain drafting-book values, internally consistent
    /// (rise 27 ≈ outseam 104 − inseam 78 = 26).
    fn default() -> Self {
        Self {
            height: 178.0,
            waist: 84.0,
            hip: 98.0,
            thigh: 58.0,
            knee: 40.0,
            ankle: 24.0,
            rise: 27.0,
            outseam: 104.0,
            inseam: 78.0,
            hip_drop: 20.0,
        }
    }
}

/// Mesh resolution. Fixed values give fixed topology, so a measurement edit
/// only moves vertices — the index buffer never changes.
#[derive(Clone, Copy)]
pub struct BodyRes {
    /// Radial segments around every cross-section.
    pub seg: u32,
    /// Interpolated rings inserted between each pair of landmark rings.
    pub loft_steps: u32,
    /// Rings from the waist up to the crown dome.
    pub dome_rings: u32,
}

impl Default for BodyRes {
    fn default() -> Self {
        Self {
            seg: 32,
            loft_steps: 4,
            dome_rings: 6,
        }
    }
}
