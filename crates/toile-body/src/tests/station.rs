use super::{SEG, SPAN, parts_of};
use crate::params::BodyMeasures;
use crate::parts::Part;
use crate::station::Station;

/// The tag of the first vertex of ring `index` in a part.
fn tag_of(part: &Part, index: usize) -> u8 {
    part.2[index * (SEG + 1)]
}

/// A station's own ring carries its tag, and so do the rings nearest it, so a
/// measurement lights a band around its station rather than a hairline.
#[test]
fn each_station_ring_and_its_neighbours_carry_its_tag() {
    let [trunk, leg, _, arm, _] = parts_of(&BodyMeasures::default());
    assert_eq!(tag_of(&trunk, 2 * SPAN), Station::Waist.tag());
    assert_eq!(tag_of(&trunk, 4 * SPAN), Station::Bust.tag());
    assert_eq!(tag_of(&trunk, 6 * SPAN), Station::Shoulder.tag());
    assert_eq!(tag_of(&leg, 0), Station::Ankle.tag());
    assert_eq!(tag_of(&leg, 2 * SPAN), Station::Knee.tag());
    assert_eq!(tag_of(&arm, 0), Station::Wrist.tag());
    assert_eq!(tag_of(&arm, 3 * SPAN), Station::Biceps.tag());
    // The ring just under the waist station is in the upper half of its span,
    // so it reads as waist too: that is the band.
    assert_eq!(tag_of(&trunk, 2 * SPAN - 1), Station::Waist.tag());
    // And the one just over it, in the lower half of the next span, as well.
    assert_eq!(tag_of(&trunk, 2 * SPAN + 1), Station::Waist.tag());
}

/// The skull cap above the head's widest ring is the crown, and the hidden
/// caps take the tag of the ring they close. Cap centres trail the rings in
/// the vertex list, so the trunk ends on its crotch cap with the last dome
/// ring just before it, and a leg on its ankle cap then its thigh cap.
#[test]
fn the_skull_cap_is_the_crown_and_caps_follow_their_ring() {
    let [trunk, leg, ..] = parts_of(&BodyMeasures::default());
    let n = trunk.2.len();
    assert_eq!(trunk.2[n - 1], Station::Crotch.tag());
    assert_eq!(trunk.2[n - 2], Station::Crown.tag());
    let n = leg.2.len();
    assert_eq!(leg.2[n - 2], Station::Ankle.tag());
    assert_eq!(leg.2[n - 1], Station::Thigh.tag());
}
