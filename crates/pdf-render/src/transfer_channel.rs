//! ISO 32000-2 §11.7.5.2's transfer function, chosen per pixel by shape and applied after
//! compositing.
//!
//! §10.5's transfer function is applied to a colour on its way to the device, and this tree
//! applies it where a colour is made. §11.7.5.2 says *which* function applies at a point, and its
//! answer is not a property of the colour at all:
//!
//! > The topmost object at any point shall be defined to be the topmost elementary object in the
//! > entire page stack that has a nonzero object shape value ( f j) at that point (that is, for
//! > which the point is inside the object).
//!
//! and, for when the chosen function runs, §11.7.5.3's NOTE:
//!
//! > This differs from the current halftone and transfer function, whose values are used only
//! > when all colour compositing has been completed and rasterization is being performed.
//!
//! So the clause composites raw colours and maps the finished pixel once, through the function of
//! the topmost elementary object covering it — where that object is fully opaque, and otherwise
//! through the page's default (§11.7.5.2's last sentence, and ADR 0570 for why a mark that is not
//! fully opaque never chooses anywhere). The two orderings agree wherever an object covers a
//! pixel completely and diverge at every antialiased edge, which is what ADR 1125 settled and what
//! `render-cpu/tests/all/transfer_edge.rs` and `render-raster/tests/transfer_edge.rs` measure.
//!
//! # What this module is
//!
//! The channel is what carries the clause's answer from `pdf_model`'s interpreter to a backend,
//! and [`resolve_transfers`] — with [`resolve_press_transfers`] for the simulated press, which
//! chooses by the same walk — is the one statement of what it means — **trap 2**: two backends
//! asked to decide which function a pixel takes would answer it apart, and the divergence would
//! be invisible.
//!
//! - [`TransferMap`] is one stated transfer as each device sees it: 256-entry tables, three for
//!   the screen's additive components and five for §10.8.3's simulated press — its four process
//!   colourants and its spot colourants. Exact rather than sampled, because the value it maps is
//!   one byte and the table holds every byte there is — the same argument [`crate::Transfer`]
//!   makes for §11.5.3's mask curve.
//! - [`TransferChannel`] is the page's elementary marks grouped into **runs**: a maximal stretch
//!   of consecutive marks painted under one function. Run numbers rise with painting order, so
//!   the topmost mark covering a pixel is the one in the *highest* run covering it, and no
//!   per-pixel ordering has to be carried.
//! - Each run holds its marks as *shapes* — §11.6.4.2's shape of each, opaque and under Normal
//!   with no mask ([`shape_of`]) — so a backend rasterises them through the machinery it already
//!   has and reads §11.7.5.2's `f j` out of the alpha channel.
//!
//! # What it costs where no file states a transfer
//!
//! Nothing. [`TransferBuilder`] stays inert until a mark arrives carrying a function, so the 973
//! corpus documents that state none pay one `Option::is_none` per mark and hold no shape at all;
//! [`DisplayList::transfers`] is `None`, and no second rasterisation happens.
//!
//! [`DisplayList::transfers`]: crate::DisplayList::transfers

use std::sync::Arc;

use crate::backend::TargetSpec;
use crate::display_list::{Command, DisplayList};
use crate::paint::{BlendMode, Color, Paint};

/// §10.5's transfer function as each device this tree draws on receives it: one table per
/// component, every table indexed and answered in additive form.
///
/// > If only a single function is specified, it shall apply to all components. An RGB device
/// > shall use the first three, a monochrome device shall use the gray transfer function only,
/// > and a CMYK device shall use all four.
///
/// — so the screen, an RGB device, takes three tables, red then green then blue
/// ([`TransferMap::apply_to`]), and §10.8.3's simulated press, a CMYK device, takes four, cyan
/// then magenta then yellow then black, and one more for its spot colourants
/// ([`resolve_press_transfers`]). `pdf_model` evaluates the file's functions at every eight-bit
/// input and hands the result over, which keeps PDF function evaluation out of the backends for
/// the reason [`crate::Ramp`] is a table as well. ADR 3769.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferMap {
    /// The screen's red, green, blue; `additive[c][i]` is the component's value at input `i`.
    additive: [[u8; 256]; 3],
    /// The simulated press's cyan, magenta, yellow and black, each on its complement: §10.5's "If
    /// the component is subtractive … it shall be converted to additive form by subtracting it
    /// from 1.0 before it is passed to the transfer function".
    process: [[u8; 256]; 4],
    /// The simulated press's spot colourants, on their complements as well.
    nonprimary: [u8; 256],
}

impl TransferMap {
    /// Builds a map from the functions already evaluated at every eight-bit input: the screen's
    /// three, the simulated press's four process colourants' and its spot colourants' one.
    #[must_use]
    pub fn from_samples(
        additive: [[u8; 256]; 3],
        process: [[u8; 256]; 4],
        nonprimary: [u8; 256],
    ) -> Self {
        Self {
            additive,
            process,
            nonprimary,
        }
    }

    /// One function's samples on every component of every device: §10.5's "If only a single
    /// function is specified, it shall apply to all components."
    #[must_use]
    pub fn uniform(table: [u8; 256]) -> Self {
        Self::from_samples([table; 3], [table; 4], table)
    }

    /// Whether this map changes no component of any colour on either device, which is what
    /// `/Identity` means and what an absent `/TR` leaves in force.
    ///
    /// A page whose only stated function is this one needs no channel at all, which is what
    /// [`TransferBuilder::push`] asks it. A map that is the identity on the screen's three and not
    /// on the press's black is not one: on the simulated press it changes the page.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        let identity = |table: &[u8; 256]| {
            table
                .iter()
                .enumerate()
                .all(|(index, &value)| usize::from(value) == index)
        };
        self.additive.iter().all(identity)
            && self.process.iter().all(identity)
            && identity(&self.nonprimary)
    }

    /// Maps one RGBA pixel of the screen in place, leaving its alpha alone.
    ///
    /// §10.5 speaks of "the value of a colour component in the device's native colour space", and
    /// §11's shape and opacity are a different quantity in a different clause, so alpha is not a
    /// component this touches.
    pub fn apply_to(&self, pixel: &mut [u8]) {
        for (component, table) in pixel.iter_mut().take(3).zip(self.additive.iter()) {
            // A 256-entry table indexed by a `u8` cannot be out of bounds; `get` keeps that
            // resting on the compiler rather than on the reader.
            if let Some(&mapped) = table.get(*component as usize) {
                *component = mapped;
            }
        }
    }
}

/// One premultiplied component of a press plane through `table`, at the pixel's `alpha`.
///
/// The planes are stored as §11.3.4's additive complements premultiplied by the page's one alpha
/// (`crate::separation`), so the value the press is handed is the stored one over the alpha, which
/// is already the additive form §10.5 asks for; the answer is multiplied back by the same alpha.
/// Exact at an opaque pixel, which is every pixel an opaque object's interior covers; at an
/// antialiased edge it is one level of rounding each way.
fn premultiplied_through(table: &[u8; 256], value: u8, alpha: u8) -> u8 {
    if alpha == 0 {
        return value;
    }
    let alpha = u32::from(alpha);
    let straight = u32::from(value)
        .saturating_mul(255)
        .saturating_add(alpha / 2)
        .checked_div(alpha)
        .unwrap_or(0);
    let mapped = table
        .get(straight.min(255) as usize)
        .copied()
        .map_or(0, u32::from);
    // not a repair: arithmetic on this tree's own planes, at most 255 × 255 ÷ 255.
    u8::try_from(mapped.saturating_mul(alpha).saturating_add(127) / 255).unwrap_or(u8::MAX)
}

/// One maximal stretch of consecutive elementary marks painted under one transfer function.
#[derive(Debug, Clone, PartialEq)]
pub struct TransferRun {
    /// The function the run's marks were painted under, or `None` for the page's default.
    ///
    /// A run carrying `None` records nothing to apply and everything to *occlude*: §11.7.5.2
    /// chooses by the topmost object whatever function that object turns out to have, so a
    /// default-function mark painted over a transferred one takes the pixel back to the default.
    map: Option<Arc<TransferMap>>,
    /// The run's marks, as opaque shapes in painting order.
    shapes: Vec<Command>,
}

impl TransferRun {
    /// The function this run's marks were painted under, or `None` for the page's default.
    #[must_use]
    pub fn map(&self) -> Option<&TransferMap> {
        self.map.as_deref()
    }

    /// The run's marks, as opaque shapes in painting order.
    #[must_use]
    pub fn shapes(&self) -> &[Command] {
        &self.shapes
    }
}

/// Every elementary mark on a page that §11.7.5.2 can choose a function from, grouped into runs.
///
/// Built by [`TransferBuilder`] as `pdf_model` interprets, and read by [`resolve_transfers`].
#[derive(Debug, Clone, PartialEq)]
pub struct TransferChannel {
    runs: Vec<TransferRun>,
    /// The device the channel's functions are applied on.
    device: TransferDevice,
}

/// The device a page's §10.5 functions are applied on, which decides which of a [`TransferMap`]'s
/// tables apply and where.
///
/// > An RGB device shall use the first three, a monochrome device shall use the gray transfer
/// > function only, and a CMYK device shall use all four.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TransferDevice {
    /// The screen, an RGB device: [`resolve_transfers`], over the finished raster.
    #[default]
    Screen,
    /// §10.8.3's simulated press, a CMYK device with spot colourants:
    /// [`resolve_press_transfers`], over its planes before steps b) to d) compose them. Set by
    /// `pdf_model` on a page it drew as that device's separations, and nowhere else.
    SimulatedPress,
}

impl TransferChannel {
    /// The device the channel's functions are applied on.
    #[must_use]
    pub fn device(&self) -> TransferDevice {
        self.device
    }

    /// States the device the channel's functions are applied on.
    pub fn set_device(&mut self, device: TransferDevice) {
        self.device = device;
    }

    /// The runs, in painting order: `runs[0]` is the earliest.
    #[must_use]
    pub fn runs(&self) -> &[TransferRun] {
        &self.runs
    }
}

/// Collects a page's elementary marks into [`TransferChannel`]'s runs as they are painted.
///
/// **Inert until the first mark carrying a function arrives**, which is what keeps the common case
/// free: a page that states no transfer records nothing, and one that states a transfer half way
/// down records only from there, because every pixel above it takes the page's default either way.
#[derive(Debug, Clone, Default)]
pub struct TransferBuilder {
    runs: Vec<TransferRun>,
}

impl TransferBuilder {
    /// Records one elementary mark and the function it was painted under.
    ///
    /// `shape` is the mark itself; what is stored is [`shape_of`]'s opaque outline of it, and a
    /// command with no shape — a group, which §11.7.5.2 does not call an elementary object — is
    /// dropped.
    pub fn push(&mut self, map: Option<Arc<TransferMap>>, command: &Command) {
        let map = map.filter(|map| !map.is_identity());
        if map.is_none() && self.runs.is_empty() {
            // Nothing has carried a function yet, so every pixel this mark could occlude already
            // takes the page's default. Recording it would cost a clone and decide nothing.
            return;
        }
        let Some(shape) = shape_of(command) else {
            return;
        };
        match self.runs.last_mut() {
            Some(last) if last.map == map => last.shapes.push(shape),
            _ => self.runs.push(TransferRun {
                map,
                shapes: vec![shape],
            }),
        }
    }

    /// Whether any mark has carried a function yet, and the builder is therefore recording.
    ///
    /// A caller that has marks of its own to record as *occluders* — a tiling pattern's tiles,
    /// which are copies of a cell rather than marks an interpreter drew — asks this first, so
    /// that a page stating no transfer walks nothing.
    #[must_use]
    pub fn is_live(&self) -> bool {
        !self.runs.is_empty()
    }

    /// The channel, or `None` where no mark on the page carried a function to apply.
    #[must_use]
    pub fn finish(self) -> Option<TransferChannel> {
        if self.runs.iter().all(|run| run.map.is_none()) {
            return None;
        }
        Some(TransferChannel {
            runs: self.runs,
            device: TransferDevice::Screen,
        })
    }
}

/// One elementary mark as the opaque shape §11.7.5.2 reads its `f j` from, or `None` for a
/// command that is not an elementary object.
///
/// Opaque, Normal and unmasked on purpose. §11.6.4.4 makes a soft mask a contribution to *alpha*
/// — shape is the object's own geometry — so a mark the clause hands the page's default because a
/// mask is in force still has a shape, and still occludes what is under it. The clip stays,
/// because §8.5.4's clip does bound the area an object covers.
///
/// # Where the shape is not the whole of what the command covers
///
/// §11.6.4.2 states it per kind of object, and two of them are not the mark's own region:
///
/// > For objects painted with the sh operator (8.7.4.2, "Shading operator"), the shape shall be
/// > 1.0 inside and 0.0 outside the bounds of the shading's painti ng geometry, disregarding the
/// > Background entry in the shading dictionary (see 8.7.4.3, "Shading dictionaries").
///
/// so a path painted with a shading that does not cover it — an axial ramp that does not extend,
/// a mesh's triangles, a sampled grid's cover — marks less than its path, and the shape is
/// [`crate::Shading::opaque`] rather than a solid (§11.6.4.2's fourth bullet says the same of a
/// pattern: "the shape shall be further constrained by the objects that define the pattern").
///
/// > For images (8.9, "Images"), the shape shall be 1.0 inside the image rectangle and 0.0
/// > outside it.
///
/// so an image whose samples carry *opacity* — §11.6.5.2's `/SMask` — has the whole image
/// rectangle for a shape, which is the unit square under the image's transform; one whose samples
/// carry shape is its own shape, which is §8.9.6.2's stencil. A stencil under a soft mask of its
/// own ([`crate::SampleAlpha::Both`]) answers with the stencil alone, which its producer keeps
/// apart from the opacity for exactly this question (ADR 1218); `pdf_model` hands no command a
/// raster that multiplied the two (ADR 1279), so the product is used only for a list some other
/// producer built that way, where it is the one alpha there is.
#[must_use]
pub fn shape_of(command: &Command) -> Option<Command> {
    match command {
        Command::Fill {
            path,
            transform,
            fill_rule,
            paint,
            clip,
            ..
        } => Some(Command::Fill {
            path: Arc::clone(path),
            transform: *transform,
            fill_rule: *fill_rule,
            paint: opaque_paint(paint),
            clip: *clip,
            mask: None,
            blend: BlendMode::Normal,
        }),
        Command::Stroke {
            path,
            transform,
            stroke,
            paint,
            clip,
            ..
        } => Some(Command::Stroke {
            path: Arc::clone(path),
            transform: *transform,
            stroke: stroke.clone(),
            paint: opaque_paint(paint),
            clip: *clip,
            mask: None,
            blend: BlendMode::Normal,
        }),
        Command::Image {
            image,
            transform,
            clip,
            ..
        } => Some(image_shape(image, *transform, *clip)),
        Command::Group { .. } | Command::Shaped { .. } => None,
    }
}

/// White, as [`shape_of`] paints a shape whose geometry is the mark's own.
fn white() -> Paint {
    Paint::Solid(Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    })
}

/// A paint that marks where its argument marks, at full opacity (§11.6.4.2).
///
/// A shading keeps its own geometry and loses its alpha; every other paint marks its whole path,
/// so white stands for it. A paint of a kind this crate does not know marks its path as far as
/// anything here can tell, which is the answer that occludes rather than the one that vanishes:
/// §11.7.5.2 hands a point whose topmost object is not fully opaque the page's default, and a
/// shape dropped from the channel would let an older function reach it instead.
fn opaque_paint(paint: &Paint) -> Paint {
    match paint {
        Paint::Shading(shading) => Paint::Shading(if shading.is_opaque() {
            Arc::clone(shading)
        } else {
            Arc::new(shading.opaque())
        }),
        _ => white(),
    }
}

/// §11.6.4.2's shape of one image, as a command whose drawn alpha is that shape.
///
/// See [`shape_of`] for the clause's two sentences and for the pair that cannot be separated.
fn image_shape(
    image: &crate::ImageSource,
    transform: crate::Transform,
    clip: Option<crate::ClipId>,
) -> Command {
    let samples = |image: crate::ImageSource| Command::Image {
        image,
        transform,
        alpha: 1.0,
        clip,
        mask: None,
        blend: BlendMode::Normal,
    };
    match image.sample_alpha() {
        crate::SampleAlpha::Shape => samples(image.clone()),
        crate::SampleAlpha::Opacity => Command::Fill {
            path: Arc::new(unit_square()),
            transform,
            fill_rule: crate::FillRule::NonZero,
            paint: white(),
            clip,
            mask: None,
            blend: BlendMode::Normal,
        },
        crate::SampleAlpha::Both => samples(image.shape().unwrap_or_else(|| image.clone())),
    }
}

/// The unit square §8.9.4 maps an image's samples onto, as a path.
fn unit_square() -> crate::Path {
    let mut path = crate::Path::new();
    path.push(crate::PathCommand::MoveTo(crate::Point::new(0.0, 0.0)));
    path.push(crate::PathCommand::LineTo(crate::Point::new(1.0, 0.0)));
    path.push(crate::PathCommand::LineTo(crate::Point::new(1.0, 1.0)));
    path.push(crate::PathCommand::LineTo(crate::Point::new(0.0, 1.0)));
    path.push(crate::PathCommand::Close);
    path
}

/// Applies §11.7.5.2's chosen function to every pixel of a finished page.
///
/// `page` is the composited raster in straight-alpha RGBA8, exactly as the backend is about to
/// hand it back. `coverage` rasterises one shape list at `target` and returns straight- or
/// premultiplied-alpha RGBA8 **with no medium under it** — only the alpha channel is read, and the
/// two spellings of alpha are the same byte.
///
/// # The walk, and why it is one pass per run rather than one per mark
///
/// Run numbers rise with painting order, so the topmost elementary object covering a pixel is the
/// one in the highest run covering it. The runs are therefore walked from the top down and each
/// pixel is decided by the *first* run that covers it, which means every mark is rasterised at
/// most once and a page whose transfer stops being stated half way down stops the walk early.
///
/// "Covers" is §11.7.5.2's own word — a **nonzero** object shape value, not a full one — so the
/// test is `alpha > 0` and an antialiased edge pixel belongs to the object whose edge it is. That
/// is ADR 1125's reading, and the one place it is stated.
///
/// # Errors
///
/// Whatever `coverage` returns: a backend's own failure to rasterise a shape list.
pub fn resolve_transfers<E>(
    channel: &TransferChannel,
    list: &DisplayList,
    target: TargetSpec,
    page: &mut [u8],
    coverage: impl FnMut(&DisplayList) -> Result<Vec<u8>, E>,
) -> Result<(), E> {
    let chosen = choose(channel, list, target, coverage)?;
    for (index, map) in chosen_maps(channel, &chosen) {
        let at = index.saturating_mul(4);
        if let Some(pixel) = page.get_mut(at..at.saturating_add(4)) {
            map.apply_to(pixel);
        }
    }
    Ok(())
}

/// Applies §11.7.5.2's chosen function to the planes of §10.8.3's simulated press, before steps
/// b) to d) compose them — the press's counterpart of [`resolve_transfers`], which a backend
/// calls instead of it on a list whose channel is [`TransferDevice::SimulatedPress`]'s. ISO
/// 32000-2 §10.5:
///
/// > Transfer functions shall always operate in the native colour space of the output device,
/// > regardless of the colour space in which colours were originally specified.
///
/// The simulated device's native colour space is its process colourants and its spot colourants
/// (§10.8.3 a)), so each plane takes its own function where the planes still are: `chromatic`'s
/// three channels cyan, magenta and yellow, `black`'s the black, and every channel of every plane
/// in `spots` the spot colourants' function. All of them premultiplied RGBA8 as the backend drew
/// them, before `crate::blending::resolve` or `crate::separation::resolve` reads them; and the
/// stored values are the complements §10.5 calls additive form, so nothing is subtracted here
/// (`premultiplied_through`). The choice of function per pixel is [`resolve_transfers`]'s, by
/// the same walk, because §11.7.5.2's topmost object is the same object on every plane: §11.7.3
/// keeps "a single shape value and opacity value" for process and spot components alike.
/// ADR 3769.
///
/// A spot plane's channels past the colourants it carries are mapped too and never read.
///
/// # Errors
///
/// Whatever `coverage` returns: a backend's own failure to rasterise a shape list.
pub fn resolve_press_transfers<E>(
    channel: &TransferChannel,
    list: &DisplayList,
    target: TargetSpec,
    chromatic: &mut [u8],
    black: &mut [u8],
    spots: &mut [&mut [u8]],
    coverage: impl FnMut(&DisplayList) -> Result<Vec<u8>, E>,
) -> Result<(), E> {
    let chosen = choose(channel, list, target, coverage)?;
    for (index, map) in chosen_maps(channel, &chosen) {
        let at = index.saturating_mul(4);
        let range = at..at.saturating_add(4);
        if let Some(pixel) = chromatic.get_mut(range.clone()) {
            let alpha = pixel[3];
            for (component, table) in pixel.iter_mut().take(3).zip(map.process.iter()) {
                *component = premultiplied_through(table, *component, alpha);
            }
        }
        if let Some(pixel) = black.get_mut(range.clone()) {
            let alpha = pixel[3];
            for component in pixel.iter_mut().take(3) {
                *component = premultiplied_through(&map.process[3], *component, alpha);
            }
        }
        for plane in spots.iter_mut() {
            if let Some(pixel) = plane.get_mut(range.clone()) {
                let alpha = pixel[3];
                for component in pixel.iter_mut().take(3) {
                    *component = premultiplied_through(&map.nonprimary, *component, alpha);
                }
            }
        }
    }
    Ok(())
}

/// Every pixel §11.7.5.2 hands a function other than the page's default, with that function.
fn chosen_maps<'channel>(
    channel: &'channel TransferChannel,
    chosen: &[Option<u32>],
) -> impl Iterator<Item = (usize, &'channel TransferMap)> {
    chosen.iter().enumerate().filter_map(move |(index, slot)| {
        let number = (*slot)?;
        let map = channel
            .runs()
            .get(number as usize)
            .and_then(TransferRun::map)?;
        Some((index, map))
    })
}

/// Which run's function each pixel of `target` takes, by §11.7.5.2's topmost object: an index into
/// `channel.runs()`, or `None` for the page's default. [`resolve_transfers`] says how.
fn choose<E>(
    channel: &TransferChannel,
    list: &DisplayList,
    target: TargetSpec,
    mut coverage: impl FnMut(&DisplayList) -> Result<Vec<u8>, E>,
) -> Result<Vec<Option<u32>>, E> {
    let pixels = (target.width as usize).saturating_mul(target.height as usize);
    // Which run each pixel takes, as an index into `channel.runs()`; `None` is the page's
    // default, which is every pixel no run covers.
    let mut chosen: Vec<Option<u32>> = vec![None; pixels];
    let mut decided = 0usize;
    // §14.11.2.1 stops the page's contents before the target's edge on a window, and a mark
    // outside that boundary marks nothing — so it chooses nothing either.
    let crop = crate::crop_area(list, target);
    for (number, run) in channel.runs().iter().enumerate().rev() {
        if decided >= pixels {
            break;
        }
        let Ok(number) = u32::try_from(number) else {
            break;
        };
        let shapes = list.shape_list(run.shapes().to_vec());
        let mut drawn = coverage(&shapes)?;
        // §14.11.2.1's boundary, cut on the coverage rather than inside the shape list: the
        // backends apply it to the page's own ink at exactly this point, and a shape that marks
        // where the page may not show anything chooses nothing.
        if let Some(crop) = crop {
            crate::crop_to_page(&mut drawn, target.width, 0, crop);
        }
        for (index, slot) in chosen.iter_mut().enumerate() {
            if slot.is_some() {
                continue;
            }
            let alpha = drawn
                .get(index.saturating_mul(4).saturating_add(3))
                .copied();
            if alpha.is_some_and(|alpha| alpha > 0) {
                *slot = Some(number);
                decided = decided.saturating_add(1);
            }
        }
    }
    Ok(chosen)
}

#[cfg(test)]
mod tests {
    use super::{TransferMap, premultiplied_through};

    /// The identity table, and its complement: `1 − x`.
    fn tables() -> ([u8; 256], [u8; 256]) {
        let mut identity = [0_u8; 256];
        let mut inverse = [0_u8; 256];
        for (index, (same, opposite)) in identity.iter_mut().zip(inverse.iter_mut()).enumerate() {
            let byte = u8::try_from(index).unwrap_or(u8::MAX);
            *same = byte;
            *opposite = u8::MAX.saturating_sub(byte);
        }
        (identity, inverse)
    }

    /// ISO 32000-2 §10.5: "An RGB device shall use the first three … and a CMYK device shall use
    /// all four." A map that changes only the press's black changes the page on the press, so it
    /// is not the identity and keeps its channel.
    #[test]
    fn a_map_that_moves_only_the_press_black_is_no_identity() {
        let (identity, inverse) = tables();
        assert!(TransferMap::uniform(identity).is_identity());
        let black_only = TransferMap::from_samples(
            [identity; 3],
            [identity, identity, identity, inverse],
            identity,
        );
        assert!(!black_only.is_identity());
        let mut pixel = [10_u8, 20, 30, 255];
        black_only.apply_to(&mut pixel);
        assert_eq!(pixel, [10, 20, 30, 255], "the screen takes the first three");
    }

    /// A press plane is stored premultiplied, and §10.5's input is the component itself: at an
    /// opaque pixel the stored value is the component, and at half coverage the stored value over
    /// the alpha is.
    #[test]
    fn a_premultiplied_plane_is_mapped_on_its_straight_value() {
        let (_, inverse) = tables();
        assert_eq!(premultiplied_through(&inverse, 255, 255), 0);
        assert_eq!(premultiplied_through(&inverse, 0, 255), 255);
        // Additive 1.0 at alpha 128 is stored as 128; its inverse, 0.0, is stored as 0.
        assert_eq!(premultiplied_through(&inverse, 128, 128), 0);
        // Additive 0.0 at alpha 128 maps to 1.0, stored as the alpha.
        assert_eq!(premultiplied_through(&inverse, 0, 128), 128);
        // A pixel nothing covers is left alone.
        assert_eq!(premultiplied_through(&inverse, 0, 0), 0);
    }
}
