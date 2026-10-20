//! Clause 11: transparency groups, knockout, soft masks and blending colour spaces.
//!
//! Both halves of this tree's §11 story are here — the constructions the display list can
//! carry (§11.4.5's isolation, §11.4.6's knockout, §11.6.5's soft masks) and the analysis
//! that decides, per group, whether drawing or reporting is the honest answer.

use pdf_render::{BlendMode, Command};
use pdf_syntax::{Dictionary, Object};

use crate::colour::Compositing;

use super::reader::NestedContent;
use super::{GraphicsState, Interpreter, KnockoutKind};

mod blending_space;
mod commands;
mod departures;
mod group_runs;
mod implicit_knockout;
mod into_parent;
mod knockout;
mod press;
mod readings;
mod shape;
mod soft_mask;
mod transfer;

pub(super) use blending_space::Departure;
pub(super) use blending_space::page_blending_space;
pub(super) use blending_space::page_own_space;
use blending_space::{change_is_visible, group_blending};
pub(super) use commands::command_composites;
pub(super) use commands::outline_bounds;
pub(super) use commands::{any_command, command_blends};
use departures::GroupDrawn;
pub(super) use implicit_knockout::OneColour;
pub(super) use implicit_knockout::implicit_knockout_group;
pub(super) use into_parent::ComposedOut;
use knockout::{KnockoutConstruction, knockout_construction};
pub(super) use press::BeyondPress;
pub(super) use press::PagePress;
pub(super) use press::page_cmyk_group;
pub(super) use press::page_press;
pub(super) use readings::AlphaSourcesSeen;
pub(super) use readings::Readings;
pub(super) use readings::unseal_list;
pub(super) use shape::group_alpha_is_shape;
pub(super) use transfer::Painted;

/// What a form `XObject`'s `/Group` asks for (ISO 32000-2 §11.6.6 Table 145).
///
/// Only the three entries that change what is drawn. `/S` is not carried because a
/// dictionary whose subtype is not `/Transparency` never becomes one of these.
#[derive(Debug, Clone)]
pub(super) struct TransparencyGroup {
    /// `/I`: whether the elements are composited onto a transparent initial backdrop
    /// (§11.4.5) rather than onto the group's backdrop.
    isolated: bool,
    /// `/K`: whether each element is composited with the initial backdrop rather than with
    /// the elements below it (§11.4.6).
    knockout: bool,
    /// `/CS`: the group's blending colour space, unresolved, or `Null` where absent.
    colour_space: Object,
}

/// What a transparency group's content is, for the runs [`Interpreter::run_transparency_group`]
/// makes of it.
///
/// A group is run more than once where it composites in a space of its own or gives one up, so
/// its content is held as something that can be run again rather than as operators already
/// read. Two of the three are ISO 32000-2 §8.10.4's imported page, and the reason there are two
/// is that §8.10.4.1 and §11.4.7 each hand that page a group of its own — see
/// [`Interpreter::run_group_body`].
pub(super) enum GroupBody<'b, 'a> {
    /// A content stream of the document in force: a form's, an annotation appearance's.
    Stream(&'b NestedContent),
    /// §8.10.4's imported page, run as §11.4.7's group under its own page `/Group`, inside the
    /// proxy's group where the proxy states one.
    ImportedPage(&'b super::xobject::ImportedPage<'a>),
    /// That page's content and annotations, which are what its page group holds.
    ImportedContent(&'b super::xobject::ImportedPage<'a>),
}

/// What one run of a transparency group's content produced.
///
/// The answer [`Interpreter::group_commands`] hands back: the elements, the conversion out of
/// a space of the group's own where one was built, which of §11.6.4.3's readings the elements
/// painted under, and whether the group was drawn in the space its `/CS` names — a pair that
/// held together, a `DeviceGray` group run under [`Compositing::Grey`], or a calibrated one
/// run under [`Compositing::Calibrated`] — so that the caller reports the space only where it
/// was *not*.
pub(super) struct GroupRun {
    /// The group's elements, resolved for whatever the group composites in.
    pub(super) commands: Vec<Command>,
    /// The conversion out of the group's own space, where it states one this tree draws: the
    /// black half of a four-component pair, or a one-component space's curve.
    pub(super) pair: Option<pdf_render::GroupBlending>,
    /// Which of §11.6.4.3's readings the elements painted under.
    pub(super) alpha_sources: AlphaSourcesSeen,
    /// Whether the group composited in its own blending colour space.
    pub(super) in_own_space: bool,
}

impl<'a> Interpreter<'a> {
    /// Runs a transparency group `XObject`'s content and composites it as one object.
    ///
    /// `inner` is the state its content runs under — the form's matrix and `/BBox` clip
    /// already applied — and `outer` the state at the `Do`, which is what the group as an
    /// object is painted with.
    pub(super) fn run_transparency_group(
        &mut self,
        group: &TransparencyGroup,
        content: &NestedContent,
        resources: &Dictionary,
        inner: &GraphicsState,
        outer: &GraphicsState,
    ) {
        self.run_transparency_group_body(
            group,
            &GroupBody::Stream(content),
            resources,
            inner,
            outer,
        );
    }

    /// [`Interpreter::run_transparency_group`] for any of [`GroupBody`]'s three contents, which
    /// is where §8.10.4's imported page enters (ADR 1339).
    pub(super) fn run_transparency_group_body(
        &mut self,
        group: &TransparencyGroup,
        body: &GroupBody<'_, 'a>,
        resources: &Dictionary,
        inner: &GraphicsState,
        outer: &GraphicsState,
    ) {
        let mut inner = inner.clone();
        // §11.6.6, of what `Do` adds for a transparency group XObject:
        //
        // > Before execution of the transparency group XObject's content stream, the current
        // > blend mode in the graphics state shall be initialised to Normal , the current
        // > stroking and nonstroking alpha constants to 1.0, and the current soft mask to
        // > None .
        //
        // Its NOTE 1 gives the reason: those parameters apply to the *group*, once, when it
        // is composited into its parent, and leaving them in force would apply them a second
        // time to every element inside. All four are reset here, the soft mask included —
        // the group carries it instead, on the command below.
        inner.blend = BlendMode::Normal;
        inner.fill_alpha = 1.0;
        inner.stroke_alpha = 1.0;
        inner.soft_mask = None;

        // §11.4.6's NOTE 6, which decides what *this* group's elements composite onto. This
        // group is a direct element of `enclosing`, so its own initial backdrop is transparent
        // when Table 145's `/I` says so and where NOTE 6 hands it the transparent one an
        // isolated enclosing knockout group has; and the content below is a direct element of
        // *this* group only where this group is itself a knockout group.
        let enclosing = self.enclosing_knockout;
        let backdrop_transparent = enclosing == Some(KnockoutKind::Isolated);
        self.enclosing_knockout = if !group.knockout {
            None
        } else if group.isolated || backdrop_transparent {
            Some(KnockoutKind::Isolated)
        } else {
            Some(KnockoutKind::NonIsolated)
        };
        // §11.6.6's group colour space, which the elements composite in and which
        // [`group_blending`] resolves against §11.7.2's inheritance rule. Saved and restored
        // rather than set once, because a group is a scope: what is in force after the `Do` is
        // what was in force before it.
        let entered = group_blending(self.document, group, self.blending.as_ref());
        // A group *introduces* a departure only where the space it composites in is not the
        // one its parent was already composited in. Where it inherits, or where it restates
        // the space it inherited, the parent's report is the report — one departure named at
        // the point the file introduces it, rather than once per group that lives inside it.
        let changed = entered != self.blending;
        let introduced = changed.then(|| entered.clone()).flatten();
        let outside = std::mem::replace(&mut self.blending, entered);
        // Where the group's commands begin, for the fold of its readings below.
        let group_mark = self.list.command_count();
        let GroupRun {
            commands,
            pair,
            alpha_sources,
            in_own_space,
        } = self.group_commands(group, body, resources, (&inner, outer), changed);
        self.blending = outside;
        self.enclosing_knockout = enclosing;
        // A group that changes the space in force, with something compositing in it, and
        // is not drawn in that space is a departure the reports can only name on the
        // device's components — so on any other compositing it is *recorded* instead, and
        // the enclosing run reads the record and falls back to the device, where this same
        // group will report ordinarily. Both directions of change count, including one whose
        // `entered` is `None`: a group returning to the device's three components inside a
        // four-component pair composites in the wrong space just as surely as one leaving
        // them. A group drawn in its own space and converted into its parent's at its `Do`
        // (`group_commands`) changed nothing the parent has to answer for, and a change
        // nothing can see — §11.3.4 tells two spaces apart only where something composites —
        // is not one either.
        let departs = changed && !in_own_space && !self.on_a_spot_plane();
        if departs && change_is_visible(introduced.as_ref(), &commands) {
            self.blending_changed = true;
            if self.compositing != Compositing::Device {
                self.nested_space_departed = true;
            }
        }
        if commands.is_empty() {
            return;
        }

        // §11.4.4's NOTE 5 states, in full, when a group need not be built at all:
        //
        // > As a result of these corrections, the effect of compositing objects as a group is
        // > the same as that of compositing them separately (without grouping) if the following
        // > conditions hold:
        // >
        // > The group is non-isolated and has the same knockout attribute as its parent group
        // > (see 11.4.5, "Isolated groups" and 11.4.6 , 'Knockout groups').
        // >
        // > When compositing the group's results with the group backdrop, the Normal blend mode
        // > is used, and the shape and opacity inputs are always 1.0.
        //
        // Both conditions are decidable here, and together they are the *whole* of what a
        // non-isolated group's correctness needed. §11.4.4's result step removes the backdrop
        // from the group's accumulated colour — `C = Cn + (Cn − C0) × (α0/αgn − α0)` — and NOTE
        // 4 says the group alpha `αgn` has to be accumulated *separately* from the composite
        // alpha, which costs a backend a second run of the elements (ADR 1107). Flattening
        // sidesteps the arithmetic entirely by never introducing the backdrop that would have
        // to be removed: the elements composite onto the page they were always going to
        // composite onto, and every blend mode inside the group then sees the backdrop §11.4.4
        // says it should see — one pass, no removal, and the same picture.
        //
        // The clip is not a condition. It reaches every element already — PDF's clipping is
        // cumulative in the graphics state, so an element inside the form carries the clip in
        // force at the `Do` — and applying it once per element is applying it once.
        //
        // This is also strictly less work than the group it replaces: no page-sized buffer, no
        // second composite. A correctness fix that is faster means the old code was doing work
        // that was worse than useless, which is this project's own name for the shape.
        if !group.isolated
            && !group.knockout
            && enclosing.is_none()
            && outer.fill_alpha >= 1.0
            && outer.blend == BlendMode::Normal
            && outer.soft_mask.is_none()
        {
            // No `/CS` question here, and that is §11.6.6 rather than an omission: this branch
            // is reached only where the group is non-isolated, and a non-isolated group's own
            // `/CS` is not the space anything composites in — "the group colour space shall be
            // inherited from the parent group or page". Whatever it inherited is already
            // reported where it was introduced, and this branch does not change it.
            //
            // Drawn inline, the group's elements are the enclosing content's own, so the
            // readings they were painted under join its record.
            for command in commands {
                self.draw(command);
            }
            self.absorb_reading(alpha_sources, group_mark);
            return;
        }

        // §11.4.6's rule reaches the backends where every element's shape is the coverage a
        // rasteriser draws it with, and stays a report where it does not.
        //
        // Isolation is a second condition and a different one. "[A] knockout group may be
        // isolated or non-isolated; that is, isolated and knockout are independent
        // attributes", and what this backend can composite onto is a transparent backdrop —
        // §11.4.5's. For a *non-isolated* knockout group the initial backdrop is the
        // group's own, and the two coincide by exactly the argument §11.4.4's NOTE 3 makes
        // and this tree already relies on: with every element blending Normal the backdrop
        // is composited in and removed again exactly, so it cancels. Where an element
        // blends it does not, and that group is the one `note_group_structure` already
        // names — so it keeps both reports rather than gaining a second departure.
        // `backdrop_transparent` is the third answer and it is the clause's rather than an
        // approximation: NOTE 6 hands a direct element of a knockout group that group's
        // initial backdrop, so a `/I false` knockout group nested in an isolated one has
        // §11.4.5's transparent backdrop and the pair below is exact for it.
        //
        // Where an element's shape is *not* its coverage the display list states the two
        // separately, which is `Command::Shaped` and ADR 0234. The coverage case is left
        // alone rather than folded into it: it is the same arithmetic in one draw instead
        // of two, and it is what §9.3.8's text objects are made of.

        // §11.6.4.3's reading, resolved against the elements *before* the construction below
        // rewrites them: a `Command::Shaped` states its shape already and is not an element
        // the flag can reinterpret, so asking afterwards would answer a different question.
        let settled = alpha_sources.settled_over(&commands);
        let KnockoutConstruction {
            mut commands,
            mut pair,
            knockout,
            backdrop_composited,
            blend,
            knockout_shows,
        } = knockout_construction(
            group,
            commands,
            pair,
            settled,
            enclosing,
            outer.blend,
            self.image_masks.shape_masks(),
            &mut self.one_colour,
        );
        // §11.4.4's own model, for the group NOTE 5 could not flatten: the elements
        // composite onto the backdrop the group is painted over, and the display list says
        // so rather than substituting §11.4.5's transparent one. The three conditions are
        // what makes the clause's backdrop removal cancel against §11.3.3's re-compositing
        // — see `Command::Group`'s `isolated` and ADR 0237 — and each is load-bearing:
        //
        // - **The `Do`'s blend mode is Normal, or the backend keeps NOTE 4's second set of
        //   variables.** Under Normal the cancellation is of a division by Table 140's group
        //   alpha against a multiplication by it, and the two steps collapse to one
        //   interpolation. Under any other mode the group's own colour is needed, so the
        //   backend runs the elements a second time on transparency for Table 140's `αgn`,
        //   performs NOTE 3's removal and composites the result once under the mode
        //   (`render-cpu`'s `blend::remove_backdrop`, ADR 1107) — which is why this condition
        //   is the backend's to state and not this list's.
        // - **Not a knockout group whose rule can change a pixel.** §11.4.6 composites each
        //   element with the group's *initial* backdrop, which here is the page rather than
        //   transparency, so the two stages are not the pair `Command::Shaped` states. But
        //   the two backdrops are the same wherever the knockout rule can show nothing —
        //   `knockout_can_show` is that condition, and where it holds the immediate backdrop
        //   *is* the initial one at every point an element marks, which makes such a group
        //   §11.4.4's group exactly rather than a substitution for it. `/K true` alone used
        //   to stand here, and it cost `knockout_blend_multiply.pdf`'s single Multiply
        //   element the page it blends with (ADR 0307).
        // - **Not drawn as a knockout group.** The pair `Command::Shaped` states is
        //   `P' = (1 − f) × P + S` on the transparent start §11.4.5 gives, and seeding `P`
        //   from the page would put the backdrop in twice. Every group that reaches the
        //   condition above already satisfies this one — a knockout group is drawn only when
        //   it is isolated or when nothing in it blends — and it is stated rather than
        //   derived because the flag is a fact about what this group *became*, which the
        //   element list no longer says once its parts are `Command::Shaped`.
        // - **Not a direct element of an *isolated* knockout group.** There the elements are
        //   drawn on transparency, so a command seeded from its immediate backdrop would take
        //   the accumulation and neither of the backdrops NOTE 6 contrasts; the note's answer
        //   is this group's own `isolated: true` instead. A direct element of a *non-isolated*
        //   knockout group is the opposite case: that group keeps its initial backdrop and
        //   hands each element a private clone of it, which is exactly what this command is
        //   seeded from, so the note is met by stating Table 145's `/I` truthfully
        //   (ADR 1256, ADR 1265).
        //
        // And a fourth condition, which is not about correctness but about *cost*: with
        // every element painting Normal the backdrop is composited in and removed again
        // exactly, so the two models are the same page and the isolated one is what every
        // rasteriser already has. Stating the harder construction there would buy nothing
        // and would cost a surface-sized copy per group — and two of the three backends
        // cannot draw it at all, so it would take pages off the cross-backend comparison
        // for a difference that provably does not exist. This is the same condition the
        // report fired on before the construction existed, and for the same reason.
        //
        // Isolation is otherwise §11.4.5's, which is what a rasteriser's layer is —
        // except where the two flags together state §11.4.6's own backdrop, which is the
        // one shape of the command in which `false` accompanies `knockout`.
        let isolated = !backdrop_composited
            && (group.isolated
                || knockout_shows
                || knockout
                || backdrop_transparent
                || !any_command(&commands, &command_blends));
        self.note_group_departures(
            group,
            &commands,
            GroupDrawn {
                knockout,
                isolated,
                backdrop_transparent,
                alpha_sources,
            },
            // A group drawn in its own space has no colour-space departure to report.
            introduced.as_ref().filter(|_| !in_own_space),
        );
        // What §8.5.4's clip at this `Do` has to intersect with — see `group_alpha_is_shape`,
        // which asks §11.4.6's knockout groups the same question as §11.4.4's since ADR 0554.
        let alpha_is_shape = group_alpha_is_shape(&commands, settled);
        // A group whose raster is its own shape is one element to the enclosing content
        // whatever reading its content ran under (ADR 1306); one that is not is read under the
        // reading at its `Do`, so content that ran under the other states its shapes first.
        if !alpha_is_shape {
            self.describe_group_by_its_do(alpha_sources, outer, &mut commands, &mut pair);
        }
        // §11.6.6's final compositing: the group's shape "shall then be painted into the
        // parent group or page, using the group's accumulated colour and opacity at each
        // point" — under the state in force at `Do`, which is where `ca` and `/BM` were left
        // by the caller. `ca` and not `CA`, because painting a form is not a stroking
        // operation and §11.6.4.4 gives `CA` to those alone.
        self.draw(Command::Group {
            alpha_is_shape,
            commands,
            alpha: outer.fill_alpha,
            clip: inner.clip,
            isolated,
            // The mask in force at the `Do`, applied to the group as one object — which is
            // §11.6.4.3's NOTE 2 recommending exactly this construction: "[t]o apply a soft
            // mask to multiple objects, it is usually best to define the objects as a
            // transparency group and apply the mask to the group as a whole."
            mask: outer.soft_mask,
            // The mode in force at the `Do`, or the one the elements gave up to it.
            blend,
            knockout,
            blending: pair.map(Box::new),
        });
    }

    /// Reads a form `XObject`'s `/Group`, if it is a transparency group (§8.10.3, §11.6.6).
    ///
    /// `None` for a form with no `/Group` at all and for one whose group subtype is not
    /// `/Transparency`, which §11.6.6 makes the same case:
    ///
    /// > An ordinary form XObject -one having no Group entry -or having a Group entry with a
    /// > subtype other than Transparency -shall not be subject to any grouping behaviour for
    /// > transparency purposes.
    pub(super) fn transparency_group(&mut self, dict: &Dictionary) -> Option<TransparencyGroup> {
        let group = self.document.get_key(dict, "Group");
        let group = group.as_dict()?;
        // §8.10.3 Table 94: `/S` is required and "identifies the type of group whose
        // attributes this dictionary describes"; `/Transparency` is the only subtype the
        // specification defines.
        if self.document.get_key(group, "S").as_name()?.as_bytes() != b"Transparency" {
            return None;
        }
        // Table 145's `/I` and `/K`, both booleans defaulting to false.
        Some(TransparencyGroup {
            isolated: matches!(self.document.get_key(group, "I"), Object::Boolean(true)),
            knockout: matches!(self.document.get_key(group, "K"), Object::Boolean(true)),
            colour_space: self.document.get_key(group, "CS"),
        })
    }

    /// The group an imported page is composited as (ISO 32000-2 §11.4.7, §8.10.4).
    ///
    /// §11.4.7 treats a page in one of two ways, and the second is the one a reference `XObject`
    /// reaches:
    ///
    /// > In this situation the PDF 'page' shall not be composited with the media colour; instead
    /// > it shall be treated as a transparency group using the page Group attributes dictionary
    /// > and is composited with its backdrop in the usual way according to the page Group
    /// > attributes dictionary settings.
    ///
    /// The dictionary is the *imported* page's own, read here while the target document is the
    /// one in force. The page is a group whether or not it states one: a page with no `/Group`
    /// is a group under Table 145's defaults — neither isolated nor knockout, and inheriting its
    /// blending space — which §11.4.4's NOTE 5 then draws inline wherever the `Do` composites
    /// trivially, and as one object wherever it does not.
    pub(super) fn imported_page_group(&mut self, page: &Dictionary) -> TransparencyGroup {
        self.transparency_group(page).unwrap_or(TransparencyGroup {
            isolated: false,
            knockout: false,
            colour_space: Object::Null,
        })
    }

    /// §12.5.5's transparency group for an annotation's appearance stream.
    ///
    /// The clause makes **every** stored appearance a transparency group and lets the entry
    /// decide only which one:
    ///
    /// > If the appearance's stream dictionary does not contain a Group entry, it shall be
    /// > treated as a non-isolated, non-knockout transparency group. Otherwise, the isolated
    /// > and knockout values specified in the group dictionary (see 11.6.6, "Transparency
    /// > group XObjects") shall be used.
    ///
    /// So the default is stated in the clause's own words rather than assumed, and the group
    /// it describes is the one [`Interpreter::run_transparency_group`] flattens under §11.4.4's
    /// NOTE 5 whenever the annotation composites Normal at opacity 1 — which is what makes
    /// routing every appearance through the group machinery cost nothing for the appearance
    /// that states no `/Group`, while giving the one that does the `/I` and `/K` the clause
    /// asks for.
    ///
    /// A `/Group` whose subtype is not `/Transparency` takes the default too, which is
    /// §11.6.6 rather than a reading of §12.5.5's "does not contain a Group entry": such a
    /// form "shall not be subject to any grouping behaviour for transparency purposes", so
    /// its `/I` and `/K` are not values §11.6.6 gives a meaning to.
    ///
    /// Table 145's `/CS` needs nothing said about it here for the same clause: §11.6.6 gives
    /// a group colour space only to an isolated group — "[f]or non-isolated groups, or if no
    /// group colour space is specified, the group colour space shall be inherited from the
    /// parent group or page" — and [`group_blending`] already applies exactly that test.
    pub(super) fn appearance_group(&mut self, dict: &Dictionary) -> TransparencyGroup {
        self.transparency_group(dict).unwrap_or(TransparencyGroup {
            isolated: false,
            knockout: false,
            colour_space: Object::Null,
        })
    }
}

#[cfg(test)]
mod tests {
    //! §11.7.4.4's and §9.3.8's implicit knockout group, built from parts and drawn by the
    //! oracle against the clause's own arithmetic (ADR 1000).
    //!
    //! Every expected level below is computed from §11.4.6's two stages and §11.3.3's
    //! formula on the page's own numbers, never read off a renderer; and each pixel test
    //! also draws the same parts *flat*, the construction ADR 1000 replaced, so that the
    //! number discriminates (trap 13).

    #![expect(
        clippy::arithmetic_side_effects,
        reason = "test code: the arithmetic is on a 100-unit page and 0..=255 levels"
    )]

    use std::fmt::Write as _;
    use std::sync::Arc;

    use pdf_render::{
        BlendMode, Color, Command, DisplayList, FillRule, Paint, Raster, Rasterizer, Size,
        SoftMaskId, TargetSpec, Transform,
    };
    use render_cpu::CpuRasterizer;

    use super::implicit_knockout::{ImplicitKnockout, implicit_knockout_group};
    use super::readings::{AlphaSource, AlphaSourcesSeen};
    use super::shape::stated_shape;
    use crate::content::KnockoutKind;
    use crate::image::ShapeMasks;

    const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    fn fill(
        bounds: [f32; 4],
        colour: Color,
        blend: BlendMode,
        mask: Option<SoftMaskId>,
    ) -> Command {
        Command::Fill {
            path: Arc::new(test_scenes::rect(
                bounds[0], bounds[1], bounds[2], bounds[3],
            )),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(colour),
            clip: None,
            mask,
            blend,
        }
    }

    /// The group a caller builds from the answer, field for field as `path.rs` and `text.rs`
    /// state it: no constant, no mask, no clip of its own.
    fn group_of(answer: ImplicitKnockout) -> Command {
        Command::Group {
            commands: answer.elements,
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: answer.blend,
            isolated: answer.isolated,
            knockout: true,
            alpha_is_shape: false,
            blending: None,
        }
    }

    fn render(commands: Vec<Command>) -> Raster {
        let mut list = DisplayList::new(Size {
            width: 100.0,
            height: 100.0,
        });
        list.push(fill([0.0, 0.0, 100.0, 100.0], RED, BlendMode::Normal, None));
        for command in commands {
            list.push(command);
        }
        let target = TargetSpec::for_page(&list, 1.0, 1 << 24).expect("a 100x100 target");
        CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("the oracle draws every construction")
    }

    /// Device y is `100 − page y`.
    fn pixel(raster: &Raster, x: u32, y: u32) -> [u8; 3] {
        let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
        let bytes = &raster.data[index..index + 4];
        [bytes[0], bytes[1], bytes[2]]
    }

    fn assert_close(what: &str, got: [u8; 3], want: [u8; 3], tolerance: i32) {
        assert!(
            got.iter()
                .zip(want)
                .all(|(&g, w)| (i32::from(g) - i32::from(w)).abs() <= tolerance),
            "{what}: drew {got:?}, and the clause's arithmetic gives {want:?}"
        );
    }

    /// Blue at constant opacity ½ over `[10, 10]–[60, 60]`, then white at ½ over
    /// `[30.5, 30]–[80, 80]`, both painted under `blend`: the shape of a `B` whose stroke
    /// straddles its fill, with the second part's left edge on a half pixel.
    fn two_parts(blend: BlendMode) -> Vec<Command> {
        let half = |r, g, b| Color { r, g, b, a: 0.5 };
        vec![
            fill([10.0, 10.0, 60.0, 60.0], half(0.0, 0.0, 1.0), blend, None),
            fill([30.5, 30.0, 80.0, 80.0], half(1.0, 1.0, 1.0), blend, None),
        ]
    }

    /// §11.6.4.3's soft mask is opacity, so a masked part's shape is stated beside it and
    /// the group is still drawn on transparency, as `knockout_elements` states it (ADR 1000).
    #[test]
    fn a_masked_pair_states_its_shapes_on_transparency() {
        let mask = Some(SoftMaskId::new(0));
        let parts = vec![
            fill([10.0, 10.0, 60.0, 60.0], RED, BlendMode::Normal, mask),
            fill([30.0, 30.0, 80.0, 80.0], RED, BlendMode::Normal, mask),
        ];
        let answer = implicit_knockout_group(
            &parts,
            AlphaSourcesSeen::Opacity,
            Some(KnockoutKind::Isolated),
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("a masked pair is drawable");
        assert!(
            answer.isolated,
            "nothing blends, so §11.4.4's NOTE 3 makes transparency exact"
        );
        assert_eq!(answer.blend, BlendMode::Normal);
        assert_eq!(answer.elements.len(), 2);
        assert!(
            !answer.elements.iter().any(super::commands::command_blends),
            "a stated element blends as its object does, and these paint Normal"
        );
        for element in &answer.elements {
            let Command::Shaped { object, shape } = element else {
                panic!("a masked part states its shape beside itself: {element:?}");
            };
            assert!(
                object.mask().is_some(),
                "the object keeps the mask, as opacity"
            );
            assert!(
                shape.mask().is_none(),
                "the shape is the geometry alone (§11.6.4.2)"
            );
        }
    }

    /// An image among the parts states its shape, because the raster says what its alpha is.
    ///
    /// ISO 32000-2 §11.7.4.4's and §9.3.8's implicit knockout group is built by `path.rs` and
    /// `text.rs`, and until ADR 1022 neither could state an image's shape: the kind of an
    /// image's alpha was a record the interpreter kept beside the drawing and those two callers
    /// were not handed it, so a stencil glyph or a masked image among the parts refused the
    /// whole group and the parts were composited one by one — §11.6.2's report. The kind is a
    /// field of the raster now, so `implicit_knockout_group` reads it off the command.
    ///
    /// The picture is §11.4.6's two stages over an element whose alpha is **opacity**. A 2×1
    /// image whose left sample is transparent and whose right is green covers the same square
    /// as an opaque blue part below it. §11.6.4.2 gives such an image the shape "1.0 inside
    /// the image rectangle and 0.0 outside it", so the blue is knocked out under the *whole*
    /// square, and §11.4.6's NOTE 5 says what a shape of 1.0 at an opacity of 0 leaves — "the
    /// colour and opacity that result from compositing the object with the initial backdrop",
    /// which on this isolated group's transparent backdrop is nothing. So the page shows
    /// through on the left and the image on the right. Drawn flat, or with the image's alpha
    /// read as its shape, the left half would still be blue.
    #[test]
    fn an_image_among_the_parts_states_the_shape_its_raster_names() {
        let samples = pdf_render::Image {
            width: 2,
            height: 1,
            data: Arc::from([0, 0, 0, 0, 0, 255, 0, 255].as_slice()),
            interpolate: false,
            // §11.6.4.3's mask opacity: what `image::decode_parts` states for an image with an
            // `/SMask` of its own.
            sample_alpha: pdf_render::SampleAlpha::Opacity,
        };
        let parts = vec![
            fill(
                [20.0, 20.0, 80.0, 80.0],
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                },
                BlendMode::Normal,
                None,
            ),
            Command::Image {
                image: samples.into(),
                transform: Transform::scale(60.0, 60.0).then(Transform::translate(20.0, 20.0)),
                alpha: 1.0,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            },
        ];
        let answer = implicit_knockout_group(
            &parts,
            AlphaSourcesSeen::Opacity,
            Some(KnockoutKind::Isolated),
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("an image states the shape its raster names");
        assert!(
            answer.isolated,
            "nothing blends, so §11.4.4's NOTE 3 applies"
        );
        let Some(Command::Shaped { shape, .. }) = answer.elements.get(1) else {
            panic!(
                "the image states its shape beside itself: {:?}",
                answer.elements
            );
        };
        assert!(
            matches!(**shape, Command::Fill { .. }),
            "an image whose alpha is opacity has the clause's rectangle for a shape: {shape:?}"
        );

        let raster = render(vec![group_of(answer)]);
        assert_close(
            "the image's shape knocks the blue out where its opacity is 0 (NOTE 5)",
            pixel(&raster, 35, 50),
            [255, 0, 0],
            1,
        );
        assert_close(
            "and the image itself is drawn where it is opaque",
            pixel(&raster, 65, 50),
            [0, 255, 0],
            1,
        );
    }

    /// Parts sharing one mode carry it at the `Do` where the mode commutes with §11.4.6's
    /// weighted average, or where every part is one colour; otherwise the group is
    /// §11.4.6's own backdrop, which an element of a knockout group cannot take (NOTE 6).
    #[test]
    fn parts_sharing_a_mode_carry_it_at_the_do_where_that_is_the_same_picture() {
        let opacity = AlphaSourcesSeen::Opacity;
        let multiply = implicit_knockout_group(
            &two_parts(BlendMode::Multiply),
            opacity,
            None,
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("Multiply is affine in its source");
        assert!(multiply.isolated);
        assert_eq!(multiply.blend, BlendMode::Multiply);
        assert!(
            multiply
                .elements
                .iter()
                .all(|element| element.blend() == BlendMode::Normal),
            "the elements are stripped of the mode the group now carries"
        );

        let same_colour = vec![
            fill([10.0, 10.0, 60.0, 60.0], RED, BlendMode::Difference, None),
            fill(
                [30.0, 30.0, 80.0, 80.0],
                Color { a: 0.5, ..RED },
                BlendMode::Difference,
                None,
            ),
        ];
        let difference = implicit_knockout_group(
            &same_colour,
            opacity,
            None,
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("one colour under any mode is one colour after averaging");
        assert!(difference.isolated);
        assert_eq!(difference.blend, BlendMode::Difference);

        let differing = two_parts(BlendMode::Difference);
        let own_backdrop = implicit_knockout_group(
            &differing,
            opacity,
            None,
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("§11.4.6's own backdrop draws what the move cannot");
        assert!(!own_backdrop.isolated);
        assert_eq!(own_backdrop.blend, BlendMode::Normal);
        assert!(
            own_backdrop
                .elements
                .iter()
                .all(|element| matches!(element, Command::Shaped { .. })),
            "every element of a group on its own backdrop states its shape"
        );
        // §11.4.6's NOTE 6, and the two enclosing kinds part company here. An *isolated*
        // knockout group's initial backdrop is transparent, and the note hands it to this
        // group, where §11.3.6 leaves the mode no effect: the parts are drawn Normal on
        // transparency (ADR 1301). A *non-isolated* one keeps its initial backdrop and hands
        // each element a private clone of it (ADR 1256), which is what the own-backdrop
        // command is seeded from, so the note is met there too. ADR 1265.
        let inside_isolated = implicit_knockout_group(
            &differing,
            opacity,
            Some(KnockoutKind::Isolated),
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("an isolated knockout group hands its element a transparent backdrop");
        assert!(inside_isolated.isolated);
        assert_eq!(inside_isolated.blend, BlendMode::Normal);
        assert!(
            !inside_isolated
                .elements
                .iter()
                .any(super::commands::command_blends),
            "a mode composited with a transparent backdrop has no effect (§11.3.6)"
        );
        let inside_non_isolated = implicit_knockout_group(
            &differing,
            opacity,
            Some(KnockoutKind::NonIsolated),
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("a non-isolated knockout group hands its element the initial backdrop");
        assert!(!inside_non_isolated.isolated);
        assert_eq!(inside_non_isolated.blend, BlendMode::Normal);
        // These parts carry §11.6.4.4's constant, which is the input §11.6.4.3's flag
        // reinterprets, so a record of both readings leaves no one shape to state — where a
        // group of opaque parts would be described by both (`AlphaSourcesSeen::settled_over`).
        assert!(
            implicit_knockout_group(
                &differing,
                AlphaSourcesSeen::Mixed,
                None,
                &ShapeMasks::default(),
                &mut super::implicit_knockout::OneColour::default()
            )
            .is_none(),
            "content painted under both readings of /AIS has no one shape to state"
        );
    }

    /// The mode moved to the `Do` is §11.4.6's picture, pixel for pixel, and the flat
    /// drawing the callers used to make is not.
    ///
    /// Over an opaque red page, stage a) composites each part with the page under
    /// Multiply: blue at ½ gives `½ × red + ½ × (red × blue)` = `(½, 0, 0)`, and white at ½
    /// gives `½ × red + ½ × (red × white)` = red. Stage b) then weights by shape: where the
    /// second part covers the first it replaces it outright, and on the half-covered column
    /// 30 it takes half of each — `(¾, 0, 0)`. Drawn flat, the white part multiplies with
    /// the *blue part's* result instead of the page's, which halves it.
    #[test]
    fn the_mode_at_the_do_is_the_clauses_own_arithmetic() {
        let parts = two_parts(BlendMode::Multiply);
        let answer = implicit_knockout_group(
            &parts,
            AlphaSourcesSeen::Opacity,
            None,
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("Multiply moves to the Do");
        let grouped = render(vec![group_of(answer)]);
        assert_close(
            "the first part alone",
            pixel(&grouped, 15, 50),
            [128, 0, 0],
            1,
        );
        assert_close(
            "the overlap: the second part composited with the page, the first knocked out",
            pixel(&grouped, 45, 45),
            [255, 0, 0],
            1,
        );
        assert_close(
            "the half-covered column: half of each part's composite",
            pixel(&grouped, 30, 45),
            [191, 0, 0],
            2,
        );

        let flat = render(parts);
        assert_close(
            "flat, the second part multiplies with the first's result",
            pixel(&flat, 45, 45),
            [128, 0, 0],
            1,
        );
    }

    /// Parts under different modes take §11.4.6's own backdrop, and the oracle draws it:
    /// blue at 1.0 under Multiply over red is black; green at 0.3 under Normal over red is
    /// `(0.7, 0.3, 0)`, and where it covers the blue part it replaces it.
    #[test]
    fn parts_under_different_modes_are_drawn_on_the_groups_own_backdrop() {
        let blue = Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        };
        let green = Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 0.3,
        };
        let parts = vec![
            fill([10.0, 10.0, 60.0, 60.0], blue, BlendMode::Multiply, None),
            fill([30.0, 30.0, 80.0, 80.0], green, BlendMode::Normal, None),
        ];
        let answer = implicit_knockout_group(
            &parts,
            AlphaSourcesSeen::Opacity,
            None,
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("§11.4.6's own backdrop");
        assert!(!answer.isolated);
        let grouped = render(vec![group_of(answer)]);
        assert_close(
            "blue multiplied with red",
            pixel(&grouped, 15, 50),
            [0, 0, 0],
            1,
        );
        assert_close(
            "the overlap: green over red, the blue part knocked out",
            pixel(&grouped, 45, 45),
            [178, 76, 0],
            1,
        );

        let flat = render(parts);
        assert_close(
            "flat, green sits over the black the blue part left",
            pixel(&flat, 45, 45),
            [0, 76, 0],
            1,
        );
    }

    /// An element of zero opacity contributes no colour, so it neither has to share the
    /// mode nor stops a non-affine one moving: with one coloured element left, `K / Σwᵢ`
    /// is that element's colour and any mode is exact at the `Do` (ADR 1009). A nested
    /// group counts where it is isolated and refuses the move where it is not.
    #[test]
    fn a_part_of_zero_opacity_lets_any_mode_move_to_the_do() {
        let blue = Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        };
        let hole = Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.0,
        };
        let parts = vec![
            fill([10.0, 10.0, 60.0, 60.0], blue, BlendMode::Difference, None),
            fill([30.0, 30.0, 80.0, 80.0], hole, BlendMode::Normal, None),
        ];
        let answer = implicit_knockout_group(
            &parts,
            AlphaSourcesSeen::Opacity,
            None,
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("one coloured part under any mode moves it to the Do");
        assert!(answer.isolated);
        assert_eq!(answer.blend, BlendMode::Difference);

        let nested = |isolated| Command::Group {
            commands: vec![fill(
                [10.0, 10.0, 60.0, 60.0],
                blue,
                BlendMode::Normal,
                None,
            )],
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: BlendMode::Color,
            isolated,
            knockout: false,
            alpha_is_shape: true,
            blending: None,
        };
        assert_eq!(
            super::implicit_knockout::blend_at_the_do(
                &[nested(true), parts[1].clone()],
                &mut super::implicit_knockout::OneColour::default()
            ),
            Some(BlendMode::Color),
            "an isolated nested group is one coloured element under its own Do's mode"
        );
        assert_eq!(
            super::implicit_knockout::blend_at_the_do(
                &[nested(false), parts[1].clone()],
                &mut super::implicit_knockout::OneColour::default()
            ),
            None,
            "a non-isolated one is seeded differently under the two constructions"
        );
        assert_eq!(
            super::implicit_knockout::blend_at_the_do(
                &two_parts(BlendMode::Difference),
                &mut super::implicit_knockout::OneColour::default()
            ),
            None,
            "two coloured parts under a mode that is not affine still cannot move it"
        );
    }

    /// A form's knockout group carries a constant alpha at its `Do`, and the mode moved
    /// there is still §11.4.6's own-backdrop picture pixel for pixel — the claim ADR 1000
    /// section 7 priced and did not draw.
    ///
    /// Over an opaque red page, blue at 1.0 under `Difference` composited with the page is
    /// magenta `(1, 0, 1)`; a part of zero opacity over `[30.5, 30]–[80, 80]` knocks the
    /// blue out to the page inside its shape. At the group's `Do` with `ca ½`: the blue
    /// part alone gives `½ red + ½ magenta = (1, 0, ½)`; the hole gives red; and on the
    /// half-covered column 30 the blue part's weight is ½, so both constructions give
    /// `¾ red + ¼ magenta = (1, 0, ¼)` — the collapse `(1 − w_g α_K) B + w_g (…)` on each side.
    #[test]
    fn the_mode_at_the_do_survives_a_constant_alpha_at_the_do() {
        let blue = Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        };
        let hole = Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.0,
        };
        let parts = vec![
            fill([10.0, 10.0, 60.0, 60.0], blue, BlendMode::Difference, None),
            fill([30.5, 30.0, 80.0, 80.0], hole, BlendMode::Difference, None),
        ];
        let group = |commands, blend, isolated| Command::Group {
            commands,
            alpha: 0.5,
            clip: None,
            mask: None,
            blend,
            isolated,
            knockout: true,
            alpha_is_shape: false,
            blending: None,
        };
        let own_backdrop = render(vec![group(
            super::knockout::stated_elements(&parts, AlphaSource::Opacity, &ShapeMasks::default())
                .expect("statable shapes"),
            BlendMode::Normal,
            false,
        )]);
        let mode = super::implicit_knockout::blend_at_the_do(
            &parts,
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("one coloured part");
        let stripped = parts
            .iter()
            .map(super::implicit_knockout::without_blend)
            .collect::<Option<Vec<_>>>()
            .expect("elementary parts");
        let at_the_do = render(vec![group(
            super::implicit_knockout::transparent_knockout_elements(
                &stripped,
                AlphaSource::Opacity,
                &ShapeMasks::default(),
            )
            .expect("statable shapes"),
            mode,
            true,
        )]);
        assert_close(
            "the blue part alone, at half",
            pixel(&at_the_do, 15, 50),
            [255, 0, 128],
            1,
        );
        assert_close(
            "the hole: knocked out to the page",
            pixel(&at_the_do, 45, 45),
            [255, 0, 0],
            1,
        );
        assert_close(
            "the half-covered column",
            pixel(&at_the_do, 30, 45),
            [255, 0, 64],
            2,
        );
        let differing = own_backdrop
            .data
            .iter()
            .zip(&at_the_do.data)
            .filter(|(own, moved)| (i32::from(**own) - i32::from(**moved)).abs() > 1)
            .count();
        assert_eq!(
            differing, 0,
            "the two constructions are one picture, and {differing} channel values differ"
        );
    }

    /// A page of `MediaBox [0 0 100 100]` holding one form `/Fm` under `group`, with three
    /// graphics states: `/GB` Multiply, `/GD` Difference and `/GZ` a non-stroking alpha of 0.
    fn form_fixture(group: &str, form: &str, page: &str) -> Vec<u8> {
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Resources << /ExtGState << /GB << /BM /Multiply >> /GD << /BM /Difference >> \
             /GZ << /ca 0 >> >> /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
             5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] {group} \
             /Length {} >>\nstream\n{form}\nendstream\nendobj\n",
            page.len() + 1,
            form.len() + 1,
        );
        let header = "%PDF-1.7\n";
        let mut bytes = header.as_bytes().to_vec();
        bytes.extend_from_slice(body.as_bytes());
        let xref = bytes.len();
        let padded = format!("\n{body}");
        let mut table = String::from("xref\n0 6\n0000000000 65535 f \n");
        for number in 1..=5 {
            let marker = format!("\n{number} 0 obj\n");
            let offset = padded.find(&marker).expect("every object is written") + header.len();
            writeln!(table, "{offset:010} 00000 n ").expect("a String never fails to write");
        }
        write!(
            table,
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
        )
        .expect("a String never fails to write");
        bytes.extend_from_slice(table.as_bytes());
        bytes
    }

    fn interpret_fixture(bytes: Vec<u8>) -> crate::Interpretation {
        let document = pdf_syntax::Document::open(bytes).expect("the fixture is a valid PDF");
        let page = crate::Pages::new(&document).get(0).expect("page one");
        crate::interpret(&document, &page)
    }

    fn page_pixel(interpretation: &crate::Interpretation, x: u32, y: u32) -> [u8; 3] {
        let list = &interpretation.display_list;
        let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 100x100 target");
        let raster = CpuRasterizer::new()
            .rasterize(list, target)
            .expect("the fixture rasterises");
        pixel(&raster, x, y)
    }

    /// A form `XObject`'s non-isolated knockout group whose coloured elements share a mode
    /// takes it to its `Do` and is an isolated group to every backend — the caller ADR 1000
    /// section 7 left on §11.4.6's own backdrop. Cyan under Multiply over a yellow page is green,
    /// and a magenta fill at `ca 0` over it is a hole showing the page; `Difference` under
    /// two colours is the control that still takes the own-backdrop construction.
    #[test]
    fn a_forms_knockout_group_takes_a_shared_mode_to_its_do() {
        let drawn = interpret_fixture(form_fixture(
            "/Group << /S /Transparency /I false /K true >>",
            "/GB gs 0 1 1 rg 20 20 60 60 re f /GZ gs 1 0 1 rg 30 30 60 60 re f",
            "1 1 0 rg 0 0 100 100 re f /Fm Do",
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert!(
            drawn.display_list.commands().iter().any(|command| matches!(
                command,
                Command::Group {
                    isolated: true,
                    knockout: true,
                    blend: BlendMode::Multiply,
                    ..
                }
            )),
            "the mode is at the group's Do and the group is on transparency: {:?}",
            drawn.display_list.commands()
        );
        assert_close(
            "cyan multiplied with the page",
            page_pixel(&drawn, 25, 75),
            [0, 255, 0],
            1,
        );
        assert_close(
            "the hole, knocked out to the page",
            page_pixel(&drawn, 50, 50),
            [255, 255, 0],
            1,
        );
        assert_close(
            "the page where the group does not mark",
            page_pixel(&drawn, 5, 95),
            [255, 255, 0],
            1,
        );

        let control = interpret_fixture(form_fixture(
            "/Group << /S /Transparency /I false /K true >>",
            "/GD gs 0 1 1 rg 20 20 60 60 re f 1 0 1 rg 30 30 60 60 re f",
            "1 1 0 rg 0 0 100 100 re f /Fm Do",
        ));
        assert!(control.is_complete(), "{:?}", control.unsupported);
        assert!(
            control
                .display_list
                .commands()
                .iter()
                .any(|command| matches!(
                    command,
                    Command::Group {
                        isolated: false,
                        knockout: true,
                        blend: BlendMode::Normal,
                        ..
                    }
                )),
            "two colours under Difference keep §11.4.6's own backdrop: {:?}",
            control.display_list.commands()
        );
    }

    /// A document out of whole objects: `objects[i]` is the body of object `i + 1`, and a
    /// body may hold a binary stream, which is why this builds bytes rather than a string.
    fn objects_fixture(objects: &[Vec<u8>]) -> Vec<u8> {
        let header = b"%PDF-1.7\n";
        let mut bytes = header.to_vec();
        let mut offsets = Vec::with_capacity(objects.len());
        for (index, body) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            bytes.extend_from_slice(body);
            bytes.extend_from_slice(b"\nendobj\n");
        }
        let xref = bytes.len();
        let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
        for offset in offsets {
            writeln!(table, "{offset:010} 00000 n ").expect("a String never fails to write");
        }
        write!(
            table,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .expect("a String never fails to write");
        bytes.extend_from_slice(table.as_bytes());
        bytes
    }

    /// A stream object's body: `dict`'s entries and the data's length, then the data.
    fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
        let mut body = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        body.extend_from_slice(data);
        body.extend_from_slice(b"\nendstream");
        body
    }

    /// A yellow page invoking one form `XObject` whose group is an isolated knockout group,
    /// with `resources` the form's own and `images` the objects numbered from 6.
    fn knockout_form_fixture(resources: &str, form: &str, images: &[Vec<u8>]) -> Vec<u8> {
        let page = "1 1 0 rg 0 0 100 100 re f /Fm Do";
        let mut objects = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
              /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>"
                .to_vec(),
            stream("", page.as_bytes()),
            stream(
                &format!(
                    "/Type /XObject /Subtype /Form /BBox [0 0 100 100] \
                     /Group << /S /Transparency /I true /K true >> /Resources << {resources} >>"
                ),
                form.as_bytes(),
            ),
        ];
        objects.extend_from_slice(images);
        objects_fixture(&objects)
    }

    /// The one knockout group on the page, or a panic naming what the list holds instead.
    fn the_knockout_group(interpretation: &crate::Interpretation) -> &[Command] {
        interpretation
            .display_list
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::Group {
                    commands,
                    knockout: true,
                    isolated: true,
                    ..
                } => Some(commands.as_slice()),
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "an isolated knockout group is drawn: {:?}",
                    interpretation.display_list.commands()
                )
            })
    }

    /// The `(object, shape)` of the one stated element in `elements`.
    fn the_stated_element(elements: &[Command]) -> (&Command, &Command) {
        elements
            .iter()
            .find_map(|element| match element {
                Command::Shaped { object, shape } => Some((&**object, &**shape)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("one element states its shape: {elements:?}"))
    }

    /// A stencil's alpha is §11.6.4.2's shape — "1.0 for painted areas and 0.0 for masked
    /// areas" — so inside a knockout group it knocks out only where it paints. A 2×1 stencil
    /// painting its left sample in red at `ca ½` over an opaque blue square: §11.4.6 gives
    /// red at ½ composited with the transparent initial backdrop where the stencil paints —
    /// over the yellow page, `(255, 128, 0)` — and the blue untouched where it does not. Read
    /// as opacity the shape would be the whole rectangle and the right half would be knocked
    /// out to the page; drawn flat the left half would be red over blue, `(128, 0, 128)`.
    #[test]
    fn a_stencils_alpha_is_its_shape_inside_a_knockout_group() {
        let drawn = interpret_fixture(knockout_form_fixture(
            "/ExtGState << /GH << /ca 0.5 >> >> /XObject << /St 6 0 R >>",
            "0 0 1 rg 10 10 80 80 re f /GH gs 1 0 0 rg q 80 0 0 80 10 10 cm /St Do Q",
            &[stream(
                "/Type /XObject /Subtype /Image /ImageMask true /Width 2 /Height 1 \
                 /BitsPerComponent 1",
                // §8.9.6.2 with the default `/Decode [0 1]`: a 0 sample paints. The left
                // sample is the high bit.
                &[0b0100_0000],
            )],
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        let (object, shape) = the_stated_element(the_knockout_group(&drawn));
        assert!(
            matches!(object, Command::Image { .. }),
            "the stencil is the object: {object:?}"
        );
        assert!(
            matches!(shape, Command::Image { alpha, mask: None, .. } if *alpha >= 1.0),
            "the stencil at alpha 1 is its own shape: {shape:?}"
        );
        assert_close(
            "where the stencil paints, red at ½ over the page",
            page_pixel(&drawn, 30, 50),
            [255, 128, 0],
            1,
        );
        assert_close(
            "where the stencil does not paint, the blue beneath",
            page_pixel(&drawn, 70, 50),
            [0, 0, 255],
            1,
        );
    }

    /// An image's `/SMask` is §11.6.4.3's soft mask, opacity, so the image's shape is
    /// §11.6.4.2's "1.0 inside the image rectangle and 0.0 outside it" whatever the mask
    /// says. A 2×1 red image whose mask is 0 on the left and 1 on the right, over an opaque
    /// blue square: §11.4.6 knocks the blue out under the whole rectangle, and the left half
    /// — shape 1, opacity 0 — shows the yellow page (NOTE 5: the object composited with the
    /// initial backdrop, which at opacity 0 is that backdrop). Read as shape the left half
    /// would keep the blue; drawn flat it would too.
    #[test]
    fn an_smasks_alpha_is_opacity_and_the_shape_is_the_image_rectangle() {
        let drawn = interpret_fixture(knockout_form_fixture(
            "/XObject << /Im 6 0 R >>",
            "0 0 1 rg 10 10 80 80 re f q 80 0 0 80 10 10 cm /Im Do Q",
            &[
                stream(
                    "/Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceRGB \
                     /BitsPerComponent 8 /SMask 7 0 R",
                    &[255, 0, 0, 255, 0, 0],
                ),
                stream(
                    "/Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceGray \
                     /BitsPerComponent 8",
                    &[0, 255],
                ),
            ],
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        let (object, shape) = the_stated_element(the_knockout_group(&drawn));
        assert!(
            matches!(object, Command::Image { .. }),
            "the masked image is the object: {object:?}"
        );
        assert!(
            matches!(
                shape,
                Command::Fill {
                    paint: Paint::Solid(Color::WHITE),
                    mask: None,
                    ..
                }
            ),
            "the image rectangle is the shape: {shape:?}"
        );
        assert_close(
            "where the mask is 0, the page: knocked out and nothing added",
            page_pixel(&drawn, 30, 50),
            [255, 255, 0],
            1,
        );
        assert_close(
            "where the mask is 1, the image",
            page_pixel(&drawn, 70, 50),
            [255, 0, 0],
            1,
        );
    }

    /// A shading's constant alpha is folded into its colours, and its shape is §11.6.4.2's
    /// "1.0 inside and 0.0 outside the bounds of the shading's painti ng geometry": an axial
    /// shading that does not extend paints between the perpendiculars through its two
    /// points and nowhere else. Red from `x = 10` to `x = 50` at `ca ½` over an opaque blue
    /// square: inside the geometry §11.4.6 gives red at ½ over the page, `(255, 128, 0)`;
    /// outside it the blue stands. Drawn flat the inside would be red over blue,
    /// `(128, 0, 128)`.
    #[test]
    fn a_translucent_shadings_shape_is_where_it_paints() {
        let drawn = interpret_fixture(knockout_form_fixture(
            "/ExtGState << /GH << /ca 0.5 >> >> /Shading << /Sh << /ShadingType 2 \
             /ColorSpace /DeviceRGB /Coords [10 0 50 0] /Extend [false false] \
             /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [1 0 0] /N 1 >> >> >>",
            "0 0 1 rg 10 10 80 80 re f /GH gs /Sh sh",
            &[],
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        let (object, shape) = the_stated_element(the_knockout_group(&drawn));
        assert!(
            matches!(
                object,
                Command::Fill {
                    paint: Paint::Shading(_),
                    ..
                }
            ),
            "the shading is the object: {object:?}"
        );
        assert!(
            matches!(
                shape,
                Command::Fill {
                    paint: Paint::Shading(shading),
                    mask: None,
                    ..
                } if shading.is_opaque()
            ),
            "the shading made opaque is the shape: {shape:?}"
        );
        assert_close(
            "inside the shading's geometry, red at ½ over the page",
            page_pixel(&drawn, 30, 50),
            [255, 128, 0],
            1,
        );
        assert_close(
            "outside it, the blue beneath",
            page_pixel(&drawn, 70, 50),
            [0, 0, 255],
            1,
        );
    }

    /// A stencil painted through a pattern is a fill of the unit square through a soft mask
    /// made of the stencil (ADR 0151), and that mask is §8.9.6.2's shape rather than
    /// §11.6.4.3's opacity — so stating the element's shape keeps it where every other mask
    /// comes off. The same stencil as the first test, painted with a red shading pattern at
    /// `ca ½`: the same two pixels, and without the record ([`crate::image::ShapeMasks`]) the
    /// right half would be knocked out to the page.
    #[test]
    fn a_stencil_painted_through_a_pattern_keeps_its_shape() {
        let drawn = interpret_fixture(knockout_form_fixture(
            "/ExtGState << /GH << /ca 0.5 >> >> /XObject << /St 6 0 R >> \
             /Pattern << /P << /PatternType 2 /Shading << /ShadingType 2 \
             /ColorSpace /DeviceRGB /Coords [0 0 100 0] /Extend [true true] \
             /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [1 0 0] /N 1 >> >> >> >>",
            "0 0 1 rg 10 10 80 80 re f /GH gs /Pattern cs /P scn \
             q 80 0 0 80 10 10 cm /St Do Q",
            &[stream(
                "/Type /XObject /Subtype /Image /ImageMask true /Width 2 /Height 1 \
                 /BitsPerComponent 1",
                &[0b0100_0000],
            )],
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        let (object, shape) = the_stated_element(the_knockout_group(&drawn));
        let (
            Command::Fill {
                mask: Some(of_object),
                ..
            },
            Command::Fill {
                mask: Some(of_shape),
                ..
            },
        ) = (object, shape)
        else {
            panic!("both halves are fills through the stencil: {object:?} / {shape:?}");
        };
        assert_eq!(of_object, of_shape, "the shape keeps the stencil's mask");
        assert_close(
            "where the stencil paints, the pattern at ½ over the page",
            page_pixel(&drawn, 30, 50),
            [255, 128, 0],
            1,
        );
        assert_close(
            "where the stencil does not paint, the blue beneath",
            page_pixel(&drawn, 70, 50),
            [0, 0, 255],
            1,
        );
    }

    /// The same stencil painted through a **tiling** pattern, whose cells reach the page as a
    /// group carrying the stencil as its mask (`Interpreter::tile`). The group is an element
    /// here, and §11.3.7.2 makes its shape "the union ... of the shapes of the objects it
    /// contains" as they reach the page — through the stencil, which §11.6.4.2 makes shape.
    /// So the stencil's mask stays on the group's shape exactly as it stays on a fill's, and
    /// the two pixels are the shading pattern's: red at ½ where the stencil paints, `(255,
    /// 128, 0)` over the yellow page, and the blue beneath where it does not. With the mask
    /// removed from the group's shape the right half is knocked out to the page, `(255, 255,
    /// 0)`, which is what planting `mask: None` back into the group arm draws.
    #[test]
    fn a_stencil_painted_through_a_tiling_pattern_keeps_its_shape() {
        let drawn = interpret_fixture(knockout_form_fixture(
            "/ExtGState << /GH << /ca 0.5 >> >> /XObject << /St 6 0 R >> \
             /Pattern << /P 7 0 R >>",
            "0 0 1 rg 10 10 80 80 re f /GH gs /Pattern cs /P scn \
             q 80 0 0 80 10 10 cm /St Do Q",
            &[
                stream(
                    "/Type /XObject /Subtype /Image /ImageMask true /Width 2 /Height 1 \
                     /BitsPerComponent 1",
                    &[0b0100_0000],
                ),
                stream(
                    "/PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
                     /XStep 10 /YStep 10 /Resources << >>",
                    b"1 0 0 rg 0 0 10 10 re f",
                ),
            ],
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_close(
            "where the stencil paints, the cells at ½ over the page",
            page_pixel(&drawn, 30, 50),
            [255, 128, 0],
            1,
        );
        assert_close(
            "where the stencil does not paint, the blue beneath",
            page_pixel(&drawn, 70, 50),
            [0, 0, 255],
            1,
        );
    }

    /// A stencil painted through a pattern under an `/SMask` of its own: the one stencil whose
    /// mask on the fill is not its shape alone. §11.6.4.2 makes the stencil's painted areas the
    /// shape — "the shape shall be 1.0 for painted areas and 0.0 for masked areas" — and
    /// §11.6.4.3 makes the image's `/SMask` opacity, so inside a knockout group the element knocks
    /// out wherever the stencil paints, whatever the mask says there. Both samples paint; the
    /// mask is 0 on the left and 1 on the right. On the right the red fills over the blue; on
    /// the left §11.4.6's NOTE 5 gives shape 1.0 "the colour and opacity that result from
    /// compositing the object with the initial backdrop", which at opacity 0 over the transparent
    /// initial backdrop is nothing — so the blue is knocked out and the yellow page shows. The
    /// product read as shape leaves the blue at a shape of 0, `(0, 0, 255)`, which is what
    /// recording the drawn mask as the shape (`ShapeMasks::record`) draws (ADR 1301).
    #[test]
    fn a_stencil_under_its_own_soft_mask_through_a_pattern_knocks_out_where_it_paints() {
        let drawn = interpret_fixture(knockout_form_fixture(
            "/XObject << /St 6 0 R >> \
             /Pattern << /P << /PatternType 2 /Shading << /ShadingType 2 \
             /ColorSpace /DeviceRGB /Coords [0 0 100 0] /Extend [true true] \
             /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [1 0 0] /N 1 >> >> >> >>",
            "0 0 1 rg 10 10 80 80 re f /Pattern cs /P scn q 80 0 0 80 10 10 cm /St Do Q",
            &[
                stream(
                    "/Type /XObject /Subtype /Image /ImageMask true /Width 2 /Height 1 \
                     /BitsPerComponent 1 /SMask 7 0 R",
                    &[0b0000_0000],
                ),
                stream(
                    "/Type /XObject /Subtype /Image /Width 2 /Height 1 \
                     /ColorSpace /DeviceGray /BitsPerComponent 8",
                    &[0, 255],
                ),
            ],
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        let (object, shape) = the_stated_element(the_knockout_group(&drawn));
        let (
            Command::Fill {
                mask: Some(of_object),
                ..
            },
            Command::Fill {
                mask: Some(of_shape),
                ..
            },
        ) = (object, shape)
        else {
            panic!("both halves are fills through a mask: {object:?} / {shape:?}");
        };
        assert_ne!(
            of_object, of_shape,
            "the page is drawn through the product and the shape through the stencil alone"
        );
        assert_close(
            "where the stencil paints at opacity 0, the blue is knocked out to the page",
            page_pixel(&drawn, 30, 50),
            [255, 255, 0],
            1,
        );
        assert_close(
            "where the stencil paints at opacity 1, the red",
            page_pixel(&drawn, 70, 50),
            [255, 0, 0],
            1,
        );
    }

    /// §11.7.4.4's implicit group for a `B` whose portions blend under a mode no construction
    /// moves, as a direct element of an **isolated** knockout group. §11.4.6's NOTE 6 gives it
    /// the outer group's initial backdrop — "the initial backdrop of the inner group is the same
    /// as that of the outer group" — and that backdrop is transparent, where §11.3.6 leaves a
    /// blend mode nothing to do: "[a]n alpha value of αs = 0.0 or αb = 0.0 results in no blend
    /// mode effect". So the portions are drawn on transparency under the knockout rule and
    /// `/Color` changes no pixel. The fill is opaque red; the stroke, blue at `CA ½`, knocks the
    /// fill out where it covers it and composites with the transparent backdrop alone. Over the
    /// yellow page: red inside, `(128, 128, 128)` under the stroke inside the fill and outside it
    /// alike. Refusing the position left the portions flat and reported, which draws the stroke
    /// blended with the fill beneath it — NOTE 2's double border (ADR 1301).
    #[test]
    fn a_blending_pair_inside_an_isolated_knockout_group_is_drawn_on_its_transparent_backdrop() {
        let drawn = interpret_fixture(knockout_form_fixture(
            "/ExtGState << /GC << /BM /Color /CA 0.5 >> >>",
            "/GC gs 1 0 0 rg 0 0 1 RG 20 w 20 20 60 60 re B",
            &[],
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_close("the fill alone", page_pixel(&drawn, 50, 50), [255, 0, 0], 1);
        assert_close(
            "the stroke over the fill, which it knocks out",
            page_pixel(&drawn, 25, 50),
            [128, 128, 128],
            1,
        );
        assert_close(
            "the stroke outside the fill",
            page_pixel(&drawn, 15, 50),
            [128, 128, 128],
            1,
        );
    }

    /// A page stating `resources` and painting `content` on a white 100 × 100 page.
    fn page_fixture(resources: &str, content: &str) -> Vec<u8> {
        objects_fixture(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
                 /Resources << {resources} >> /Contents 4 0 R >>"
            )
            .into_bytes(),
            stream("", content.as_bytes()),
        ])
    }

    /// A `B` is painted under the `/AIS` in force when it is painted, whatever the page stated
    /// earlier inside a `q`…`Q` that has since been undone. §11.6.4.4: "the AIS ('alpha is
    /// shape') entry in a graphics state parameter dictionary shall determine whether the alpha
    /// constants are interpreted as shape values ( true ) or opacity values ( false )", and the
    /// entry is a graphics state parameter, so the `Q` restores it. The stroke's `CA ½` is
    /// therefore opacity, and §11.7.4.4's knockout gives it shape 1.0 where it covers the fill:
    /// the fill is knocked out and the stroke composites with the page alone, `½ blue + ½ white
    /// = (128, 128, 255)`. Read as shape, the stroke would weight the red fill beneath it by ½ —
    /// `(128, 0, 128)` — which is what a reading left at the undone `true` drew; and a record of
    /// both readings refused the pair and drew NOTE 2's double border instead (ADR 1301).
    #[test]
    fn a_pair_is_read_under_the_alpha_source_in_force_when_it_is_painted() {
        for inside in ["", "0 1 0 rg 0 0 5 5 re f"] {
            let drawn = interpret_fixture(page_fixture(
                "/ExtGState << /GA << /AIS true /ca 0.5 >> /GH << /CA 0.5 >> >>",
                &format!("q /GA gs {inside} Q /GH gs 1 0 0 rg 0 0 1 RG 20 w 20 20 60 60 re B"),
            ));
            assert!(drawn.is_complete(), "{inside:?}: {:?}", drawn.unsupported);
            assert_close(
                "the stroke over the fill it knocks out",
                page_pixel(&drawn, 25, 50),
                [128, 128, 255],
                1,
            );
            assert_close("the fill alone", page_pixel(&drawn, 50, 50), [255, 0, 0], 1);
        }
    }

    /// A `Q` restores §11.6.4.3's reading as it restores every graphics state parameter, so a
    /// knockout group whose content opens by stating `/AIS true` inside a `q`…`Q` that paints
    /// nothing paints everything after it under `false`. §11.6.4.4: "the AIS ('alpha is shape')
    /// entry in a graphics state parameter dictionary shall determine whether the alpha constants
    /// are interpreted as shape values ( true ) or opacity values ( false )". The elements are
    /// two shadings, clipped — marks a path's own record of its reading does not reach — a blue
    /// one opaque and a red one at `ca ½`, and the red's constant is opacity: its shape is 1.0,
    /// so it knocks the blue out whole and composites with the transparent initial backdrop, red
    /// at ½ over the yellow page, `(255, 128, 0)`. The record left at the undone `true` read the
    /// constant as shape and weighted the opaque red against the blue by ½, `(128, 0, 128)`,
    /// without a word (ADR 1301).
    #[test]
    fn a_restored_alpha_source_is_the_one_a_knockout_group_reads() {
        let axial = |colour: &str| {
            format!(
                "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] \
                 /Function << /FunctionType 2 /Domain [0 1] /C0 [{colour}] /C1 [{colour}] \
                 /N 1 >> >>"
            )
        };
        let drawn = interpret_fixture(knockout_form_fixture(
            &format!(
                "/ExtGState << /GA << /AIS true >> /GH << /ca 0.5 >> >> \
                 /Shading << /Sb {} /Sr {} >>",
                axial("0 0 1"),
                axial("1 0 0")
            ),
            "q /GA gs Q q 10 10 80 80 re W n /Sb sh Q \
             /GH gs q 20 20 60 60 re W n /Sr sh Q",
            &[],
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_close(
            "the translucent shading knocks the blue out whole",
            page_pixel(&drawn, 50, 50),
            [255, 128, 0],
            1,
        );
        assert_close("the blue alone", page_pixel(&drawn, 15, 50), [0, 0, 255], 1);
    }

    /// A knockout group whose elements were painted under both readings of §11.6.4.3's `/AIS`
    /// reads each element's constant under the reading it was painted under. §11.6.4.4: "the
    /// AIS ('alpha is shape') entry in a graphics state parameter dictionary shall determine
    /// whether the alpha constants are interpreted as shape values ( true ) or opacity values (
    /// false )", and the entry is a graphics state parameter, so it is a fact of each element.
    ///
    /// The form's group is isolated and knockout, over a yellow page. The blue shading at
    /// `ca ½` is painted under `/AIS false` — shape 1.0 in its clip, opacity ½ — and the two red
    /// ones under `true`, the second after `false` and `true` were stated with nothing painted
    /// between — each shape ½, opacity 1.0. On the transparent initial backdrop §11.4.6's two
    /// stages come to `P' = (1 − f) × P + f × q × C` premultiplied. Where all three paint, the
    /// blue leaves `(0, 0, ½)` at ½, the first red `(½, 0, ¼)` at ¾, the second `(¾, 0, ⅛)` at ⅞,
    /// and §11.4.7 composites that over yellow: `(⅞, ⅛, ⅛)`, `(223, 32, 32)`. Where only the
    /// blue and the first red paint it is `(¾, ¼, ¼)`, `(191, 64, 64)`. Read under one reading
    /// for the whole group — opacity, the entry's default — the reds would be shape 1.0 at
    /// opacity ½ and draw `(255, 128, 0)` at both; the record of both readings refused the
    /// group by name and drew it flat (ADR 1301, ADR 1319).
    #[test]
    fn a_knockout_group_reads_each_element_under_the_alpha_source_it_was_painted_under() {
        let axial = |colour: &str| {
            format!(
                "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] \
                 /Function << /FunctionType 2 /Domain [0 1] /C0 [{colour}] /C1 [{colour}] \
                 /N 1 >> >>"
            )
        };
        let drawn = interpret_fixture(knockout_form_fixture(
            &format!(
                "/ExtGState << /GA << /AIS true >> /GO << /AIS false >> /GH << /ca 0.5 >> >> \
                 /Shading << /Sb {} /Sr {} >>",
                axial("0 0 1"),
                axial("1 0 0")
            ),
            "/GH gs q 10 10 80 80 re W n /Sb sh Q /GA gs q 20 20 60 60 re W n /Sr sh Q \
             /GO gs /GA gs q 30 30 40 40 re W n /Sr sh Q",
            &[],
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_close(
            "all three elements, each read under its own reading",
            page_pixel(&drawn, 50, 50),
            [223, 32, 32],
            2,
        );
        assert_close(
            "the blue and the first red",
            page_pixel(&drawn, 25, 50),
            [191, 64, 64],
            2,
        );
        assert_close(
            "the blue alone, half over yellow",
            page_pixel(&drawn, 15, 50),
            [128, 128, 128],
            2,
        );
    }

    /// A white page whose content is `content`, with `/GT` stating `/AIS true`, `/GH` stating
    /// `CA ½`, and `/P` a tiling pattern, one cell the size of the page, whose content is
    /// `cell` under a `/Gc` stating `ca ½` and a `/Gt` stating `/AIS true` beside it.
    fn tiling_pair_fixture(content: &str, cell: &str) -> Vec<u8> {
        objects_fixture(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
              /Resources << /ExtGState << /GT << /AIS true >> /GH << /CA 0.5 >> >> \
              /Pattern << /P 5 0 R >> >> /Contents 4 0 R >>"
                .to_vec(),
            stream("", content.as_bytes()),
            stream(
                "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
                 /BBox [0 0 100 100] /XStep 100 /YStep 100 \
                 /Resources << /ExtGState << /Gc << /ca 0.5 >> /Gt << /AIS true /ca 0.5 >> >> >>",
                cell.as_bytes(),
            ),
        ])
    }

    /// A tiling cell's marks are read under the cell's own `/AIS`, not the painting mark's.
    /// §8.7.3.1 runs the cell under "the graphics state that was in effect at the beginning of
    /// the pattern's parent content stream", and §11.6.7 adds that "[t]he definition shall not
    /// inherit the current values of the graphics state parameters at the time it is
    /// evaluated", while making what the cell evaluates to the object's own shape and opacity.
    ///
    /// The stroke is painted with the pattern at the page's `CA ½`, over an opaque green fill,
    /// on white. The cell states `/AIS true` and paints red at `ca ½`, so its red is shape ½ at
    /// opacity 1; the mark's `CA ½` is opacity, since the page never stated the entry. The
    /// stroke's shape is ½ and its opacity ½, and §11.7.4.4's knockout weights the fill beneath
    /// it by `1 − ½` against the stroke composited with the white initial backdrop at opacity ½:
    /// `½ × green + ½ × (½ × red + ½ × white) = (128, 191, 64)`, and outside the fill the same
    /// weighting against white gives `(255, 191, 191)`. The cell's reading folded into
    /// the pair's record refused the pair as painted under both readings, and the parts drawn
    /// flat composite the stroke's alpha of ¼ over the green, `(64, 191, 0)` (ADR 1306).
    #[test]
    fn a_tiling_cell_is_read_under_its_own_alpha_source() {
        let drawn = interpret_fixture(tiling_pair_fixture(
            "/GH gs 0 1 0 rg /Pattern CS /P SCN 20 w 20 20 60 60 re B",
            "/Gt gs 1 0 0 rg 0 0 100 100 re f",
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_close(
            "the stroke over the fill it half knocks out",
            page_pixel(&drawn, 25, 50),
            [128, 191, 64],
            1,
        );
        assert_close(
            "the stroke outside the fill, over the white page",
            page_pixel(&drawn, 15, 50),
            [255, 191, 191],
            1,
        );
        assert_close("the fill alone", page_pixel(&drawn, 50, 50), [0, 255, 0], 1);
    }

    /// A white page invoking a form `XObject`, with no group, under `/GA`'s `/AIS true` and
    /// `ca ½`; the form fills and strokes one path with `B`, the stroke through `/P`, a tiling
    /// pattern named in the *form's* resources whose one cell the size of the page is `cell`
    /// under a `/Gc` stating `ca ½`.
    fn form_tiling_fixture(cell: &str) -> Vec<u8> {
        objects_fixture(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
              /Resources << /ExtGState << /GA << /AIS true /ca 0.5 >> >> \
              /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>"
                .to_vec(),
            stream("", b"/GA gs /Fm Do"),
            stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 100 100] \
                 /Resources << /Pattern << /P 6 0 R >> >>",
                b"0 1 0 rg /Pattern CS /P SCN 20 w 20 20 60 60 re B",
            ),
            stream(
                "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
                 /BBox [0 0 100 100] /XStep 100 /YStep 100 \
                 /Resources << /ExtGState << /Gc << /ca 0.5 >> >> >>",
                cell.as_bytes(),
            ),
        ])
    }

    /// A pattern named inside a form starts from the form's graphics state at its `Do`, with
    /// §11.6.7's transparency parameters reset and nothing else. §8.7.3.1's second step:
    ///
    /// > Installs the graphics state that was in effect at the beginning of the pattern's
    /// > parent content stream, with the current transformation matrix altered by the pattern
    /// > matrix as described in 8.7.2, "General properties of patterns"
    ///
    /// and §8.7.2 names the form's space "at the time the form is painted with the Do
    /// operator" as that stream's. §11.6.7 resets "blend mode, soft mask, and alpha constant",
    /// which leaves `/AIS true` in force in the cell, so the cell's own `ca ½` is shape
    /// (§11.6.4.4). The form paints under the page's `/GA`: its green fill at `ca ½` is shape
    /// ½ at opacity 1, and its stroke is the cell's red at shape ½, opacity 1.
    ///
    /// §11.7.4.4's knockout pair over the white page, by §11.4.6's two stages: the fill alone
    /// is `½ × green + ½ × white = (128, 255, 128)`; the stroke over it composites with the
    /// initial backdrop (red, since its opacity is 1) and takes half of that against the
    /// fill's result, `½ × (½, 1, ½) + ½ × red = (191, 128, 64)`; outside the fill it takes half
    /// against white, `(255, 128, 128)`. The cell started from §8.4's initial state read its
    /// constant as opacity, and the pair's record of both readings refused it by name.
    ///
    /// The second cell states no constant, which is where §11.6.7's reset shows: the form's
    /// `ca ½` does not reach it, so the stroke is opaque red of shape 1 and knocks the fill out
    /// whole, `(255, 0, 0)` — a cell that inherited the constant would draw `(191, 128, 64)`.
    #[test]
    fn a_pattern_in_a_form_starts_from_the_forms_state_at_its_do() {
        let drawn = interpret_fixture(form_tiling_fixture("/Gc gs 1 0 0 rg 0 0 100 100 re f"));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_close(
            "the stroke over the fill it half knocks out",
            page_pixel(&drawn, 25, 50),
            [191, 128, 64],
            1,
        );
        assert_close(
            "the stroke outside the fill, over the white page",
            page_pixel(&drawn, 15, 50),
            [255, 128, 128],
            1,
        );
        assert_close(
            "the fill alone, at shape ½",
            page_pixel(&drawn, 50, 50),
            [128, 255, 128],
            1,
        );

        let reset = interpret_fixture(form_tiling_fixture("1 0 0 rg 0 0 100 100 re f"));
        assert!(reset.is_complete(), "{:?}", reset.unsupported);
        assert_close(
            "the form's constant does not reach the cell",
            page_pixel(&reset, 25, 50),
            [255, 0, 0],
            1,
        );
    }

    /// A cell painted under the other reading than the mark, whose raster is not its own shape,
    /// is read under its own. The page states `/AIS true` and the cell, starting from §8.7.3.1's
    /// state — the page's initial one, since the pattern is the page's — does not, so its red
    /// at `ca ½` is opacity; the stroke's shape is then 1.0 where it covers the green fill, which
    /// it knocks out whole and composites with the white page alone: `½ × red + ½ × white =
    /// (255, 128, 128)`, inside the fill and out. A pair read under the mark's `true` would have
    /// drawn `(128, 128, 0)` at the first; the record of both readings refused it by name
    /// (ADR 1306, ADR 1319).
    #[test]
    fn a_cell_under_the_other_reading_that_is_not_its_own_shape_is_read_under_its_own() {
        let drawn = interpret_fixture(tiling_pair_fixture(
            "/GT gs 0 1 0 rg /Pattern CS /P SCN 20 w 20 20 60 60 re B",
            "/Gc gs 1 0 0 rg 0 0 100 100 re f",
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_close(
            "the stroke over the fill it knocks out",
            page_pixel(&drawn, 25, 50),
            [255, 128, 128],
            1,
        );
        assert_close(
            "the stroke outside the fill",
            page_pixel(&drawn, 15, 50),
            [255, 128, 128],
            1,
        );
        assert_close("the fill alone", page_pixel(&drawn, 50, 50), [0, 255, 0], 1);
    }

    /// A part painted through a tiling pattern composites by its cell's own marks, which the
    /// state at the `B` does not show. §11.6.7 makes what the cell evaluates to "the object's
    /// source colour ( 𝐶𝑠 ), object shape ( f j ), and object opacity ( qi )", so the stroke
    /// below — through a cell painting red at `ca ½` — is translucent, and §11.6.2's "[p]ortions
    /// of an object shall not be composited with one another" makes the pair one knockout group
    /// (§11.7.4.4). Where the stroke covers the green fill it knocks it out and composites with
    /// the white page alone, `(255, 128, 128)`; the parts drawn apart, as a state of opacity 1.0
    /// left them, put the stroke over the fill — NOTE 2's double border, `(128, 128, 0)`, with
    /// no report (ADR 1306).
    #[test]
    fn a_part_painted_through_a_translucent_cell_is_one_object_with_the_other() {
        let drawn = interpret_fixture(tiling_pair_fixture(
            "0 1 0 rg /Pattern CS /P SCN 20 w 20 20 60 60 re B",
            "/Gc gs 1 0 0 rg 0 0 100 100 re f",
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_close(
            "the stroke over the fill it knocks out",
            page_pixel(&drawn, 25, 50),
            [255, 128, 128],
            1,
        );
        assert_close("the fill alone", page_pixel(&drawn, 50, 50), [0, 255, 0], 1);
    }

    /// And a glyph's: mode 2 strokes the glyph through the same translucent cell, and its fill
    /// and stroke are one knockout group rather than two marks (ADR 1306).
    #[test]
    fn a_glyph_stroked_through_a_translucent_cell_is_one_object_with_its_fill() {
        let drawn = interpret_fixture(objects_fixture(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
              /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> \
              >> /Pattern << /P 5 0 R >> >> /Contents 4 0 R >>"
                .to_vec(),
            stream(
                "",
                b"0 1 0 rg /Pattern CS /P SCN 4 w BT /F1 60 Tf 10 30 Td 2 Tr (I) Tj ET",
            ),
            stream(
                "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
                 /BBox [0 0 100 100] /XStep 100 /YStep 100 \
                 /Resources << /ExtGState << /Gc << /ca 0.5 >> >> >>",
                b"/Gc gs 1 0 0 rg 0 0 100 100 re f",
            ),
        ]));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert!(
            drawn.display_list.commands().iter().any(|command| matches!(
                command,
                Command::Group {
                    knockout: true,
                    commands,
                    ..
                } if commands.len() == 2
            )),
            "the glyph's fill and stroke are one knockout group: {:?}",
            drawn.display_list.commands()
        );
    }

    /// A white page showing two `I`s of Helvetica at 60 points, the second 3 units right of the
    /// first, filled through `/P`, a tiling pattern whose one cell the size of the page paints
    /// `cell` under a `/Gc` stating `ca ½`.
    fn tiled_text_fixture(cell: &str) -> Vec<u8> {
        objects_fixture(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
              /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> \
              >> /Pattern << /P 5 0 R >> >> /Contents 4 0 R >>"
                .to_vec(),
            stream(
                "",
                b"/Pattern cs /P scn BT /F1 60 Tf 10 30 Td (I) Tj 3 0 Td (I) Tj ET",
            ),
            stream(
                "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
                 /BBox [0 0 100 100] /XStep 100 /YStep 100 \
                 /Resources << /ExtGState << /Gc << /ca 0.5 >> >> >>",
                cell.as_bytes(),
            ),
        ])
    }

    /// A text object whose glyphs composite only through their tiling cell is §9.3.8's knockout
    /// group all the same. The clause:
    ///
    /// > If the parameter is true , the behaviour shall be equivalent to treating the entire
    /// > text object as if it were a non-isolated knockout transparency group; see 11.4.6,
    /// > "Knockout groups" where each glyph is an individual element in that group's
    /// > transparency stack. When glyphs overlap, later glyphs shall overwrite ("knock out")
    /// > earlier ones in the area of overlap.
    ///
    /// and §11.6.7 makes what the cell evaluates to the glyph's own "object opacity ( qi )", so
    /// each `I` below is red at opacity ½ although the state at the `Tj` is opaque. Where the
    /// two overlap the second knocks the first out and composites with the white page alone,
    /// `½ × red + ½ × white = (255, 128, 128)` — the value where one glyph paints — and the two
    /// composited one over the other, as a state of opacity 1.0 left them, give `(255, 64, 64)`
    /// without a word.
    #[test]
    fn glyphs_translucent_through_their_cell_knock_one_another_out() {
        let drawn = interpret_fixture(tiled_text_fixture("/Gc gs 1 0 0 rg 0 0 100 100 re f"));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert!(
            drawn
                .display_list
                .commands()
                .iter()
                .any(|command| matches!(command, Command::Group { knockout: true, .. })),
            "the text object is one knockout group: {:?}",
            drawn.display_list.commands()
        );
        assert_close(
            "the second glyph over the first, which it knocks out",
            page_pixel(&drawn, 20, 50),
            [255, 128, 128],
            1,
        );
        assert_close(
            "the first glyph alone",
            page_pixel(&drawn, 16, 50),
            [255, 128, 128],
            1,
        );
        assert_close(
            "between the glyphs' stems",
            page_pixel(&drawn, 30, 50),
            [255, 255, 255],
            1,
        );
    }

    /// §9.3.8's group reads each glyph under the `/AIS` it was shown under. Two `I`s of red at
    /// `ca ½` on white, the second 3 units right of the first: one shown under `false` — shape
    /// 1.0, opacity ½ — and one under `true` — shape ½, opacity 1.0 — in both orders.
    ///
    /// The text object is "a non-isolated knockout transparency group", so each glyph composites
    /// with the white page and then takes its shape's share against what is under it (§11.4.6).
    /// Opacity then shape: the first leaves `(1, ½, ½)`, and the second, red over white at
    /// opacity 1, takes half against it — `(1, ¼, ¼)`, `(255, 64, 64)` — where reading both as
    /// opacity knocks the first out and draws `(255, 128, 128)`. Shape then opacity: the first
    /// leaves the same `(1, ½, ½)`, and the second's shape of 1.0 replaces it with red at ½
    /// over white, `(255, 128, 128)`, where reading both as shape draws `(255, 64, 64)`. Where
    /// one glyph paints alone it is `(255, 128, 128)` under either. The record of both
    /// readings refused the object by name and drew the glyphs flat (ADR 1319).
    #[test]
    fn a_text_objects_glyphs_are_each_read_under_the_alpha_source_they_were_shown_under() {
        let resources = "/ExtGState << /GH << /ca 0.5 >> /GT << /AIS true >> \
                         /GF << /AIS false >> >> \
                         /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >>";
        for (content, overlap) in [
            (
                "/GH gs 1 0 0 rg BT /F1 60 Tf 10 30 Td (I) Tj /GT gs 3 0 Td (I) Tj ET",
                [255, 64, 64],
            ),
            (
                "/GH gs /GT gs 1 0 0 rg BT /F1 60 Tf 10 30 Td (I) Tj /GF gs 3 0 Td (I) Tj ET",
                [255, 128, 128],
            ),
        ] {
            let drawn = interpret_fixture(page_fixture(resources, content));
            assert!(drawn.is_complete(), "{content}: {:?}", drawn.unsupported);
            assert_close(content, page_pixel(&drawn, 20, 50), overlap, 1);
            assert_close(content, page_pixel(&drawn, 16, 50), [255, 128, 128], 1);
            assert_close(content, page_pixel(&drawn, 23, 50), [255, 128, 128], 1);
        }
    }

    /// A group nested in a knockout group, whose content ran under the other reading and whose
    /// raster is not its shape, is read under the reading its content ran under.
    ///
    /// The outer form is an isolated knockout group over white: a blue fill at `ca ½` under
    /// `/AIS false` — shape 1.0, opacity ½ — and then an isolated group whose content states
    /// `/AIS true` and `ca`/`CA ½` and fills red and strokes blue with one `B`. Inside it the
    /// fill is shape ½ at opacity 1 and so is the stroke, and §11.7.4.4's knockout pair makes the
    /// group's raster *not* its shape — the group's shape is their union, its alpha the pair's
    /// accumulation.
    ///
    /// On the outer group's transparent backdrop §11.4.6 is `P' = (1 − f) × P + S`. The blue
    /// leaves `(0, 0, ½)` at ½. Where the red fills alone the inner group is red at alpha ½ over
    /// a shape of ½, so `P' = ½ × (0, 0, ½) + (½, 0, 0)`, `(½, 0, ¼)` at ¾, and over white
    /// `(¾, ¼, ½)` — `(191, 64, 128)`. Where the stroke covers the fill the inner group is
    /// `(¼, 0, ½)` at ¾ over a shape of ¾, so `(¼, 0, ⅝)` at ⅞ and over white `(96, 32, 191)`;
    /// that pixel drew `(64, 0, 191)` while the group's shape was its elements' stated shapes
    /// knocked out bare, which weights by coverage and replaced the union.
    /// The inner content's constants read as opacity would make the group's shape 1.0 and knock
    /// the blue out, `(255, 128, 128)` where the red fills alone; the record of both readings
    /// refused the outer group by name (ADR 1319).
    #[test]
    fn a_group_under_the_other_reading_whose_raster_is_not_its_shape_is_read_under_its_own() {
        let drawn = interpret_fixture(objects_fixture(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
              /Resources << /XObject << /K 5 0 R >> >> /Contents 4 0 R >>"
                .to_vec(),
            stream("", b"/K Do"),
            stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 100 100] \
                 /Group << /S /Transparency /I true /K true >> \
                 /Resources << /ExtGState << /GH << /ca 0.5 /CA 0.5 >> >> \
                 /XObject << /G 6 0 R >> >>",
                b"q /GH gs 0 0 1 rg 10 10 80 80 re f Q /G Do",
            ),
            stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 100 100] \
                 /Group << /S /Transparency /I true >> \
                 /Resources << /ExtGState << /GH << /ca 0.5 /CA 0.5 >> \
                 /GT << /AIS true >> >> >>",
                b"/GT gs /GH gs 1 0 0 rg 0 0 1 RG 20 w 30 30 40 40 re B",
            ),
        ]));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_close(
            "the red fill alone over the blue",
            page_pixel(&drawn, 50, 50),
            [191, 64, 128],
            2,
        );
        assert_close(
            "the stroke over the fill, over the blue",
            page_pixel(&drawn, 35, 50),
            [96, 32, 191],
            2,
        );
        // The stroke outside the fill: `½ × (0, 0, ½) + (0, 0, ½)` at ¾ over white.
        assert_close(
            "the stroke alone, over the blue",
            page_pixel(&drawn, 25, 50),
            [64, 64, 255],
            2,
        );
    }

    /// A glyph painted through a cell of two marks is *one* element of §9.3.8's knockout group,
    /// so the cell's marks composite with one another and the glyphs knock one another out.
    ///
    /// §11.6.7 makes the pattern "implicitly enclosed in a non-isolated transparency group: a
    /// non-knockout group for tiling patterns", and §9.3.8 makes "each glyph ... an individual
    /// element in that group's transparency stack". The cell paints red at `ca ½` over its
    /// lower half and blue at `ca ½` over its upper, overlapping on `y ∈ [40, 50]`. Inside the
    /// first glyph there the cell composites blue over red over white, `(½, ¼, ¾)` —
    /// `(128, 64, 191)` — and where the second glyph covers the first it knocks it out and shows
    /// the same. Tiles standing inline as elements of their own had the blue knock the red out
    /// inside one glyph, `(128, 128, 255)`; the glyphs composited over one another instead of
    /// knocking out give `(96, 16, 175)` at the overlap.
    #[test]
    fn a_glyphs_tiles_are_one_element_of_the_text_objects_knockout_group() {
        let drawn = interpret_fixture(tiled_text_fixture(
            "/Gc gs 1 0 0 rg 0 0 100 50 re f 0 0 1 rg 0 40 100 60 re f",
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_close(
            "the cell's two marks, composited inside one glyph",
            page_pixel(&drawn, 16, 55),
            [128, 64, 191],
            1,
        );
        assert_close(
            "the second glyph over the first, which it knocks out",
            page_pixel(&drawn, 20, 55),
            [128, 64, 191],
            1,
        );
        assert_close(
            "the red alone",
            page_pixel(&drawn, 16, 65),
            [255, 128, 128],
            1,
        );
        assert_close(
            "the blue alone",
            page_pixel(&drawn, 16, 45),
            [128, 128, 255],
            1,
        );
    }

    /// A glyph filled and stroked by text rendering mode 2 is read under the `/AIS` in force
    /// where it is shown, not under the enclosing content's history. §11.7.4.4 makes "the
    /// painting of glyphs with text rendering mode 2 or 6" one object, and both of its parts
    /// are painted under the graphics state at the `Tj`, where §11.6.4.4's reading is `false`:
    /// the `q`…`Q` before it painted a translucent mark under `true` and was undone. So the
    /// stroke's `CA ½` is opacity and the pair is one knockout group, where the record of both
    /// readings the page had painted under refused it and drew the parts flat — NOTE 2's double
    /// border (ADR 1306).
    #[test]
    fn a_glyphs_parts_are_read_under_the_alpha_source_in_force_when_it_is_shown() {
        let drawn = interpret_fixture(page_fixture(
            "/ExtGState << /GA << /AIS true /ca 0.5 >> /GH << /CA 0.5 >> >> \
             /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >>",
            "q /GA gs 0 1 0 rg 0 0 5 5 re f Q /GH gs 1 0 0 rg 0 0 1 RG 4 w \
             BT /F1 60 Tf 10 30 Td 2 Tr (I) Tj ET",
        ));
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert!(
            drawn.display_list.commands().iter().any(|command| matches!(
                command,
                Command::Group {
                    knockout: true,
                    commands,
                    ..
                } if matches!(commands.as_slice(), [Command::Fill { .. }, Command::Stroke { .. }])
            )),
            "the glyph's fill and stroke are one knockout group: {:?}",
            drawn.display_list.commands()
        );
    }

    /// A [`Command::Shaped`] arrives with its shape already stated, so §11.6.4.3's flag
    /// reinterprets nothing in it and content painted under **both** readings that holds one
    /// still has a reading (`flag_reinterprets_nothing`, ADR 1205).
    ///
    /// The parts are an opaque blue lower and a stated pair whose object is green at a
    /// constant of ½ and whose shape is the same rectangle opaque. §11.4.6's stage a)
    /// composites the object with the group's transparent initial backdrop, where §11.3.6
    /// leaves the blend mode nothing to do — "[a]n alpha value of αs = 0.0 or αb = 0.0
    /// results in no blend mode effect" — so it is the object itself; stage b) weights by the
    /// stated shape, which is 1.0 in the overlap, so the pair knocks the blue out whole and
    /// the group's result there is green at an alpha of ½. §11.4.7 composites that over the
    /// red page: `½ × green + ½ × red` = `(128, 128, 0)`.
    ///
    /// **Calibrated by replacing the pair with its object alone** (trap 13): the flag then
    /// does reinterpret §11.6.4.4's constant, no reading describes the content, and the group
    /// is refused — which is what the borrowed predicate answered about the pair as well.
    #[test]
    fn a_stated_pair_leaves_mixed_content_a_reading() {
        let green = Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        };
        let translucent = fill(
            [40.0, 40.0, 80.0, 80.0],
            Color { a: 0.5, ..green },
            BlendMode::Normal,
            None,
        );
        let pair = Command::Shaped {
            object: Box::new(translucent.clone()),
            shape: Box::new(fill(
                [40.0, 40.0, 80.0, 80.0],
                green,
                BlendMode::Normal,
                None,
            )),
        };
        let blue = Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        };
        let parts = vec![
            fill([20.0, 20.0, 60.0, 60.0], blue, BlendMode::Normal, None),
            pair,
        ];
        assert_eq!(
            AlphaSourcesSeen::Mixed.settled_over(&parts),
            Some(AlphaSource::Opacity),
            "the flag reinterprets nothing a stated pair carries"
        );
        let answer = implicit_knockout_group(
            &parts,
            AlphaSourcesSeen::Mixed,
            None,
            &ShapeMasks::default(),
            &mut super::implicit_knockout::OneColour::default(),
        )
        .expect("content described by both readings is drawn");
        assert!(answer.isolated);
        let drawn = render(vec![group_of(answer)]);
        assert_close(
            "the stated pair knocks the blue out and shows its own alpha over the page",
            pixel(&drawn, 50, 50),
            [128, 128, 0],
            1,
        );
        assert_close(
            "where only the lower element paints, it composites with the transparent \
             initial backdrop and covers the page",
            pixel(&drawn, 30, 70),
            [0, 0, 255],
            1,
        );
        assert_close(
            "outside both elements the group marks nothing and the page shows",
            pixel(&drawn, 90, 10),
            [255, 0, 0],
            1,
        );

        let bare = vec![
            fill([20.0, 20.0, 60.0, 60.0], blue, BlendMode::Normal, None),
            translucent,
        ];
        assert_eq!(
            AlphaSourcesSeen::Mixed.settled_over(&bare),
            None,
            "a bare constant is the input the flag reinterprets"
        );
        assert!(
            implicit_knockout_group(
                &bare,
                AlphaSourcesSeen::Mixed,
                None,
                &ShapeMasks::default(),
                &mut super::implicit_knockout::OneColour::default()
            )
            .is_none(),
            "content painted under both readings of a stated constant has no one shape"
        );
    }

    /// §11.3.7.2 gives a group's shape without reference to its backdrop — "[t]he shape of a
    /// group object shall be the union (as defined in 11.3.7.3, \"Result shape and
    /// opacity\") of the shapes of the objects it contains" — so a *non-isolated* group's
    /// stated shape is that union accumulated on transparency, and isolation is not a
    /// condition on stating it (`shape_the_alpha_already_is`, ADR 1205).
    #[test]
    fn a_non_isolated_groups_shape_is_the_union_accumulated_on_transparency() {
        let element = Command::Group {
            commands: vec![fill(
                [10.0, 10.0, 60.0, 60.0],
                RED,
                BlendMode::Multiply,
                None,
            )],
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: BlendMode::Multiply,
            isolated: false,
            knockout: false,
            alpha_is_shape: false,
            blending: None,
        };
        let shape = stated_shape(&element, AlphaSource::Shape, &ShapeMasks::default())
            .expect("a group's shape is stated whatever its backdrop was");
        let Command::Group {
            isolated,
            alpha_is_shape,
            blend,
            ..
        } = shape
        else {
            panic!("a group's shape is a group");
        };
        assert!(
            isolated,
            "the union is accumulated on transparency, where nothing but the elements is in it"
        );
        assert!(
            alpha_is_shape,
            "what the arm returns is the shape by contract"
        );
        assert_eq!(
            blend,
            BlendMode::Normal,
            "§11.4.6 leaves a knockout element nothing to blend against"
        );
    }
}
