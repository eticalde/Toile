use super::metric;

/// The leading of a block, as a share of the size each line is set at.
///
/// One and a half, which is what the two sheets have always set these words
/// at. A line of a block is a line of a sentence, so its own size sets the
/// step above it: a title followed by captions opens the block at the height
/// the title needs and then advances at the captions' pace.
const LEADING: f64 = 1.5;

/// The lines a block takes once none of them is wider than the room.
///
/// A line that does not fit is broken at a space and carries on below. It is
/// never shortened and never cut: «Cortar 2 · margen 1 cm por fuera del
/// contorno» over three rows is still the whole instruction, and half of it
/// printed is worse than none because it still reads. A word wider than the
/// room stands on its own line and sticks out, which is the one thing breaking
/// cannot help with.
///
/// A line that fits is passed through untouched rather than taken apart and
/// put back together, so whatever its author typed between two words is what
/// the sheet sets.
pub(super) fn wrapped(said: &[(f64, &str)], room: f64) -> Vec<(f64, String)> {
    let mut out = Vec::new();
    for &(size, body) in said {
        if body.is_empty() {
            continue;
        }
        if metric::wide(size, body) <= room {
            out.push((size, body.to_owned()));
            continue;
        }
        let mut line = String::new();
        for word in body.split(' ').filter(|word| !word.is_empty()) {
            if line.is_empty() {
                line.push_str(word);
            } else if metric::wide(size, &format!("{line} {word}")) <= room {
                line.push(' ');
                line.push_str(word);
            } else {
                out.push((size, std::mem::take(&mut line)));
                line.push_str(word);
            }
        }
        if !line.is_empty() {
            out.push((size, line));
        }
    }
    out
}

/// How far each line's baseline sits below the first one's.
pub(super) fn stacked(lines: &[(f64, String)]) -> Vec<f64> {
    let mut out = Vec::with_capacity(lines.len());
    let mut down = 0.0;
    for (rank, &(size, _)) in lines.iter().enumerate() {
        if rank > 0 {
            down += LEADING * size;
        }
        out.push(down);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one line of the owner's jeans that has to break, and the promise
    /// that breaking it loses no word of it.
    #[test]
    fn a_line_too_wide_for_the_room_is_broken_at_a_space() {
        let said = [(3.0, "Cortar 2 · margen 1 cm por fuera del contorno")];
        let held = wrapped(&said, 30.0);
        assert!(held.len() > 1, "{held:?}");
        let whole: Vec<&str> = held.iter().map(|(_, body)| body.as_str()).collect();
        assert_eq!(whole.join(" "), said[0].1);
    }

    /// Each line steps down at its own size and not at the one above it, so a
    /// block of captions under a title advances at a caption's pace.
    #[test]
    fn each_line_steps_down_at_the_size_it_is_set_at() {
        let lines = [
            (5.0, "A · «DELANTERO»".to_owned()),
            (3.0, "30.1 × 102.3 cm".to_owned()),
            (3.0, "Cortar 2".to_owned()),
        ];
        assert_eq!(stacked(&lines), [0.0, 4.5, 9.0]);
    }
}
