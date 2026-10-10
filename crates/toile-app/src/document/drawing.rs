use toile_engine::export;

use crate::file;

impl crate::App {
    /// Draws the pattern into an SVG at true scale.
    pub(super) fn draw(&mut self) {
        let revision = self.session.revision();
        let drawn = self.session.draft().map(export::to_svg);
        let drawing = match drawn {
            Some(Ok(drawing)) => drawing,
            Some(Err(why)) => {
                self.file
                    .warn(format!("no se pudo dibujar: {why}"), revision);
                return;
            }
            None => {
                self.file
                    .warn("no hay ningún patrón que exportar", revision);
                return;
            }
        };
        let Some(path) = file::svg_target(self.file.stem()) else {
            return;
        };
        if let Err(why) = file::write(&path, &drawing.text) {
            self.file.warn(why, revision);
            return;
        }
        // A drawing short of a label is reported as something wrong even though
        // it was written: the only other place that says so is the title of a
        // group, and nothing shows that until somebody hovers the piece.
        let said = drew(drawing.unsaid);
        if drawing.unsaid == 0 {
            self.file.say(said, revision);
        } else {
            self.file.warn(said, revision);
        }
    }
}

/// What was drawn, and how many pieces found nowhere on it to name themselves.
fn drew(unsaid: usize) -> String {
    if unsaid == 0 {
        return "SVG exportado a escala real".to_owned();
    }
    let labels = if unsaid == 1 {
        "1 pieza".to_owned()
    } else {
        format!("{unsaid} piezas")
    };
    format!(
        "SVG exportado a escala real · {labels} sin sitio para su rótulo junto a su contorno: el \
         dibujo lleva la pieza y no el rótulo"
    )
}

#[cfg(test)]
mod tests {
    use super::drew;

    /// A drawing short of a label says so in the notice, because the only
    /// other place it is said is the title of a group nobody opens.
    #[test]
    fn a_drawing_that_lost_a_label_says_so_in_the_notice() {
        assert_eq!(drew(0), "SVG exportado a escala real");
        assert!(
            drew(1).contains("1 pieza sin sitio para su rótulo"),
            "{}",
            drew(1)
        );
        assert!(drew(4).contains("4 piezas sin sitio"), "{}", drew(4));
    }
}
