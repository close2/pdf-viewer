//! The controller bar this program draws over a clip's pictures: ISO 32000-2 Table 307's
//! `/ShowControls`, "whether to display a movie controller bar while playing the movie" (ADR 3595).
//!
//! The table names the bar and says nothing of what is on it, so what it shows is **a choice**, the
//! three things every player's bar has: a button that says whether the clip runs (a triangle while
//! it is paused or held, two bars while it plays), the clip's place in its pass as a filled track
//! where the pass's length is known, and that place as minutes and seconds. It is drawn by the
//! processor into a raster of its own, [`BAR_HEIGHT`] pixels high and as wide as the clip's
//! rectangle, which each window lays over the rectangle's bottom edge — so `quorra`, GTK and Qt
//! show the same bar, and a host driving a window through the C ABI is handed [`Controller`]'s
//! three facts to draw one of its own.
//!
//! The digits are seven-segment figures drawn from rectangles: the bar carries no font, so that
//! drawing it reads no file and loads nothing at the first clip (`CLAUDE.md`'s startup rules).

use std::time::Duration;

use pdf_render::{Raster, RasterFormat};

/// The bar's height in pixels: a button's square and the digits' height with a margin.
pub const BAR_HEIGHT: u32 = 20;

/// The bar's colour, and the colour of what is drawn on it, red, green, blue and alpha.
const GROUND: [u8; 4] = [32, 32, 32, 224];
const MARK: [u8; 4] = [235, 235, 235, 255];
const TRACK: [u8; 4] = [96, 96, 96, 255];

/// A seven-segment figure's cell: its width, its height and its segments' thickness.
const DIGIT_WIDTH: u32 = 7;
const DIGIT_HEIGHT: u32 = 12;
const STROKE: u32 = 2;
/// The pixels between two figures, and around the colon.
const SPACING: u32 = 2;
/// A figure's cell and a colon's, each with the spacing after it.
const FIGURE_STEP: u32 = DIGIT_WIDTH + SPACING;
const COLON_STEP: u32 = STROKE + SPACING;
/// The digits' top edge, centred on the bar.
const DIGITS_TOP: u32 = (BAR_HEIGHT - DIGIT_HEIGHT) / 2;
/// The track's left edge, past the button's square, and its top, centred on the bar.
const TRACK_LEFT: u32 = BAR_HEIGHT + SPACING * 2;
const TRACK_TOP: u32 = BAR_HEIGHT / 2 - 1;
const TRACK_HEIGHT: u32 = 3;
/// The button's square, inset from the bar's left edge, top and bottom.
const BUTTON_INSET: u32 = 5;
const BUTTON_SIDE: u32 = BAR_HEIGHT - BUTTON_INSET * 2;
const PAUSE_STROKE: u32 = 3;
const PAUSE_SECOND: u32 = BUTTON_INSET + BUTTON_SIDE - PAUSE_STROKE;
/// The seven segments' places in a figure's cell: the middle bar's top, and half the cell.
const SEGMENT_MIDDLE: u32 = (DIGIT_HEIGHT - STROKE) / 2;
const SEGMENT_HALF: u32 = DIGIT_HEIGHT / 2;
const SEGMENT_RIGHT: u32 = DIGIT_WIDTH - STROKE;
const SEGMENT_BOTTOM: u32 = DIGIT_HEIGHT - STROKE;
const SEGMENT_LOWER: u32 = DIGIT_HEIGHT - SEGMENT_MIDDLE;
/// A colon's two dots, from the figures' top.
const COLON_UPPER: u32 = 3;
const COLON_LOWER: u32 = DIGIT_HEIGHT - 5;

/// What the bar shows: the three facts a host drawing its own bar is handed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Controller {
    /// Whether the clip's pictures run: false while it is paused, or held at its start by Table
    /// 291's `/A` false.
    pub playing: bool,
    /// The clip's place in the pass it is in.
    pub position: Duration,
    /// The pass's length, where it is known: a stated duration, or the clip's own once its first
    /// pass has been decoded; `None` before then and for an infinite duration.
    pub length: Option<Duration>,
}

/// Where on the bar a press landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pressed {
    /// The button: a running clip pauses, a paused or held one resumes.
    PlayPause,
    /// Elsewhere on the bar, which does nothing.
    Bar,
}

impl Controller {
    /// The bar, `width` pixels wide and [`BAR_HEIGHT`] high; `None` for a width of nothing.
    #[must_use]
    pub fn bar(&self, width: u32) -> Option<Raster> {
        if width == 0 {
            return None;
        }
        let mut bar = Canvas::new(width, BAR_HEIGHT);
        bar.fill(0, 0, width, BAR_HEIGHT, GROUND);
        self.button(&mut bar);
        let time = minutes_and_seconds(self.position);
        let digits_left = width
            .saturating_sub(text_width(&time))
            .saturating_sub(SPACING * 2);
        draw_text(&mut bar, digits_left, DIGITS_TOP, &time);
        // The track runs from the button to the digits.
        let track_right = digits_left.saturating_sub(SPACING * 2);
        if track_right > TRACK_LEFT {
            let span = track_right.saturating_sub(TRACK_LEFT);
            bar.fill(TRACK_LEFT, TRACK_TOP, span, TRACK_HEIGHT, TRACK);
            if let Some(length) = self.length.filter(|length| !length.is_zero()) {
                let done = (self.position.as_secs_f64() / length.as_secs_f64()).clamp(0.0, 1.0);
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a fraction in 0..=1 of a bar's width, which fits a u32"
                )]
                let filled = (f64::from(span) * done).round() as u32;
                bar.fill(TRACK_LEFT, TRACK_TOP, filled, TRACK_HEIGHT, MARK);
            }
        }
        Some(bar.raster)
    }

    /// What a press at `x` pixels from the bar's left edge reaches.
    #[must_use]
    pub fn pressed(x: u32) -> Pressed {
        if x < BAR_HEIGHT {
            Pressed::PlayPause
        } else {
            Pressed::Bar
        }
    }

    /// The button, in the square at the bar's left: two bars while the clip plays, which a press
    /// pauses, and a triangle pointing forward while it does not, which a press plays.
    fn button(&self, bar: &mut Canvas) {
        if self.playing {
            bar.fill(BUTTON_INSET, BUTTON_INSET, PAUSE_STROKE, BUTTON_SIDE, MARK);
            bar.fill(PAUSE_SECOND, BUTTON_INSET, PAUSE_STROKE, BUTTON_SIDE, MARK);
        } else {
            // Each column of the triangle reaches in from the edges by half its distance from the
            // left, so the point is at the square's middle on the right.
            for column in 0..BUTTON_SIDE {
                let inset = column / 2;
                bar.fill(
                    BUTTON_INSET.saturating_add(column),
                    BUTTON_INSET.saturating_add(inset),
                    1,
                    BUTTON_SIDE.saturating_sub(inset.saturating_mul(2)),
                    MARK,
                );
            }
        }
    }
}

/// Where the bar goes over `quad`, the clip's rectangle in window pixels: the bottom
/// [`BAR_HEIGHT`] pixels of its bounding box, as wide as it — the quadrilateral, corners in the
/// order `quad` has them — and the bar's width in whole pixels. `None` for a rectangle too small to
/// hold one.
#[must_use]
pub fn over(quad: [f32; 8]) -> Option<([f32; 8], u32)> {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    let left = xs.iter().copied().fold(f32::INFINITY, f32::min);
    let right = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let top = ys.iter().copied().fold(f32::INFINITY, f32::min);
    let bottom = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    #[expect(
        clippy::cast_precision_loss,
        reason = "the bar's height is a small constant"
    )]
    let height = BAR_HEIGHT as f32;
    if !(right - left >= 1.0 && bottom - top >= height) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive window width in pixels, far below u32's range"
    )]
    let width = (right - left).round() as u32;
    let upper = bottom - height;
    Some((
        [left, upper, right, upper, right, bottom, left, bottom],
        width,
    ))
}

/// A place in time as the bar shows it, "m:ss": minutes without a bound, seconds rounded down.
fn minutes_and_seconds(at: Duration) -> String {
    let seconds = at.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The pixels `text` — figures and colons — takes.
fn text_width(text: &str) -> u32 {
    text.chars()
        .map(|character| {
            if character == ':' {
                COLON_STEP
            } else {
                FIGURE_STEP
            }
        })
        .fold(0, u32::saturating_add)
}

/// `text` drawn from `left`, `top`: each figure in seven segments, each colon two dots.
fn draw_text(canvas: &mut Canvas, left: u32, top: u32, text: &str) {
    let mut x = left;
    for character in text.chars() {
        if character == ':' {
            canvas.fill(x, top.saturating_add(COLON_UPPER), STROKE, STROKE, MARK);
            canvas.fill(x, top.saturating_add(COLON_LOWER), STROKE, STROKE, MARK);
            x = x.saturating_add(COLON_STEP);
            continue;
        }
        if let Some(value) = character.to_digit(10) {
            draw_digit(canvas, x, top, value);
        }
        x = x.saturating_add(FIGURE_STEP);
    }
}

/// The segments each figure lights, in the order a, b, c, d, e, f, g: top, upper right, lower
/// right, bottom, lower left, upper left, middle.
const SEGMENTS: [[bool; 7]; 10] = [
    [true, true, true, true, true, true, false],
    [false, true, true, false, false, false, false],
    [true, true, false, true, true, false, true],
    [true, true, true, true, false, false, true],
    [false, true, true, false, false, true, true],
    [true, false, true, true, false, true, true],
    [true, false, true, true, true, true, true],
    [true, true, true, false, false, false, false],
    [true, true, true, true, true, true, true],
    [true, true, true, true, false, true, true],
];

/// Figure `value` in its cell at `left`, `top`.
fn draw_digit(canvas: &mut Canvas, left: u32, top: u32, value: u32) {
    let Some(lit) = usize::try_from(value)
        .ok()
        .and_then(|value| SEGMENTS.get(value))
    else {
        return;
    };
    let right = left.saturating_add(SEGMENT_RIGHT);
    let middle = top.saturating_add(SEGMENT_MIDDLE);
    let segments = [
        (left, top, DIGIT_WIDTH, STROKE),
        (right, top, STROKE, SEGMENT_HALF),
        (right, middle, STROKE, SEGMENT_LOWER),
        (
            left,
            top.saturating_add(SEGMENT_BOTTOM),
            DIGIT_WIDTH,
            STROKE,
        ),
        (left, middle, STROKE, SEGMENT_LOWER),
        (left, top, STROKE, SEGMENT_HALF),
        (left, middle, DIGIT_WIDTH, STROKE),
    ];
    for (on, (x, y, width, height)) in lit.iter().zip(segments) {
        if *on {
            canvas.fill(x, y, width, height, MARK);
        }
    }
}

/// A raster drawn in axis-aligned rectangles.
struct Canvas {
    raster: Raster,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Self {
        let bytes = u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(4);
        // A bar is as wide as a window; one past memory is drawn as nothing.
        let bytes = usize::try_from(bytes).unwrap_or(0);
        Self {
            raster: Raster {
                width,
                height,
                format: RasterFormat::Rgba8,
                data: vec![0; bytes],
            },
        }
    }

    /// The rectangle at `x`, `y`, `width` by `height`, in `colour`, cut to the raster.
    fn fill(&mut self, x: u32, y: u32, width: u32, height: u32, colour: [u8; 4]) {
        let right = x.saturating_add(width).min(self.raster.width);
        let bottom = y.saturating_add(height).min(self.raster.height);
        let bytes_of = |pixels: u32| {
            usize::try_from(pixels).map_or(usize::MAX, |pixels| pixels.saturating_mul(4))
        };
        let stride = bytes_of(self.raster.width);
        let from = bytes_of(x);
        let to = bytes_of(right);
        for row in y..bottom {
            let row = usize::try_from(row).unwrap_or(usize::MAX);
            let start = row.saturating_mul(stride);
            if let Some(pixels) = self
                .raster
                .data
                .get_mut(start.saturating_add(from)..start.saturating_add(to))
            {
                for pixel in pixels.as_chunks_mut::<4>().0 {
                    *pixel = colour;
                }
            }
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test arithmetic on a bar of a few hundred pixels"
)]
mod tests {
    use super::*;

    fn pixel(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
        let at = usize::try_from((y * raster.width + x) * 4).expect("in range");
        let mut colour = [0; 4];
        colour.copy_from_slice(&raster.data[at..at + 4]);
        colour
    }

    /// The bar is as wide as asked and [`BAR_HEIGHT`] high; a playing clip's button is two bars
    /// with ground between them, a paused one's a triangle whose middle column is lit; the track is
    /// filled to the position's share of the length.
    #[test]
    fn the_bar_shows_the_button_the_track_and_the_time() {
        let playing = Controller {
            playing: true,
            position: Duration::from_secs(30),
            length: Some(Duration::from_secs(60)),
        };
        let bar = playing.bar(200).expect("a bar");
        assert_eq!((bar.width, bar.height), (200, BAR_HEIGHT));
        assert_eq!(pixel(&bar, 6, 10), MARK, "the pause button's left bar");
        assert_eq!(
            pixel(&bar, 10, 10),
            GROUND,
            "between the pause button's bars"
        );
        // The time "0:30" takes 9 + 4 + 9 + 9 pixels, ending 4 pixels from the right edge.
        let digits_left = 200 - text_width("0:30") - SPACING * 2;
        let track_left = BAR_HEIGHT + SPACING * 2;
        let track_right = digits_left - SPACING * 2;
        let half = track_left + (track_right - track_left) / 2;
        assert_eq!(pixel(&bar, half - 2, BAR_HEIGHT / 2), MARK, "played half");
        assert_eq!(
            pixel(&bar, half + 2, BAR_HEIGHT / 2),
            TRACK,
            "the half to come"
        );
        let paused = Controller {
            playing: false,
            ..playing
        };
        let bar = paused.bar(200).expect("a bar");
        assert_eq!(pixel(&bar, 10, 10), MARK, "the play triangle's middle");
        assert_eq!(pixel(&bar, 14, 5), GROUND, "outside the triangle's slope");
    }

    /// The time is minutes and seconds, the seconds rounded down; an unknown length leaves the
    /// track unfilled; a bar of no width is none.
    #[test]
    fn the_time_is_minutes_and_seconds_and_an_unknown_length_fills_nothing() {
        assert_eq!(minutes_and_seconds(Duration::from_millis(125_900)), "2:05");
        let unknown = Controller {
            playing: true,
            position: Duration::from_secs(5),
            length: None,
        };
        let bar = unknown.bar(120).expect("a bar");
        let track_left = BAR_HEIGHT + SPACING * 2;
        assert_eq!(pixel(&bar, track_left + 1, BAR_HEIGHT / 2), TRACK);
        assert!(unknown.bar(0).is_none());
        assert_eq!(Controller::pressed(3), Pressed::PlayPause);
        assert_eq!(Controller::pressed(BAR_HEIGHT + 1), Pressed::Bar);
    }

    /// The bar lies over the bottom of the rectangle's box, as wide as it; a rectangle lower than
    /// the bar holds none.
    #[test]
    fn the_bar_lies_along_the_rectangle_s_bottom() {
        let quad = [10.0, 20.0, 110.0, 20.0, 110.0, 70.0, 10.0, 70.0];
        let (bar, width) = over(quad).expect("room for a bar");
        assert_eq!(width, 100);
        assert_eq!(bar, [10.0, 50.0, 110.0, 50.0, 110.0, 70.0, 10.0, 70.0]);
        assert!(over([0.0, 0.0, 50.0, 0.0, 50.0, 10.0, 0.0, 10.0]).is_none());
    }

    /// Each figure lights its own segments: a 1 only the right side, an 8 all seven.
    #[test]
    fn each_figure_lights_its_segments() {
        let mut one = Canvas::new(DIGIT_WIDTH, DIGIT_HEIGHT);
        draw_digit(&mut one, 0, 0, 1);
        assert_eq!(pixel(&one.raster, 0, 0), [0; 4], "a 1 has no top");
        assert_eq!(pixel(&one.raster, DIGIT_WIDTH - 1, 2), MARK);
        let mut eight = Canvas::new(DIGIT_WIDTH, DIGIT_HEIGHT);
        draw_digit(&mut eight, 0, 0, 8);
        assert_eq!(pixel(&eight.raster, 3, 0), MARK, "top");
        assert_eq!(
            pixel(&eight.raster, 3, (DIGIT_HEIGHT - STROKE) / 2),
            MARK,
            "middle"
        );
        assert_eq!(pixel(&eight.raster, 3, DIGIT_HEIGHT - 1), MARK, "bottom");
    }
}
