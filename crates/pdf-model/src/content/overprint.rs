//! ISO 32000-2 §8.6.7's overprint parameters, and §11.7.4's reading of them.
//!
//! # Where overprinting can change a pixel here, and where it cannot
//!
//! §8.6.7 is the *opaque* imaging model, and on this device its own sentences settle it: the
//! marks go to three additive colourants and no separations, so "[i]f overprinting is not
//! supported, the value of the overprint parameter shall be ignored" (NOTE 1), and the
//! overprint mode "shall not apply if the native colour space of the output device does not
//! include CMYK device colourants; in that case, source colours shall be converted to the
//! device's native colour space, and all components participate in the conversion, whatever
//! their values" — which is what `ColourSpace::to_rgb` does.
//!
//! §11.7.4 asks the question again of the *transparent* model, and its condition is a
//! different one. §11.7.4.3's first bullet needs "the current colour space and group colour
//! space [to be] both DeviceCMYK", not the device's; §11.7.4.2 states outright that the
//! computation happens in the group's components and reaches the device's afterwards —
//!
//! > If the group colour space is different from the native colour space of the output
//! > device, its components are not the device's actual process colourants; the blending
//! > computations shall affect the process colour ants only after the group's results have
//! > been converted to the device colour space.
//!
//! — and §11.7.4.5's NOTE 1 names this exact circumstance as the one where the two models
//! part company: "[t]his difference between opaque and transparent overprinting and erasing
//! rules arises only within a transparency group (including the page group, if its colour
//! space is different from the native colour space of the output device)."
//!
//! So the cell that is owed here is Table 146's first row under `OP true, OPM 1`, and it is
//! owed exactly where [`Compositing::Subtractive`] is in force — the page or isolated group
//! whose blending space has four components, which §11.7.2 makes `DeviceCMYK` within the
//! group even when it is an `ICCBased` 'CMYK' profile:
//!
//! > If an isolated transparency group or page has an ICCBased 'CMYK' colour space ,
//! > DeviceCMYK shall be redefined within the transparency group to be the same as the
//! > blending colour space and references to the process colourants Cyan , Magenta , Yellow
//! > and Black are defined to be references to the corresponding colourants in the blending
//! > colour space, even where the actual or simulated output device is not CMYK.
//!
//! Every other cell of Table 146 gives `C_s` in all three of its columns and is therefore the
//! Normal blend function, which is what this tree composites through. The three spot-colourant
//! rows are unreachable because the group's components are four process ones and no spot
//! colourant is maintained beside them; the `Separation` and `DeviceN` rows are unreachable
//! because §11.7.3 requires such a space to revert inside a group that states its own space —
//! "[i]f any other colour space has been specified for the group, the Separation or DeviceN
//! colour space shall be converted to its alternate colour space". ADR 1157 has the reading
//! and ADR 1158 what it costs; ADR 0028 is what it supersedes.
//!
//! # `may`, and what still follows from it
//!
//! §11.7.4.3's own verb is permissive — "a PDF processor may consider implementing a special
//! blend mode that consults the overprint-related graphics state parameters" — so declining
//! the whole mode is something the standard allows. What it does not allow is declining it
//! *and* claiming the clause is inapplicable: the permission is a choice, and this project's
//! answer to a choice between a producer's stated intent and a cheaper picture is
//! `CLAUDE.md`'s. The `shall`s that follow the permission are conditional on taking it, and
//! they are what this module computes.

#![expect(
    clippy::doc_markdown,
    reason = "this module's prose is largely quotation, and a quotation may not gain backticks"
)]

use pdf_render::{BlendMode, Overprint};

use crate::colour::{Compositing, Half};

use super::report::Unsupported;
use super::{GraphicsState, Interpreter};

impl Interpreter<'_> {
    /// The blend mode a fill, stroke or glyph is painted under, §11.7.4.3's special one
    /// included.
    ///
    /// `stroking` picks between §8.6.7's two overprint parameters, the same way
    /// [`Interpreter::fill_paint`] and [`Interpreter::stroke_paint`] pick between the two
    /// pattern slots. The answer is `state.blend` in every case but one, and the cost of
    /// asking on a page that states no overprint is one integer comparison.
    ///
    /// # The five conditions, each from its clause
    ///
    /// - The group colour space is `DeviceCMYK` ([`Compositing::Subtractive`], §11.7.2).
    /// - The overprint mode is 1 (§8.6.7; the mode "shall have an effect only when the
    ///   overprint parameter is true", which the next condition is).
    /// - The overprint parameter for this kind of painting operation is true (§11.7.4.3: the
    ///   mode "may be implicitly invoked whenever an elementary graphics object is painted
    ///   while overprinting is enabled").
    /// - The current colour is a `DeviceCMYK` one the content stream stated directly, which is
    ///   Table 146's first row and §8.6.7's "painting operations that use the current colour
    ///   in the graphics state when the current colour space is DeviceCMYK". A pattern is not
    ///   such a colour: §8.6.7 excludes shadings outright, and a tiling pattern's cell paints
    ///   its own.
    /// - At least one component's tint is zero, which is the only way the bullet's value
    ///   differs from `C_s`.
    ///
    /// # Why a non-Normal current blend mode is reported rather than built
    ///
    /// §11.7.4.3's last paragraph asks for an implicit non-isolated, non-knockout group in
    /// that case, with the object painted under the special mode inside it and the group
    /// painted under the current mode. Its NOTE 3 makes the group unnecessary when the current
    /// mode is Normal, which is the case this builds; the other is named
    /// ([`Unsupported::Overprint`]) and painted under the document's own mode, so the page is
    /// short of the clause's construction and says so rather than silently dropping either the
    /// overprint or the mode.
    ///
    /// # Why it is two functions
    ///
    /// This is asked twice per painting operator, so what it costs on a page that states no
    /// overprint is what the whole feature costs there. Measured under callgrind on page 101 of
    /// ISO 32000-2 interpreted fifty times, against the same build with the call planted away:
    /// as one function, **+3 754 490 instructions, 0.30%**; split so that the caller inlines an
    /// integer comparison and a discriminant test and everything else is a cold call the page
    /// never makes, **+396 494, 0.032%**. The whole of the rest of this feature on that page —
    /// Table 57's three lookups per `gs`, the tints beside every colour, the larger graphics
    /// state a `q` clones — is 38 531 instructions of the first figure, which is why the split
    /// is here and nowhere else. One hop of indirection for a tenfold cut, stated with the
    /// number because `CLAUDE.md` asks for the benchmark beside the technique.
    #[inline]
    pub(super) fn overprint_blend(&mut self, state: &GraphicsState, stroking: bool) -> BlendMode {
        if state.overprint_mode != 1 || !matches!(self.compositing, Compositing::Subtractive(..)) {
            return state.blend;
        }
        self.special_overprint(state, stroking)
    }

    /// [`Interpreter::overprint_blend`]'s cold half: everything past the two tests above.
    #[inline(never)]
    fn special_overprint(&mut self, state: &GraphicsState, stroking: bool) -> BlendMode {
        let Compositing::Subtractive(half, _) = self.compositing else {
            return state.blend;
        };
        let enabled = if stroking {
            state.overprint_stroking
        } else {
            state.overprint_filling
        };
        let tints = if stroking {
            state.stroke_tints
        } else {
            state.fill_tints
        };
        let patterned = if stroking {
            state.stroke_pattern.is_some()
        } else {
            state.fill_pattern.is_some()
        };
        let (true, Some(tints), false) = (enabled, tints, patterned) else {
            return state.blend;
        };
        // Which of the raster's three channels this half carries, and therefore which of the
        // four tints decides each of them (`crate::colour::Half`).
        let kept = match half {
            Half::Chromatic => [tints[0] == 0.0, tints[1] == 0.0, tints[2] == 0.0],
            Half::Black => [tints[3] == 0.0; 3],
        };
        let Some(overprint) = Overprint::new(kept) else {
            // No component's tint is zero, so the bullet's value is `C_s` for every one of
            // them — Normal, and the same picture the implicit group below would have
            // computed. Nothing is owed and nothing is reported.
            return state.blend;
        };
        if state.blend != BlendMode::Normal {
            self.note(Unsupported::Overprint {
                detail: format!(
                    "§11.7.4.3's implicit non-isolated, non-knockout group for an object \
                     painted under the {:?} blend mode while overprinting is enabled is not \
                     built; the object is painted under that mode without the special \
                     overprinting blend mode",
                    state.blend
                ),
            });
            return state.blend;
        }
        self.list.note_overprinting();
        BlendMode::Overprint(overprint)
    }

    /// §11.7.4.4's first bullet for a combined fill and stroke, where this tree's construction
    /// is not it.
    ///
    /// The clause gives the pair two constructions and the condition between them is
    /// overprinting:
    ///
    /// > If overprinting is enabled (the overprint parameters for both stroking and
    /// > non-stroking operations in the graphics state are true ) and the current stroking and
    /// > nonstroking alpha constants are equal, a non-isolated, non-knockout transparency
    /// > group shall be established. Within the group, the fill and stroke shall be performed
    /// > with an alpha value of 1.0 but with the special overprinting blend mode described in
    /// > 11.7.4.3 , ' Compatibility with opaque overprinting ' . The group results shall then
    /// > be composited with the backdrop, using the originally specified alpha and blend mode.
    ///
    /// **Where the two alpha constants are 1.0 and the blend mode is Normal, that group is the
    /// two commands drawn one after the other**, and [`Interpreter::end_path`] and
    /// [`Interpreter::show_text`] already draw them: a non-isolated group composited onto its
    /// own backdrop at an alpha of 1.0 under Normal returns its elements unchanged, which is
    /// §11.4.4's own cancellation (NOTE 3) and the identity `interpolate` rests on in
    /// `render-cpu`. So nothing is owed there and nothing is reported.
    ///
    /// What is not built is the general case, where the pair's own alpha or mode is not the
    /// identity. `true` from here means the caller must leave the parts as they are and say
    /// so; `false` means the first bullet does not apply and the caller builds the second.
    pub(super) fn combined_overprint(
        &mut self,
        state: &GraphicsState,
        parts: [BlendMode; 2],
        what: &'static str,
    ) -> bool {
        let special = parts
            .iter()
            .any(|blend| matches!(blend, BlendMode::Overprint(_)));
        let both_enabled = state.overprint_stroking && state.overprint_filling;
        // The clause's condition is that the two constants are *equal*, so this is an exact
        // comparison of the numbers the file stated rather than one within a tolerance: a
        // producer that wrote two different constants asked for the second bullet, however
        // close the two are, and NOTE 3 says outright that "the results are discontinuous at
        // the transition between equal and unequal values of the stroking and nonstroking
        // alpha constants".
        #[expect(
            clippy::float_cmp,
            reason = "§11.7.4.4 branches on the two constants being equal, which is a property                       of the numbers the file stated and not of their distance apart"
        )]
        let equal_alphas = state.fill_alpha == state.stroke_alpha;
        // The bullet's own condition, plus the one that makes the two bullets differ at all:
        // with the special mode equal to Normal both constructions composite each part
        // against the same backdrop under the same alpha and mode, so the choice between them
        // is only visible where a component of one part is left to what the other painted.
        if !(special && both_enabled && equal_alphas) {
            return false;
        }
        if state.fill_alpha < 1.0 || state.blend != BlendMode::Normal {
            self.note(Unsupported::Overprint {
                detail: format!(
                    "§11.7.4.4's first bullet asks for {what} to be a non-isolated, \
                     non-knockout group whose parts paint at an alpha of 1.0 under the \
                     special overprinting blend mode, composited at the stated alpha and \
                     blend mode; the parts are painted directly instead"
                ),
            });
        }
        true
    }
}
