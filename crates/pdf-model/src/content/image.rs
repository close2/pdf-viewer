//! Image `XObject`s: §8.9's samples, stencils, masks and alternates on their way into a
//! [`Command::Image`].
//!
//! The decoding itself is `crate::image`'s; what is decided here is what the graphics
//! state does to a decoded image — §10.5's transfer, §11.6.4.3's mask override, and
//! §8.9.6.2's stencil whose current colour is a pattern.

use std::sync::Arc;

use pdf_render::{BlendMode, Color, Command, FillRule, Path, PathCommand, Point};
use pdf_syntax::{Dictionary, Object};

use crate::colour::Conversion;

use super::colour::Intent;
use super::ext_gstate::Transfer;
use super::pattern::PatternPaint;
use super::report::Unsupported;
use super::transparency::Painted;
use super::{GraphicsState, Interpreter};

/// One decoded image through §10.5's transfer function, or unchanged where none is in effect.
///
/// Straight alpha in, straight alpha out: the samples are RGBA and only the three colour
/// components are mapped, for [`Transfer::apply`]'s reason.
///
/// **The cost is one lookup per sample and it is paid only where a file states a transfer** — 1 of
/// the 974 corpus documents, measured by `examples/transfer_function_census`, and 13 state a `/TR`
/// at all with the other 12 saying `/Identity`. An image with no transfer is moved rather than
/// touched.
fn transferred_image(image: pdf_render::Image, transfer: Option<&Transfer>) -> pdf_render::Image {
    let Some(transfer) = transfer else {
        return image;
    };
    let mut image = image;
    // A memo over the 8-bit triple, because a transfer is a pure function of a colour and a
    // photograph repeats its colours: the same argument `image::SampleMemo` records for
    // §8.6's spaces, one clause along.
    let mut memo: std::collections::HashMap<[u8; 3], [u8; 3]> = std::collections::HashMap::new();
    // The samples are shared, so a transfer takes a copy — which is right rather than merely
    // necessary: the same XObject drawn twice under two graphics states is two pictures, and
    // writing through the `Arc` would make the second overwrite the first.
    let mut data = image.data.to_vec();
    for pixel in data.chunks_exact_mut(4) {
        let Some(rgb) = pixel.get(..3) else { continue };
        let key = [rgb[0], rgb[1], rgb[2]];
        let mapped = *memo.entry(key).or_insert_with(|| {
            let out = transfer.apply(Color {
                r: f32::from(key[0]) / 255.0,
                g: f32::from(key[1]) / 255.0,
                b: f32::from(key[2]) / 255.0,
                a: 1.0,
            });
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped to 0..=1 by Transfer::apply, so the product is a byte"
            )]
            let bytes = [
                (out.r * 255.0).round() as u8,
                (out.g * 255.0).round() as u8,
                (out.b * 255.0).round() as u8,
            ];
            bytes
        });
        if let Some(three) = pixel.get_mut(..3) {
            three.copy_from_slice(&mapped);
        }
    }
    image.data = Arc::from(data.as_slice());
    image
}

impl Interpreter<'_> {
    /// How this image's samples are converted: §11.4's target, and §8.6.5.9's black point under
    /// the intent Table 87 lets the image state for itself.
    ///
    /// ISO 32000-2 §8.6.5.8 names three routes to a rendering intent and this is the third:
    ///
    /// > Rendering intents shall be specified with the ri operator (see 8.4.4, "Graphics state
    /// > operators"), the RI entry in a graphics state parameter dictionary (see 8.4.5,
    /// > "Graphics state parameter dictionaries"), or with the Intent entry in image
    /// > dictionaries (see 8.9.5, "Image dictionaries").
    ///
    /// §8.9.5.1 Table 87 states the entry and its two conditions:
    ///
    /// > The name of a colour rendering intent that shall be used in rendering any image that
    /// > is not an image mask (see 8.6.5.8, "Rendering intents"). This value is ignored if
    /// > ImageMask is true . Default value: the current rendering intent in the graphics state.
    ///
    /// The second condition needs no branch and is left to `ImageMask` itself: §8.9.6.2's
    /// stencil "does not specify colours", so its samples carry the fill colour, which was
    /// converted under the graphics state's own intent before it ever reached this function.
    fn image_conversion(&mut self, dict: &Dictionary, state: &GraphicsState) -> Conversion {
        let intent = match self.document.get_key(dict, "Intent") {
            Object::Name(name) => Intent::read(name.as_bytes()),
            _ => state.intent,
        };
        Conversion::new(
            self.compositing.clone(),
            state.black_point_under(intent).applies(),
        )
    }

    /// §8.9.5.4 step d): which of a base image's `/Alternates` is drawn in its place.
    ///
    /// # The algorithm this implements is Errata Collection 3's, not `doc/md/`'s
    ///
    /// Issue #79, `/State` `Review` `Completed`. The five steps below are the 2020 clause with
    /// the erratum's strikeouts and carets applied in the order the annotations sit on pages 279
    /// and 280 of the sponsored copy, which `tools/spec-errata` reads back and `doc/md/` shows
    /// none of; the carets' own words are quoted:
    ///
    /// - a) "If the base image contains an OC entry that specifies that the content is not
    ///   visible, then nothing shall be shown."
    /// - b) unamended: "[i]f the base image contains an OC entry that specifies that the base
    ///   image is visible, then the base image shall be rendered."
    /// - c) "Otherwise if the PDF is being printed and any of the Alternates entries has
    ///   `DefaultForPrinting` set to true, then that alternate image shall be printed."
    /// - d) "Otherwise, the list of alternates specified by the base image Alternates entry is
    ///   examined, and the first alternate containing an OC entry specifying that its content is
    ///   visible shall be shown (Alternates that have no OC entry shall not be shown.)
    ///   Furthermore if the image dictionary that forms the value of the Image key of the
    ///   selected alternate contains an OC entry, then that OC in the image dictionary shall not
    ///   be examined."
    /// - e) "If steps c and d above do not identify an alternate to be rendered then the base
    ///   image shall be rendered."
    ///
    /// **This function is d), and nothing else.** a) and b) are `xobject.rs`'s, because they are
    /// about the base image and never reach an alternate; c) addresses printing and this device
    /// is a screen, which is why `/DefaultForPrinting` is read by nothing here; e) is the
    /// caller's fall-through when this answers `None`.
    ///
    /// # Why the amended clause replaced a documented contradiction rather than adding one
    ///
    /// The 2020 step c) said in one place that "the first entry not containing an OC key … shall
    /// be selected" and in another that where "none of the alternate image dictionaries have an
    /// OC key … nothing shall be shown", and this function used to implement the first of those
    /// with the second reported beside it. The erratum settles it the other way — an alternate
    /// with no `/OC` is not shown — and the settlement costs nothing, because e) draws the base
    /// image where d) selects nothing at all.
    ///
    /// The rewrite was declined once, in the four-hundred-and-seventeenth session, on the ground
    /// that the amended a) "reads as terminal and would leave the amended d) unreachable". It is
    /// terminal, and d) is unreachable **for a hidden base image** — which is the amendment
    /// rather than a contradiction in it: a) and b) between them dispose of every base image that
    /// states an `/OC`, so c) and d) begin at "Otherwise" and are reached by a base image that
    /// states none. Read that way the five steps are total, disjoint and reachable, which the
    /// 2020 four were not.
    ///
    /// No corpus document carries an `/Alternates` entry at all — measured over all 964 openable
    /// ones — so every rule here rests on the clause and on the tests beside it.
    pub(super) fn alternate_image(
        &mut self,
        base: &Dictionary,
        name: &str,
    ) -> Option<Arc<pdf_syntax::Stream>> {
        let stated = self.document.get_key(base, "Alternates");
        let alternates = stated.as_array()?;
        for entry in alternates {
            let resolved = self.document.resolve(entry);
            let Some(alternate) = resolved.as_dict() else {
                continue;
            };
            // "the first alternate containing an OC entry specifying that its content is visible
            // shall be shown (Alternates that have no OC entry shall not be shown.)"
            let Some(group) = alternate.get("OC").cloned() else {
                continue;
            };
            if !self.shows_optional_content(&group) {
                continue;
            }

            // Table 89 makes `/Image` required, so an entry without one identifies no alternate
            // to be rendered — which is step e)'s condition, so the base image is drawn and the
            // document's broken entry is named rather than swallowed.
            let image = self.document.get_key(alternate, "Image");
            let Some(image) = image.as_stream().cloned() else {
                self.note(Unsupported::Image {
                    name: format!("{name}: an alternate image dictionary states no /Image"),
                });
                return None;
            };
            // "that OC in the image dictionary shall not be examined" — Table 87's `/OC` on the
            // alternate's own image `XObject` is deliberately not consulted here. The dictionary
            // that selected it has already answered the visibility question.
            return Some(image);
        }
        None
    }

    pub(super) fn draw_image(
        &mut self,
        image: crate::image::NamedStream<'_>,
        name: &str,
        resources: &Dictionary,
        state: &GraphicsState,
    ) {
        let stream = image.stream;
        // §8.6.8, of a `d1` glyph description or an uncoloured tiling pattern's stream:
        // "unless painting an image mask, all image painting operators shall be ignored".
        // Its NOTE 1 gives the reason, and it is the whole of what those two circumstances
        // are about — a stencil "does not specify colours; instead, it designates places
        // where the current colour is painted".
        if self.uncoloured
            && !matches!(
                self.document.get_key(&stream.dict, "ImageMask"),
                Object::Boolean(true)
            )
        {
            return;
        }

        // A soft mask whose grid is not the image's is mapped onto the same unit square and
        // combined at output resolution (§11.6.5.2 Table 143). Two rasters are combined
        // instead — on the finer of the two grids where that grid can be built, and by the
        // backend at device resolution where it cannot. What is left to report is a mask
        // neither route can read, which `image::unapplied_soft_mask` names.
        if let Some(detail) =
            crate::image::unapplied_soft_mask(self.document, &stream.dict, resources)
        {
            self.note(Unsupported::Image {
                name: format!("{name}: {detail}"),
            });
        }
        // `/Mask` makes part of the image transparent, either through an explicit mask — a
        // second image naming the areas to leave unpainted (§8.9.6.3) — or through a
        // colour-key range array (§8.9.6.4). Both are applied as of the fourteenth session;
        // what remains reportable is the cases they refuse, and `image::unapplied_mask` is
        // asked rather than the dictionary so that a report cannot outlive the gap.
        //
        // Not to be confused with §8.9.6.2, *stencil* masking, which is this image's own
        // `/ImageMask` and is implemented — see `tests/image_masks.rs`.
        if let Some(detail) = crate::image::unapplied_mask(self.document, &stream.dict, resources) {
            self.note(Unsupported::Image {
                name: format!("{name}: {detail}"),
            });
        }
        // §7.4.8 puts a JPEG's dimensions in the codestream and this tree draws them from
        // there, so an image whose dictionary says something else is *drawn* rather than
        // refused — and said out loud all the same, because the picture on the page is then
        // not the one the file described. `image::contradicted_frame` has the reading.
        if let Some(detail) = crate::image::contradicted_frame(self.document, stream) {
            self.note(Unsupported::Image {
                name: format!("{name}: {detail}"),
            });
        }
        // §7.3.8.2 infers an image's extent from its own dictionary, so a stream that decodes to
        // fewer bytes than the grid needs is a picture the file describes and does not carry.
        // The samples it does carry are drawn and the rest of the grid is left unpainted, which
        // is why this is a report beside the drawing rather than a refusal:
        // `image::short_of_its_grid` has the reading.
        if let Some(detail) = crate::image::short_of_its_grid(self.document, stream, resources) {
            self.note(Unsupported::Image {
                name: format!("{name}: {detail}"),
            });
        }
        // §7.4.6 Table 11 lets `/EndOfBlock false` bound a CCITT decode by `/Rows`, which may be
        // fewer scan lines than `/Height`. The lines the filter delivers are drawn and the rest
        // are blank — a choice, because the standard states nothing about them — so this is a
        // report beside the drawing: `image::ccitt_bound_below_its_height` has the reading.
        if let Some(detail) = crate::image::ccitt_bound_below_its_height(self.document, stream) {
            self.note(Unsupported::Image {
                name: format!("{name}: {detail}"),
            });
        }
        // §8.9.6.2 with §8.7.3.3: a stencil "does not specify colours; instead, it
        // designates places where the current colour is painted", and the current colour may
        // be a *pattern*, which is not a colour this or any other command can carry.
        if matches!(
            self.document.get_key(&stream.dict, "ImageMask"),
            Object::Boolean(true)
        ) && state.fill_pattern.is_some()
        {
            self.stencil_through_a_pattern(stream, name, resources, state);
            return;
        }

        // A PDF image occupies the unit square in user space, so the command's transform is
        // the current transform and nothing else.
        //
        // Through the cache rather than through `image::decode_parts` directly, because the
        // decode is per `Do` and the raster is not: `RasterCache` carries the measurement and
        // says what its key claims. Everything reported about this image was reported above,
        // out of the dictionary, so a raster answered from the cache says what a fresh decode
        // says — which is the property trap 5 is about, and `tests/image_reuse.rs` pins it.
        let conversion = self.image_conversion(&stream.dict, state);
        // §11.7.5.2's fourth condition is about the image's own dictionary rather than about the
        // graphics state — "[i]f the object is an image XObject and there is not an SMask entry
        // in its image dictionary" — so it is answered here, where the dictionary is.
        self.note_transfer(
            state,
            Painted::Image {
                soft_mask: !matches!(self.document.get_key(&stream.dict, "SMask"), Object::Null),
            },
        );
        match self.image_rasters.parts(
            self.document,
            image,
            resources,
            state.fill,
            &conversion,
            &mut self.image_masks,
        ) {
            Ok(decoded) => self.list.push(Command::Image {
                // §10.5 applies to "any object for which transfer functions are in effect", and an
                // image is one object however many samples it has: the clause's input is "the
                // value of a colour component in the device's native colour space", which by this
                // point every sample is. Done here rather than in `image::decode_parts` because a
                // transfer belongs to the *graphics state* the image is drawn under and not to the
                // image, and the same XObject drawn twice under two states is two pictures.
                image: decoded.source(|image| transferred_image(image, state.transfer.as_deref())),
                transform: state.transform,
                alpha: state.fill_alpha,
                clip: state.clip,
                // §11.6.4.3: an image's own `/SMask`, `/SMaskInData` or `/Mask` "shall
                // override, for this image object only, the current soft mask in the
                // graphics state" — so the two are never applied together, and the state's
                // mask survives for whatever is drawn next.
                mask: (!crate::image::overrides_graphics_state_mask(self.document, &stream.dict))
                    .then_some(state.soft_mask)
                    .flatten(),
                blend: state.blend,
            }),
            Err(error) => self.note(Unsupported::Image {
                name: format!("{name}: {error}"),
            }),
        }
    }

    /// Paints a stencil mask whose current colour is a pattern (§8.7.2 with §8.9.6.2).
    ///
    /// > Sample values in the image do not represent black and white pixels; rather, they
    /// > designate places on the page that should either be marked with the current colour or
    /// > masked out (not marked at all)
    ///
    /// A stencil is normally drawn as an image whose samples carry the fill colour, which is
    /// what [`crate::image::decode`]'s `fill` parameter is for. A **pattern** is not a colour
    /// an image sample can carry, and §8.7.2 makes one the current colour all the same:
    ///
    /// > All patterns shall be treated as colours; a Pattern colour space shall be
    /// > established with the CS or cs operator just like other colour spaces, and a
    /// > particular pattern shall be installed as the current colour with the SCN or scn
    /// > operator
    ///
    /// So the two halves are separated and recomposed out of what the
    /// display list already has: the stencil becomes a §11.5.2 *alpha* soft mask — its marked
    /// samples are opaque and the rest are not, which is exactly the areas the clause names —
    /// and the pattern paints the image's unit square through it.
    ///
    /// `issue13372.pdf` is the corpus witness, a CCITT stencil over an axial shading pattern,
    /// and this reader drew **nothing** for it and said nothing either: `image::decode` was
    /// handed `state.fill`, which a pattern leaves at its initial black with zero alpha.
    ///
    /// **A tiling pattern goes the same way since the two-hundred-and-eighteenth session**, and
    /// what makes that possible is that the mask is on the *state* rather than on a command:
    /// `Interpreter::tile` already ends by putting the state's soft mask on the group it builds
    /// out of the cells, because §11.6.7 asks for the cells to composite once. So the stencil is
    /// handed to it as that mask and the unit square is the path whose cells are drawn — the
    /// same two halves, recomposed at the only other place in this file that can hold them.
    ///
    /// One case is still refused by name rather than approximated: a stencil under a
    /// *graphics-state* soft mask would need two masks where a command carries one, which
    /// §11.6.5 makes a composition rather than a choice.
    fn stencil_through_a_pattern(
        &mut self,
        stream: &Arc<pdf_syntax::Stream>,
        name: &str,
        resources: &Dictionary,
        state: &GraphicsState,
    ) {
        if state.fill_pattern.is_none() {
            self.note(Unsupported::Image {
                name: format!("{name}: a stencil mask painted with no pattern (§8.9.6.2)"),
            });
            return;
        }
        if state.soft_mask.is_some() {
            self.note(Unsupported::Image {
                name: format!(
                    "{name}: a stencil mask painted with a pattern under a soft mask, \
                     which would be two masks on one command (§8.9.6.2, §11.6.5)"
                ),
            });
            return;
        }
        // The colour handed to the decode is irrelevant and must be opaque: §11.5.2 derives
        // the mask "from the alpha of the group", so only the samples' coverage is read.
        // The stencil carries no colour of its own — §11.5.2 derives the mask "from the
        // alpha of the group" — so what is composited into decides nothing here, and
        // `Conversion::device()` says that rather than borrowing an answer from the state.
        let image = match crate::image::decode(
            self.document,
            stream,
            resources,
            Color::BLACK,
            &Conversion::device(),
        ) {
            Ok(image) => image,
            Err(error) => {
                self.note(Unsupported::Image {
                    name: format!("{name}: {error}"),
                });
                return;
            }
        };
        let mask = pdf_render::SoftMask {
            commands: vec![Command::Image {
                image: image.into(),
                transform: state.transform,
                alpha: 1.0,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            }],
            kind: pdf_render::SoftMaskKind::Alpha,
            transfer: None,
        };
        let Ok(mask) = self.list.add_soft_mask(mask) else {
            self.note(Unsupported::LimitReached {
                limit: "max_soft_masks",
            });
            return;
        };

        // The image's own unit square, which is the region the stencil can mark.
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(1.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(1.0, 1.0)));
        path.push(PathCommand::LineTo(Point::new(0.0, 1.0)));
        path.push(PathCommand::Close);

        if let Some(PatternPaint::Tiling(tiling)) = state.fill_pattern.clone() {
            // The cells go through the mask the same way a `ca` or a blend mode does: on the
            // group `tile` builds when the state composites non-trivially. Everything else
            // about the state is the caller's, which is why this is a copy with one field
            // changed rather than a second construction.
            let mut masked = state.clone();
            masked.soft_mask = Some(mask);
            self.tile(
                &Arc::new(path),
                state.transform,
                FillRule::NonZero,
                &tiling,
                &masked,
            );
            return;
        }

        // The pattern's own `/BBox` and a type 1 shading's domain are composed here, as they
        // are for any other fill through a shading pattern.
        let clip = self.paint_clip(state, true);
        self.note_transfer(state, Painted::of(state, false));
        self.list.push(Command::Fill {
            path: Arc::new(path),
            transform: state.transform,
            fill_rule: FillRule::NonZero,
            paint: state.fill_paint(),
            clip,
            mask: Some(mask),
            blend: state.blend,
        });
    }
}
