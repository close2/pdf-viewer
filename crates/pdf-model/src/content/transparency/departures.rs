//! What a group asked for and did not get: the reports of §11.4's constructions that the display
//! list cannot carry, made once the group has been drawn (ADR 2080).

use pdf_render::Command;

use crate::colour::Compositing;

use crate::content::Interpreter;
use crate::content::report::Unsupported;

use super::TransparencyGroup;
use super::blending_space::Departure;
use super::commands::{any_command, command_blends, command_composites};
use super::knockout::{knockout_can_show, unstatable_shape};
use super::readings::AlphaSourcesSeen;

/// What was decided about a group, for the reports that ask what it did not get.
///
/// Grouped because these four always travel together and a report reads three of them
/// against one entry of Table 145 apiece.
#[derive(Debug, Clone, Copy)]
pub(super) struct GroupDrawn {
    /// Whether §11.4.6's rule reached the display list rather than a report.
    pub(super) knockout: bool,
    /// What the elements were composited onto — see `Command::Group`'s `isolated`.
    pub(super) isolated: bool,
    /// §11.4.6's NOTE 6 — see [`Interpreter::enclosing_knockout`].
    pub(super) backdrop_transparent: bool,
    /// Which readings of §11.6.4.3's `/AIS` the group's content painted under.
    pub(super) alpha_sources: AlphaSourcesSeen,
}

impl Interpreter<'_> {
    /// Reports the parts of §11.4 this group asks for and does not get.
    ///
    /// A group is composited here under its own constant alpha and blend mode, onto the
    /// backdrop [`GroupDrawn::isolated`] names. Three of Table 145's answers can ask for more
    /// than the display list carries, and each is reported only where it can change a pixel —
    /// a report that fires where the output is provably identical costs the page its place
    /// in the oracle's comparison and buys nothing.
    pub(super) fn note_group_departures(
        &mut self,
        group: &TransparencyGroup,
        commands: &[Command],
        drawn: GroupDrawn,
        introduced: Option<&Departure>,
    ) {
        self.note_group_structure(group, commands, drawn);

        // §11.6.6: for an isolated group, a `/CS` means "all painting operators shall
        // convert source colours ... to the group colour space before compositing objects
        // into the group", and the result is interpreted in that space. Compositing here
        // happens on the device's RGB components, so a group asking for any other space is
        // blended with different arithmetic — visible only where something composites at
        // all, since an opaque Normal paint carries its colour through unchanged, **except
        // in a space of one component**, where the conversion in keeps a grey and nothing
        // else (`Departure::loses_chroma`).
        //
        // `introduced` is `run_transparency_group`'s answer to *which* space, after
        // §11.6.6's inheritance and §11.7.2's rule about a non-isolated group. A group that
        // inherits a departing space is drawn no better and no worse than the page or group
        // that introduced it, and that one carries the report.
        if let Some(departure) = introduced
            && self.compositing == Compositing::Device
            && (departure.loses_chroma() || any_command(commands, &command_composites))
        {
            self.note(Unsupported::TransparencyGroup {
                detail: format!("blending colour space {}", departure.name),
            });
        }
    }

    /// Reports Table 145's `/I` and `/K`, which mean the same thing wherever a group is used.
    ///
    /// Split from [`Interpreter::note_group_departures`] because a *soft mask's* group asks
    /// the same two questions and a different colour-space question: §11.6.5.1 makes its
    /// `/CS` the space the mask's luminosity is computed in, where §11.6.6 makes a painted
    /// group's the space its elements are composited in. `crate::soft_mask` decides the
    /// first; this decides what the two share.
    ///
    /// [`GroupDrawn::backdrop_transparent`] is §11.4.6's NOTE 6: a group that is a direct
    /// element of a knockout group whose initial backdrop is transparent **is** §11.4.5's
    /// isolated group by that clause's own definition, whatever Table 145's `/I` says here,
    /// so both questions below are asked of that rather than of the entry. See
    /// [`Interpreter::enclosing_knockout`], and `knockout_inner_backdrop.pdf` is
    /// the page that showed the difference: its inner group states `/I false` inside an
    /// isolated knockout group, is drawn on the transparency the clause asks for, and was
    /// reported as departing from it (ADR 0307).
    pub(super) fn note_group_structure(
        &mut self,
        group: &TransparencyGroup,
        commands: &[Command],
        drawn: GroupDrawn,
    ) {
        let GroupDrawn {
            knockout: knockout_drawn,
            isolated: isolated_drawn,
            backdrop_transparent,
            alpha_sources,
        } = drawn;
        let isolated_by_clause = group.isolated || backdrop_transparent;
        // §11.4.4 composites a non-isolated group's elements onto the group's backdrop and
        // then removes that backdrop's contribution again (its NOTE 3). Under the Normal
        // blend mode the removal is exact and the backdrop cancels, which is what §11.6.7's
        // NOTE 1 states for the same computation applied to a pattern cell: "in the common
        // case in which the pattern consists entirely of objects painted with the Normal
        // blend mode, this behaviour can be optimised by treating the pattern cell as if it
        // were an isolated group. Since in this case the results depend only on the colour,
        // shape, and opacity of the pattern cell and not on those of the backdrop". So a
        // group all of whose elements paint Normal is drawn as an isolated one whatever it
        // says, and only a blend mode inside it can tell the difference — the same sentence
        // §11.4.4's NOTE 2 gives as the reason the two kinds of group differ at all.
        //
        // Where one does blend, the display list states the group's backdrop instead of
        // substituting §11.4.5's (ADR 0237), and `isolated_drawn` is false. What is left
        // here is the population that construction refuses: a knockout group and an element
        // of one — plus a *mask* group, which is evaluated into a raster built on
        // transparency whatever it declares. A blend mode at the `Do` is not in this
        // population (ADR 1107): the group's own colour is computed there rather than
        // substituted for.
        if !isolated_by_clause
            && isolated_drawn
            && any_command(commands, &|command| command_blends(command))
        {
            self.note(Unsupported::TransparencyGroup {
                detail: "non-isolated, and an element blends with the backdrop it excludes"
                    .to_owned(),
            });
        }

        // §11.4.6: "In a knockout group, each individual element shall be composited with
        // the group's initial backdrop rather than with the stack of preceding elements in
        // the group." Where the upper of two overlapping elements is opaque and blends
        // Normal it overwrites either way, so the two models differ only where a later
        // element that composites covers an earlier one — which is the condition below,
        // and the same shape as §9.3.8's for a text object.
        // The display list can carry the rule itself, for
        // the groups whose elements have a shape a rasteriser can draw — see
        // [`knockout_shape_is_coverage`] — and since ADR 0234 for those whose shape it can
        // *state*, see [`stated_shape`]. `knockout_drawn` answers both. What is left here
        // is the population they refuse, and the report says which of the two refused it:
        // a report that names its condition is worth more than one that names its clause,
        // and this one had named neither.
        if group.knockout && !knockout_drawn && knockout_can_show(commands) {
            // Ordered by how precisely each condition is known, not by how it is tested:
            // the first names an element, the second names this group, and the third names
            // a history of the group's whole run and over-approximates within it. More than
            // one can hold, and the most precise true statement is the one worth printing.
            // The first is asked only where there *is* a settled reading of `/AIS`, because
            // which elements have a statable shape is a different question under each — and
            // where there is not, the third is the answer by construction. This group was
            // refused, so its elements are the ones `settled_over` was asked about at the
            // close and the answer here is that one.
            let refusal = if let Some(element) = alpha_sources
                .settled_over(commands)
                .and_then(|alpha| unstatable_shape(commands, alpha, self.image_masks.shape_masks()))
            {
                element
            } else if !isolated_by_clause && any_command(commands, &command_blends) {
                "non-isolated, and an element blends with the backdrop it excludes"
            } else {
                "an element painted under the other reading of /AIS, whose shape could not be \
                 stated beside it, and §11.6.4.3 gives the mask and the alpha constants a \
                 different meaning under each"
            };
            self.note(Unsupported::TransparencyGroup {
                detail: format!("knockout, and an element composites over another ({refusal})"),
            });
        }
    }
}
