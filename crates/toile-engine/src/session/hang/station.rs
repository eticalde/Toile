use super::super::Session;
use super::super::place::Worn;
use super::super::run::edge_bases;

impl Session {
    /// Where the document declares its product belongs: every station it names,
    /// the line of cloth hung from each, and the pieces each is written on.
    ///
    /// The highest declared line first and never the order the document holds
    /// them in. A garment hangs by one line of itself and everything else hangs
    /// below it, so the one to be read first is the topmost — and "the first"
    /// would be storage order, which is no part of what a person wrote.
    ///
    /// Every station and no longer one, because a garment need not go round one
    /// ring: a shirt's body is hung from the chest and its collar from the
    /// neck, and placing both is what lets the panel between them carry a
    /// third seam.
    ///
    /// Read against the meshes on the stand rather than off the drawing, for
    /// the reason [`Session::hung_on`] is: the ordinate wanted here is the one
    /// the placement will roll, and only the cloth carries that.
    pub(in crate::session) fn declared(&self) -> Vec<Worn> {
        let Some(draft) = self.draft.as_ref() else {
            return Vec::new();
        };
        let pipes = self.pipelines();
        let bases = edge_bases(&pipes);
        let mut lines: Vec<Line<'_>> = Vec::new();
        for (_, hang) in draft.doc().hangs.iter() {
            for run in self.runs_of(draft, &pipes, &bases, hang.at) {
                let pipe = pipes[run.at];
                let ordinates = run.verts.iter().map(|&v| pipe.pos2d[v as usize][1]);
                tally(&mut lines, &hang.station, run.at, ordinates);
            }
        }
        let mut worn: Vec<Worn> = lines
            .into_iter()
            .filter(|line| line.counted > 0)
            .map(|line| Worn {
                station: line.station.to_owned(),
                at: line.sum / line.counted as f64,
                on: line.on,
            })
            .collect();
        worn.sort_by(|a, b| b.at.total_cmp(&a.at));
        worn
    }
}

/// One station's line of cloth as the hangs on it are read: the running mean of
/// its ordinate, and the pieces it is written on.
struct Line<'a> {
    station: &'a str,
    sum: f64,
    counted: usize,
    on: Vec<usize>,
}

/// Adds a run's ordinates to its station's running mean, and its piece to the
/// ring that station names.
///
/// Per station and not per hang, for the reason `elastic::widen` counts per
/// piece: two hangs written on the two halves of one waistline are one line of
/// cloth, and a mean taken per hang would weight whichever half carries more
/// vertices. The two stretches of a folded piece meet here the same way, and so
/// do two hangs naming one piece — it goes on that ring once.
fn tally<'a>(
    lines: &mut Vec<Line<'a>>,
    station: &'a str,
    piece: usize,
    ordinates: impl Iterator<Item = f64>,
) {
    if lines.iter().all(|line| line.station != station) {
        lines.push(Line {
            station,
            sum: 0.0,
            counted: 0,
            on: Vec::new(),
        });
    }
    let Some(line) = lines.iter_mut().find(|line| line.station == station) else {
        return;
    };
    for y in ordinates {
        line.sum += y;
        line.counted += 1;
    }
    if let Err(at) = line.on.binary_search(&piece) {
        line.on.insert(at, piece);
    }
}
