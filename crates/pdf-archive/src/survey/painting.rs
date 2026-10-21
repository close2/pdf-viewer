//! A painting operator, and what marking in the colour space in force records.

use crate::finding::Where;

use super::{
    Cmyk, Context, DeviceFamily, IccCmykPaint, Observation, ProcessCmykOverprint, Route, Selected,
    State, Walk, blends_in_four_components,
};

impl Walk<'_> {
    /// A painting operator, which marks the page in whatever space is in force.
    ///
    /// The only case this adds to what selection already recorded is §8.6.8's initial space: a
    /// stream that paints without ever naming a colour space is painting in `DeviceGray`, and
    /// the file never wrote the operator that would have said so.
    pub(super) fn paint(&mut self, fill: bool, stroke: bool, state: &State, context: &Context<'_>) {
        let implicit = (fill && state.fill == Selected::Initial)
            || (stroke && state.stroke == Selected::Initial);
        if implicit {
            self.record_colour(
                (DeviceFamily::Gray, Route::Direct),
                context,
                "the initial colour space, which no operator replaced",
                state.blending.kind,
            );
        }
        if fill && state.fill_cmyk == Cmyk::Icc {
            self.record_icc_paint(false, state.overprint.fill, state, context);
        }
        if stroke && state.stroke_cmyk == Cmyk::Icc {
            self.record_icc_paint(true, state.overprint.stroke, state, context);
        }
        // §8.6.7: the mode "shall have an effect only when the overprint parameter is true".
        if state.overprint.mode != 0 {
            if fill && state.fill_cmyk == Cmyk::Process && state.overprint.fill {
                self.record_process_overprint(false, state, context);
            }
            if stroke && state.stroke_cmyk == Cmyk::Process && state.overprint.stroke {
                self.record_process_overprint(true, state, context);
            }
        }
    }

    /// One side of a painting operator that marked the page in `DeviceCMYK` under a non-zero
    /// overprint mode with overprinting on.
    fn record_process_overprint(&mut self, stroking: bool, state: &State, context: &Context<'_>) {
        let mode_set_by = state.overprint.mode_set_by;
        let in_four_components = blends_in_four_components(&state.blending);
        if !self.first_time(Observation::ProcessOverprint(
            context.page,
            stroking,
            mode_set_by,
            in_four_components,
        )) {
            return;
        }
        self.survey.process_overprints.push(ProcessCmykOverprint {
            page: context.page,
            stroking,
            mode_set_by,
            in_four_components,
        });
    }

    /// One side of a painting operator that marked the page in an `ICCBased` CMYK space.
    fn record_icc_paint(
        &mut self,
        stroking: bool,
        overprinting: bool,
        state: &State,
        context: &Context<'_>,
    ) {
        let mode = state.overprint.mode;
        if overprinting && mode == 1 {
            self.survey
                .icc_overprint_modes
                .insert(state.overprint.mode_set_by);
            self.survey.icc_overprint_blended |= blends_in_four_components(&state.blending);
        }
        if !self.first_time(Observation::Overprint(
            context.page,
            context.what,
            stroking,
            overprinting,
            mode,
        )) {
            return;
        }
        self.survey.icc_paints.push(IccCmykPaint {
            page: context.page,
            place: Where::page(context.page),
            what: context.what,
            stroking,
            overprinting,
            mode,
        });
    }
}
