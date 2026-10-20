//! A transparency group on the scene, a soft mask's body, and the knockout state a group sets.
//!
//! A group's elements are the same command walk as the page's, so the group is translated by
//! recursion into [`Encoder::commands`]; this module is what wraps that recursion in raster's
//! group vocabulary (ISO 32000-2 §11.4) and refuses the shapes that vocabulary cannot state.

use pdf_render::{BlendMode, Command, SoftMaskId, SoftMaskKind};
use raster_scene::SceneBuilder;

use crate::QuorraRasterError;

use super::{Admitted, Encoder, GroupParts, blend_mode, colour};

/// Refuses the shapes of `Command::Group` raster's vocabulary cannot state.
///
/// - **A non-isolated knockout group** (`isolated: false` beside `knockout: true`):
///   §11.4.6 composites each element with the group's *initial* backdrop — here the
///   group's own — which needs that backdrop retained beside the accumulation and a
///   scratch per element. A `GroupSpec` carries the two flags but a scene states no
///   per-element backdrop, and raster's staged `DestOut`/`Plus` pair is written on the
///   transparent start §11.4.5 gives (its ADRs 0025, 0032). Passing the flags through
///   would substitute one backdrop for the other in silence.
/// - **A non-isolated group carrying a blending colour space of its own**, which `pdf-model`
///   never builds: §11.6.6 gives a non-isolated group its parent's space, and the CPU backend
///   refuses the same combination. An isolated one is drawn by `own_space` (ADR 1471).
///
/// Both go to the CPU backend; refusing here is what keeps either from becoming a wrong
/// picture in silence, which is trap 5.
pub(super) fn refuse_untranslatable_group(
    isolated: bool,
    knockout: bool,
    own_space: Option<&pdf_render::GroupBlending>,
) -> Result<(), QuorraRasterError> {
    if knockout && !isolated {
        return Err(QuorraRasterError::Unsupported(
            "a non-isolated knockout group: each element composites with the group's own \
             initial backdrop, which a scene cannot retain beside the accumulation \
             (ISO 32000-2 §11.4.6)"
                .to_owned(),
        ));
    }
    if own_space.is_some() && !isolated {
        return Err(QuorraRasterError::Unsupported(
            "a non-isolated group carrying a blending colour space of its own: a non-isolated \
             group inherits its parent's space (ISO 32000-2 §11.6.6)"
                .to_owned(),
        ));
    }
    Ok(())
}

impl Encoder<'_> {
    /// Whether this command is being encoded inside ISO 32000-2 §11.4.6's knockout group.
    ///
    /// See [`Self::knockouts`]: it is the one place a coverage folded into a paint's alpha would
    /// be read back as opacity, so the substitutions §10.7.4 asks for are withheld under it.
    pub(crate) fn inside_knockout(&self) -> bool {
        self.knockouts > 0
    }

    /// Whether this command is a direct element of ISO 32000-2 §11.4.6's knockout group.
    ///
    /// Such an element composites with the group's initial backdrop, which raster's scene keeps
    /// transparent, and §11.3.6 gives every blend function Normal's arithmetic there — "An alpha
    /// value of αs = 0.0 or αb = 0.0 results in no blend mode effect" — so §11.7.4.3's mode needs
    /// no group of its own in this position, and one would be refused as a knockout element
    /// whose shape a layer does not carry (`doc/adr/1295`).
    pub(crate) fn knockout_element(&self) -> bool {
        self.element_of_knockout
    }

    /// One transparency group, composited by `compose` (ISO 32000-2 §11.4.1).
    ///
    /// See [`refuse_untranslatable_group`] for the shapes of the command every caller
    /// screens out before building a [`raster_scene::GroupSpec`], and `own_space` for the group
    /// that composites in a blending colour space of its own (ADR 1471).
    ///
    /// `compose` is [`raster_scene::Compose::SrcOver`] for every group a page states, and one
    /// of §11.4.6's two staged operators where this group is one half of a
    /// [`Command::Shaped`] — see [`Self::shaped`].
    pub(super) fn group(
        &mut self,
        builder: &mut SceneBuilder,
        parts: GroupParts<'_>,
        compose: raster_scene::Compose,
    ) -> Result<(), QuorraRasterError> {
        // §8.5.4's third sentence gives a group's *result* a shape of its own — "the shape of
        // a transparency group (defined as the union of the shapes of its constituent objects)"
        // — and nothing here bounds that union without walking the elements, so a group keeps
        // its clip whatever it contains. `doc/QUORRA_FEEDBACK.md` section 36 is that seam.
        let Admitted::Chain(clip) = self.clip_chain(builder, parts.clip, None)? else {
            return Ok(()); // the clip admits nothing: the group draws nothing
        };
        let mask = self.mask_id(builder, parts.mask)?;
        // A group whose own blend is §11.7.4.3's mode states it as its compositing
        // operator, which a group that is also one of §11.4.6's staged halves cannot hold
        // twice. Table 146's last row has a group revert to Normal and the interpreter
        // emits none such, so this refuses a construction rather than a page.
        let compose = match (compose, super::compose(parts.blend)) {
            (stated, raster_scene::Compose::SrcOver) => stated,
            (raster_scene::Compose::SrcOver, own) => own,
            (_, _) => {
                return Err(QuorraRasterError::Unsupported(
                    "a group that is one of §11.4.6's staged halves and composites under \
                     §11.7.4.3's special overprinting blend mode"
                        .to_owned(),
                ));
            }
        };
        if let Some(blending) = parts.blending {
            return self.group_in_own_space(builder, (parts, clip, mask), compose, blending);
        }
        let spec = raster_scene::GroupSpec {
            alpha: parts.alpha,
            blend: blend_mode(parts.blend),
            clip,
            knockout: parts.knockout,
            mask,
            compose,
            // Table 145's `/I`, straight through. §11.4.5's isolated group is
            // what a layer in any rasterising library is; §11.4.4's other
            // initial backdrop — "the group's backdrop" — is the one raster
            // gained in its ADR 0019, and the flag is how a scene asks for it.
            //
            // **The conditions are not re-checked here, and that is
            // deliberate.** `pdf-model` emits `isolated: false` only outside
            // every knockout group (`Command::Group`'s `isolated` states the
            // guarantee), and raster draws that set under every blend mode:
            // under Normal by the collapse ADR 0237 derives, under any other by
            // §11.4.4's result step from the group alpha a second accumulator
            // carries (ADR 1307), which is how §11.7.4.3's last paragraph — an
            // object under the special overprinting mode painted with a
            // non-Normal `/BM` — reaches the device. What raster still refuses
            // at `SceneBuilder::group` arrives below as a typed
            // `QuorraRasterError::Scene` naming the condition, as
            // `SceneError::NonIsolatedGroupUnsupported`. A copy of the condition
            // here would be a second reading of §11.4.4 free to drift from the
            // one that decides the picture.
            isolated: parts.isolated,
        };
        let mut walked = Ok(());
        // §11.4.6's group is counted around the walk rather than tested inside it, because what
        // its elements may not do — carry a coverage in the paint's alpha — is true of every
        // element at every depth under it, not only of its immediate children.
        let outer = self.knockouts;
        let outer_element = self.element_of_knockout;
        self.knockouts = outer.saturating_add(u32::from(parts.knockout));
        self.element_of_knockout = parts.knockout;
        builder.group(spec, |body| {
            walked = self.commands(body, parts.commands);
            // The builder's own error channel carries scene refusals;
            // upload and translation errors travel beside it.
            Ok(())
        })?;
        self.knockouts = outer;
        self.element_of_knockout = outer_element;
        walked
    }

    /// §11.4.6's two stages, for an element whose shape the display list states apart from
    /// its alpha ([`Command::Shaped`]).
    ///
    /// On the transparent initial backdrop a group is built on, the clause's weighted
    /// average is one line per pixel in premultiplied form — with the accumulated result
    /// `P`, the element's shape `f` and its premultiplied colour `S`:
    ///
    /// ```text
    /// P' = (1 − f) × P + S
    /// ```
    ///
    /// which is Porter-Duff Destination-Out with the shape half, then Plus with the object.
    /// [`raster_scene::Compose::Src`] cannot say it: that operator reads the shape off the
    /// coverage a mark is drawn with, which is the assumption this element exists to
    /// contradict.
    ///
    /// **Both halves are emitted or neither is**, which is this caller's obligation rather
    /// than something the library can check — `Plus` alone drives a premultiplied channel
    /// past its alpha, and one mark cannot tell a library that the other is coming
    /// ([`raster_scene::Compose::Plus`]). The two halves carry the same clip and the same
    /// geometry (`pdf_model`'s `stated_shape` derives one from the other), so every route
    /// that draws nothing removes both.
    ///
    /// Outside a knockout group the shape is unused and the object would be drawn alone —
    /// but [`Command::Shaped`] guarantees this command occurs nowhere else, so there is no
    /// such branch here.
    pub(super) fn shaped(
        &mut self,
        builder: &mut SceneBuilder,
        object: &Command,
        shape: &Command,
    ) -> Result<(), QuorraRasterError> {
        self.stage(builder, shape, raster_scene::Compose::DestOut)?;
        self.stage(builder, object, raster_scene::Compose::Plus)
    }

    /// One half of [`Self::shaped`], drawn with the operator that stage is.
    ///
    /// A **group** states the operator itself: §11.3.7.2 makes a group's shape "the union
    /// […] of the shapes of the objects it contains", which no single mark can state, and
    /// `raster_scene::GroupSpec::compose` is where raster took that ask (its ADR 0033).
    ///
    /// Everything else is drawn inside a group of one element, which is the same
    /// arithmetic: an isolated group at alpha 1 under no mask, clip or blend holds exactly
    /// the element's own premultiplied colour, so compositing it is compositing the
    /// element. That is one buffer per staged mark and it buys uniformity — `stroke` and
    /// `image` carry no compositing operator in this vocabulary at all, and a `fill` whose
    /// paint is a sampled shading is drawn as an image too (see [`Self::sampled_fill`]), so
    /// a per-mark route would be correct for some paints and silently wrong for the rest.
    fn stage(
        &mut self,
        builder: &mut SceneBuilder,
        half: &Command,
        compose: raster_scene::Compose,
    ) -> Result<(), QuorraRasterError> {
        if let Command::Group {
            commands,
            alpha,
            clip,
            mask,
            blend,
            isolated,
            knockout,
            // Unread here for the reason `Encoder::commands` gives.
            alpha_is_shape: _,
            blending,
        } = half
        {
            refuse_untranslatable_group(*isolated, *knockout, blending.as_deref())?;
            return self.group(
                builder,
                GroupParts {
                    commands,
                    alpha: *alpha,
                    blend: *blend,
                    clip: *clip,
                    mask: *mask,
                    isolated: *isolated,
                    knockout: *knockout,
                    blending: blending.as_deref(),
                },
                compose,
            );
        }
        self.group(
            builder,
            GroupParts {
                commands: std::slice::from_ref(half),
                alpha: 1.0,
                blend: BlendMode::Normal,
                // The element carries its own clip and mask, which the walk below applies;
                // stating them here as well would multiply each in twice.
                clip: None,
                mask: None,
                isolated: true,
                knockout: false,
                blending: None,
            },
            compose,
        )
    }

    /// The raster soft mask for a display-list one, realised on first use through
    /// the same command walk as the page (a mask that took a different path would
    /// be code the cross-backend tests never exercise).
    pub(crate) fn mask_id(
        &mut self,
        builder: &mut SceneBuilder,
        mask: Option<SoftMaskId>,
    ) -> Result<Option<raster_scene::MaskId>, QuorraRasterError> {
        let Some(id) = mask else { return Ok(None) };
        if let Some(mapped) = self.masks.get(&id.index()) {
            return Ok(Some(*mapped));
        }
        let def = self
            .list
            .soft_mask(id)
            .ok_or(QuorraRasterError::UnknownSoftMask(id))?;
        // §11.5.3's `Y` of a group composited in a CIE-based space, or in `DeviceCMYK`'s four
        // components, is read off that space's own arithmetic per pixel (`pdf_render::Luminance`),
        // where raster's luminosity mask weighs §10.4.2.2's coefficients in its own shader, and
        // the four-component shape is §11.4.7's pair of rasters where a scene's mask names one
        // body. Both are drawn as frames of their own and reduced by the shared arithmetic
        // (`own_space`, ADR 1471).
        if def.black.is_some() || def.luminance.is_some() {
            let mapped = self.mask_in_own_space(builder, def)?;
            self.masks.insert(id.index(), mapped);
            return Ok(Some(mapped));
        }
        let kind = match def.kind {
            SoftMaskKind::Alpha => raster_scene::MaskKind::Alpha,
            SoftMaskKind::Luminosity { backdrop } => raster_scene::MaskKind::Luminosity {
                backdrop: colour(backdrop),
            },
        };
        let transfer = def.transfer.as_ref().map(|t| {
            // The display list's table is private behind `apply`; raster's is the
            // table itself. Reconstructing through `apply` keeps the one source.
            raster_scene::Transfer(std::array::from_fn(|i| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the index enumerates exactly 0..=255"
                )]
                t.apply(i as u8)
            }))
        });
        let commands = def.commands.clone();
        let mut walked = Ok(());
        let mapped = builder.mask(kind, transfer, |body| {
            walked = self.commands(body, &commands);
            Ok(())
        })?;
        walked?;
        self.masks.insert(id.index(), mapped);
        Ok(Some(mapped))
    }
}
