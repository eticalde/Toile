use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};

/// Screen points per centimetre at 1:1, on a nominal 96 dpi display.
const POINTS_PER_CM: f64 = 96.0 / 2.54;

/// How much of the drawing area a framed piece leaves as air, per side.
const MARGIN: f32 = 0.08;

/// The scales a view will not go past, in screen points per centimetre.
const MIN_SCALE: f64 = 0.4;
const MAX_SCALE: f64 = 240.0;

/// Where the document lies on the glass.
///
/// `origin` is the screen point the document's (0, 0) lands on, and `scale` is
/// what one centimetre is worth there. Nothing is flipped: the document's y
/// already grows downward, the way the screen's does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    origin: Pos2,
    scale: f64,
}

impl Default for View {
    fn default() -> View {
        View {
            origin: Pos2::ZERO,
            scale: POINTS_PER_CM / 4.0,
        }
    }
}

impl View {
    /// Screen points per centimetre.
    pub fn scale(self) -> f64 {
        self.scale
    }

    /// Where a document point, in centimetres, lands on the glass.
    pub fn to_screen(self, cm: [f64; 2]) -> Pos2 {
        pos2(
            self.origin.x + (cm[0] * self.scale) as f32,
            self.origin.y + (cm[1] * self.scale) as f32,
        )
    }

    /// The document point, in centimetres, under a place on the glass.
    pub fn to_document(self, at: Pos2) -> [f64; 2] {
        [
            f64::from(at.x - self.origin.x) / self.scale,
            f64::from(at.y - self.origin.y) / self.scale,
        ]
    }

    /// Zooms about the cursor, leaving the point under it where it was.
    ///
    /// The origin follows the scale the clamp actually granted, so zooming
    /// against either stop still does not slide the drawing.
    pub fn zoom_at(&mut self, cursor: Pos2, factor: f64) {
        let before = self.scale;
        self.scale = (self.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        let granted = (self.scale / before) as f32;
        self.origin = cursor + (self.origin - cursor) * granted;
    }

    /// Slides the drawing under the pointer.
    pub fn pan(&mut self, delta: Vec2) {
        self.origin += delta;
    }

    /// Frames a document bounding box, in centimetres, inside an area.
    pub fn fit(&mut self, bbox: Rect, area: Rect) {
        let span = vec2(bbox.width().max(1.0e-3), bbox.height().max(1.0e-3));
        let air = 1.0 - 2.0 * MARGIN;
        let room = (area.width() * air / span.x).min(area.height() * air / span.y);
        self.scale = f64::from(room).clamp(MIN_SCALE, MAX_SCALE);
        self.origin = Pos2::ZERO;
        let centre = self.to_screen([f64::from(bbox.center().x), f64::from(bbox.center().y)]);
        self.origin = area.center() - centre.to_vec2();
    }

    /// The scale at which a centimetre of pattern measures a centimetre of
    /// glass.
    ///
    /// A screen point is not a pixel: the nominal density is shared out among
    /// however many pixels the window packs into one point, so the chip keeps
    /// its meaning on a dense display instead of lying by that factor.
    pub fn one_to_one(&mut self, pixels_per_point: f32) {
        self.scale = POINTS_PER_CM / f64::from(pixels_per_point);
    }

    /// What the zoom reads: 100 when a centimetre measures a centimetre.
    pub fn zoom_percent(self, pixels_per_point: f32) -> f64 {
        self.scale * f64::from(pixels_per_point) * 100.0 / POINTS_PER_CM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Screen points are coarse enough that a tenth of one is exact agreement.
    const EPS: f64 = 0.05;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn screen_and_document_round_trip() {
        let mut view = View::default();
        view.zoom_at(pos2(120.0, 80.0), 1.7);
        view.pan(vec2(-31.0, 12.0));
        let cm = [42.5, -13.25];
        let back = view.to_document(view.to_screen(cm));
        assert!(close(back[0], cm[0]) && close(back[1], cm[1]), "{back:?}");
    }

    #[test]
    fn zoom_at_cursor_keeps_the_point_under_it() {
        let mut view = View::default();
        let cursor = pos2(310.0, 190.0);
        let under = view.to_document(cursor);
        for factor in [1.25, 1.25, 0.5, 0.8] {
            view.zoom_at(cursor, factor);
        }
        let still = view.to_document(cursor);
        assert!(
            close(still[0], under[0]) && close(still[1], under[1]),
            "{still:?}"
        );
    }

    #[test]
    fn zooming_against_the_stop_still_holds_the_cursor() {
        let mut view = View::default();
        let cursor = pos2(64.0, 64.0);
        let under = view.to_document(cursor);
        view.zoom_at(cursor, 1.0e6);
        let still = view.to_document(cursor);
        assert!(
            close(still[0], under[0]) && close(still[1], under[1]),
            "{still:?}"
        );
    }

    #[test]
    fn one_to_one_is_a_centimetre_per_centimetre_at_any_pixels_per_point() {
        for ppp in [1.0_f32, 1.5, 2.0, 3.0] {
            let mut view = View::default();
            view.one_to_one(ppp);
            let ten_cm = f64::from(view.to_screen([10.0, 0.0]).x - view.to_screen([0.0, 0.0]).x);
            assert!(
                close(ten_cm * f64::from(ppp), 10.0 * POINTS_PER_CM),
                "{ppp}"
            );
            assert!(close(view.zoom_percent(ppp), 100.0), "{ppp}");
        }
    }

    #[test]
    fn fit_frames_the_bbox() {
        let mut view = View::default();
        let bbox = Rect::from_min_max(pos2(-6.0, 0.0), pos2(26.0, 104.0));
        let area = Rect::from_min_max(pos2(240.0, 40.0), pos2(940.0, 700.0));
        view.fit(bbox, area);
        let (min, max) = (view.to_screen([-6.0, 0.0]), view.to_screen([26.0, 104.0]));
        assert!(area.contains(min) && area.contains(max), "{min:?} {max:?}");
        let drawn = Rect::from_min_max(min, max);
        assert!(close(
            f64::from(drawn.center().x),
            f64::from(area.center().x)
        ));
        assert!(close(
            f64::from(drawn.center().y),
            f64::from(area.center().y)
        ));
        assert!(
            drawn.height() > area.height() * 0.8,
            "the fit fills its area"
        );
    }
}
