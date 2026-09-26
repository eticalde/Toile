use super::*;

/// A dart does not un-place the garment it is cut into.
///
/// A dart is a piece sewn to itself, and a piece sewn to itself is the very
/// shape the strip walk refuses: it chains nothing while looking exactly like
/// a seam that should have chained two pieces. Measured before the placement
/// learned to leave a dart's seam out, one dart in the back turned
/// `layout()` into `None` and every piece fell back to the flat release — so
/// the first dart anybody cut took the whole product off the body.
#[test]
fn a_dart_in_one_piece_leaves_the_product_where_it_was_placed() {
    let (plain, _, back) = trousers();
    let ring = plain.layout().expect("the seams place the product");

    let mut doc = plain
        .draft()
        .expect("the block has a document")
        .doc()
        .clone();
    // The wedge the block's own dart tests cut, on the back's waistline: three
    // places the contour takes, so what this measures is the placement and not
    // a piece that stopped meshing.
    let after = doc
        .shows_label(back, "cintura_cb")
        .expect("the block names the back's centre waist");
    let at = |x: f64, y: f64| WedgeNode::line(Identity::New, Point::at(x, y));
    Command::AddDart {
        identity: Identity::New,
        dart: Dart {
            apex: after,
            legs: (after, after),
            seam: SeamKey::new(0, 0),
            fold: FoldDirection::TowardEnd,
        },
        wedge: Box::new(DartWedge {
            piece: back,
            after: Some(after),
            nodes: [at(51.0, 0.0), at(52.0, 12.0), at(53.0, 0.0)],
        }),
    }
    .apply(&mut doc)
    .expect("a wedge the back's waistline takes");

    let darted = Session::from_doc(doc, Collider::demo()).expect("it still drapes");
    let again = darted.layout().expect("and it is still placed on the body");
    // The same ring, not the same number: a wedge takes cloth out, so the
    // widest the product carries moves a hair — measured, three micrometres of
    // radius. `clear_of` opens a ring in whole cells of the field, so anything
    // under one cell is the ring the garment would have been let go on anyway.
    assert!(
        (again.radius - ring.radius).abs() < crate::body::bake::CELL,
        "on the same ring: {} against {}",
        again.radius,
        ring.radius
    );
}

/// The seams and the body place the product: the two pieces are rolled onto
/// opposite sides of one ring, and that ring is the garment's own size.
///
/// Front before the body and back behind it are not declared anywhere. The
/// document says only which stretch of one meets which stretch of the other,
/// and walking that ring is what puts them across the axis from each other.
///
/// The demo ball carries no measurements, so no ring on it can be matched: the
/// product is let go about the whole of it, at its own cloth's size, opened
/// only as far as the ball makes necessary — well inside the ball's own box.
#[test]
fn the_seams_put_one_piece_before_the_body_and_one_behind_it() {
    let (session, front, back) = trousers();
    let split = session.offset(back).expect("the back drapes") as usize;
    let released = session.released();
    let at = released.as_chunks::<3>().0;
    assert_eq!(at.len(), session.n_vertices());
    assert_eq!(session.offset(front), Some(0));

    let ring = session.layout().expect("the seams place the product");
    let mean_z = |run: &[[f32; 3]]| run.iter().map(|p| p[2]).sum::<f32>() / run.len() as f32;
    assert!(mean_z(&at[..split]) > 0.0, "the front is before the body");
    assert!(mean_z(&at[split..]) < 0.0, "the back is behind it");
    for p in at {
        let r = f64::from(p[0].hypot(p[2]));
        assert!((r - ring.radius).abs() < 1.0e-5, "a vertex sits at {r}");
    }
    assert!(
        ring.radius < f64::from(0.15f32.hypot(0.15)),
        "the ring is the cloth's own size and not the ball's: {}",
        ring.radius
    );
}

/// And the seams reach the solver: every document seam pairs, into the one
/// index space the combined state uses.
#[test]
fn the_block_arrives_at_the_solver_sewn() {
    let (session, front, back) = trousers();
    let split = session.offset(back).expect("the back drapes");
    let pairs = session.sewn_pairs();
    assert!(session.seam_faults().is_empty(), "the block's seams pair");
    assert!(!pairs.is_empty(), "and they reach the solver");
    for &(a, b) in &pairs {
        assert!(a < split, "side a stays in the front's block");
        assert!(
            (split..session.n_vertices() as u32).contains(&b),
            "side b stays in the back's"
        );
    }
    assert_eq!(session.contour_m(front).len(), 47);
}

/// A lone piece has no partner to be placed against, so nothing places it: it
/// is let go flat, where it is let go today. This is the reduction the drape
/// goldens stand on, asserted through the session rather than under it.
#[test]
fn a_product_of_one_piece_is_still_let_go_flat() {
    let session =
        Session::from_doc(block::trouser_front(), Collider::demo()).expect("the block drapes");
    assert!(session.sewn_pairs().is_empty(), "nothing is sewn to it");
    let released = session.released();
    let at = released.as_chunks::<3>().0;
    let height = session.collider().release_height();
    assert!(at.iter().all(|p| (p[1] - height).abs() < 1.0e-7));
}
