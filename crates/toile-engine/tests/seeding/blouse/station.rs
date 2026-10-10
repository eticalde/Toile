use std::f64::consts::TAU;

use toile_engine::body::Collider;
use toile_engine::couture::Round;
use toile_engine::session::Session;

use super::{CHEST, Cut, blouse};
use crate::fit::{placed, report};
use crate::watch::reference;

/// How many ordinates the surface is read at, looking for its worst hoop.
const SAMPLES: u32 = 400;

/// What one release came to.
struct Read {
    /// How high the surface was let go, in metres.
    stand: f32,
    /// How far round the hoop the hung line landed on goes, in metres.
    hoop: f64,
    /// The cloth that line carries, in metres.
    cloth: f64,
    /// The widest any hoop was opened past the cloth on it, as a ratio, with
    /// that hoop in metres and the pattern ordinate it is at.
    ///
    /// The hung line's own ratio says what the garment was let go at where it
    /// is held, and this says what the body under the rest of it cost. A tube
    /// stood over two thighs is opened to clear them whatever its band does.
    worst: (f64, f64, f64),
    /// Lowest and highest released particle, in metres.
    span: (f32, f32),
    /// Worst and mean seam gap at release, in metres.
    seams: Option<(f32, f32)>,
}

impl Read {
    /// How much bigger than its own cloth the hung line was let go, as a ratio.
    fn opened(&self) -> f64 {
        self.hoop / self.cloth
    }
}

/// The widest a surface was opened past the cloth it carries, and where.
///
/// Sampled rather than walked band by band, for the reason `fit/surface.rs`
/// samples: the bands are the profile's own and a reading taken at them would
/// be the profile read back, where what is wanted is the surface a vertex at
/// any ordinate actually lands on.
fn widest(round: &Round) -> (f64, f64, f64) {
    let (from, to) = round.span();
    let mut worst = (0.0, 0.0, from);
    for k in 0..=SAMPLES {
        let y = from + (to - from) * f64::from(k) / f64::from(SAMPLES);
        let (hoop, cloth) = (TAU * round.radius(y), round.girth(y));
        if cloth > 0.0 && hoop / cloth > worst.0 {
            worst = (hoop / cloth, hoop, y);
        }
    }
    worst
}

/// Lets `cut` go over the reference body, declaring `station` or nothing.
///
/// The release and not a drape. What a declaration changes is where the cloth
/// is put down, and that is decided before the solver has run a substep: a
/// scene that integrated first would read the body's grip and the floor on top
/// of it.
fn let_go(scene: &str, cut: Cut, station: Option<&str>, body: &Collider) -> Read {
    let doc = blouse(cut, station);
    let session = Session::from_doc(doc, body.clone()).expect("the blouse drapes");
    assert!(
        session.seam_faults().is_empty(),
        "{scene}: the blouse's seams all pair: {:?}",
        session.seam_faults()
    );
    let ring = session
        .layout()
        .expect("the two seams chain the three panels");
    let points = placed(&session.released());
    let read = Read {
        stand: ring.stand,
        hoop: ring.radius * TAU,
        cloth: ring.round.girth(ring.crest),
        worst: widest(&ring.round),
        span: crate::watch::span(&points),
        seams: report(scene, "at release", &session.sewn_pairs(), &points),
    };
    println!(
        "{scene}: let go at {:.4} · hung line carries {:.5} m of cloth onto a {:.5} m hoop, \
         {:.1} % bigger than itself · worst hoop {:.5} m, {:.1} % over its cloth, at \
         ordinate {:.4} · cloth {:.4}..{:.4} · the frame says {}",
        read.stand,
        read.cloth,
        read.hoop,
        100.0 * (read.opened() - 1.0),
        read.worst.1,
        100.0 * (read.worst.0 - 1.0),
        read.worst.2,
        read.span.0,
        read.span.1,
        match session.worn_elsewhere() {
            Some(apart) => format!("{} · inferido {}", apart.declared, apart.inferred),
            None => "nothing".to_owned(),
        }
    );
    read
}

/// The narrow blouse goes to the ring it declares, and the same blouse
/// declaring nothing goes to the ring its own size points at.
///
/// Both and not one: a reading of the declared release alone says nothing, and
/// the control is the same document with the hang taken off. The control is
/// also the whole of the old behaviour, so the two columns of this one scene
/// are the before and the after.
///
/// What the control does, measured: 0.8800 m of cloth drafted for a narrower
/// chest cannot be worn at this body's waist, because the hip below the waist
/// is wider than the garment and a garment is only worn where everything it
/// would pass is narrower than it is. So the inference goes to the hip, and the
/// blouse's chest line is let go at 0.2040 against the 0.5853 it declares —
/// 38.1 cm down, its hem at the knees instead of the waist. Nothing about that
/// is a near miss: it is the wrong part of the body.
#[test]
#[ignore = "release-only: a real body baked"]
fn a_declared_blouse_is_let_go_at_its_own_station_and_an_undeclared_one_is_not() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let chest = body
        .belt_at(CHEST)
        .copied()
        .expect("the baked body carries its chest ring");
    println!(
        "the body's {CHEST}: {:.4} m round at {:.4}",
        chest.girth, chest.height
    );

    let told = let_go("narrow, declared", Cut::NARROW, Some(CHEST), &body);
    let guessed = let_go("narrow, nothing declared", Cut::NARROW, None, &body);

    // The fixture carries the cloth it says it does, within the one boundary
    // sample the mesher leaves short of each corner: a hoop is read at an
    // ordinate and the topmost vertex of a panel sits a little under its own
    // top edge. Everything below is a ratio of this garment and not of a
    // rounding, which is the whole of what this line is for.
    for read in [&told, &guessed] {
        assert!(
            (read.cloth - Cut::NARROW.girth()).abs() < 0.02,
            "the hung line carries {:.5} m where the cut draws {:.5}",
            read.cloth,
            Cut::NARROW.girth()
        );
    }

    // The declared one stands on the ring it named, exactly: the height is that
    // ring's own, copied, and not a number this placement worked out.
    assert_eq!(
        told.stand, chest.height,
        "the declared blouse is let go at the ring it names"
    );
    // And the control does not, by a distance a person would see across a room.
    let adrift = f64::from(chest.height - guessed.stand);
    assert!(
        adrift > 0.30,
        "the control is let go {:.1} cm under the chest, which is the defect \
         this scene exists to show",
        adrift * 100.0
    );
    // What the height costs the rest of the garment. The hung line is let go on
    // much the same hoop either way — both are opened one cell of the field and
    // no more — so this is not a scene about a garment released too big at its
    // band. It is one about *which part of the person* the cloth was wrapped
    // around, and the reading that shows it is the worst hoop over the whole
    // surface: dropped to the hip, the blouse's hem reaches the thighs and has
    // to be opened to clear two of them.
    assert!(
        told.worst.0 < guessed.worst.0,
        "the declared blouse's worst hoop is {:.1} % over its cloth against \
         the control's {:.1} %",
        100.0 * (told.worst.0 - 1.0),
        100.0 * (guessed.worst.0 - 1.0)
    );
    // And what the ring does not cost, read here so that the line above is not
    // taken for a claim about fit. A seam at release stands apart by whatever
    // the two panels' boundary sampling leaves between them, which is a fact of
    // the cloth and not of the hoop: measured, 13.2 mm declared against 12.3 mm
    // for the control, and the declared one is the wider of the two.
    let (shut, adrift) = (
        told.seams.expect("the blouse carries two seams").0,
        guessed.seams.expect("and so does the control").0,
    );
    assert!(
        f64::from(shut - adrift).abs() < 0.005,
        "the declared blouse's worst seam starts {:.1} mm open and the \
         control's {:.1} mm",
        shut * 1000.0,
        adrift * 1000.0
    );
}

/// A blouse drafted for the body wearing it reads the same ring both ways, and
/// then the release says nothing at all.
///
/// The other half of the rule, and the half that keeps it quiet. 1.0400 m of
/// cloth is this body's own chest less seven millimetres, so the inference
/// already names the chest and the declaration asks for nothing new. A box lit
/// on this garment too would be a box nobody reads by the week's end.
///
/// The narrow cut is the contrast, measured in the same run: the two readings
/// name `pecho_alto` and `cadera`, and that pair is what reaches a person.
#[test]
#[ignore = "release-only: a real body baked"]
fn a_blouse_the_body_agrees_with_says_nothing_and_a_narrow_one_names_both_rings() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let worn = |cut| {
        let session =
            Session::from_doc(blouse(cut, Some(CHEST)), body.clone()).expect("the blouse drapes");
        session.worn_elsewhere().cloned()
    };
    assert_eq!(
        worn(Cut::REFERENCE),
        None,
        "a blouse cut to this chest is read onto the chest both ways"
    );
    let apart = worn(Cut::NARROW).expect("a narrow blouse's own size points elsewhere");
    assert_eq!(apart.declared, CHEST);
    assert_eq!(apart.inferred, "cadera");
}
