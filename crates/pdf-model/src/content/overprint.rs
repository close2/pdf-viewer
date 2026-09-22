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
//! colour space shall be converted to its alternate colour space".
//!
//! **Reverting is what puts such a space in the first row rather than outside the table.**
//! §11.7.4.3's NOTE 2 makes the current colour space of a space "that revert[s] to [its]
//! alternate colour space" *be* that alternate, so a `Separation` or `DeviceN` over a
//! `DeviceCMYK` alternate meets the first bullet on the four components the alternate
//! receives. §8.6.7's own EXAMPLE requires it: under `OP true` and `OPM 1` that clause calls
//! `0.2 0.3 0.0 1.0 k` equivalent to `0.2 0.3 1.0 scn` in a `DeviceN` whose alternate is
//! `DeviceCMYK`, and two operators a clause calls equivalent may not take different blend
//! functions. `content::colour::cmyk_tints` is where that is answered (ADR 1241, amending
//! ADR 1157).
//!
//! ADR 1157 has the reading and ADR 1158 what it costs; ADR 0028 is what it supersedes.
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

use pdf_render::{BlendMode, Command, Overprint};

use crate::colour::{Compositing, Half};

use super::{GraphicsState, Interpreter, KnockoutKind};

/// What ISO 32000-2 §11.7.4.4's first bullet asks of a combined fill and stroke.
///
/// Three answers rather than two, because the bullet's group is sometimes the two commands
/// that are already there: see [`Interpreter::combined_overprint`], which decides between
/// them, and ADR 1170.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FirstBullet {
    /// The bullet does not apply, and §11.7.4.4's second is the construction owed.
    No,
    /// It applies, and the two commands as they stand are what it asks for — exactly, where
    /// its group is the identity, and reported where a knockout group above makes the group
    /// unstatable.
    AsPainted,
    /// It applies and its group is built, around parts painted with §11.6.4.4's two constants
    /// at 1.0.
    Group,
}

/// Whether §11.7.4.3's last paragraph asks for an implicit group around an object whose parts
/// are painted under `parts`.
///
/// > If the current blend mode is any mode other than Normal when invoking this special
/// > overprinting blend mode, the object being painted shall be implicitly treated as if it
/// > were defined in a non-isolated, non-knockout transparency group, and painted using the
/// > this special blend mode. The group's results shall then be painted using the current
/// > blend mode in the graphics state.
///
/// The special mode being *invoked* is what a part carrying it says, and
/// [`Interpreter::special_overprint`] hands one out only where the group can be built, so a
/// caller reading this needs no second condition.
///
/// `parts` is by value and this is `#[inline]` so that a caller's own test of `state.blend` is
/// all a page that states no overprint pays. Measured under callgrind on page 101 of ISO 32000-2
/// interpreted fifty times, against the same build with §11.7.4's decisions and constructions
/// planted away: taking the pair by **slice**, so that the array is built and the call made once
/// per path-painting operator before anything is tested, cost **+4 879 021 instructions,
/// 0.391%** of one interpretation. By value behind the caller's test it is part of the
/// **+513 947, 0.041%** that everything but the per-glyph wrap point costs — which is ADR 1158's
/// own order of magnitude for the two `overprint_blend` calls beside it.
/// `Interpreter::wrap_in_the_implicit_group` carries the other figure and the whole.
#[inline]
pub(super) fn implicit_group_owed(state: &GraphicsState, parts: [BlendMode; 2]) -> bool {
    state.blend != BlendMode::Normal
        && parts
            .iter()
            .any(|blend| matches!(blend, BlendMode::Overprint(_)))
}

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
    /// - The current colour space is `DeviceCMYK` — stated as such, or a `Separation` or
    ///   `DeviceN` reverting to a `DeviceCMYK` alternate, which §11.7.4.3's NOTE 2 makes the
    ///   same thing. That is Table 146's first row and §8.6.7's "painting operations that use
    ///   the current colour in the graphics state when the current colour space is
    ///   DeviceCMYK"; `content::colour::cmyk_tints` is where the two routes meet. A pattern is
    ///   not such a colour: §8.6.7 excludes shadings outright, and a tiling pattern's cell
    ///   paints its own.
    /// - At least one of the four tints is zero, which is the only way the bullet's value
    ///   differs from `C_s`.
    ///
    /// # The fourth condition is asked of four tints and not of this raster's three
    ///
    /// §11.4.7's page pair is two interpretations of one page, and
    /// [`pdf_render::DisplayList::geometry_digest`] refuses a pair whose commands differ in
    /// structure — a blend mode's discriminant among them — because the halves are resolved
    /// per pixel and a command in one and not the other would be converted against a shape
    /// that never drew it. A colour whose only zero tint is black keeps all three channels of
    /// the black raster and none of the chromatic one, so a question asked of *this half's*
    /// three tints answers differently in the two runs. Asked of the four the clause decides,
    /// it answers once, and the mode that keeps nothing is `pdf_render::Overprint`'s empty
    /// set rather than Normal. ADR 1169 is what that cost before it was asked this way: the
    /// pair failed its own guard and the page fell back to the device's three components,
    /// silently.
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
        if !tints.contains(&0.0) {
            // The bullet's value is `C_s` for every one of the group's four components —
            // Normal, and the same picture the implicit group below would have computed.
            // Nothing is owed and nothing is reported.
            return state.blend;
        }
        self.list.note_overprinting();
        // Which of the raster's three channels this half carries, and therefore which of the
        // four tints decides each of them (`crate::colour::Half`).
        BlendMode::Overprint(Overprint::new(match half {
            Half::Chromatic => [tints[0] == 0.0, tints[1] == 0.0, tints[2] == 0.0],
            Half::Black => [tints[3] == 0.0; 3],
        }))
    }

    /// §11.7.4.3's last paragraph, around the commands from `mark` on.
    ///
    /// > the object being painted shall be implicitly treated as if it were defined in a
    /// > non-isolated, non-knockout transparency group, and painted using the this special
    /// > blend mode. The group's results shall then be painted using the current blend mode in
    /// > the graphics state.
    ///
    /// The object keeps its own alpha and soft mask inside the group, because the clause moves
    /// the *blend mode* and says nothing of either — and its NOTE 3 is what checks that
    /// reading: "[i]t is not necessary to create such an implicit transparency group if the
    /// current blend mode is Normal ; simply substituting the special blend mode while painting
    /// the object produces equivalent results". A non-isolated group composited onto its own
    /// backdrop at an alpha of 1.0 under Normal returns its elements unchanged (§11.4.4 NOTE
    /// 3), so with the object's constants left on the object the equivalence NOTE 3 states is
    /// exact; with them moved to the group it is not.
    pub(super) fn implicit_overprint_group(&mut self, state: &GraphicsState, mark: usize) {
        self.non_isolated_group(mark, 1.0, state.blend);
    }

    /// §11.7.4.4's first bullet, around the commands from `mark` on.
    ///
    /// The parts were painted with §11.6.4.4's two constants at 1.0, which is what the bullet
    /// asks for, and the group composites "with the backdrop, using the originally specified
    /// alpha and blend mode" — `state`'s, the two constants being equal by the bullet's own
    /// condition. The soft mask stays on the parts: the bullet's nouns are the *alpha
    /// constants*, and §11.7.4.4's second bullet spends them the same way.
    pub(super) fn first_bullet_group(&mut self, state: &GraphicsState, mark: usize) {
        self.non_isolated_group(mark, state.fill_alpha, state.blend);
    }

    /// The non-isolated, non-knockout group both of §11.7.4's implicit constructions are.
    fn non_isolated_group(&mut self, mark: usize, alpha: f32, blend: BlendMode) {
        let commands = self.list.split_off_commands(mark);
        if commands.is_empty() {
            return;
        }
        self.draw(Command::Group {
            commands,
            alpha,
            clip: None,
            mask: None,
            blend,
            // §11.4.4's own model, which is what both clauses name. `render-cpu` runs the
            // elements a second time on transparency for Table 140's group alpha and performs
            // NOTE 3's removal (`blend::remove_backdrop`); the two backends that cannot refuse
            // the whole list by name, because it overprints (ADR 1158).
            //
            // §11.4.6's NOTE 6 decides the one position where that is not what is stated. This
            // group is an element of whatever paints the object, so as a direct element of a
            // knockout group it takes that group's initial backdrop: where that backdrop is
            // transparent the clause's non-isolated group **is** §11.4.5's, exactly (§11.4.4
            // NOTE 3, the backdrop composited in and removed again being nothing either way);
            // where it is not, the enclosing knockout group hands each element a private clone
            // of its initial backdrop (ADR 1256), which is what a command stating `false` is
            // seeded from. So the flag says which backdrop the note gives this group rather
            // than refusing the position. ADR 1170, ADR 1265.
            isolated: self.enclosing_knockout == Some(KnockoutKind::Isolated),
            knockout: false,
            // Stated rather than asked: this group carries no clip of its own, and §8.5.4's
            // intersection at the blit is the only thing the flag decides. A round that gives
            // it one owes the question (ADR 0554).
            alpha_is_shape: false,
            blending: None,
        });
    }

    /// §11.7.4.4's first bullet for a combined fill and stroke, and which of its three shapes
    /// this pair takes.
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
    /// Everywhere else the group is built ([`Interpreter::first_bullet_group`]), with the
    /// parts painted at an alpha constant of 1.0 — which is the caller's to arrange, because
    /// it happens before the two commands exist. A pair that is a direct element of a knockout
    /// group is built there too: §11.4.6's NOTE 6 decides which backdrop that group's element
    /// composites onto, and [`Interpreter::non_isolated_group`] states it rather than refusing
    /// the position (ADR 1265).
    pub(super) fn combined_overprint(state: &GraphicsState, parts: [BlendMode; 2]) -> FirstBullet {
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
            return FirstBullet::No;
        }
        if state.fill_alpha >= 1.0 && state.blend == BlendMode::Normal {
            return FirstBullet::AsPainted;
        }
        FirstBullet::Group
    }
}
