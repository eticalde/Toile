/// A person's body measurements, in centimetres.
///
/// English field names by STD-001; each maps to one catalogue name the
/// Maniquies tab edits (the Spanish-name mapping is done in
/// `toile_engine::body`, where the names stay data rather than identifiers).
/// Semantics follow ISO 8559-1 so the same tape that drafts a pattern sizes
/// the dummy. A value the person omits is completed by `PartialMeasures`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyMeasures {
    /// Stature ("`estatura`"): floor to the crown of the head.
    pub height: f64,
    /// Waist girth ("`cintura`").
    pub waist: f64,
    /// Hip girth ("`cadera`").
    pub hip: f64,
    /// Thigh girth ("`muslo`").
    pub thigh: f64,
    /// Knee girth ("`rodilla`").
    pub knee: f64,
    /// Ankle girth ("`tobillo`").
    pub ankle: f64,
    /// Rise ("`tiro`"): waist to crotch, which anchors the crotch height.
    pub rise: f64,
    /// Outseam ("`largo_lateral`"): waist to ankle down the outside.
    pub outseam: f64,
    /// Inseam ("`entrepierna`"): crotch to ankle; a documented cross-check, not
    /// an anchor.
    pub inseam: f64,
    /// Hip drop ("`altura_cadera`"): the drop from waist to hip.
    pub hip_drop: f64,
    /// Neck-base girth ("`cuello`"), where a collar sits.
    pub neck: f64,
    /// Bust or chest girth ("`pecho`") at the fullest point.
    pub bust: f64,
    /// Upper chest girth ("`pecho_alto`") at armpit level.
    pub upper_chest: f64,
    /// Underbust girth ("`bajo_pecho`"), directly under the bust.
    pub underbust: f64,
    /// Shoulder width ("`hombros`"): shoulder point to shoulder point across
    /// the back.
    pub shoulder_width: f64,
    /// Arm length ("`brazo`"): shoulder point to wrist with the arm hanging.
    pub arm_length: f64,
    /// Upper-arm girth ("`brazo_contorno`") at the fullest point.
    pub upper_arm: f64,
    /// Wrist girth ("`muneca`").
    pub wrist: f64,
    /// Back length ("`largo_espalda`"): nape to waist down the spine. It scales
    /// the whole torso stack, so it is not re-measurable on the dummy.
    pub back_length: f64,
    /// Head girth ("`cabeza`") above the brows.
    pub head: f64,
}

impl Default for BodyMeasures {
    /// Public-domain drafting-book values for a 178 cm reference body,
    /// internally consistent (rise 27 ≈ outseam 104 − inseam 78 = 26; bust 96
    /// over underbust 88 gives the chest-versus-bust rules room to act).
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
            neck: 39.0,
            bust: 96.0,
            upper_chest: 94.0,
            underbust: 88.0,
            shoulder_width: 46.0,
            arm_length: 60.0,
            upper_arm: 30.0,
            wrist: 17.0,
            back_length: 44.5,
            head: 57.0,
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
    /// Rings of the skull cap from the head's widest ring to the crown.
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
