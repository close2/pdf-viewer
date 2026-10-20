//! §11.4.6: a form `XObject`'s knockout group, built from its elements' shapes by one of three
//! constructions, tried in order, and the decision whether the rule can change a pixel (ADR 2080).

use pdf_render::{BlendMode, Command, Rect};

use crate::image::ShapeMasks;

use crate::content::KnockoutKind;

use super::TransparencyGroup;
use super::commands::{any_command, command_blends, command_bounds, command_composites};
use super::implicit_knockout::{
    OneColour, blend_at_the_do, is_a_non_isolated_group, transparent_knockout_elements,
    without_blend,
};
use super::into_parent::blending_paired;
use super::readings::AlphaSource;
use super::shape::{element_shape_is_coverage, knockout_shape_is_coverage, stated_shape};

/// A knockout group's elements, each carrying the shape it knocks out with (§11.4.6).
///
/// `None` where one element's shape cannot be stated, which leaves the whole group an
/// ordinary one with the report [`crate::content::Interpreter::note_group_structure`] gives it —
/// per group rather than per element, because the model the clause states is the group's.
pub(super) fn knockout_elements(
    commands: &[Command],
    alpha: AlphaSource,
    shape_masks: &ShapeMasks,
) -> Option<Vec<Command>> {
    commands
        .iter()
        .map(|command| {
            if element_shape_is_coverage(command, alpha)
                || matches!(command, Command::Shaped { .. })
            {
                return Some(command.clone());
            }
            Some(Command::Shaped {
                object: Box::new(command.clone()),
                shape: Box::new(stated_shape(command, alpha, shape_masks)?),
            })
        })
        .collect()
}

/// Every element of a knockout group as a [`Command::Shaped`], for a group whose initial
/// backdrop is **not** transparent (§11.4.6).
///
/// [`knockout_elements`] leaves an element whose shape *is* its coverage bare, because on a
/// transparent backdrop one draw carries both quantities. Against the group's own backdrop a
/// backend needs the shape of **every** element per pixel — the weighted average's factor —
/// so each is stated, even where a single draw could have carried it. `None` where any
/// element's shape cannot be stated, which leaves the group the report it has.
pub(super) fn stated_elements(
    commands: &[Command],
    alpha: AlphaSource,
    shape_masks: &ShapeMasks,
) -> Option<Vec<Command>> {
    commands
        .iter()
        .map(|command| {
            if matches!(command, Command::Shaped { .. }) {
                return Some(command.clone());
            }
            Some(Command::Shaped {
                object: Box::new(command.clone()),
                shape: Box::new(stated_shape(command, alpha, shape_masks)?),
            })
        })
        .collect()
}

/// Whether two interpretations of one content stream drew the same structure.
///
/// The guard [`pdf_render::DisplayList::geometry_digest`] provides for §11.4.7's page pair, asked
/// of a *group's* pair: the two lists differ only in what each colour resolved to, so their
/// variants, nesting, clips, paths and blend modes must agree — the halves are resolved per pixel,
/// and a command in one and not the other would be converted against a shape that never drew it.
/// Soft masks are compared by *presence* rather than identity, because the second run registers its
/// own copies of the same masks and the identifiers differ by construction.
pub(super) fn paired(first: &[Command], second: &[Command]) -> bool {
    first.len() == second.len()
        && first.iter().zip(second).all(|(left, right)| {
            if std::mem::discriminant(left) != std::mem::discriminant(right)
                || left.clip() != right.clip()
                || left.mask().is_some() != right.mask().is_some()
                || std::mem::discriminant(&left.blend()) != std::mem::discriminant(&right.blend())
                || left.path().map(|path| path.commands().len())
                    != right.path().map(|path| path.commands().len())
            {
                return false;
            }
            match (left, right) {
                (
                    Command::Group {
                        commands: ours,
                        blending: pair,
                        ..
                    },
                    Command::Group {
                        commands: theirs,
                        blending: other,
                        ..
                    },
                ) => blending_paired(pair.as_deref(), other.as_deref()) && paired(ours, theirs),
                (
                    Command::Shaped {
                        object: left_object,
                        shape: left_shape,
                    },
                    Command::Shaped {
                        object: right_object,
                        shape: right_shape,
                    },
                ) => {
                    paired(
                        std::slice::from_ref(left_object),
                        std::slice::from_ref(right_object),
                    ) && paired(
                        std::slice::from_ref(left_shape),
                        std::slice::from_ref(right_shape),
                    )
                }
                _ => true,
            }
        })
}

/// The first element of a knockout group whose shape this renderer cannot state, named for
/// the report. See [`stated_shape`] for why each is refused.
pub(super) fn unstatable_shape(
    commands: &[Command],
    alpha: AlphaSource,
    shape_masks: &ShapeMasks,
) -> Option<&'static str> {
    commands.iter().find_map(|command| {
        if element_shape_is_coverage(command, alpha)
            || stated_shape(command, alpha, shape_masks).is_some()
        {
            return None;
        }
        Some(match (command, alpha) {
            (Command::Fill { .. } | Command::Stroke { .. }, _) => {
                "a paint this renderer cannot describe the shape of"
            }
            _ => "an element this renderer cannot describe the shape of",
        })
    })
}

/// Whether §11.4.6's knockout could change a pixel of this group.
///
/// True when an element that composites overlaps an element painted before it. Where the
/// upper element is opaque and blends Normal it overwrites the lower one under either model,
/// and where two elements do not overlap there is nothing to knock out.
///
/// An element whose ink cannot be bounded is taken to overlap everything, which is the same
/// direction [`crate::content::text::TextObject::note_knockout`] errs in and for the same reason.
pub(super) fn knockout_can_show(commands: &[Command]) -> bool {
    let mut painted: Vec<Option<Rect>> = Vec::with_capacity(commands.len());
    for command in commands {
        let bounds = command_bounds(command);
        if command_composites(command)
            && painted.iter().any(|below| match (below, bounds) {
                (Some(first), Some(second)) => {
                    first.min.x < second.max.x
                        && second.min.x < first.max.x
                        && first.min.y < second.max.y
                        && second.min.y < first.max.y
                }
                _ => true,
            })
        {
            return true;
        }
        painted.push(bounds);
    }
    false
}

/// What §11.4.6 made of a knockout group's elements — see [`knockout_construction`].
pub(super) struct KnockoutConstruction {
    /// The elements as the display list states them, rewritten where a shape had to be.
    pub(super) commands: Vec<Command>,
    /// The group's conversion out, with the black half of a press pair rewritten alongside
    /// `commands` — [`commit_knockout`] — or carried through untouched.
    pub(super) pair: Option<pdf_render::GroupBlending>,
    /// Whether the group goes to the backends with `knockout: true`.
    pub(super) knockout: bool,
    /// Whether it goes with `isolated: false` beside it — §11.4.6's own backdrop (ADR 0327).
    pub(super) backdrop_composited: bool,
    /// The mode at the group's `Do`: the one in force there, or the one its elements gave up.
    pub(super) blend: BlendMode,
    /// Whether §11.4.6's rule could change a pixel at all ([`knockout_can_show`]).
    pub(super) knockout_shows: bool,
}

/// §11.4.6's three constructions for a form `XObject`'s knockout group, tried in order, and
/// the flat drawing with a report where none applies (ISO 32000-2 §11.4.6, §11.4.4, §11.6.4.3).
///
/// `alpha` is the one reading of §11.6.4.3's `/AIS` that describes the content, or `None`:
/// the shape §11.4.6 weights by is built one way under each reading, so a group whose content
/// painted under both *and states something the flag reinterprets* is refused
/// ([`super::readings::AlphaSourcesSeen::settled_over`]). `enclosing` is NOTE 6's answer for a group that is a
/// direct element of a knockout group: which of §11.4.6's two initial backdrops that group
/// composites its elements with, or `None` where there is no such enclosing group.
///
/// - **On transparency**, where the group is isolated, NOTE 6 makes it so, or nothing in it
///   blends: §11.4.5's group stands in for the non-isolated one exactly, by §11.4.4's NOTE 3
///   (the backdrop is composited in and removed again). An element whose shape is its
///   coverage draws bare and every other states it ([`knockout_elements`]).
/// - **The mode at the `Do`**, where the elements that carry colour share one blend mode
///   that commutes with the weighted average ([`blend_at_the_do`]): the mode moves from
///   the elements to the group's `Do`, the elements draw Normal on transparency, and every
///   backend draws the result. ADR 1000 built this for §11.7.4.4's and §9.3.8's implicit
///   groups and priced this caller; ADR 1009 takes it, and `issue18032.pdf` is the page.
///   Tried before the construction below because two backends refuse that one.
/// - **The group's own backdrop**: "\[a] nonisolated knockout group composites its topmost
///   enclosing element with the group's backdrop." Where an element blends and no mode
///   moves, the group goes to the backends with `isolated: false` beside `knockout: true`,
///   every element a `Command::Shaped`, and a backend retains the initial backdrop beside
///   the accumulation (ADR 0327). Under a mode at the `Do` other than Normal the backend
///   takes the accumulation through §11.4.4's result step and composites it as one object
///   under that mode — §11.4.8 states the step once for every kind of group (ADR 1305).
///   Two conditions bound it, each the clause's: no
///   enclosing knockout group that is *isolated*, because there the elements are drawn on
///   transparency and this command would be seeded from the accumulation rather than from
///   either of the backdrops NOTE 6 contrasts — a non-isolated one keeps its initial backdrop
///   and hands each element a private clone of it (ADR 1256, ADR 1265); and every element's
///   shape statable, because the weighted average's factor has to come from somewhere.
///
/// The second construction asks for a Normal `Do` — the group has to have a mode to give
/// away — and its constant alpha and mask pass through the collapse unchanged
/// (`blend_at_the_do`). It does not share the third's first condition: an element of an
/// enclosing knockout group is composited with that group's initial backdrop, which NOTE 6
/// makes this group's own, so the derivation is the same one.
///
/// # The pair
///
/// A group compositing in a press is one content stream interpreted twice, and the second
/// list travels in `pair` (`pdf_render::GroupBlending::FourComponents`). A backend resolves
/// the two against one another per pixel, so each of the rewrites above is applied to both
/// halves or to neither — [`commit_knockout`], which is what lets
/// [`crate::content::Interpreter::group_press`] admit a knockout group at all. Where the black half will not
/// take the same rewrite the group falls to the flat drawing with the report §11.4.6 already
/// had, which is the answer it had before the pair existed.
#[expect(
    clippy::too_many_arguments,
    reason = "§11.4.6's inputs as the caller holds them; the last is the interpreter's one-colour \
              admission, which a pair's second attempt refuses (ADR 1657)"
)]
pub(super) fn knockout_construction(
    group: &TransparencyGroup,
    commands: Vec<Command>,
    pair: Option<pdf_render::GroupBlending>,
    alpha: Option<AlphaSource>,
    enclosing: Option<KnockoutKind>,
    outer_blend: BlendMode,
    shape_masks: &ShapeMasks,
    one_colour: &mut OneColour,
) -> KnockoutConstruction {
    // Whether §11.4.6's rule can change a pixel of this group, which decides whether its
    // initial backdrop and §11.4.4's immediate one are the same thing. Asked of the file's
    // own elements, before the rewrite that turns any of them into a `Command::Shaped` whose
    // bounds and blending this predicate cannot read.
    let knockout_shows = group.knockout && knockout_can_show(&commands);
    let mut construction = KnockoutConstruction {
        commands,
        pair,
        knockout: false,
        backdrop_composited: false,
        blend: outer_blend,
        knockout_shows,
    };
    let Some(alpha) = alpha.filter(|_| group.knockout) else {
        return construction;
    };
    // §11.4.6's NOTE 6, over this group's own elements: one that is itself a non-isolated group
    // takes *this* group's initial backdrop, which only a construction that keeps that backdrop
    // can hand over. The two routes below that draw the elements on transparency cannot — there
    // an element's backdrop is the accumulation — so a group holding such an element goes
    // straight to the own-backdrop construction. ADR 1256, ADR 1265.
    let nested_non_isolated = construction.commands.iter().any(is_a_non_isolated_group);
    let backdrop_transparent = enclosing == Some(KnockoutKind::Isolated);
    if !nested_non_isolated
        && (group.isolated
            || backdrop_transparent
            || !any_command(&construction.commands, &command_blends))
    {
        // The bare case rewrites nothing, so both halves of a pair are already the lists the
        // backends will resolve — but the question is still asked of each, because what the
        // backend reads off an element is that element's own shape.
        if knockout_shape_is_coverage(&construction.commands, alpha)
            && construction
                .pair
                .as_ref()
                .and_then(pdf_render::GroupBlending::black)
                .is_none_or(|black| knockout_shape_is_coverage(black, alpha))
        {
            construction.knockout = true;
        } else {
            commit_knockout(&mut construction, |commands| {
                knockout_elements(commands, alpha, shape_masks)
            });
        }
    } else if knockout_shows {
        // The third condition is what keeps `note_group_structure` honest rather than a
        // limit of the construction: a nested isolated group's inner elements blend against
        // its own transparency either way, but the report reads a group's tree, and a mode
        // left anywhere in it would be named as blending with the backdrop this group
        // excludes.
        // `nested_non_isolated` blocks this route for the reason above: moving the mode to the
        // `Do` draws the elements on transparency too. A `Do` under a mode of its own blocks
        // it as well, because there is no Normal left at the `Do` for a mode to move into.
        let mode = if nested_non_isolated || outer_blend != BlendMode::Normal {
            None
        } else {
            blend_at_the_do(&construction.commands, one_colour)
        };
        let moved = mode.is_some()
            && commit_knockout(&mut construction, |commands| {
                let stripped = commands
                    .iter()
                    .map(without_blend)
                    .collect::<Option<Vec<_>>>()?;
                if any_command(&stripped, &command_blends) {
                    return None;
                }
                transparent_knockout_elements(&stripped, alpha, shape_masks)
            });
        if let Some(mode) = mode.filter(|_| moved) {
            construction.blend = mode;
        } else if enclosing != Some(KnockoutKind::Isolated)
            && commit_knockout(&mut construction, |commands| {
                stated_elements(commands, alpha, shape_masks)
            })
        {
            construction.backdrop_composited = true;
        }
    }
    construction
}

/// Applies one of §11.4.6's rewrites to a group's elements, and to the black half of its
/// press pair beside them — or to neither.
///
/// A group compositing in a four-component space runs its content twice and the two lists are
/// resolved against one another per pixel (`pdf_render::GroupBlending::FourComponents`), so a
/// rewrite that reached one half and not the other would leave the backend converting a
/// colour against a shape that never drew it. The rewrite is a function of the list alone —
/// the halves differ only in what each colour resolved to — so it is run on both and
/// committed only where both succeed and the results still [`paired`].
///
/// Answers whether it was committed; where it was not, the caller's construction is untouched
/// and the group keeps §11.4.6's report.
fn commit_knockout(
    construction: &mut KnockoutConstruction,
    rewrite: impl Fn(&[Command]) -> Option<Vec<Command>>,
) -> bool {
    let Some(elements) = rewrite(&construction.commands) else {
        return false;
    };
    if let Some(pdf_render::GroupBlending::FourComponents { black, .. }) = &mut construction.pair {
        let Some(rewritten) = rewrite(black).filter(|rewritten| paired(&elements, rewritten))
        else {
            return false;
        };
        *black = rewritten;
    }
    construction.commands = elements;
    construction.knockout = true;
    true
}
