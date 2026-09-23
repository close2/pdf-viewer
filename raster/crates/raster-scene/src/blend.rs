//! How a mark combines with what is already there.
//!
//! Three enumerations, all of them transcriptions of a specification rather than design
//! choices of ours, which is why they are real code in a skeleton (`raster/doc/adr/0003`):
//! [`BlendMode`] because ISO 32000-2 §11.3.5 names sixteen modes, [`Compose`] because
//! §11.4.6 needs a second compositing behaviour that a general vector API does not have
//! and §11.7.4.3 a third, and [`FillRule`] because §8.5.3.3 defines two.
//!
//! The functions that *implement* these arrive with M6 and are ours alone: the caller's
//! CPU backend implements the sixteen modes itself rather than using `tiny-skia`'s,
//! because three of `tiny-skia`'s were wrong — one by 113 of 255 — and because sharing an
//! implementation between the two backends would make the cross-backend comparison
//! compare one implementation with itself.

/// One of ISO 32000-2 §11.3.5's sixteen blend modes.
///
/// The clause divides them into the twelve *separable* modes of §11.3.5.2, each defined
/// by a function applied to one colour component at a time, and the four *non-separable*
/// modes of §11.3.5.3, defined by the clause's `Lum`, `ClipColor`, `SetLum` and `SetSat`
/// functions over all three components at once. No per-component formula produces the
/// non-separable four, and a backend that gets one subtly wrong still produces a
/// plausible picture — which is why [`Self::is_separable`] exists and why the
/// sixteen-mode conformance scene is part of M6 rather than a later tidy-up.
///
/// PDF's deprecated `/Compatible` name, which means `Normal`, is resolved by the caller;
/// sixteen is what reaches us.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    /// §11.3.5.2. The backdrop is not used: the result is the source.
    ///
    /// The initial value of the blend mode in §11.6.6's graphics state, and what the
    /// overwhelming majority of commands on a page carry. Fourteen cross-backend
    /// fixtures in the caller's tree carried nothing else, which is how sixteen blend
    /// functions came to have never been compared at all.
    #[default]
    Normal,
    /// §11.3.5.2. Multiplies the backdrop and source; the result is at least as dark as
    /// either.
    Multiply,
    /// §11.3.5.2. Multiplies the complements; the result is at least as light as either.
    Screen,
    /// §11.3.5.2. `Multiply` or `Screen` depending on the backdrop, so the backdrop
    /// decides the contrast.
    Overlay,
    /// §11.3.5.2. The darker of backdrop and source, per component.
    Darken,
    /// §11.3.5.2. The lighter of backdrop and source, per component.
    Lighten,
    /// §11.3.5.2. Brightens the backdrop in proportion to the source.
    ColorDodge,
    /// §11.3.5.2. Darkens the backdrop in proportion to the source.
    ColorBurn,
    /// §11.3.5.2. `Multiply` or `Screen` depending on the source — `Overlay` with the
    /// roles exchanged.
    HardLight,
    /// §11.3.5.2. Darkens or lightens depending on the source, with the clause's own
    /// auxiliary function `D` in the middle band.
    SoftLight,
    /// §11.3.5.2. The absolute difference of backdrop and source.
    Difference,
    /// §11.3.5.2. Excludes rather than differences: light where exactly one is light.
    Exclusion,
    /// §11.3.5.3, non-separable. The source's hue with the backdrop's saturation and
    /// luminosity.
    Hue,
    /// §11.3.5.3, non-separable. The source's saturation with the backdrop's hue and
    /// luminosity.
    Saturation,
    /// §11.3.5.3, non-separable. The source's hue and saturation with the backdrop's
    /// luminosity.
    Color,
    /// §11.3.5.3, non-separable. The source's luminosity with the backdrop's hue and
    /// saturation.
    Luminosity,
}

impl BlendMode {
    /// Every mode ISO 32000-2 §11.3.5 defines, in the clause's own order.
    ///
    /// Exhaustive by construction rather than by good intentions: the conformance scene
    /// of brief section 4.3 is generated from this array, so a mode that exists cannot be a mode the
    /// suite does not draw.
    pub const ALL: [Self; 16] = [
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Overlay,
        Self::Darken,
        Self::Lighten,
        Self::ColorDodge,
        Self::ColorBurn,
        Self::HardLight,
        Self::SoftLight,
        Self::Difference,
        Self::Exclusion,
        Self::Hue,
        Self::Saturation,
        Self::Color,
        Self::Luminosity,
    ];

    /// Whether this mode is one of §11.3.5.2's separable twelve, which act on each
    /// colour component independently.
    ///
    /// The four for which this is false are §11.3.5.3's, and they are the ones a shader
    /// cannot express as a per-component formula.
    #[must_use]
    pub const fn is_separable(self) -> bool {
        !matches!(
            self,
            Self::Hue | Self::Saturation | Self::Color | Self::Luminosity
        )
    }
}

/// Which Porter-Duff compositing operator a mark uses.
///
/// This enumeration is the reason a general 2D vector library cannot be patched into
/// ISO 32000-2 clause 11, and `raster/doc/RENDER_LIBRARY.md` §4.1 is the argument. §11.4.6:
///
/// > In a knockout group, each individual element shall be composited with the group's
/// > initial backdrop rather than with the stack of preceding elements in the group.
///
/// The initial backdrop is transparent, so compositing an element with it yields the
/// element; the group's accumulated result is then replaced by *a fraction* of that, and
/// the fraction is the element's **shape**. For a rasteriser, shape is the coverage the
/// element was drawn with — and a raster of premultiplied samples carries opacity, not
/// shape. Vello's layers composite over the layer's whole *bounding box*, so its
/// `peniko::Compose::Copy` erases a row of pixels outside the shape entirely, and no arrangement
/// of an SVG-shaped API recovers the difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Compose {
    /// Porter-Duff Source-over: the source over the backdrop, weighted by coverage.
    ///
    /// Every ordinary mark on a page, and the initial state.
    #[default]
    SrcOver,
    /// Porter-Duff Source, **modulated by coverage**: an element with 40% coverage
    /// replaces 40% of what was there and leaves the rest.
    ///
    /// §11.4.6's knockout groups, applied per element. The modulation is the whole
    /// point: a plain "replace the destination" operator is the same thing wherever
    /// coverage is 1 and wrong everywhere else, which is why the scene that tests this
    /// has a diagonal edge on purpose — a scene of axis-aligned rectangles would agree
    /// while being wrong.
    Src,
    /// Porter-Duff Destination-Out, weighted by **shape**: the backdrop is scaled by
    /// `1 − shape`, and nothing is deposited.
    ///
    /// The first of §11.4.6's two stages when an element's shape is *not* its coverage
    /// — a nested group, or an element under a soft mask, where §11.6.4.2 gives shape
    /// from geometry alone while §11.6.4.3's mask and §11.6.4.4's constant alpha are
    /// opacity. [`Compose::Src`] reads shape off the alpha a mark is drawn with, which
    /// those elements contradict; this pair states the two quantities apart.
    ///
    /// **The weight is the shape, deliberately not the paint's alpha.** A mark's shape
    /// here is its coverage under its clip — the same quantity the knockout erase pass
    /// uses — so a caller draws the object with every source of opacity removed and
    /// gets exactly §11.6.4.2's shape. Using the alpha instead would repeat the defect
    /// this operator exists to fix.
    ///
    /// Safe where a bounding-box composite is not: at zero coverage it leaves the
    /// destination exactly, so it cannot erase outside the shape.
    DestOut,
    /// Porter-Duff Plus: the source is **added** to the destination, premultiplied.
    ///
    /// §11.4.6's second stage, after [`Compose::DestOut`] has made room for it:
    /// together the pair is `P' = (1 − f) × P + S` in one mark each.
    ///
    /// # This operator is only correct in that pair
    ///
    /// Source-over here is *not* the clause — it weights the backdrop a second time, by
    /// `1 − shape × opacity` where §11.4.6 weights it by `1 − shape` alone, and the two
    /// differ by **32 of 255** at a half-covered pixel under a half-opaque mark. That is
    /// what the pair exists for. But addition alone saturates: `Plus` without the
    /// matching `DestOut` before it drives a premultiplied channel past its alpha, and
    /// no library can tell from one mark whether the other is coming. **The pairing is
    /// the caller's obligation** — it is the one thing in this vocabulary a scene cannot
    /// be refused for getting wrong, and it is stated here rather than discovered.
    ///
    /// At zero coverage it adds nothing, so like [`Compose::DestOut`] it cannot mark
    /// outside the shape.
    Plus,
    /// Porter-Duff Destination-over, weighted by coverage: the backdrop stays where it is
    /// and the source fills in only as much as the backdrop's own alpha leaves uncovered.
    ///
    /// ISO 32000-2 §11.7.4.3's special overprinting blend mode where it keeps **every**
    /// channel of the backdrop. The derivation is [`Compose::DestOverIn`]'s, in the case
    /// where every channel takes `B = C_b`; it is its own operator because that case is
    /// nearly the whole population and costs one fixed-function blend state rather than a
    /// layer (`doc/adr/1295`).
    ///
    /// At zero coverage it deposits nothing, so like [`Compose::DestOut`] it cannot mark
    /// outside the shape.
    DestOver,
    /// Destination-over in the colour channels marked `true` and source-over in the rest,
    /// with the union alpha either way: ISO 32000-2 §11.7.4.3's special overprinting blend
    /// mode, stated channel by channel.
    ///
    /// # What the clause asks
    ///
    /// The mode is one no document can name — ISO 32000-2 §11.7.4.3:
    ///
    /// > It shall not be invoked explicitly; rather, it may be implicitly invoked whenever an
    /// > elementary graphics object is painted while overprinting is enabled (that is, when
    /// > the overprint parameter in the graphics state is true ).
    ///
    /// and its value is not arithmetic over the two colours but a choice between them, per
    /// component:
    ///
    /// > If the overprint mode is 1 (nonzero overprint mode) and the current colour space and
    /// > group colour space are both DeviceCMYK , then process colour components with nonzero
    /// > values shall replace the corresponding component values of the backdrop; components
    /// > with zero values leave the existing backdrop value unchanged.
    ///
    /// Table 146 states the same cell as `B = C_s` where the source tint is nonzero and
    /// `B = C_b` where it is zero, and the paragraph under it, ISO 32000-2 §11.7.4.5, says
    /// the choice is made on additive values like every other blend function's:
    ///
    /// > In reality, however, the special overprinting blend mode (like all blend modes)
    /// > shall treat colour components as additive values; subtractive components shall be
    /// > complemented before and after application of the special blend function.
    ///
    /// A selection commutes with a complement, so the channel that keeps the backdrop's
    /// tint keeps its additive value too, and nothing here complements anything.
    ///
    /// # What that is, in this vocabulary
    ///
    /// §11.3.3's basic compositing formula, with §11.3.7.3's union for the result alpha
    /// `αr = αb + αs − αb·αs`, reads in premultiplied form (`c = α·C`):
    ///
    /// ```text
    /// cr = (1 − αs)·cb + (1 − αb)·cs + αs·αb·B(Cb, Cs)
    /// ```
    ///
    /// — the weighting §11.3.6 describes, the backdrop and source alphas controlling their
    /// own colours and their product controlling the blend function's. Substituting the
    /// clause's two values of `B`:
    ///
    /// - `B = Cb`, a channel the mark leaves alone: `αs·αb·Cb = αs·cb`, and the line is
    ///   `cr = cb + (1 − αb)·cs` — Porter-Duff **destination-over**;
    /// - `B = Cs`, a channel the mark replaces: `αs·αb·Cs = αb·cs`, and the line is
    ///   `cr = cs + (1 − αs)·cb` — Porter-Duff **source-over**, which is Normal.
    ///
    /// `αr` does not depend on `B`, so the alpha is the union in every channel. Nothing in
    /// the selection varies within a mark: the caller decides it on the tints the document
    /// stated (§8.6.7 makes the zero test on "the tint value defined within the PDF file,
    /// before quantisation into a device tint value"), which a shader handed an eight-bit
    /// colour could not repeat, so it travels on the command.
    ///
    /// # How it is drawn
    ///
    /// No single fixed-function blend state states two operators in two channels. Two draws
    /// under write masks could — destination-over into the kept channels first, while the
    /// backdrop's alpha is still there to be read, then source-over into the rest and the
    /// alpha — at two more pipelines per lane for each of the six proper subsets. The
    /// caller counted this operator's population at 0.048% of the marks under the mode, so
    /// a mark under it is drawn into a layer of its own instead and composited once, by
    /// `composite.wgsl`, with the formula above and the selected `B` (`doc/adr/1295`).
    /// [`Compose::keeping`] sends the two uniform selections to [`Compose::DestOver`] and
    /// [`Compose::SrcOver`], which need no layer.
    DestOverIn([bool; 3]),
}

impl Compose {
    /// ISO 32000-2 §11.7.4.3's special overprinting blend mode for a mark that keeps the
    /// backdrop in exactly the channels marked `true`: [`Compose::DestOverIn`], or one of
    /// the two operators its uniform cases are.
    ///
    /// Keeping every channel is [`Compose::DestOver`] and keeping none is
    /// [`Compose::SrcOver`] — the same arithmetic the per-channel operator would compute,
    /// reached without a layer. The derivation is on [`Compose::DestOverIn`].
    #[must_use]
    pub const fn keeping(kept: [bool; 3]) -> Self {
        match kept {
            [true, true, true] => Self::DestOver,
            [false, false, false] => Self::SrcOver,
            _ => Self::DestOverIn(kept),
        }
    }

    /// Whether this is §11.7.4.3's special overprinting blend mode: [`Compose::DestOver`]
    /// or [`Compose::DestOverIn`].
    ///
    /// The builder asks it to refuse the positions where the mode would meet a second
    /// compositing rule it cannot be combined with — see
    /// [`SceneError::OverprintComposeUnsupported`](crate::error::SceneError::OverprintComposeUnsupported).
    #[must_use]
    pub const fn overprints(self) -> bool {
        matches!(self, Self::DestOver | Self::DestOverIn(_))
    }
}

/// Which of ISO 32000-2 §8.5.3.3's two rules decides the inside of a path.
///
/// Both appear on real pages, including the case that catches an implementation out: a
/// nested subpath wound in the *same* direction as its parent, where the two rules
/// disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FillRule {
    /// §8.5.3.3.2, the nonzero winding number rule. The `f` operator's rule, and the
    /// default.
    #[default]
    NonZero,
    /// §8.5.3.3.3, the even-odd rule. The `f*` operator's.
    EvenOdd,
}

#[cfg(test)]
mod tests {
    use super::{BlendMode, Compose, FillRule};

    /// ISO 32000-2 §11.3.5 names sixteen modes; [`BlendMode::ALL`] is what the
    /// conformance scene is generated from, so a missing entry would silently shrink the
    /// suite rather than fail a build.
    #[test]
    fn all_holds_sixteen_distinct_modes() {
        let mut seen = BlendMode::ALL.to_vec();
        seen.sort_by_key(|mode| format!("{mode:?}"));
        seen.dedup();
        assert_eq!(seen.len(), 16, "ALL must list sixteen distinct modes");
    }

    /// §11.3.5.3 defines exactly four non-separable modes. A fifth, or a third, means
    /// the shader that dispatches on this predicate is dispatching on the wrong set.
    #[test]
    fn exactly_four_modes_are_non_separable() {
        let non_separable = BlendMode::ALL
            .iter()
            .filter(|mode| !mode.is_separable())
            .count();
        assert_eq!(non_separable, 4);
    }

    /// ISO 32000-2 §11.7.4.3's selection has two uniform cases, and each is an operator
    /// this vocabulary already has: keeping every channel is destination-over and keeping
    /// none is `B = C_s` everywhere, which is Normal's source-over. Only a proper subset
    /// needs the per-channel operator, and it carries exactly the channels it was given.
    #[test]
    fn keeping_sends_the_uniform_selections_to_their_operators() {
        assert_eq!(Compose::keeping([true; 3]), Compose::DestOver);
        assert_eq!(Compose::keeping([false; 3]), Compose::SrcOver);
        for bits in 1_u8..7 {
            let kept = [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0];
            assert_eq!(Compose::keeping(kept), Compose::DestOverIn(kept));
            assert!(Compose::keeping(kept).overprints());
        }
        assert!(!Compose::SrcOver.overprints());
        assert!(Compose::DestOver.overprints());
    }

    /// §11.6.6 initialises the blend mode to Normal, and an ordinary mark composites
    /// Source-over under the nonzero rule. A `Default` that drifted from the clause
    /// would be wrong in a way no single test of a later stage would localise.
    #[test]
    fn defaults_are_the_initial_graphics_state() {
        assert_eq!(BlendMode::default(), BlendMode::Normal);
        assert_eq!(Compose::default(), Compose::SrcOver);
        assert_eq!(FillRule::default(), FillRule::NonZero);
    }
}
