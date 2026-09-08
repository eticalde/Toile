use crate::params::BodyMeasures;

/// The stature of the reference body every stature-only rule scales from.
const REF_HEIGHT: f64 = 178.0;

/// What the person actually wrote; a `None` is completed by proportion.
///
/// Stature-only rules scale the 178 cm reference body (a factor of exactly 1
/// at the reference), girth-hung rules are drafting-book affine rules chosen
/// so the reference comes out exact: nothing given derives
/// `BodyMeasures::default()` bit for bit. A given value always wins and is
/// never sanity-clamped here; the landmark clamps make junk loft.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PartialMeasures {
    /// Stature ("`estatura`").
    pub height: Option<f64>,
    /// Waist girth ("`cintura`").
    pub waist: Option<f64>,
    /// Hip girth ("`cadera`").
    pub hip: Option<f64>,
    /// Thigh girth ("`muslo`").
    pub thigh: Option<f64>,
    /// Knee girth ("`rodilla`").
    pub knee: Option<f64>,
    /// Ankle girth ("`tobillo`").
    pub ankle: Option<f64>,
    /// Rise ("`tiro`").
    pub rise: Option<f64>,
    /// Outseam ("`largo_lateral`").
    pub outseam: Option<f64>,
    /// Inseam ("`entrepierna`").
    pub inseam: Option<f64>,
    /// Hip drop ("`altura_cadera`").
    pub hip_drop: Option<f64>,
    /// Neck-base girth ("`cuello`").
    pub neck: Option<f64>,
    /// Bust or chest girth ("`pecho`").
    pub bust: Option<f64>,
    /// Upper chest girth ("`pecho_alto`").
    pub upper_chest: Option<f64>,
    /// Underbust girth ("`bajo_pecho`").
    pub underbust: Option<f64>,
    /// Shoulder width ("`hombros`").
    pub shoulder_width: Option<f64>,
    /// Arm length ("`brazo`").
    pub arm_length: Option<f64>,
    /// Upper-arm girth ("`brazo_contorno`").
    pub upper_arm: Option<f64>,
    /// Wrist girth ("`muneca`").
    pub wrist: Option<f64>,
    /// Back length ("`largo_espalda`").
    pub back_length: Option<f64>,
    /// Head girth ("`cabeza`").
    pub head: Option<f64>,
}

impl PartialMeasures {
    /// Completes every omitted value from the stature and the given girths.
    pub fn complete(&self) -> BodyMeasures {
        let g = self;
        let height = g
            .height
            .filter(|h| h.is_finite() && *h > 0.0)
            .unwrap_or(REF_HEIGHT);
        let k = height / REF_HEIGHT;
        let scaled = |v: f64| v * k;

        let waist = g.waist.unwrap_or(scaled(84.0));
        let hip = g.hip.unwrap_or(scaled(98.0));
        // Multiply first so the reference thigh comes out as exactly 58.
        let thigh = g.thigh.unwrap_or(hip * 58.0 / 98.0);
        let knee = g.knee.unwrap_or(scaled(40.0));
        let ankle = g.ankle.unwrap_or(scaled(24.0));
        // The outseam/rise/inseam triangle closes only from two *given*
        // partners; a derived third would otherwise leave the reference.
        let outseam = g
            .outseam
            .or_else(|| g.inseam.zip(g.rise).map(|(i, r)| i + r))
            .unwrap_or(scaled(104.0));
        let rise = g
            .rise
            .or_else(|| g.outseam.zip(g.inseam).map(|(o, i)| o - i))
            .unwrap_or(scaled(27.0));
        let inseam = g
            .inseam
            .or_else(|| g.outseam.zip(g.rise).map(|(o, r)| o - r))
            .unwrap_or(scaled(78.0));
        let hip_drop = g.hip_drop.unwrap_or(scaled(20.0));

        // The bust hangs on the underbust when there is one (a drop of 8 is
        // the drafting-book reference), else on the waist-hip average, so a
        // small waist under a full hip derives a real bust prominence.
        let bust = g
            .bust
            .or_else(|| g.underbust.map(|u| u + 8.0))
            .unwrap_or_else(|| f64::midpoint(waist, hip) + 5.0);
        let underbust = g.underbust.unwrap_or(f64::midpoint(bust, waist) - 2.0);
        let upper_chest = g
            .upper_chest
            .unwrap_or(underbust + (bust - underbust) * 0.75);
        // Vincent's collar rule: three eighths of the breast plus an inch and
        // a half.
        let neck = g.neck.unwrap_or(0.375 * bust + 3.0);
        let upper_arm = g.upper_arm.unwrap_or(bust / 3.0 - 2.0);
        let wrist = g.wrist.unwrap_or(bust / 6.0 + 1.0);
        let shoulder_width = g.shoulder_width.unwrap_or(scaled(46.0));
        let arm_length = g.arm_length.unwrap_or(scaled(60.0));
        let back_length = g.back_length.unwrap_or(scaled(44.5));
        let head = g.head.unwrap_or(scaled(57.0));

        BodyMeasures {
            height,
            waist,
            hip,
            thigh,
            knee,
            ankle,
            rise,
            outseam,
            inseam,
            hip_drop,
            neck,
            bust,
            upper_chest,
            underbust,
            shoulder_width,
            arm_length,
            upper_arm,
            wrist,
            back_length,
            head,
        }
    }
}
