//! §11.5 and §11.6.5: a soft mask's group, run and registered, with the compositing a mask group
//! puts in force for its content and the restoring of what it replaced (ADR 2080).

use std::sync::Arc;

use pdf_render::{
    BlackHalf, BlendMode, ClipId, Color, Command, FillRule, Paint, Path, PathCommand, Point,
    SoftMaskId, SoftMaskKind, Transform,
};
use pdf_syntax::Dictionary;

use crate::colour::{ColourSpace, Compositing, InkScale};

use crate::content::reader::NestedContent;
use crate::content::report::Unsupported;
use crate::content::{GraphicsState, Interpreter, MAX_SOFT_MASK_DEPTH};

use super::GroupBody;
use super::commands::{any_command, command_blends};
use super::departures::GroupDrawn;
use super::knockout::paired;
use super::press::stated_cmyk_group;
use super::readings::unseal;

/// What [`Interpreter::enter_mask_compositing`] replaces for a soft mask's group and
/// [`Interpreter::leave_mask_compositing`] puts back: the compositing, the spot colourants beside
/// it, and §11.7.2's redefinition of `DeviceCMYK`.
type MaskScope = (Compositing, crate::colour::DeviceSpots, Option<ColourSpace>);

impl Interpreter<'_> {
    /// Reports a blend mode inside a mask group whose channel is more than one component.
    ///
    /// §11.3.5.2 applies a separable blend function "separately to each set of corresponding
    /// components", and it says which components:
    ///
    /// > where the lowercase variables 𝑐 𝑟 , 𝑐 𝑏 , and 𝑐 𝑠 denote corresponding components of
    /// > the colours 𝐶𝑟 , 𝐶𝑏 , and 𝐶𝑠 , expressed in additive form.
    ///
    /// A subtractive group's components in additive form are the complements of its ink, and
    /// what this tree paints such a group in is one *weighted average* of those complements:
    /// `1 − ink ÷ scale` is `(0.3(1 − c) + 0.59(1 − m) + 0.11(1 − y) + (1 − k)) ÷ 2` for
    /// `DeviceCMYK`, whose weights sum to 1. Source-over is affine and passes through an
    /// average unchanged, which is what makes the whole construction exact; no other blend
    /// function does, because `B` of an average is not the average of `B`.
    ///
    /// So the condition is a scale of more than one component, which is
    /// [`crate::colour::InkScale::Double`] and nothing else: a `DeviceGray` group's channel
    /// *is* its one component in additive form, so every blend mode is exact there.
    ///
    /// ADR 0217's report of a mask group composited in device RGB fires only for `Lab`; this
    /// one says the part of it that is still true of a `DeviceCMYK` mask group.
    fn note_blended_luminosity(&mut self, compositing: &Compositing, commands: &[Command]) {
        if *compositing != Compositing::Luminosity(InkScale::Double)
            || !any_command(commands, &|command| command_blends(command))
        {
            return;
        }
        self.note(Unsupported::TransparencyGroup {
            detail: "a soft mask's group blends in a space of four components, which this \
                     composites on one weighted average of them (§11.3.5.2, §11.5.3)"
                .to_owned(),
        });
    }

    /// Runs a mask group's content and collects its commands — twice where the group's
    /// blending colour space has four components (ISO 32000-2 §11.5.3, §11.3.4, §11.4.7).
    ///
    /// The first run is already in `self.list` above `mark` when this is called, which is
    /// what lets the second one be taken against it without a third traversal. §11.3.4
    /// composites per component and a rasteriser has three channels, so four components are
    /// two interpretations of one content stream: the first resolves colours into the press's
    /// chromatic half and the second into its black half, and `pdf_render::SoftMask::black`
    /// carries the second beside the first.
    ///
    /// **Two conditions give the pair up**, each with its own report, and in both the group is
    /// re-run on the device's three components so that what the mask carries and what it says
    /// about itself agree:
    ///
    /// - **A group inside introduced a blending space of its own with something compositing in
    ///   it** (`nested_space_departed`), which owes a conversion between two spaces per pixel
    ///   that no list here carries — `group_commands` gives its own pair up on the same
    ///   condition.
    /// - **§11.7.5.3's black generation was stated**, which the conversion into the space does
    ///   not read.
    ///
    /// A third gives it up after the fact: two interpretations that drew different structures
    /// are not a pair, because the halves are resolved together per pixel. A valid content
    /// stream can do that where a construction rests on a colour comparison each run makes on
    /// its own plane, which is [`super::implicit_knockout::blend_at_the_do`]'s one solid colour; where the runs parted
    /// after that admission (`admitted` is the count before the first run), both are run again
    /// with it refused, as [`Interpreter::black_half`] does for a group's pair (ADR 1657).
    fn mask_halves(
        &mut self,
        request: &crate::soft_mask::SoftMaskRequest,
        content: &NestedContent,
        resources: &Dictionary,
        inner: &GraphicsState,
        mark: usize,
        admitted: usize,
    ) -> (Vec<Command>, Option<BlackHalf>) {
        let commands = self.list.split_off_commands(mark);
        let (Some(backdrop), Compositing::Subtractive(_, press, _)) =
            (request.black_backdrop, &request.compositing)
        else {
            return (commands, None);
        };
        let press = Arc::clone(press);
        let on_device = |interpreter: &mut Self, detail: String| {
            interpreter.note(Unsupported::TransparencyGroup { detail });
            let saved = std::mem::replace(&mut interpreter.compositing, Compositing::Device);
            let redrawn =
                interpreter.rerun_inheriting(&GroupBody::Stream(content), resources, inner, mark);
            interpreter.compositing = saved;
            (redrawn, None)
        };
        if self.nested_space_departed {
            return on_device(
                self,
                "a soft mask's group states a four-component ICCBased /CS, and a group inside \
                 it names a blending space of its own — so it is composited on device RGB \
                 rather than in its own four components and its luminosity is taken there"
                    .to_owned(),
            );
        }
        // A mask group carries no spot plane: §11.7.3's "spot colours shall not be available in a
        // transparency group XObject that is used to define a soft mask".
        let black_plane = Compositing::Subtractive(
            crate::colour::Plane::Black,
            press,
            crate::colour::DeviceSpots::default(),
        );
        let rewind = self.readback_mark();
        let black = self.mask_run_in(&black_plane, content, resources, inner, mark);
        self.rewind_readback(rewind);
        let halves = |commands, black| {
            (
                commands,
                Some(BlackHalf {
                    commands: black,
                    backdrop,
                }),
            )
        };
        if paired(&commands, &black) {
            return halves(commands, black);
        }
        if !self.one_colour.refused && self.one_colour.admitted != admitted {
            self.one_colour.refused = true;
            let rewind = self.readback_mark();
            let chromatic = self.compositing.clone();
            let again = self.mask_run_in(&chromatic, content, resources, inner, mark);
            let black = self.mask_run_in(&black_plane, content, resources, inner, mark);
            self.rewind_readback(rewind);
            self.one_colour.refused = false;
            if paired(&again, &black) {
                return halves(again, black);
            }
        }
        on_device(
            self,
            "a soft mask's group states a four-component ICCBased /CS and its two \
             interpretations drew different structures, so the luminosity of the four \
             composited components cannot be read off the pair (§11.3.4, §11.4.7)"
                .to_owned(),
        )
    }

    /// One run of a soft mask's group compositing in `compositing`, its elements split off the
    /// list at `mark`, with the reading scope restarted for it and `self.compositing` put back.
    fn mask_run_in(
        &mut self,
        compositing: &Compositing,
        content: &NestedContent,
        resources: &Dictionary,
        inner: &GraphicsState,
        mark: usize,
    ) -> Vec<Command> {
        let saved = std::mem::replace(&mut self.compositing, compositing.clone());
        self.restart_reading_scope(inner.alpha_is_shape, mark);
        self.run(content, resources, inner);
        self.compositing = saved;
        self.list.split_off_commands(mark)
    }

    /// Puts a soft mask's group's compositing in force, with no spot colourant beside it —
    /// §11.7.3: "spot colours shall not be available in a transparency group XObject that is used
    /// to define a soft mask; the alternate colour space shall always be substituted in that
    /// case" — and hands back what was in force, for [`Interpreter::leave_mask_compositing`].
    ///
    /// And §11.7.2's redefinition of `DeviceCMYK`, which the same `/CS` decides and only it: the
    /// group is one "that has no parent group or page from which to inherit" (Table 145), so
    /// nothing outside it redefines anything inside ([`stated_cmyk_group`], ADR 2012).
    /// `resources` is the dictionary the group's content is read against.
    #[expect(
        clippy::doc_markdown,
        reason = "the paragraph quotes §11.7.3 verbatim, and a quotation may not gain backticks"
    )]
    fn enter_mask_compositing(
        &mut self,
        request: &crate::soft_mask::SoftMaskRequest,
        resources: &Dictionary,
    ) -> MaskScope {
        let stated = match self.transparency_group(&request.group.dict) {
            Some(group) => {
                let document = self.document;
                stated_cmyk_group(document, &group.colour_space, resources, |entry| {
                    self.blending_space_of(entry)
                })
            }
            None => None,
        };
        (
            std::mem::replace(&mut self.compositing, request.compositing.clone()),
            std::mem::take(&mut self.spots_beside),
            std::mem::replace(&mut self.cmyk_group, stated),
        )
    }

    /// Restores what [`Interpreter::enter_mask_compositing`] handed back.
    fn leave_mask_compositing(&mut self, (compositing, spots, cmyk_group): MaskScope) {
        self.compositing = compositing;
        self.spots_beside = spots;
        self.cmyk_group = cmyk_group;
    }

    /// Evaluates a soft mask's transparency group and registers it (§11.5, §11.6.5.1).
    ///
    /// Returns `None` when the group draws nothing at all, which §11.5.2's NOTE 2 makes a
    /// mask of zero — but only for the alpha derivation, where an empty group masks
    /// everything away; a luminosity mask over a white backdrop is a mask of *one*. Both
    /// answers are the mask's own, so the group is registered either way and only an
    /// unreadable one gives up here.
    ///
    /// A group stating no `/Resources` of its own is read against the **page's** dictionary,
    /// the same rule `draw_xobject` applies to a form a `Do` names and `text` to a Type 3
    /// glyph description. Table 142 makes `/G` a transparency group `XObject`, so §7.8.3's
    /// page fallback reaches a mask's group as it reaches any other form. ISO 32000-2:2020
    /// printed it as a fourth bullet — quoted below as printed, because that is what
    /// `doc/md/` holds — and Errata Collection 3 Issue #128 retired the bullet into the
    /// informative NOTE 3 that now stands in its place and names the same dictionary
    /// (`doc/errata-read.md`, §7.8.3):
    ///
    /// > PDF files written obeying earlier versions of PDF may have omitted the Resources
    /// > entry in all form XObjects and Type 3 fonts used on a page. All resources that are
    /// > referenced from those forms and fonts shall be inherited from the resource
    /// > dictionary of the page on which they are used.
    ///
    /// ADR 1059 is why that dictionary is the page's and not the stream that invoked the
    /// group; the two are one dictionary until a form is nested inside a form.
    /// `GHOSTSCRIPT-691218-1.pdf` is the witness: a Ghostscript 8.71 page whose every form
    /// omits the entry, whose photograph and price figures are painted through luminosity
    /// masks, and whose mask groups ran against an empty dictionary here — every `gs`, `Do`,
    /// `Tf` and `scn` inside them reported as a name nobody defined, and the marks under the
    /// masks lost.
    pub(in crate::content) fn build_soft_mask(
        &mut self,
        request: &crate::soft_mask::SoftMaskRequest,
        state: &GraphicsState,
    ) -> Option<SoftMaskId> {
        if self.soft_mask_depth >= MAX_SOFT_MASK_DEPTH {
            self.note(Unsupported::LimitReached {
                limit: "MAX_SOFT_MASK_DEPTH",
            });
            return None;
        }
        // §11.6.5.1: "The group shall be defined by a transparency group XObject (see 11.6.6,
        // 'Transparency group XObjects') designated by the G entry in the soft-mask
        // dictionary" — a form `XObject`, so §7.8.2's rule for a damaged prefix reaches here
        // too. **What makes it the same rule and not the sampled function's** (ADR 0356) is
        // that this clause states the mask's value where the group painted nothing: the
        // transfer function of 0.0 for `Alpha`, `/BC`'s luminosity for `Luminosity`. A place
        // the damage took is a place the group did not paint, and the clause already answers
        // for one of those — where a sampled function's missing samples are values it has no
        // answer for and interpolates from. Places, not values, so the prefix is drawn.
        let Some(content) = self.content_stream(
            &request.group,
            "a soft mask's transparency group /G (§11.6.5.1)",
        ) else {
            self.note(Unsupported::SoftMask {
                detail: "/SMask names an undecodable /G".to_owned(),
            });
            return None;
        };

        // §11.6.5.1: "The mask's coordinate system shall be defined by concatenating the
        // transformation matrix specified by the Matrix entry in the transparency group's
        // form dictionary … with the current transformation matrix at the moment the soft
        // mask is established in the graphics state with the gs operator." The mask is
        // therefore fixed here, at the `gs`, and does not move with whatever transform is in
        // force when it is finally used.
        let mut inner = GraphicsState::initial(state.transform);
        if let Some(matrix) = self.matrix(&request.group.dict) {
            inner.transform = matrix.then(inner.transform);
        }
        // The clip chain starts fresh rather than inheriting the caller's. A mask is not
        // painted, so nothing about it is clipped by the path in force; its own `/BBox` is
        // the whole of its extent, and §11.6.5.1 gives the mask a value everywhere outside
        // that box — the transfer function of 0.0, or of the backdrop's luminosity — which
        // an inherited clip would have no way to express.
        let group_box = self.rectangle(&request.group.dict, "BBox");
        if let Some(bbox) = group_box {
            inner.clip = self.rect_clip(bbox, inner.transform, None);
            if inner.clip.is_none() {
                self.note(Unsupported::LimitReached { limit: "max_clips" });
                return None;
            }
        }

        let group_resources = self
            .document
            .get_key(&request.group.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_else(|| self.page_resources.as_ref().clone());

        for detail in &request.departures {
            self.note(Unsupported::TransparencyGroup {
                detail: detail.clone(),
            });
        }

        let mark = self.list.command_count();
        self.soft_mask_depth = self.soft_mask_depth.saturating_add(1);
        // §8.6.8's restriction does **not** reach in here (ADR 0173). The clause applies "[i]n any
        // glyph description that uses the d1 operator … and to all other content streams invoked
        // from within the same glyph description", and it says why in the sentence before: "when
        // defining graphical figures whose colours shall be specified separately each time they are
        // used". A soft mask is not such a figure. It carries no colour to the page at all —
        // §11.6.5.2 turns the group's result into a luminosity and uses it as *alpha* — so NOTE 1's
        // own reason for exempting a stencil applies verbatim: it "does not specify colours;
        // instead, it designates places where the current colour is painted". Worse, the
        // restriction is actively destructive here: a `/Luminosity` mask's values *are* the group's
        // colours, so ignoring `rg` inside it changes the mask, and ignoring the group's images
        // leaves a mask of zero that erases the very marks the glyph exists to make.
        //
        // `issue19634.pdf` is the witness — a Skia blur test whose red text is a Type 3 font
        // whose glyph procedure is `d1`, a `gs` naming a `/Luminosity` mask, and one `re f`.
        // The mask group draws a blurred greyscale image; with the flag leaking in, the image
        // is skipped by §8.6.8's image rule, the mask comes out zero and the text vanishes.
        // Ink 2.87 against `mupdf`'s 7.63 and `hayro`'s 8.11 (ADR 0173).
        let saved_uncoloured = std::mem::replace(&mut self.uncoloured, false);
        // §11.6.5.1 makes the group's `/CS` "the colour space in which the compositing
        // computation is to be performed", and `crate::soft_mask` has already decided which of
        // this tree's two routes answers that space. A mask group nested inside another one
        // may name a different space, so this is saved and restored like `uncoloured` rather
        // than set once.
        // And no spot colourant reaches a mask's group, whatever it composites in (§11.7.3, ADR
        // 1329): `Interpreter::spots_beside`.
        let saved_compositing = self.enter_mask_compositing(request, &group_resources);
        // And §11.6.6's blending space stops being a departure in here, which is ADR 0220's
        // finding rather than a simplification: a mask group whose space is subtractive is
        // painted in the ink §10.4.2.3 weighs, that weighting is linear in the components, and
        // a linear functional of a convex combination is the convex combination of the
        // functional. So the compositing this tree performs inside such a group *is* the
        // compositing the clause asks for, and the one thing that is not — a blend function,
        // which is not affine — is `note_blended_luminosity`'s report and not this one.
        let saved_blending = self.blending.take();
        // **And so does the flag that records one.** [`Interpreter::blending_changed`] answers
        // exactly one question — whether the *page* may be composited in the space §11.4.7 gives it
        // — and the line above makes every group inside a mask compare its space against `None`, so
        // a mask group holding an isolated `/DeviceCMYK` group set the flag on a page that
        // composites in `/DeviceCMYK` and departs from nothing. **77 of the 85 web documents
        // reported for §11.6.6 and all three of the corpus's were that**, measured by asking each
        // change its `soft_mask_depth` (ADR 0276). A mask's group is not painted onto the page at
        // all — §11.5.3 turns its result into one luminosity — so no space inside it is a space the
        // page composites in.
        let saved_change = std::mem::replace(&mut self.blending_changed, false);
        // And so does §11.4.6, for the reason the `None` below it states: the mask's group is
        // not an element of the knockout group the `gs` appears in — an `/SMask` is named by an
        // `/ExtGState` rather than being an element of anything — so nothing inside it is a
        // direct element of that group and nothing inside it inherits that group's initial
        // backdrop.
        let saved_backdrop = self.enclosing_knockout.take();
        // And §11.6.4.3's `/AIS`, restored exactly rather than folded back: the record answers
        // "which readings did the elements of the group being built paint under", and a
        // mask's content is not an element of anything — its marks become one alpha per pixel.
        let saved_ais = self.open_reading_scope(inner.alpha_is_shape);
        // And the record of a space departure met on a subtractive run, for ADR 0276's
        // reason one construction over: a space declared inside a mask is answered by
        // §11.5.3's own derivation and says nothing about the group the `gs` sits in.
        let saved_departed = std::mem::replace(&mut self.nested_space_departed, false);
        // §11.7.5.2's record of a transfer function having reached a mark is **not** scoped here,
        // and no longer needs to be: `Interpreter::transfer_for_mark` reads `soft_mask_depth`
        // itself and hands every mark inside a mask the page's default, so nothing in here can
        // set the record or raise the report. The clause is about "any given point on the page",
        // and §11.5.3 turns this group's result into one luminosity rather than painting it at a
        // point. Saving the flag across the run said the same thing one indirection later, and
        // said it only about the report.
        //
        // And pattern space (ADR 0456). §8.7.2 states where a pattern's matrix lands:
        //
        // > Similarly, if a pattern is used within a form XObject (see 8.10, "Form XObjects"
        // > ), the pattern matrix maps pattern space to the form's default user space (that
        // > is, the form coordinate space at the time the form is painted with the Do
        // > operator).
        //
        // A mask's `/G` is a form XObject, and §11.6.5.1's sentence quoted above is what
        // stands in for the `Do`: its coordinate space is the group's `/Matrix` concatenated
        // with the transform at the `gs`, which is `inner.transform` and nothing else. Left
        // at the *page's* default space, a shading pattern painted inside a mask lands where
        // the page would have put it rather than where the mask does — visible whenever the
        // two differ, which is every page that scales its own content before setting a mask.
        // `draw_xobject` has made this replacement since the clause was first read; this is
        // the second door into the same room.
        let saved_base = std::mem::replace(&mut self.base, inner.transform);
        // The group's own object is not to hand here — `SoftMaskRequest` carries the stream
        // resolved — and the ledger needs the route more than the object: a soft mask's group
        // is the construct the survey does not walk, so nothing under it is compared.
        self.enter_ledger_frame(crate::content::ledger::Route::SoftMask, None);
        let admitted = self.one_colour.admitted;
        self.run(&content, &group_resources, &inner);
        // §11.4.7's second raster, where the mask group's blending colour space has four
        // components: the same content stream interpreted again in the black component, with
        // the readback its first run collected put back off (`ReadbackMark`). Taken here,
        // before any of the scoped state is restored, because the second run has to see
        // exactly what the first did.
        //
        // **Unconditional where the space has four components**, which is where this differs
        // from `group_commands`' pair: that one skips the second run when nothing inside the
        // group composites, because a group's four components are converted to the device at
        // the end and an opaque Normal mark carries its colour through whatever space it was
        // carried in. A mask's four components are converted to *one number* by §11.5.3's
        // `Y`, which is a function of all four however opaque the marks are.
        let (mut commands, mut black) =
            self.mask_halves(request, &content, &group_resources, &inner, mark, admitted);
        self.leave_ledger_frame();
        self.base = saved_base;
        self.nested_space_departed = saved_departed;
        let mask_readings = std::mem::replace(&mut self.readings, saved_ais);
        let mask_alpha_sources = mask_readings.seen();
        // A mask's list goes to a backend as it stands, so its seals come off here rather than
        // with the page's (`unseal_list`).
        if mask_readings.sealed() {
            unseal(&mut commands, false);
            if let Some(black) = black.as_mut() {
                unseal(&mut black.commands, false);
            }
        }
        self.enclosing_knockout = saved_backdrop;
        self.blending_changed = saved_change;
        self.blending = saved_blending;
        self.leave_mask_compositing(saved_compositing);
        self.uncoloured = saved_uncoloured;
        self.soft_mask_depth = self.soft_mask_depth.saturating_sub(1);

        self.note_mask_group_structure(
            request,
            group_box.map(|corners| (corners, inner.transform, inner.clip)),
            &mut commands,
            black.as_mut(),
            mask_alpha_sources,
        );
        self.note_blended_luminosity(&request.compositing, &commands);

        // A four-axis `Y` reads the pair and nothing else, so the two stand or fall together:
        // where `mask_halves` gave the pair up it also drew the group on the device's three
        // components, and the luminance goes with it.
        let luminance = request
            .luminance
            .clone()
            .filter(|luminance| luminance.axes() == 3 || black.is_some());
        let evaluated = pdf_render::SoftMask {
            commands,
            kind: request.kind,
            transfer: request.transfer.clone(),
            luminance,
            black,
        };
        let Ok(id) = self.list.add_soft_mask(evaluated) else {
            self.note(Unsupported::LimitReached {
                limit: "max_soft_masks",
            });
            return None;
        };
        Some(id)
    }

    /// Reports what a mask's group says about its own structure, after the list that stands for
    /// it is complete, and states its backdrop in that list where one is owed.
    ///
    /// `extent` is the group's `/BBox` with the transform and clip it was clipped by.
    fn note_mask_group_structure(
        &mut self,
        request: &crate::soft_mask::SoftMaskRequest,
        extent: Option<([f32; 4], Transform, Option<ClipId>)>,
        commands: &mut Vec<Command>,
        black: Option<&mut BlackHalf>,
        alpha_sources: super::readings::AlphaSourcesSeen,
    ) {
        // §11.5.3, of the group a mask is derived from: "G may be any kind of group -
        // isolated or not, knockout or not - producing various effects on the C result in
        // each case." So Table 145's two flags mean here what they mean anywhere, and the
        // group is evaluated as the isolated non-knockout one either way, except where
        // `state_mask_backdrop` states the group's backdrop in the list (ADR 2508) — the same
        // departure, reported on the same conditions, rather than a second reading of the
        // same table.
        if let Some(group) = self.transparency_group(&request.group.dict) {
            let backdrop_stated =
                state_mask_backdrop(request.kind, &group, extent, commands, black);
            // `false`: a mask's group is evaluated into a mask raster by
            // `pdf_render::SoftMask`, which carries no knockout flag, so a knockout here is
            // a departure whatever its elements are.
            // For the second, a mask raster is built on transparency, so a non-isolated mask
            // group is drawn as §11.4.5's isolated one and reports on the condition a painted
            // group reports on (`note_group_structure`) — except where the backdrop is stated in the list
            // (`state_mask_backdrop`), or where the derivation ignores colour: §11.6.5.1's
            // Alpha subtype evaluates "a group alpha only", a blend function moves colour and
            // never alpha, and there is no pixel for the report to be about.
            // `false` for the third: §11.4.6's NOTE 6 gives its backdrop to the *elements* of
            // a knockout group, and a soft mask is named by an `/ExtGState` rather than being
            // an element of anything — so a mask group's non-isolation is a departure however
            // it was reached.
            self.note_group_structure(
                &group,
                commands,
                GroupDrawn {
                    knockout: false,
                    isolated: !(backdrop_stated || request.kind == SoftMaskKind::Alpha),
                    backdrop_transparent: false,
                    alpha_sources,
                },
            );
        }
    }
}

/// Puts §11.5.3's opaque backdrop under a luminosity mask's non-isolated group, so that a blend
/// mode inside the group sees it; says whether it did.
///
/// ISO 32000-2 §11.6.5.1, of a luminosity mask's group G: it "shall be composited with a fully
/// opaque backdrop whose colour is everywhere defined by the softmask dictionary's BC entry."
/// Table 145's `/I` says where a group that is not isolated takes its elements' backdrop
/// from: "if false , they shall be composited against the group's backdrop." Here that backdrop
/// is the `/BC` colour (§11.4.3), and §11.4.4 composites the
/// group's result with its backdrop afterwards, so a non-knockout group painted Normal at full
/// opacity is its elements painted onto the backdrop directly. So the elements are drawn over
/// the `/BC` colour: a fill of it under them, bounded by the group's own `/BBox` clip, which is
/// the whole extent the clause gives the group — outside it the mask is the backdrop's
/// luminosity, and the raster's transparent pixels already say that.
///
/// A raster drawn on transparency composites onto `/BC` after the fact
/// ([`pdf_render::SoftMask::value`]), which is the isolated group's reading, and the two agree
/// except where an element's blend function reads the backdrop: `Multiply` over a black `/BC` is
/// black, and over transparency it is the source. Stated only where a command blends, so a mask
/// whose group paints Normal throughout is left unchanged.
///
/// The fill is larger than the box by one unit on every side and takes the box's clip as its
/// own, so the edge pixels have the clip's coverage once rather than twice. A group without a
/// `/BBox` has no extent to fill and is left to the report (ADR 2508).
fn state_mask_backdrop(
    kind: SoftMaskKind,
    group: &super::TransparencyGroup,
    extent: Option<([f32; 4], Transform, Option<ClipId>)>,
    commands: &mut Vec<Command>,
    black: Option<&mut BlackHalf>,
) -> bool {
    let SoftMaskKind::Luminosity { backdrop } = kind else {
        return false;
    };
    let Some((corners, transform, clip)) = extent else {
        return false;
    };
    if group.isolated
        || group.knockout
        || !any_command(commands, &|command| command_blends(command))
    {
        return false;
    }
    let under = |colour: Color| {
        let [x0, y0, x1, y1] = corners;
        let (x0, y0, x1, y1) = (
            x0.min(x1) - 1.0,
            y0.min(y1) - 1.0,
            x0.max(x1) + 1.0,
            y0.max(y1) + 1.0,
        );
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(x0, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y1)));
        path.push(PathCommand::LineTo(Point::new(x0, y1)));
        path.push(PathCommand::Close);
        Command::Fill {
            path: Arc::new(path),
            transform,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(colour),
            clip,
            mask: None,
            blend: BlendMode::Normal,
        }
    };
    commands.insert(0, under(backdrop));
    if let Some(half) = black {
        half.commands.insert(0, under(half.backdrop));
    }
    true
}
