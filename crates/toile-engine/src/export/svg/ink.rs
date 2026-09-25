use std::fmt::Write;

use super::super::drawn::Run;
use super::super::units::{BROKEN, DRAWN, INK, millimetres, number};
use super::xml::escape;

/// Every run of ink a piece carries besides its cut line, each its own path,
/// inside that piece's group.
pub(super) fn runs(out: &mut String, runs: &[Run]) {
    for run in runs {
        path(out, run);
    }
}

/// One run as a path on the drawing.
///
/// A run that carries a name carries it as a title, which is where a drawing
/// puts a name that is not part of the drawing.
fn path(out: &mut String, run: &Run) {
    let mut data = String::new();
    for (rank, &at) in run.at.iter().enumerate() {
        let [x, y] = millimetres(at);
        let verb = if rank == 0 { 'M' } else { 'L' };
        let _ = write!(data, "{verb} {} {} ", number(x), number(y));
    }
    let dash = if run.broken {
        format!(
            " stroke-dasharray=\"{} {}\"",
            number(BROKEN[0]),
            number(BROKEN[1])
        )
    } else {
        String::new()
    };
    let title = run
        .label
        .as_deref()
        .map(|name| format!("<title>{}</title>", escape(name)))
        .unwrap_or_default();
    let _ = writeln!(
        out,
        "    <path d=\"{}\" fill=\"none\" stroke=\"{INK}\" stroke-width=\"{}\"{dash}>{title}\
         </path>",
        data.trim_end(),
        number(DRAWN)
    );
}
