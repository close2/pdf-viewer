//! §14.11.2.2's guidelines, drawn over a page a person has asked to see them on.
//!
//! `crate::guidelines` says which boundaries carry one and how each looks; this is where they
//! become marks. They go over everything the page draws, its annotations included, because a
//! guideline is a statement about the page's boundaries rather than part of what lies inside
//! them, and they are drawn in the display list's own space under the page's base transform so
//! that `/Rotate`, the crop box's origin and the magnification place them exactly where the page
//! content they bound is placed. ADR 2440.

use std::sync::Arc;

use pdf_render::{BlendMode, Color, Command, Paint, Path, PathCommand, Point, Stroke, Transform};

use crate::colour::Compositing;
use crate::optional_content::Purpose;
use crate::page::Page;

use super::Interpreter;
use super::report::Unsupported;

impl Interpreter<'_> {
    /// Draws the page's guidelines, where a person has asked for them and the output is a screen.
    ///
    /// **Only over a page drawn on the device's own components.** A guideline's `/C` is a
    /// `DeviceRGB` colour "that shall be used for displaying the guidelines", and a page composited
    /// in a blending space of its own (§11.4.7) carries every mark through that space's
    /// conversion on the way out — a guideline drawn into it would come out in a colour the page
    /// never asked for. Such a page, and one drawn as §10.8.3's separations, gets no guideline
    /// and a sentence saying so, rather than a guideline in the wrong colour (trap 5).
    pub(super) fn draw_guidelines(&mut self, page: &Page, base: Transform) {
        // "[O]n the screen": a page put onto paper carries none.
        if !self.view.guidelines() || self.view.purpose() != Purpose::View {
            return;
        }
        let guidelines = crate::guidelines::of(self.document, page);
        for detail in guidelines.malformed {
            self.note(Unsupported::Guidelines { detail });
        }
        if guidelines.lines.is_empty() {
            return;
        }
        if !matches!(self.compositing, Compositing::Device) || self.view.separation_simulation() {
            self.note(Unsupported::Guidelines {
                detail: "the page is composited in a colour space of its own or drawn as \
                         separations, so its guidelines, whose colour is DeviceRGB (Table 397), \
                         are not drawn over it (§11.4.7, §10.8.3)"
                    .to_owned(),
            });
            return;
        }
        for line in guidelines.lines {
            let [x0, y0, x1, y1] = line.rect;
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(x0, y0)));
            path.push(PathCommand::LineTo(Point::new(x1, y0)));
            path.push(PathCommand::LineTo(Point::new(x1, y1)));
            path.push(PathCommand::LineTo(Point::new(x0, y1)));
            path.push(PathCommand::Close);
            let mut stroke = Stroke {
                width: line.style.width,
                ..Stroke::default()
            };
            // Table 397's `/D` is "in the same format as in the line dash pattern parameter of
            // the graphics state", and "[t]he dash phase shall not be specified and shall be
            // assumed to be 0" — so it is §8.4.3.6's pattern at phase 0, put into the stroke by
            // the one function that decides what a pattern means.
            super::apply_dash(line.style.dash, 0.0, &mut stroke);
            let [r, g, b] = line.style.colour;
            self.draw(Command::Stroke {
                path: Arc::new(path),
                transform: base,
                stroke,
                paint: Paint::Solid(Color::rgb(r, g, b)),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
    }
}
