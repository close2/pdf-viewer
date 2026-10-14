//! The pictures a frame puts on the screen, and what each one stands in for while its page is drawn (ADR 1843).

use gtk4::prelude::*;
use viewer_core::{Answer, Query};

use viewer_host::trace::Topic;

use crate::page;

use super::Host;
use super::window::logical;

impl Host {
    /// The textures to put on the screen, and where.
    ///
    /// **One texture per page of Table 29's arrangement**, since `/PageLayout` was obeyed; under
    /// `SinglePage` this is the one placement it has always been. §12.4.4.1: while one of Table
    /// 164's effects is in flight the window shows the effect and not the page — one texture at the
    /// viewport's own origin, because a frame is already a picture of two pages placed where they
    /// belong. And a page still being drawn is shown as it stands where it belongs
    /// (`viewer_host::drawing::over_frames`, ADR 1825), which is why [`Pictures::held`] counts the
    /// viewer's own pixels apart: the first frame is a finished page, so a page shown part-way is
    /// not what `first frame on the screen` says.
    pub(super) fn placements(&mut self) -> Pictures {
        if let Some(frame) = self.transition_placement() {
            return Pictures {
                placements: vec![frame],
                held: 1,
                glimpsed: None,
            };
        }
        let frames = match self.state.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames,
            _ => Vec::new(),
        };
        let held = frames.len();
        let partial = self.state.drawing.partial();
        let geometry = |page| match self.state.viewer.query(Query::PageGeometry(page)) {
            Answer::Geometry(geometry) => Some(geometry),
            _ => None,
        };
        // A page drawn part-way where it belongs (ADR 1825), a frame from before a zoom scaled to
        // the extent the page has now until its own lands (ADR 1843), and a page with no frame
        // at all shown from its retained picture (ADR 1866).
        let focused = self.state.documents.focused();
        let frames = match self.state.read_look.as_ref() {
            // Page one as it stood at its last look while it was read, until the page has pixels
            // of its own (ADR 1867): the page's own picture at the target its first frame is drawn
            // at, placed where that frame will be — and only while that frame is still coming, so
            // a page whose read failed is not left showing part of itself (trap 5).
            Some(look)
                if held == 0
                    && partial.is_none()
                    && (self.anticipated.is_some() || self.state.drawing.interval().is_some()) =>
            {
                vec![look.placed()]
            }
            // A page past the whole-page bound shown over them from what its window drew (ADR 1892).
            _ => self.state.oversized.place(
                self.state.retained.fill(
                    viewer_host::stand_in::placed(
                        viewer_host::drawing::over_frames(frames, partial, focused, geometry),
                        geometry,
                    ),
                    focused,
                    geometry,
                ),
                partial,
                focused,
                geometry,
            ),
        };
        let glimpsed = partial
            .filter(|partial| {
                frames
                    .iter()
                    .any(|placed| std::ptr::eq(placed.frame.raster, partial.raster()))
            })
            .map(|partial| (partial.page(), partial.fraction()));
        for placed in frames.iter().filter(|placed| placed.stands_in()) {
            self.state
                .trace
                .say(Topic::Frames, format_args!("{placed}"));
        }
        let placements = frames
            .into_iter()
            .filter_map(|placed| match page::texture(placed.frame.raster) {
                Ok(texture) => Some(Picture {
                    texture,
                    origin: placed.frame.origin,
                    extent: placed.extent,
                    bytes: placed.frame.raster.data.len(),
                }),
                Err(error) => {
                    eprintln!("note: {error}");
                    None
                }
            })
            .collect();
        Pictures {
            placements,
            held,
            glimpsed,
        }
    }
}

/// Puts one picture where its frame belongs: laid out at its texture's size, moved to its origin,
/// and — for a stand-in only — scaled to the extent the page has now (ADR 1843).
pub(super) fn place(fixed: &gtk4::Fixed, slot: &gtk4::Picture, picture: &Picture, scale: f64) {
    slot.set_paintable(Some(&picture.texture));
    let (width, height) = (picture.texture.width(), picture.texture.height());
    slot.set_size_request(
        logical(f64::from(width), scale),
        logical(f64::from(height), scale),
    );
    slot.set_visible(true);
    // `GtkFixed::move` is a translation, and a stand-in is the same translation with
    // a scale after it: the old frame laid out at its own size and drawn by GSK at the
    // extent the page has now, linearly, on whichever renderer the window has. A size
    // request cannot do it, because `GtkFixed` gives a child its natural size and a
    // picture's is its texture's, so a notch out would have stayed large (ADR 1843).
    #[expect(
        clippy::cast_precision_loss,
        reason = "extents are under 2^24 pixels: `MAX_PIXELS` bounds the raster"
    )]
    let fill = (
        picture.extent.0 as f32 / width.max(1) as f32,
        picture.extent.1 as f32 / height.max(1) as f32,
    );
    let (x, y) = (
        f64::from(picture.origin.0) / scale,
        f64::from(picture.origin.1) / scale,
    );
    let scaled = i64::from(picture.extent.0) != i64::from(width)
        || i64::from(picture.extent.1) != i64::from(height);
    if scaled {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a viewport's logical position is far inside an `f32`"
        )]
        let at = gtk4::graphene::Point::new(x as f32, y as f32);
        fixed.set_child_transform(
            slot,
            Some(
                &gtk4::gsk::Transform::new()
                    .translate(&at)
                    .scale(fill.0, fill.1),
            ),
        );
    } else {
        // The translation alone, to the whole device pixel nearest the origin: GSK draws a
        // texture at a fractional offset by filtering it, which moves every row of the page by
        // a fraction and blends the page's edge row with the ground (ADR 1865).
        let (x, y) = viewer_host::stand_in::whole_pixel(picture.origin);
        fixed.move_(slot, f64::from(x) / scale, f64::from(y) / scale);
    }
}

/// One texture a frame puts on the screen.
pub(super) struct Picture {
    /// The pixels, copied once into the toolkit's own texture.
    pub(super) texture: gtk4::gdk::MemoryTexture,
    /// Where its top-left corner sits in the viewport, in device pixels.
    pub(super) origin: (f32, f32),
    /// The width and height it fills, in device pixels: the texture's own, or the extent the page
    /// has now where the texture is its last frame from before a zoom (`viewer_host::stand_in`).
    pub(super) extent: (u32, u32),
    /// How many bytes the copy into the texture took.
    pub(super) bytes: usize,
}

/// What [`Host::placements`] puts on the screen.
pub(super) struct Pictures {
    /// One texture per page shown.
    pub(super) placements: Vec<Picture>,
    /// How many of them are the viewer's own pixels — a finished page or a transition's face.
    pub(super) held: usize,
    /// The page shown as it stands, and how much of it is drawn, where one is.
    pub(super) glimpsed: Option<(usize, f64)>,
}
