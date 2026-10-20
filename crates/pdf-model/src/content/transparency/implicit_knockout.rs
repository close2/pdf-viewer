//! §11.7.4.4 and §9.3.8: the knockout group the standard makes out of parts the file never grouped,
//! and the one blend mode its elements may carry to the group's `Do` (ADR 2080).

use pdf_render::{BlendMode, Command, Paint};

use crate::image::ShapeMasks;

use crate::content::KnockoutKind;

use super::commands::{any_command, command_blends};
use super::knockout::{knockout_elements, stated_elements};
use super::readings::{AlphaSource, AlphaSourcesSeen};

/// A knockout group the specification makes out of parts the file never grouped, as the
/// display list states it (ISO 32000-2 §11.7.4.4, §9.3.8, §11.6.2).
///
/// The two clauses say the same two things about such a group. Its parts are painted
/// "with their respective prevailing alpha constants and the prevailing blend mode"
/// (§11.7.4.4), and its results are composited onto the backdrop "using an alpha value of
/// 1.0 and the Normal blend mode" — §9.3.8's "using the Normal blend mode and alpha and
/// soft mask values of 1.0". So the group carries no constant, no mask and no clip of its
/// own; what varies between one such group and the next is inside it, and this is the
/// answer [`implicit_knockout_group`] gives for [`Command::Group`]'s three fields that do.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::content) struct ImplicitKnockout {
    /// The group's elements, each with the shape §11.4.6 weights by where a backend cannot
    /// read it off the coverage (`Command::Shaped`).
    pub(in crate::content) elements: Vec<Command>,
    /// The blend mode the group composites onto its backdrop with — `Normal`, as the
    /// clauses state, or the one mode every element carried where that is the same
    /// picture and the elements are stripped of it (see [`blend_at_the_do`]).
    pub(in crate::content) blend: BlendMode,
    /// `Command::Group`'s `isolated`: `true` where the elements are drawn on transparency and
    /// `false` where §11.4.6's initial backdrop has to be the group's own.
    pub(in crate::content) isolated: bool,
}

/// §11.4.6's construction for a knockout group the standard makes out of `commands`, or
/// `None` where none of the three below can draw it (ISO 32000-2 §11.7.4.4, §9.3.8).
///
/// Three constructions, tried in order, and each is exact rather than a substitute:
///
/// - **Nothing blends.** The elements are drawn on transparency and the group composited
///   over the page — §11.4.5's isolated group standing in for the non-isolated one the
///   clauses name, which §11.4.4's NOTE 3 makes exact wherever every element paints Normal:
///   the backdrop is composited in and removed again, so it cancels
///   ([`transparent_knockout_elements`]).
/// - **Every element blends the same way, and the mode commutes with the weighted
///   average.** The mode moves from the elements to the group's `Do`, and the elements are
///   drawn Normal on transparency ([`blend_at_the_do`] has the derivation and the
///   condition). Every backend draws this, which is what the corpus's two witnesses
///   (`issue17215.pdf`'s `b` under `Difference`, `issue14438.pdf`'s `B` under `Multiply`)
///   were owed.
/// - **Otherwise, §11.4.6's own backdrop.** Every element states its shape and the group
///   goes to the backends with `isolated: false` beside `knockout: true` — ADR 0327's
///   construction, which retains the initial backdrop beside the accumulation and is drawn
///   by the oracle alone. Not inside a knockout group: §11.4.6's NOTE 6 gives a nested
///   group the *outer* group's initial backdrop, "not the immediate backdrop of the inner
///   group", and this command is seeded from the immediate one wherever the enclosing
///   knockout group is not itself drawn against a retained initial backdrop.
///
/// # An element that is itself a non-isolated group (§11.4.6's NOTE 6)
///
/// > When a non-isolated group is nested within a knockout group, the initial backdrop of
/// > the inner group is the same as that of the outer group; it is not the immediate
/// > backdrop of the inner group.
///
/// The third construction satisfies that by itself, and it takes no flag to do it. §11.4.6
/// composites "each individual element ... with the group's initial backdrop rather than
/// with the stack of preceding elements in the group", so for a *direct* element of a
/// knockout group the immediate backdrop the note contrasts with **is** the group's initial
/// backdrop: a backend that composites each element against a private copy of that backdrop
/// hands a nested non-isolated group exactly the backdrop the note asks for, which is the
/// note's own "consequence of the group compositing formulas when b = 0". So the third
/// construction admits such an element and the first two do not — they draw the elements on
/// transparency, where an inner group would seed from the accumulation instead. ADR 1256.
///
/// Two routes arrive with one: §11.7.4's implicit groups (ADR 1170), and a form `XObject`
/// inside a Type 3 glyph.
///
/// The shape each element states is §11.6.4.3's `/AIS` reading's — `None` where the content
/// painted under both readings *and the flag reinterprets something in these parts*
/// ([`AlphaSourcesSeen::settled_over`]), or where an element's shape cannot be stated at all
/// ([`super::shape::stated_shape`]). Under `/AIS true` an element's drawn alpha *is* its shape
/// (`shape::shape_the_alpha_already_is`), so every element states the pair; under `/AIS false`
/// an element whose shape is its coverage is drawn bare and the rest state it
/// ([`knockout_elements`]).
///
/// **All three constructions are here, and the first states a masked element's shape**, as
/// [`knockout_elements`] does for a form's group (ADR 0234): without them every `B` under a
/// blend mode is drawn flat, its stroke composited over its own fill — the double border
/// §11.7.4.4's NOTE 2 exists to prevent (ADR 1000).
pub(in crate::content) fn implicit_knockout_group(
    commands: &[Command],
    seen: AlphaSourcesSeen,
    enclosing: Option<KnockoutKind>,
    shape_masks: &ShapeMasks,
    one_colour: &mut OneColour,
) -> Option<ImplicitKnockout> {
    let alpha = seen.settled_over(commands)?;
    // §11.4.6's NOTE 6 first, because where it applies it decides everything: this group is a
    // direct element of `enclosing`, so its initial backdrop is that group's — "the initial
    // backdrop of the inner group is the same as that of the outer group" — and an isolated
    // knockout group's is transparent. Each part is then composited with a transparent backdrop,
    // where §11.3.6 leaves a blend mode nothing to do ("[a]n alpha value of αs = 0.0 or αb = 0.0
    // results in no blend mode effect"), and a part that is itself a non-isolated group is handed
    // the same transparent backdrop by the same note. So the parts are drawn Normal, any nested
    // group isolated, on transparency: exact whatever the parts blend with (ADR 1301).
    if enclosing == Some(KnockoutKind::Isolated) {
        let parts = commands
            .iter()
            .map(on_a_transparent_backdrop)
            .collect::<Option<Vec<_>>>()?;
        return Some(ImplicitKnockout {
            elements: transparent_knockout_elements(&parts, alpha, shape_masks)?,
            blend: BlendMode::Normal,
            isolated: true,
        });
    }
    // §11.4.6's NOTE 6 decides which of the three constructions may take an element that is
    // itself a non-isolated group: such an element's own initial backdrop is *this* group's
    // initial backdrop, so only a construction that composites each element against that
    // backdrop can hand it over. The two below that draw the elements on transparency cannot
    // — there an element's backdrop is the accumulation, and §11.4.4's NOTE 3 cancellation
    // that makes them exact is a statement about the group's own result rather than about
    // what a nested group is given. ADR 1256.
    let nested_non_isolated = commands.iter().any(is_a_non_isolated_group);
    if !nested_non_isolated {
        if !any_command(commands, &command_blends) {
            return Some(ImplicitKnockout {
                elements: transparent_knockout_elements(commands, alpha, shape_masks)?,
                blend: BlendMode::Normal,
                isolated: true,
            });
        }
        if let Some(blend) = blend_at_the_do(commands, one_colour) {
            let stripped = commands
                .iter()
                .map(without_blend)
                .collect::<Option<Vec<_>>>()?;
            return Some(ImplicitKnockout {
                elements: transparent_knockout_elements(&stripped, alpha, shape_masks)?,
                blend,
                isolated: true,
            });
        }
    }
    // §11.4.6's NOTE 6 from the other side: this group is a direct element of `enclosing`, so
    // the backdrop the clause hands it is *that* group's initial backdrop. The enclosing group
    // is non-isolated here — the isolated one was answered above — and ADR 1256's construction
    // gives each of its elements a private clone of exactly that backdrop, which is what the
    // command below is seeded from. ADR 1265.
    Some(ImplicitKnockout {
        elements: stated_elements(commands, alpha, shape_masks)?,
        blend: BlendMode::Normal,
        isolated: false,
    })
}

/// A knockout group's elements for a group drawn on transparency, under either reading of
/// §11.6.4.3's `/AIS`.
///
/// Under `/AIS false` an element whose shape is its coverage draws bare and every other one
/// states its shape beside itself; under `/AIS true` there is no bare draw, because the
/// drawn alpha is the shape and a bare knockout draw would read it as opacity
/// ([`super::shape::element_shape_is_coverage`]).
pub(super) fn transparent_knockout_elements(
    commands: &[Command],
    alpha: AlphaSource,
    shape_masks: &ShapeMasks,
) -> Option<Vec<Command>> {
    match alpha {
        AlphaSource::Opacity => knockout_elements(commands, alpha, shape_masks),
        AlphaSource::Shape => stated_elements(commands, alpha, shape_masks),
    }
}

/// The one blend mode every element of a knockout group carries, where compositing the
/// group with it is the same picture as compositing each element with it — or `None`.
///
/// # The derivation
///
/// §11.4.6 composites each element `Eᵢ` with the group's initial backdrop `B` under its
/// own blend function `M`, and takes "a weighted average of this result with the object's
/// immediate backdrop, using the source shape as the weighting factor". For two elements
/// over `B` (alpha `α_b`, colour `C_b`), with `wᵢ` the weight the second stage leaves each
/// — `(1 − f₂) × α₁` for the first and `f₂ × q₂` for the second — the accumulation is, in
/// premultiplied form,
///
/// ```text
/// P = (1 − w₁ − w₂) × α_b × C_b + (1 − α_b) × (w₁ C₁ + w₂ C₂)
///                                 + α_b × (w₁ M(C_b, C₁) + w₂ M(C_b, C₂))
/// ```
///
/// Drawing the elements Normal on transparency accumulates `K = w₁ C₁ + w₂ C₂` at alpha
/// `w₁ + w₂`, and compositing *that* onto `B` under `M` (§11.3.3) gives the same first two
/// terms and `α_b × (w₁ + w₂) × M(C_b, K / (w₁ + w₂))` for the third. The two agree exactly
/// where `M` is affine in its source argument for a fixed backdrop — Multiply, Screen,
/// Overlay and Exclusion are, per component, and Normal trivially — or where `C₁ = C₂`, in
/// which case `K / (w₁ + w₂)` *is* that colour whatever `M` does to it. At every *point* the
/// shapes are 0 or 1 (§11.6.4.2) and one weight vanishes, so the two agree everywhere the
/// standard defines a value; the condition is about the fractional pixel a rasteriser makes
/// of an edge, and it is held exactly rather than allowed as anti-aliasing residue.
///
/// # Two more cases the same line settles, and a group with alpha at its `Do`
///
/// **An element of zero opacity has `wᵢ = 0` everywhere** — §11.6.4.4's constant is a
/// factor of its opacity, and under `/AIS true` of its shape — so it contributes no colour
/// to `K` and its own mode is applied to nothing; it is still an element of the group, and
/// under the opacity reading it still knocks out what is under it (§11.4.6's NOTE 5: "\[a]
/// shape value of 1.0 (inside) yields the colour and opacity that result from compositing
/// the object with the initial backdrop", which for opacity 0 is the backdrop). So only the
/// elements that carry colour have to share the mode, and **where exactly one does**,
/// `K / Σwᵢ` is that element's colour at every pixel and *any* mode moves, §11.3.5.3's
/// non-separable ones included. `issue18032.pdf`'s knockout group is that shape: a nested
/// group holding a shading, invoked under `/BM /Color`, and a nested group invoked at `ca 0`
/// cutting a hole in it.
///
/// **A nested group may be an element** where it is isolated: its own elements are then
/// drawn on §11.4.5's transparency whichever construction this group takes, so its colour
/// at a pixel is the same `Cᵢ` in both, and the mode this move takes off it is the one at
/// its own `Do`. A non-isolated one is refused — under §11.4.6's own backdrop it is seeded
/// from that backdrop and on transparency from nothing, which is a different picture.
///
/// **And a constant alpha `w_g` and a mask at the group's own `Do`** (a form's, never an
/// implicit group's) pass through: §11.4.4's final composite is `(1 − w_g α_K) B + w_g × (…)`
/// on both sides of the equality, the same collapse ADR 0237 derived, so the condition is
/// unchanged. The mode at that `Do` has to be Normal — the caller's condition — because the
/// cancellation is the Normal blend function's.
pub(super) fn blend_at_the_do(
    commands: &[Command],
    one_colour: &mut OneColour,
) -> Option<BlendMode> {
    let mut shared = None;
    let mut coloured = Vec::with_capacity(commands.len());
    for command in commands {
        if !carries_colour(command)? {
            continue;
        }
        coloured.push(command);
        match shared {
            None => shared = Some(command.blend()),
            Some(mode) if mode == command.blend() => {}
            Some(_) => return None,
        }
    }
    let blend = shared?;
    (affine_in_the_source(blend)
        || coloured.len() == 1
        || (one_solid_colour(&coloured) && one_colour.admits()))
    .then_some(blend)
}

/// Whether [`blend_at_the_do`] may rest on `C₁ = C₂`, and how many times it has.
///
/// The derivation's equality is between colours in the group's colour space, and a group
/// compositing in a press is one content stream run once per plane (§11.4.7, ADR 0272): the
/// chromatic run compares the three components it carries and the black run the one it carries.
/// So two parts stating `0 0 0 0.3 k` and `0 0 0 0.7 k` are one colour to the first run and two
/// to the second, the runs take different constructions, and the pair no longer pairs.
/// `Interpreter::black_half` counts the admissions across a pair's runs and, where the runs
/// parted after one, runs both again with the admission refused — so each half takes the same
/// construction, one the derivation holds for whatever the colours are (ADR 1657).
#[derive(Debug, Default)]
pub(in crate::content) struct OneColour {
    /// Whether the admission is refused, which only a pair's second attempt sets.
    pub(super) refused: bool,
    /// How many times the admission has been made in this interpretation.
    pub(super) admitted: usize,
}

impl OneColour {
    /// Whether one solid colour admits the move here, counting it where it does.
    fn admits(&mut self) -> bool {
        if self.refused {
            return false;
        }
        self.admitted = self.admitted.saturating_add(1);
        true
    }
}

/// Whether an element of a knockout group can contribute colour to the group's accumulation
/// — `Some(false)` for one whose §11.4.6 weight is zero at every pixel — or `None` for an
/// element [`blend_at_the_do`]'s argument does not cover.
///
/// Zero everywhere is a *constant* of zero: a solid paint's alpha, an image's or a nested
/// group's constant. A soft mask alone is not, because it is zero only where it is zero. A
/// shading's constant is folded into its colours (`Shading::with_alpha`) and is read as
/// carrying colour, which errs towards refusing the move.
fn carries_colour(command: &Command) -> Option<bool> {
    Some(match command {
        Command::Fill { paint, .. } | Command::Stroke { paint, .. } => match paint {
            Paint::Solid(colour) => colour.a > 0.0,
            Paint::Shading(_) => true,
            // `Paint` is non-exhaustive: a paint whose opacity is unknown refuses the move.
            _ => return None,
        },
        Command::Image { alpha, .. }
        | Command::Group {
            alpha,
            isolated: true,
            ..
        } => *alpha > 0.0,
        _ => return None,
    })
}

/// Whether §11.3.5.2's blend function is affine in its source component for a fixed
/// backdrop component, which is what lets a weighted average of sources pass through it.
///
/// `Multiply` is `c_b × c_s`; `Screen` is `c_b + c_s − c_b × c_s`; `Overlay` is
/// `HardLight(c_s, c_b)`, whose branch is on the *backdrop* and whose arms are those two;
/// `Exclusion` is `c_b + c_s − 2 × c_b × c_s`. `Darken`, `Lighten`, `ColorDodge`,
/// `ColorBurn`, `HardLight`, `SoftLight` and `Difference` each branch, divide or take a
/// magnitude on `c_s`, and §11.3.5.3's four are not per-component at all.
const fn affine_in_the_source(blend: BlendMode) -> bool {
    matches!(
        blend,
        BlendMode::Normal
            | BlendMode::Multiply
            | BlendMode::Screen
            | BlendMode::Overlay
            | BlendMode::Exclusion
    )
}

/// Whether every element is a fill or stroke of one solid colour, alpha aside.
///
/// The colour is compared bit for bit: the parts of one `B` come from the same `rg` and
/// `RG` operands, and a producer that meant two colours to be the same wrote the same
/// numbers.
fn one_solid_colour(commands: &[&Command]) -> bool {
    let mut seen: Option<[u32; 3]> = None;
    commands.iter().all(|command| {
        let (Command::Fill { paint, .. } | Command::Stroke { paint, .. }) = command else {
            return false;
        };
        let Paint::Solid(colour) = paint else {
            return false;
        };
        let rgb = [colour.r.to_bits(), colour.g.to_bits(), colour.b.to_bits()];
        match seen {
            None => {
                seen = Some(rgb);
                true
            }
            Some(first) => first == rgb,
        }
    })
}

/// An element with its blend mode taken off, for [`blend_at_the_do`]'s group to carry
/// instead — or `None` for anything that function does not admit.
///
/// A nested group loses the mode at its own `Do` and nothing inside it: its elements'
/// modes act on the transparency it is built on, which the move leaves as it was.
pub(super) fn without_blend(command: &Command) -> Option<Command> {
    let mut stripped = command.clone();
    match &mut stripped {
        Command::Fill { blend, .. }
        | Command::Stroke { blend, .. }
        | Command::Image { blend, .. }
        | Command::Group {
            blend,
            isolated: true,
            ..
        } => {
            *blend = BlendMode::Normal;
        }
        _ => return None,
    }
    Some(stripped)
}

/// Whether an element is a non-isolated group — itself, or as the object of a stated pair
/// (`seal`) — which is §11.4.6's NOTE 6's question about what the element composites with.
pub(super) fn is_a_non_isolated_group(command: &Command) -> bool {
    match command {
        Command::Group { isolated, .. } => !*isolated,
        Command::Shaped { object, .. } => is_a_non_isolated_group(object),
        _ => false,
    }
}

/// A part of a knockout group whose initial backdrop is transparent, as §11.3.6 makes it there:
/// its blend mode has no effect, so it is Normal, and a group among the parts is isolated,
/// because §11.4.6's NOTE 6 hands a non-isolated one the same transparent backdrop. `None` for a
/// command of a kind this crate does not know. See [`implicit_knockout_group`].
fn on_a_transparent_backdrop(command: &Command) -> Option<Command> {
    let mut part = command.clone();
    match &mut part {
        Command::Fill { blend, .. }
        | Command::Stroke { blend, .. }
        | Command::Image { blend, .. } => *blend = BlendMode::Normal,
        Command::Group {
            blend, isolated, ..
        } => {
            *blend = BlendMode::Normal;
            *isolated = true;
        }
        Command::Shaped { object, .. } => **object = on_a_transparent_backdrop(object)?,
        _ => return None,
    }
    Some(part)
}
