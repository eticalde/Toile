use crate::params::BodyMeasures;

/// The ankle joint sits this far above the floor; the crown is `height` above
/// it.
pub(crate) const ANKLE_FLOOR: f64 = 0.07;

/// The vertical landmarks, in metres, y up from the ankle joint at 0.
///
/// Strictly ordered bottom to top by construction — the torso stations are
/// fractions of the back length above the waist, the head stations fractions
/// of the stature below the crown, and two clamps keep the two stacks apart —
/// so the loft never folds back on itself, whatever the measurements say.
pub(crate) struct Landmarks {
    pub calf: f64,
    pub knee: f64,
    pub crotch: f64,
    pub hip: f64,
    pub waist: f64,
    pub underbust: f64,
    pub bust: f64,
    pub armpit: f64,
    pub shoulder: f64,
    pub neck_base: f64,
    pub neck_top: f64,
    pub jaw: f64,
    pub cheek: f64,
    pub head_max: f64,
    pub crown: f64,
}

impl Landmarks {
    /// Every landmark, bottom to top, for the order checks.
    #[cfg(test)]
    pub(crate) fn stack(&self) -> [f64; 15] {
        [
            self.calf,
            self.knee,
            self.crotch,
            self.hip,
            self.waist,
            self.underbust,
            self.bust,
            self.armpit,
            self.shoulder,
            self.neck_base,
            self.neck_top,
            self.jaw,
            self.cheek,
            self.head_max,
            self.crown,
        ]
    }
}

/// Places the landmarks from a measure set, clamping so their order always
/// holds (ISO 8559 semantics: outseam is waist→ankle, rise is waist→crotch,
/// `hip_drop` is waist→hip, `back_length` is nape→waist; inseam is redundant
/// with outseam and rise and is not used as a second crotch anchor).
pub(crate) fn landmarks(m: &BodyMeasures) -> Landmarks {
    let cm = 0.01;
    // A stature under 57 cm would put the crown below every clamp window, so
    // the shortest body lofted is that tall.
    let stature = (m.height * cm).max(0.57);
    let crown = stature - ANKLE_FLOOR;
    // The head is a fixed fraction of the stature (about a seventh, chin
    // included); the jaw ring's front depth supplies the chin.
    let head_max = crown - 0.055 * stature;
    let cheek = crown - 0.085 * stature;
    let jaw = crown - 0.115 * stature;
    let waist = (m.outseam * cm).clamp(0.10, jaw - 0.20);
    let crotch = (waist - m.rise * cm).clamp(waist * 0.1, waist - 0.02);
    let mut hip = waist - (m.hip_drop * cm).max(0.005);
    if hip <= crotch || hip >= waist {
        hip = f64::midpoint(crotch, waist);
    }
    let knee = crotch * 0.5;
    // The torso stack is the back length above the waist, kept at least 3 cm
    // short of the jaw so the neck top always sits under it.
    let len = (m.back_length * cm).clamp(0.10, (jaw - 0.03 - waist) / 0.97);
    Landmarks {
        calf: knee * 0.7,
        knee,
        crotch,
        hip,
        waist,
        underbust: waist + 0.22 * len,
        bust: waist + 0.40 * len,
        armpit: waist + 0.47 * len,
        shoulder: waist + 0.79 * len,
        neck_base: waist + 0.90 * len,
        neck_top: waist + 0.97 * len,
        jaw,
        cheek,
        head_max,
        crown,
    }
}
