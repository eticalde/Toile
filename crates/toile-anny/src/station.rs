/// Where on the body a vertex sits: the anatomical station its ring is
/// nearest to.
///
/// Every vertex the mesh carries has one of these, so the interface can
/// light the region a measurement is read at while the person handles it.
/// The tags are bookkeeping over the mesh, not geometry: the baker assigns
/// them once from the neutral template's own joint positions, they never
/// change with the phenotype or a lever, and they never enter a mesh
/// golden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Station {
    /// The ankle joint, where a leg's tape ends.
    Ankle,
    /// The calf at its fullest.
    Calf,
    /// The knee.
    Knee,
    /// The thigh at its fullest, just above the crotch.
    Thigh,
    /// The fork of the legs: the trunk's lowest ring.
    Crotch,
    /// The seat at its fullest.
    Hip,
    /// The waist, where a garment's waistline is tied.
    Waist,
    /// Directly under the bust.
    Underbust,
    /// The bust or chest at its fullest.
    Bust,
    /// The upper chest at armpit level.
    Armpit,
    /// The shoulder points, across the back.
    Shoulder,
    /// The base of the neck, where a collar sits.
    NeckBase,
    /// The top of the neck, under the jaw.
    NeckTop,
    /// The jaw, whose front depth is the chin.
    Jaw,
    /// The cheekbones.
    Cheek,
    /// The head at its widest, above the brows.
    HeadMax,
    /// The skull cap, up to the crown.
    Crown,
    /// The wrist, where an arm's tape ends.
    Wrist,
    /// The forearm at its fullest.
    Forearm,
    /// The elbow.
    Elbow,
    /// The upper arm at its fullest.
    Biceps,
    /// The top of the arm, hidden inside the shoulder.
    Deltoid,
}

impl Station {
    /// How many stations there are; every tag a mesh carries is below this.
    pub const COUNT: u8 = 22;

    /// Every station in tag order, for a mask that lights the whole body.
    pub const ALL: [Station; 22] = [
        Station::Ankle,
        Station::Calf,
        Station::Knee,
        Station::Thigh,
        Station::Crotch,
        Station::Hip,
        Station::Waist,
        Station::Underbust,
        Station::Bust,
        Station::Armpit,
        Station::Shoulder,
        Station::NeckBase,
        Station::NeckTop,
        Station::Jaw,
        Station::Cheek,
        Station::HeadMax,
        Station::Crown,
        Station::Wrist,
        Station::Forearm,
        Station::Elbow,
        Station::Biceps,
        Station::Deltoid,
    ];

    /// The tag a mesh carries for this station.
    pub fn tag(self) -> u8 {
        self as u8
    }
}
