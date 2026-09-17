#![allow(
    clippy::float_cmp,
    reason = "a cached voxel reads back as the very bits that were written"
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;

/// A fresh folder under the system's temporary directory, removed when the
/// test ends.
///
/// The cache under test is only ever given a folder inside it, so whatever a
/// test does, it cannot reach the person's real one.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Scratch {
        static MADE: AtomicU32 = AtomicU32::new(0);
        let n = MADE.fetch_add(1, Ordering::Relaxed);
        let name = format!("toile-sdf-{}-{n}-{test}", std::process::id());
        let root = std::env::temp_dir().join(name);
        std::fs::create_dir(&root).expect("a scratch folder nobody else has");
        Scratch { root }
    }

    fn cache(&self) -> Cache {
        Cache::at(self.root.join("sdf"))
    }

    /// How many entries the folder holds.
    fn count(&self) -> usize {
        std::fs::read_dir(self.root.join("sdf"))
            .map(|read| read.flatten().count())
            .unwrap_or_default()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if self.root.starts_with(std::env::temp_dir()) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}

/// A body small enough to check by hand. Only its positions are read here,
/// for the extent a release height comes from.
fn body() -> BodyMesh {
    BodyMesh {
        positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0],
        normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
        indices: vec![0, 1, 2],
        stations: vec![0, 0, 0],
    }
}

/// A field with three different counts, so a transposed index would show.
fn field() -> SdfGrid {
    SdfGrid {
        dims: [3, 4, 5],
        cell: 0.01,
        origin: [-0.1, -0.2, -0.3],
        data: (0..60).map(|i| i as f32 * 0.5 - 7.0).collect(),
    }
}

fn collider() -> Collider {
    Collider::over(field(), &body())
}

fn tape(waist: f64) -> MeasureSet {
    MeasureSet::new("Ana", [("cintura", waist), ("cadera", 98.0)])
}

/// What was baked comes back voxel for voxel, with the counts, the spacing
/// and the corner that place it.
#[test]
fn a_field_stored_comes_back_exactly() {
    let scratch = Scratch::new("roundtrip");
    let cache = scratch.cache();
    let key = key(&tape(84.0));
    assert!(cache.load(key, &body()).is_none(), "nothing is filed yet");

    cache.store(key, &collider()).expect("the field is filed");
    let back = cache.load(key, &body()).expect("and read back");
    let (was, now) = (field(), back.field());
    assert_eq!(now.dims, was.dims);
    assert_eq!(now.cell, was.cell);
    assert_eq!(now.origin, was.origin);
    assert_eq!(now.data, was.data);
    // The extent is measured off the mesh again, never stored, so the height
    // a garment is let go from survives the round trip with it.
    assert_eq!(back.release_height(), collider().release_height());
}

/// The key is the body and nothing else about it: a centimetre moved is a
/// different entry, and the same tape under another name is the same one.
#[test]
fn a_measurement_moved_misses_the_key_and_a_rename_does_not() {
    let eighty_four = key(&tape(84.0));
    assert_eq!(eighty_four, key(&tape(84.0)), "the key is a function");
    assert_ne!(
        eighty_four,
        key(&tape(84.1)),
        "a millimetre is another body"
    );

    let mut renamed = tape(84.0);
    renamed.name = "Begoña".to_owned();
    assert_eq!(key(&renamed), eighty_four, "a name is not a measurement");

    let shaped = tape(84.0).shaped(crate::draft::BodyShape {
        sex: 1.0,
        ..crate::draft::BodyShape::default()
    });
    assert_ne!(key(&shaped), eighty_four, "the shape is part of the body");
}

/// A set that stores no shape and one that stores the default shape generate
/// the byte-identical mesh, so they share one entry rather than filing tens of
/// megabytes twice out of a folder that keeps four bodies.
///
/// The normalising happens where the key is made and nowhere else: the
/// fingerprint a library link is stamped with still tells the two apart, and
/// asking for the key leaves the set as it was.
#[test]
fn an_absent_shape_is_filed_under_the_default_one() {
    let bare = tape(84.0);
    let shaped = bare.clone().shaped(crate::draft::BodyShape::default());
    assert_eq!(key(&bare), key(&shaped), "one body, one entry");
    assert_eq!(bare.phenotype, None, "and the set was not written into");
    assert_ne!(
        bare.fingerprint(),
        shaped.fingerprint(),
        "a stamp that moved would break every link carrying it"
    );
}

/// A stored body is not handed to whoever asks: an entry carries the key it
/// was filed under, and a file that has been renamed is thrown away.
#[test]
fn an_entry_filed_under_another_body_is_not_taken_for_this_one() {
    let scratch = Scratch::new("wrongkey");
    let cache = scratch.cache();
    let mine = key(&tape(84.0));
    cache.store(mine, &collider()).expect("the field is filed");
    let theirs = key(&tape(72.0));
    std::fs::rename(cache.path(mine), cache.path(theirs)).expect("renamed by hand");

    assert!(cache.load(theirs, &body()).is_none(), "refused");
    assert_eq!(scratch.count(), 0, "and thrown away rather than kept");
}

/// An entry from a build that baked differently invalidates the whole folder,
/// because every entry in it was computed the same way.
#[test]
fn an_entry_from_another_bake_invalidates_every_entry() {
    let scratch = Scratch::new("version");
    let cache = scratch.cache();
    let mine = key(&tape(84.0));
    let other = key(&tape(72.0));
    cache.store(mine, &collider()).expect("filed");
    cache.store(other, &collider()).expect("filed");
    assert_eq!(scratch.count(), 2);

    // The very bytes, from a build whose bake version was one behind.
    let mut bytes = encode(mine, collider().field());
    bytes[12..16].copy_from_slice(&(BAKE_VERSION - 1).to_le_bytes());
    std::fs::write(cache.path(mine), &bytes).expect("an older entry");

    assert!(cache.load(mine, &body()).is_none(), "refused");
    assert_eq!(scratch.count(), 0, "and so is everything beside it");
}

/// A file that stops before its voxels do is refused whole. Reading it
/// half-way would hand the solver a body with a hole in it.
#[test]
fn a_truncated_entry_is_refused_and_never_read_half_way() {
    let scratch = Scratch::new("truncated");
    let cache = scratch.cache();
    let mine = key(&tape(84.0));
    let whole = encode(mine, collider().field());

    for cut in [0, 12, HEADER_LEN, HEADER_LEN + 4, whole.len() - 4] {
        std::fs::create_dir_all(cache.path(mine).parent().expect("a folder")).expect("made");
        std::fs::write(cache.path(mine), &whole[..cut]).expect("a half-written entry");
        assert!(cache.load(mine, &body()).is_none(), "refused at {cut}");
    }
    // A whole entry with something else's bytes after it is refused too.
    let mut grown = whole.clone();
    grown.extend_from_slice(&[0u8; 4]);
    std::fs::write(cache.path(mine), &grown).expect("an entry with a tail");
    assert!(cache.load(mine, &body()).is_none());

    std::fs::write(cache.path(mine), &whole).expect("the whole entry");
    assert!(
        cache.load(mine, &body()).is_some(),
        "and the whole one reads"
    );
}

/// A header claiming a grid too thin to sample is refused, even though its
/// length arithmetic checks out.
///
/// The payload agrees with the counts, so the length check passes it: what
/// does not survive is the trilinear read, which steps to `i + 1` off the end
/// of the very data the header was just proved consistent with.
#[test]
fn a_header_claiming_a_grid_too_thin_to_sample_is_refused() {
    let key = key(&tape(84.0));
    for dims in [[1, 4, 5], [3, 1, 5], [3, 4, 1], [0, 0, 0]] {
        let thin = SdfGrid {
            dims,
            cell: 0.01,
            origin: [0.0; 3],
            data: vec![0.0; dims[0] * dims[1] * dims[2]],
        };
        let bytes = encode(key, &thin);
        assert!(
            matches!(decode(&bytes, key), Err(Stale::Unreadable)),
            "a {dims:?} grid was handed back"
        );
    }
}

/// A file that is not an entry at all is refused rather than measured.
#[test]
fn something_that_is_not_an_entry_is_refused() {
    let scratch = Scratch::new("alien");
    let cache = scratch.cache();
    let mine = key(&tape(84.0));
    cache.store(mine, &collider()).expect("filed");
    std::fs::write(cache.path(mine), b"no soy un campo").expect("a hand-made file");
    assert!(cache.load(mine, &body()).is_none());
}

/// The folder cannot grow without bound: a body is tens of megabytes, so the
/// least recently written one goes once there are enough of them.
#[test]
fn the_folder_keeps_only_the_last_few_bodies() {
    let scratch = Scratch::new("evict");
    let cache = scratch.cache();
    for n in 0..ENTRIES + 3 {
        let waist = 70.0 + n as f64;
        cache.store(key(&tape(waist)), &collider()).expect("filed");
        // Entries are dropped oldest first, and the file system's clock is
        // coarse enough that same-millisecond writes would tie.
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(scratch.count(), ENTRIES);
    let newest = key(&tape(70.0 + (ENTRIES + 2) as f64));
    assert!(cache.load(newest, &body()).is_some(), "the last one stayed");
    let oldest = key(&tape(70.0));
    assert!(cache.load(oldest, &body()).is_none(), "the first one went");
}
