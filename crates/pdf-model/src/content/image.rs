//! Image `XObject`s: §8.9's samples, stencils, masks and alternates on their way into a
//! [`Command::Image`].
//!
//! The decoding itself is `crate::image`'s; what is decided here is what the graphics
//! state does to a decoded image — §10.5's transfer, §11.6.4.3's mask override, and
//! §8.9.6.2's stencil whose current colour is a pattern.

use std::sync::Arc;

use pdf_render::{BlendMode, Command, FillRule, Path, PathCommand, Point, SoftMaskId, Transform};
use pdf_syntax::{Dictionary, Document, Object, Token};

use crate::colour::Conversion;

use super::colour::{BlackPoint, Intent};
use super::pattern::{PatternPaint, Tiled};
use super::reader::{ContentReader, NestedContent};
use super::report::Unsupported;
use super::transparency::Painted;
use super::{GraphicsState, Interpreter};

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
        let intent = image_intent(self.document, dict, state.intent);
        self.conversion_under_intent(intent, state)
    }

    /// [`Self::image_conversion`] once the intent is chosen: the one construction, so that a
    /// decode started ahead of its `Do` names the conversion the `Do` will name (ADR 1321).
    fn conversion_under_intent(&self, intent: Intent, state: &GraphicsState) -> Conversion {
        // And the page's §14.11.5 intent, for the same reason `Interpreter::conversion_under`
        // gives: Table 87's `/ColorSpace` is parsed in `crate::image`, after this point.
        Conversion::new(self.compositing.clone(), state.rendering_under(intent))
            .under_output_intent(self.output_intent.as_ref())
    }

    /// Starts decoding the page's images ahead of the `Do`s that draw them, where there is a
    /// pool to decode them on and a resource dictionary that could name one (ADR 1321).
    ///
    /// The answer is the plan [`walk_ahead`] reads; the table it fills is installed in the
    /// interpreter's [`crate::image::RasterCache`] here, so the `Do`s find it.
    pub(super) fn plan_decodes_ahead(
        &mut self,
        resources: &Dictionary,
        initial: &GraphicsState,
    ) -> Option<AheadPlan> {
        // A page that names no `XObject` and no pattern draws no image that could be decoded
        // ahead: an inline image is built at its `BI`. Asked of the dictionary rather than of
        // the content, so that such a page pays one lookup and starts nothing — and asked
        // *first*, because the pool's own size is not a free question: the first asking starts
        // rayon's threads, which a page of text must not pay for.
        if resources.get("XObject").is_none() && resources.get("Pattern").is_none() {
            return None;
        }
        // And a page whose dictionaries reach fewer than two images a decode ahead could take has
        // nothing to decode ahead, the first being the interpreter's (`WalkAhead::offer`). Asked
        // here, on the interpreter's thread and before the pool, for the pool's reason above.
        let mut asked = Vec::new();
        if reachable(self.document, resources, 0, &mut asked) < 2
            || rayon::current_num_threads() < 2
        {
            return None;
        }
        let ahead = Arc::new(crate::image::DecodesAhead::default());
        self.image_rasters.decode_ahead(Arc::clone(&ahead));
        // Every pair of §8.6.5.8's intent and §8.6.5.9's `/UseBlackPtComp` the walk can meet,
        // each converted through the one construction the `Do` uses, under a copy of the state
        // the page begins in with only that pair changed.
        let mut conversions = Vec::with_capacity(12);
        for intent in [
            Intent::Absolute,
            Intent::Relative,
            Intent::Saturation,
            Intent::Perceptual,
        ] {
            for black in [BlackPoint::On, BlackPoint::Off, BlackPoint::Default] {
                let mut state = initial.clone();
                state.use_black_pt_comp = black;
                let tone = Tone { intent, black };
                conversions.push((tone, self.conversion_under_intent(intent, &state)));
            }
        }
        Some(AheadPlan {
            ahead,
            conversions,
            initial: Tone {
                intent: initial.intent,
                black: initial.use_black_pt_comp,
            },
        })
    }

    /// Ends what [`Self::plan_decodes_ahead`] started: the `Do`s stop asking the table.
    pub(super) fn end_decodes_ahead(&mut self) {
        self.image_rasters.stop_ahead();
    }

    /// §8.9.5.4 steps c) and d): which of a base image's `/Alternates` is drawn in its place.
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
    /// **This function is c) and d).** a) and b) are `xobject.rs`'s, because they are about the
    /// base image and never reach an alternate; e) is the caller's fall-through when this
    /// answers `None`.
    ///
    /// # c) is a question about what the output is for, and a host answers it
    ///
    /// The amended c)'s "the PDF is being printed" is a fact about the operation under way and
    /// not about the file, so it arrives the way every such fact arrives here — as an input a host or an
    /// operation supplies, [`crate::optional_content::Purpose`], reached through the view state
    /// (ADR 1173). `Purpose::View` and `Purpose::Export` both fail c)'s condition and fall to
    /// d), which is the clause's own arrangement: c) opens "Otherwise if the PDF is being
    /// printed", and an export is not a printing.
    ///
    /// Two things c) does **not** say, both of which its retired predecessor did and this
    /// function therefore must not: it does not re-examine the selected alternate's own `/OC`
    /// (the erratum strikes that sentence), and it states no fallback of its own, because e)
    /// carries the fallback for c) and d) together. Table 89 is what makes "any of the
    /// Alternates entries" identify one entry — "[a]t most one alternate for a given base image
    /// shall be so designated" — so the first is the one, and a document that designates two has
    /// contradicted its own requirement and gets the first deterministically.
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
        if self.view.purpose() == crate::optional_content::Purpose::Print
            && let Some(printed) = self.default_for_printing(alternates, name)
        {
            return Some(printed);
        }
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

    /// §8.9.5.4 step c): the alternate Table 89 designates as the one to print.
    ///
    /// The step is Errata Collection 3's and `doc/md/` carries none of it, so its words are
    /// quoted in prose as the rest of this algorithm's are: "Otherwise if the PDF is being
    /// printed and any of the Alternates entries has `DefaultForPrinting` set to true, then that
    /// alternate image shall be printed."
    ///
    /// `None` where no entry designates itself, which is step d)'s turn; and `None` where the
    /// designated entry states no `/Image`, which Table 89 makes required — that entry
    /// identifies no alternate to be printed, so the document's defect is named and the rest of
    /// the algorithm runs.
    fn default_for_printing(
        &mut self,
        alternates: &[Object],
        name: &str,
    ) -> Option<Arc<pdf_syntax::Stream>> {
        for entry in alternates {
            let resolved = self.document.resolve(entry);
            let Some(alternate) = resolved.as_dict() else {
                continue;
            };
            // Table 89: "A flag indicating whether this alternate image is the default version
            // to be used for printing … Default value: false ."
            if self.document.get_key(alternate, "DefaultForPrinting") != Object::Boolean(true) {
                continue;
            }
            let image = self.document.get_key(alternate, "Image");
            let Some(image) = image.as_stream().cloned() else {
                self.note(Unsupported::Image {
                    name: format!(
                        "{name}: the /DefaultForPrinting alternate image dictionary states no \
                         /Image"
                    ),
                });
                return None;
            };
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
        if let Some(detail) = crate::image::unapplied_mask(self.document, stream, resources) {
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
        let soft_mask = !matches!(self.document.get_key(&stream.dict, "SMask"), Object::Null);
        let transfer = self.mark_transfer(state, Painted::Image { soft_mask });
        match self.image_rasters.parts(
            self.document,
            image,
            resources,
            state.fill,
            &conversion,
            &mut self.image_masks,
        ) {
            Ok(crate::image::Parts {
                picture,
                shortfall,
                contradiction,
            }) => {
                // §7.4.8 puts a JPEG's dimensions in the codestream and this tree draws them from
                // there, so an image whose dictionary says something else is *drawn* rather than
                // refused — and said out loud all the same, because the picture on the page is
                // then not the one the file described. The decode read the frame, so the sentence
                // travels with the raster: `image::Parts::contradiction` has the reading.
                if let Some(detail) = contradiction {
                    self.note(Unsupported::Image {
                        name: format!("{name}: {detail}"),
                    });
                }
                // A filter that stopped on damaged data delivered the rows before it, and the
                // report travels with the raster rather than being made here, so a second `Do`
                // answered from the cache says it too: `image::Parts::shortfall` has the reading.
                if let Some(detail) = shortfall {
                    self.note(Unsupported::Image {
                        name: format!("{name}: {detail}"),
                    });
                }
                // §10.5 applies to "any object for which transfer functions are in effect", and
                // an image is one object however many samples it has — but §11.7.5.2 chooses the
                // function by the topmost object covering a *point* and §11.7.5.3's NOTE applies
                // it "only when all colour compositing has been completed", so it no longer goes
                // into the samples here. It rides on the mark instead and a backend applies it
                // once over the finished raster (`Interpreter::draw_mark`, ADR 1125), which is
                // what makes the image's own antialiased edge take the clause's value rather than
                // the composite of an already-transferred colour.
                let image = picture.source(|image| image);
                self.draw_mark(
                    Command::Image {
                        image,
                        transform: state.transform,
                        alpha: state.fill_alpha,
                        clip: state.clip,
                        // §11.6.4.3: an image's own `/SMask`, `/SMaskInData` or `/Mask` "shall
                        // override, for this image object only, the current soft mask in the
                        // graphics state" — so the two are never applied together, and the state's
                        // mask survives for whatever is drawn next.
                        mask: (!crate::image::overrides_graphics_state_mask(
                            self.document,
                            &stream.dict,
                        ))
                        .then_some(state.soft_mask)
                        .flatten(),
                        blend: state.blend,
                    },
                    transfer,
                );
            }
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
        let (image, shape) = match crate::image::decode_stencil_reporting_frame(
            self.document,
            stream,
            resources,
            &Conversion::device(),
        ) {
            Ok((crate::image::Flattened { image, shortfall }, shape, contradiction)) => {
                for detail in contradiction.into_iter().chain(shortfall) {
                    self.note(Unsupported::Image {
                        name: format!("{name}: {detail}"),
                    });
                }
                (image, shape)
            }
            Err(error) => {
                self.note(Unsupported::Image {
                    name: format!("{name}: {error}"),
                });
                return;
            }
        };
        let Some(mask) = self.stencil_mask(image, state.transform) else {
            return;
        };
        // This mask is §8.9.6.2's stencil wearing §11.5.2's vocabulary, and a stencil is
        // *shape* — "the shape shall be 1.0 for painted areas and 0.0 for masked areas"
        // (§11.6.4.2) — where every other mask a command carries is §11.6.4.3's opacity. A
        // knockout group states an element's shape by removing the opacity from it, and this
        // is the one mask it must keep; the record is what tells it so (`image::ShapeMasks`).
        // A stencil under an `/SMask` of its own is drawn through the product, which is what
        // the page is owed, and its shape is the stencil alone, which is a second mask
        // (ADR 1301).
        match shape {
            None => self.image_masks.shape_masks_mut().record(mask),
            Some(shape) => {
                let Some(shape) = self.stencil_mask(shape, state.transform) else {
                    return;
                };
                self.image_masks.shape_masks_mut().record_apart(mask, shape);
            }
        }

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
                Tiled::Fill(FillRule::NonZero),
                &tiling,
                &masked,
            );
            return;
        }

        // The pattern's own `/BBox` and a type 1 shading's domain are composed here, as they
        // are for any other fill through a shading pattern.
        let clip = self.paint_clip(state, true);
        self.note_colourants_without_a_plane(&state.fill_space);
        let transfer = self.mark_transfer(state, Painted::of(state, false));
        let paint = self.fill_paint(state);
        self.draw_mark(
            Command::Fill {
                path: Arc::new(path),
                transform: state.transform,
                fill_rule: FillRule::NonZero,
                paint,
                clip,
                mask: Some(mask),
                blend: state.blend,
            },
            transfer,
        );
    }

    /// §11.5.2's alpha mask made of a stencil's raster placed under `transform`, or `None`
    /// once the list's bound on soft masks is reached, which is noted.
    fn stencil_mask(
        &mut self,
        image: pdf_render::Image,
        transform: Transform,
    ) -> Option<SoftMaskId> {
        let mask = pdf_render::SoftMask {
            commands: vec![Command::Image {
                image: image.into(),
                transform,
                alpha: 1.0,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            }],
            kind: pdf_render::SoftMaskKind::Alpha,
            transfer: None,
            luminance: None,
            black: None,
        };
        let Ok(mask) = self.list.add_soft_mask(mask) else {
            self.note(Unsupported::LimitReached {
                limit: "max_soft_masks",
            });
            return None;
        };
        Some(mask)
    }
}

/// The intent an image is converted under: Table 87's `/Intent` where the dictionary states one,
/// and the graphics state's otherwise.
fn image_intent(document: &Document, dict: &Dictionary, current: Intent) -> Intent {
    match document.get_key(dict, "Intent") {
        Object::Name(name) => Intent::read(name.as_bytes()),
        _ => current,
    }
}

/// What [`walk_ahead`] needs to offer a decode under the conversion its `Do` will name.
///
/// The conversion is the one input of [`crate::image::RasterCache`]'s key a walk cannot read off
/// the file, because it is the graphics state's: §8.6.5.8's intent, §8.6.5.9's black point
/// compensation and §11.4's target. The walk follows the first two as the interpreter does — `ri`,
/// an `/ExtGState`'s `/RI` and `/UseBlackPtComp`, saved and restored by `q` and `Q` — and reads
/// Table 87's `/Intent` from the image as the `Do` will. The third it takes as the page begins,
/// so a transparency group with a blending space of its own changes the conversion by the time
/// the `Do` arrives; the slot's inputs then disagree with the `Do`'s and the interpreter decodes
/// for itself — wasted work, never a different picture.
#[derive(Debug)]
pub(super) struct AheadPlan {
    /// The table the decodes report into.
    ahead: Arc<crate::image::DecodesAhead>,
    /// The conversion under each pair of intent and black point, in the page's initial state.
    conversions: Vec<(Tone, Conversion)>,
    /// The pair the page's content stream begins with.
    initial: Tone,
}

/// The two parameters of the graphics state a walk ahead follows, because they choose the
/// conversion: §8.6.5.8's rendering intent and §8.6.5.9's `/UseBlackPtComp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tone {
    /// The rendering intent in force.
    intent: Intent,
    /// `/UseBlackPtComp` in force.
    black: BlackPoint,
}

impl AheadPlan {
    /// The table, for the interpreter to close once the content stream has been read.
    pub(super) fn ahead(&self) -> &crate::image::DecodesAhead {
        &self.ahead
    }

    /// The conversion an image with this dictionary is offered under, where `tone` is in force.
    fn conversion(
        &self,
        document: &Document,
        dict: &Dictionary,
        tone: Tone,
    ) -> Option<&Conversion> {
        let tone = Tone {
            intent: image_intent(document, dict, tone.intent),
            ..tone
        };
        self.conversions
            .iter()
            .find_map(|(each, conversion)| (*each == tone).then_some(conversion))
    }
}

/// How deep a walk ahead follows forms and pattern cells, which is the interpreter's own bound.
const AHEAD_DEPTH: usize = super::MAX_FORM_DEPTH;

/// How many resource entries [`reachable`] asks before it answers with what it has found.
///
/// The question costs a lookup and an object load per entry and is asked on the interpreter's
/// thread, so a dictionary shared by every page of a long document — one names 159 images — is
/// not read whole to answer it. A bound rather than a measurement, and wrong only in the direction
/// that costs nothing: a page whose first sixty-four entries hold fewer than two such images is
/// interpreted as it was before ADR 1321.
const REACHABLE_ENTRIES: usize = 64;

/// How many images a decode ahead could take the resource dictionaries reach, counted to two.
///
/// Dictionaries only — an `/XObject` or `/Pattern` entry, and a form's or a cell's own
/// `/Resources` — so it costs lookups rather than a read of any content. `asked` holds the
/// streams already asked, so a dictionary shared by a thousand forms is asked once, and bounds
/// the question at [`REACHABLE_ENTRIES`].
fn reachable(
    document: &Document,
    resources: &Dictionary,
    depth: usize,
    asked: &mut Vec<Arc<pdf_syntax::Stream>>,
) -> usize {
    let mut found = 0usize;
    for category in ["XObject", "Pattern"] {
        let table = document.get_key(resources, category);
        let Some(table) = table.as_dict() else {
            continue;
        };
        for (_, entry) in table.iter() {
            if found >= 2 || depth >= AHEAD_DEPTH || asked.len() >= REACHABLE_ENTRIES {
                return found;
            }
            let object = document.resolve(entry);
            let Some(stream) = object.as_stream() else {
                continue;
            };
            if asked.iter().any(|seen| Arc::ptr_eq(seen, stream)) {
                continue;
            }
            asked.push(Arc::clone(stream));
            let dict = &stream.dict;
            let subtype = document.get_key(dict, "Subtype");
            if subtype
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Image")
            {
                if offerable(document, stream).is_some() {
                    found = found.saturating_add(1);
                }
            } else if dict.get("OC").is_none()
                && let Some(inner) = document.get_key(dict, "Resources").as_dict()
            {
                found = found.saturating_add(reachable(
                    document,
                    inner,
                    depth.saturating_add(1),
                    asked,
                ));
            }
        }
    }
    found
}

/// Reads a page's content stream ahead of the interpreter, on a pool thread, and offers a decode
/// of each image it finds a `Do` for (ADR 1321).
///
/// **The walk reads; it never interprets.** It follows the two operators that name an image —
/// `Do`, and `scn`/`SCN` through §8.7.3's tiling pattern, whose cell may draw one — into forms
/// and cells, and nothing it finds reaches the display list. What it may get wrong is only
/// *which* decodes are worth starting, and a decode started for an image the page does not draw
/// is paid for twice: on another core, and at the end of the run, which waits for every decode
/// it started. So the walk declines what §8.11 may hide — an image or a form stating `/OC`, and
/// anything inside a `BDC` tagged `/OC` — and it is started only where the resource dictionaries
/// reach two images it could offer ([`reachable`]), since the first is the interpreter's (see
/// [`WalkAhead::offer`]).
pub(super) fn walk_ahead<'s>(
    scope: &rayon::Scope<'s>,
    document: &'s Document,
    page: &'s crate::page::Page,
    plan: &'s AheadPlan,
) {
    let mut walk = WalkAhead {
        scope,
        document,
        page_resources: &page.resources,
        plan,
        walked: Vec::new(),
        first: true,
    };
    let mut reader = ContentReader::for_page(document, page);
    walk.stream(&mut reader, &page.resources, 0, plan.initial);
}

/// One walk ahead: what it offers into, and which nested streams it has already read.
struct WalkAhead<'s, 'w> {
    /// The scope the decodes are started in, which the interpretation waits on.
    scope: &'w rayon::Scope<'s>,
    /// The document.
    document: &'s Document,
    /// The page's resources, which a form stating none inherits (`Interpreter::draw_xobject`).
    page_resources: &'s Dictionary,
    /// The plan.
    plan: &'s AheadPlan,
    /// Forms and pattern cells already read, by address: a pattern painted a thousand times is
    /// walked once.
    walked: Vec<Arc<pdf_syntax::Stream>>,
    /// Whether the next image this walk could offer is the first, which it leaves to the
    /// interpreter — see [`WalkAhead::offer`].
    first: bool,
}

impl WalkAhead<'_, '_> {
    /// Reads one content stream, following what it names.
    ///
    /// An operator's operands are the objects immediately before it (§7.8.2), and the two this
    /// reads are names: the last one for `Do` and `scn`, the first one — `BDC`'s tag — for
    /// §14.6's marked content. `hidden` counts the open sections tagged `/OC`, inside which
    /// nothing is offered; `sections` says which of the open ones those are, so an `EMC` closes
    /// the section it belongs to. `tone` is the [`AheadPlan`]'s pair of state parameters, with
    /// §8.4.2's stack of its own.
    fn stream(
        &mut self,
        reader: &mut ContentReader<'_>,
        resources: &Dictionary,
        depth: usize,
        initial: Tone,
    ) {
        let xobjects = self.document.get_key(resources, "XObject");
        let patterns = self.document.get_key(resources, "Pattern");
        let states = self.document.get_key(resources, "ExtGState");
        let mut tone = initial;
        let mut saved: Vec<Tone> = Vec::new();
        let mut first: Option<Vec<u8>> = None;
        let mut last: Option<Vec<u8>> = None;
        let mut sections: Vec<bool> = Vec::new();
        let mut hidden = 0usize;
        loop {
            if self.plan.ahead.closed() {
                return;
            }
            let step = reader.with_token(|token| match token {
                None => Step::End,
                Some(Token::Name(name)) => Step::Name(name),
                Some(Token::Keyword(b"Do")) => Step::Do,
                Some(Token::Keyword(b"scn" | b"SCN")) => Step::Colour,
                Some(Token::Keyword(b"BDC" | b"BMC")) => Step::Open,
                Some(Token::Keyword(b"EMC")) => Step::Close,
                Some(Token::Keyword(b"q")) => Step::Save,
                Some(Token::Keyword(b"Q")) => Step::Restore,
                Some(Token::Keyword(b"ri")) => Step::Intent,
                Some(Token::Keyword(b"gs")) => Step::State,
                Some(_) => Step::Other,
            });
            match step {
                Step::End => return,
                Step::Name(name) => {
                    if first.is_none() {
                        first = Some(name.clone());
                    }
                    last = Some(name);
                    continue;
                }
                Step::Do if hidden == 0 => {
                    if let (Some(name), Some(table)) = (last.take(), xobjects.as_dict()) {
                        self.xobject(table, &name, resources, depth, tone);
                    }
                }
                Step::Colour if hidden == 0 => {
                    if let (Some(name), Some(table)) = (last.take(), patterns.as_dict()) {
                        self.pattern(table, &name, depth, tone);
                    }
                }
                Step::Open => {
                    let optional = first.as_deref() == Some(b"OC".as_slice());
                    hidden = hidden.saturating_add(usize::from(optional));
                    sections.push(optional);
                }
                Step::Close => {
                    if sections.pop() == Some(true) {
                        hidden = hidden.saturating_sub(1);
                    }
                }
                Step::Save => saved.push(tone),
                Step::Restore => tone = saved.pop().unwrap_or(tone),
                Step::Intent => {
                    if let Some(name) = &last {
                        tone.intent = Intent::read(name);
                    }
                }
                // Read as `Interpreter::apply_ext_gstate` reads the same two entries.
                Step::State => {
                    let entry = last
                        .as_ref()
                        .zip(states.as_dict())
                        .and_then(|(name, table)| {
                            table.get_by_name(&pdf_syntax::Name::new(name.as_slice()))
                        })
                        .map(|entry| self.document.resolve(entry));
                    if let Some(dict) = entry.as_ref().and_then(Object::as_dict) {
                        if let Object::Name(value) = self.document.get_key(dict, "UseBlackPtComp") {
                            tone.black = match value.as_bytes() {
                                b"ON" => BlackPoint::On,
                                b"OFF" => BlackPoint::Off,
                                _ => BlackPoint::Default,
                            };
                        }
                        if let Object::Name(intent) = self.document.get_key(dict, "RI") {
                            tone.intent = Intent::read(intent.as_bytes());
                        }
                    }
                }
                Step::Do | Step::Colour | Step::Other => {}
            }
            first = None;
            last = None;
        }
    }

    /// What `/name Do` finds: an image to offer, or a form to read.
    fn xobject(
        &mut self,
        table: &Dictionary,
        name: &[u8],
        resources: &Dictionary,
        depth: usize,
        tone: Tone,
    ) {
        let Some(entry) = table.get_by_name(&pdf_syntax::Name::new(name)) else {
            return;
        };
        let object = self.document.resolve(entry);
        let Some(stream) = object.as_stream() else {
            return;
        };
        match self.document.get_key(&stream.dict, "Subtype") {
            Object::Name(subtype) if subtype.as_bytes() == b"Image" => {
                self.offer(stream, resources, tone);
            }
            // §8.11.3.3's `/OC` on a form is the interpreter's to decide at the `Do`.
            Object::Name(subtype)
                if subtype.as_bytes() == b"Form" && stream.dict.get("OC").is_none() =>
            {
                let inner = self
                    .document
                    .get_key(&stream.dict, "Resources")
                    .as_dict()
                    .cloned()
                    .unwrap_or_else(|| self.page_resources.clone());
                self.nested(stream, &inner, depth, tone);
            }
            _ => {}
        }
    }

    /// What `/name scn` finds: a coloured tiling pattern's cell to read. A shading pattern draws
    /// no image, and §8.6.8 has an uncoloured cell ignore every image that is not a stencil.
    fn pattern(&mut self, table: &Dictionary, name: &[u8], depth: usize, tone: Tone) {
        let Some(entry) = table.get_by_name(&pdf_syntax::Name::new(name)) else {
            return;
        };
        let object = self.document.resolve(entry);
        let Some(stream) = object.as_stream() else {
            return;
        };
        if self
            .document
            .get_key(&stream.dict, "PaintType")
            .as_integer()
            == Some(2)
        {
            return;
        }
        // §8.7.3.3 gives a tiling pattern no fallback resource dictionary, and
        // `Interpreter::tile` reads it the same way.
        let inner = self
            .document
            .get_key(&stream.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_default();
        self.nested(stream, &inner, depth, tone);
    }

    /// Reads a form's or a cell's content stream, once per walk.
    fn nested(
        &mut self,
        stream: &Arc<pdf_syntax::Stream>,
        resources: &Dictionary,
        depth: usize,
        tone: Tone,
    ) {
        if depth >= AHEAD_DEPTH || self.walked.iter().any(|seen| Arc::ptr_eq(seen, stream)) {
            return;
        }
        self.walked.push(Arc::clone(stream));
        let Ok(content) = NestedContent::of(self.document, stream, String::new()) else {
            return;
        };
        let mut reader = content.reader();
        self.stream(&mut reader, resources, depth.saturating_add(1), tone);
    }

    /// Offers a decode of one image, where it is one a decode ahead can answer ([`offerable`]).
    ///
    /// **The first such image the walk finds is the interpreter's**, which is a measurement of
    /// this machine rather than a rule about images (ADR 1321). Its cores are of two classes, and
    /// a pool thread on the slower one decodes a photograph in about half again the time the
    /// interpreter's own thread takes on the faster; the first image is the one the interpreter
    /// reaches soonest, so a decode ahead of it can at best be a little early and at worst run
    /// on the slower core while the interpreter waits. A page of one photograph is then exactly
    /// the page it was, and on a page of several the others are decoded beside the first.
    fn offer(&mut self, stream: &Arc<pdf_syntax::Stream>, resources: &Dictionary, tone: Tone) {
        let document = self.document;
        let Some(bytes) = offerable(document, stream) else {
            return;
        };
        if std::mem::replace(&mut self.first, false) {
            return;
        }
        let Some(into) = self.plan.conversion(document, &stream.dict, tone) else {
            return;
        };
        let colour_spaces = resources.get("ColorSpace").cloned().unwrap_or(Object::Null);
        if !self
            .plan
            .ahead
            .offer(stream, colour_spaces, into.clone(), bytes)
        {
            return;
        }
        let ahead = Arc::clone(&self.plan.ahead);
        let stream = Arc::clone(stream);
        self.scope.spawn(move |_| ahead.decode(document, &stream));
    }
}

/// What decoding this image ahead would hold until its `Do`, or `None` where it is left to the
/// `Do`.
///
/// Four kinds are left to the `Do`, each for a reason of its own. §8.9.6.2's stencil, whose raster
/// is the fill colour's and the fill colour is the state's. An image stating `/OC` or
/// `/Alternates`, because §8.9.5.4 decides at the `Do` whether it or another stream is drawn. One
/// whose codec is §7.4.6's, §7.4.7's or §7.4.9's, which `pdf_sandbox`'s worker decodes one request
/// at a time behind one connection, so a decode started here would queue in front of the
/// interpreter's own rather than beside it. And one below [`crate::image::AHEAD_FLOOR`] samples,
/// which is cheaper to decode than to hand over.
fn offerable(document: &Document, stream: &pdf_syntax::Stream) -> Option<usize> {
    let dict = &stream.dict;
    if matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true))
        || dict.get("OC").is_some()
        || dict.get("Alternates").is_some()
    {
        return None;
    }
    if let Some(codec) = document.image_codec(stream)
        && !matches!(codec.as_slice(), b"DCTDecode" | b"DCT")
    {
        return None;
    }
    let dimension = |key| {
        document
            .get_key(dict, key)
            .as_integer()
            .and_then(|value| u64::try_from(value).ok())
            .unwrap_or(0)
    };
    let samples = dimension("Width").saturating_mul(dimension("Height"));
    if samples < crate::image::AHEAD_FLOOR {
        return None;
    }
    Some(usize::try_from(samples.saturating_mul(4)).unwrap_or(usize::MAX))
}

/// What one token of a walk ahead is to it.
enum Step {
    /// The end of the stream.
    End,
    /// A name, which the next operator may take as its operand.
    Name(Vec<u8>),
    /// `Do`.
    Do,
    /// `scn` or `SCN`, whose last operand may name a pattern.
    Colour,
    /// `BDC` or `BMC`, opening §14.6's marked content.
    Open,
    /// `EMC`, closing it.
    Close,
    /// `q`.
    Save,
    /// `Q`.
    Restore,
    /// `ri`, whose operand is a rendering intent.
    Intent,
    /// `gs`, whose operand names a graphics state parameter dictionary.
    State,
    /// Anything else.
    Other,
}
