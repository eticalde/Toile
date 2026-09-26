use toile_engine::export;

use crate::file;

impl crate::App {
    /// Draws the pattern into an SVG at true scale.
    pub(super) fn draw(&mut self) {
        let revision = self.session.revision();
        let drawn = self.session.draft().map(export::to_svg);
        let text = match drawn {
            Some(Ok(text)) => text,
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
        match file::write(&path, &text) {
            Ok(()) => self.file.say("SVG exportado a escala real", revision),
            Err(why) => self.file.warn(why, revision),
        }
    }
}
