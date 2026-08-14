//! Image `XObject`s: §8.9's samples, stencils, masks and alternates on their way into a
//! [`Command::Image`].
//!
//! The decoding itself is `crate::image`'s; what is decided here is what the graphics
//! state does to a decoded image — §10.5's transfer, §11.6.4.3's mask override, and
//! §8.9.6.2's stencil whose current colour is a pattern.

use std::sync::Arc;

use pdf_render::{BlendMode, Color, Command, FillRule, Path, PathCommand, Point};
use pdf_syntax::{Dictionary, Object};

use crate::colour::Compositing;

use super::ext_gstate::Transfer;
use super::pattern::PatternPaint;
use super::report::Unsupported;
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
    // photograph repeats its colours: the same argument `image::Conversion` records for
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
    /// Draws one image `XObject`.
    /// §8.9.5.4 step c): which of a hidden base image's `/Alternates` is drawn in its place.
    ///
    /// The clause states an algorithm and **contradicts itself inside step c)**, which is why
    /// this function has a comment as long as it is. The selection sentence:
    ///
    /// > the list of alternate image dictionaries specified by the base image Alternates entry
    /// > shall be examined in order, and the first entry not containing an OC key, or containing
    /// > an OC entry specifying that the alternate image should be visible, shall be selected
    ///
    /// and then, four sentences later:
    ///
    /// > If none of the alternate image dictionaries have an OC key, or none of the alternate
    /// > image dictionaries with an OC entry specify that that alternate image is visible, then
    /// > nothing shall be shown.
    ///
    /// An alternate with **no** `/OC` is selectable by the first and unshowable by the second.
    /// The two cannot both be obeyed, and this project's own habit for that is to ask which
    /// reading makes a file's own words mean nothing: under the second, the phrase "the first
    /// entry not containing an OC key" can never lead to a mark, so the clause would be naming a
    /// case it has already excluded. So **the selection sentence is taken**, and the one place
    /// the two readings differ — a selected alternate with no `/OC` of its own — is reported, so
    /// that a document relying on the other reading is named rather than silently drawn.
    ///
    /// The "Further" sentence is read as being about the *image*, which is what makes it mean
    /// something: Table 89's `/OC` is on the alternate image **dictionary**, and Table 87's is on
    /// the image `XObject` the `/Image` entry names. So a dictionary may be selected and its image
    /// still hidden.
    ///
    /// > Further, if this selected alternate image has an OC entry, then that OC entry shall also
    /// > be processed to determine if the alternate image shall be rendered or not.
    ///
    /// `None` is the clause's "nothing shall be shown", which is a *decision* rather than a gap
    /// and so is not reported. No corpus document carries an `/Alternates` entry at all —
    /// measured over all 964 openable ones — so every rule here rests on the clause and on the
    /// tests beside it.
    ///
    /// # Errata Collection 3 rewrites this algorithm, and settles the contradiction the other way
    ///
    /// Issue #79, `/State` `Review` `Completed`. Every blockquote above is struck out, so the
    /// argument they support is an argument about text the standard no longer has — and the
    /// amended steps disagree with this function in three places. Their words, from the carets
    /// (ADR 0253; `doc/md/` shows none of it):
    ///
    /// - "Alternates that have no OC entry shall not be shown." **This function selects exactly
    ///   those**, on the reading that the retired selection sentence would otherwise name a case
    ///   it had excluded. The erratum deletes the selection sentence instead, which is the same
    ///   repair made from the other end.
    /// - "Furthermore if the image dictionary that forms the value of the Image key of the
    ///   selected alternate contains an OC entry, then that OC in the image dictionary shall not
    ///   be examined." The "Further" sentence read above is **inverted**: Table 87's `/OC` on the
    ///   alternate's own image is now to be ignored rather than processed.
    /// - A new step: "If steps c and d above do not identify an alternate to be rendered then the
    ///   base image shall be rendered." So the fall-through is the **base image**, not nothing.
    ///
    /// This is left as it stands rather than half-corrected, because the amended clause's step
    /// ordering has its own question — the amended a) ends "then nothing shall be shown", which
    /// reads as terminal and would leave d)'s alternate selection unreachable for a hidden base —
    /// and a rewrite that guessed at it would replace one contradiction with another. `doc/todo/48`
    /// carries it with the erratum's own text. Nothing on any corpus page moves either way.
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
            let group = alternate.get("OC").cloned();
            if let Some(group) = &group
                && !self.shows_optional_content(group)
            {
                continue;
            }
            if group.is_none() {
                self.note(Unsupported::Image {
                    name: format!(
                        "{name}: §8.9.5.4 step c) selects an alternate with no /OC and its own                          closing sentence says nothing shall be shown; the selection is taken"
                    ),
                });
            }

            // Table 89 makes `/Image` required, so an entry without one has selected nothing.
            let image = self.document.get_key(alternate, "Image");
            let Some(image) = image.as_stream().cloned() else {
                self.note(Unsupported::Image {
                    name: format!("{name}: an alternate image dictionary states no /Image"),
                });
                return None;
            };
            // The "Further" sentence: the *image*'s own `/OC` (Table 87) decides whether the
            // alternate this dictionary selected is rendered at all.
            if let Some(own) = image.dict.get("OC").cloned()
                && !self.shows_optional_content(&own)
            {
                return None;
            }
            return Some(image);
        }
        None
    }

    pub(super) fn draw_image(
        &mut self,
        stream: &Arc<pdf_syntax::Stream>,
        name: &str,
        resources: &Dictionary,
        state: &GraphicsState,
    ) {
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
        match crate::image::decode_parts(
            self.document,
            stream,
            resources,
            state.fill,
            self.compositing,
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
        // `Compositing::Device` says that rather than borrowing an answer from the state.
        let image = match crate::image::decode(
            self.document,
            stream,
            resources,
            Color::BLACK,
            Compositing::Device,
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
