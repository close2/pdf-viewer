//! Recording an observation, once: a colour, a space or a profile, and the resource lookups that feed them.

use crate::finding::Where;
use pdf_syntax::Name;
use pdf_syntax::Object;

use super::{
    Context, DeviceColour, DeviceFamily, IccProfile, IccSelection, MAX_OBSERVATIONS,
    MissingResource, Observation, Route, SpaceKind, Walk, device_uses, icc_uses,
};

impl Walk<'_> {
    /// Records one device colour space selection at the site the walk is standing on.
    pub(super) fn record_colour(
        &mut self,
        used: (DeviceFamily, Route),
        context: &Context<'_>,
        what: &'static str,
        blending: Option<SpaceKind>,
    ) {
        let place = Where::page(context.page);
        self.push_colour(used, place, context, what, blending);
    }

    /// Every device colour space one named space reaches, recorded at this site.
    pub(super) fn record_space(
        &mut self,
        space: &Object,
        place: &Where,
        context: &Context<'_>,
        what: &'static str,
        blending: Option<SpaceKind>,
        blending_profile: Option<&IccProfile>,
    ) {
        for (profile, via) in icc_uses(self.document, space, context.resources) {
            self.push_icc(profile, via, place.clone(), context, what, blending_profile);
        }
        for used in device_uses(self.document, space, context.resources) {
            self.push_colour(used, place.clone(), context, what, blending);
        }
    }

    /// Records one `ICCBased` colour space selection, with the blending profile then in force.
    fn push_icc(
        &mut self,
        profile: IccProfile,
        via: Route,
        place: Where,
        context: &Context<'_>,
        what: &'static str,
        blending: Option<&IccProfile>,
    ) {
        let blending_id = blending.and_then(|blending| blending.id);
        if !self.first_time(Observation::Icc(
            context.page,
            what,
            via,
            profile.id,
            blending_id,
        )) {
            return;
        }
        let mut place = place.named("ICCBased");
        place.page = Some(context.page);
        self.survey.icc.push(IccSelection {
            page: context.page,
            place,
            what,
            via,
            profile,
            blending: blending.cloned(),
        });
    }

    /// The same, where the caller has a better witness than the page index.
    pub(super) fn push_colour(
        &mut self,
        used: (DeviceFamily, Route),
        place: Where,
        context: &Context<'_>,
        what: &'static str,
        blending: Option<SpaceKind>,
    ) {
        let (family, via) = used;
        let default = context.defaults[family.index()];
        if !self.first_time(Observation::Colour(
            context.page,
            family,
            via,
            what,
            default,
            blending,
        )) {
            return;
        }
        let mut place = place.named(family.name());
        place.page = Some(context.page);
        self.survey.colours.push(DeviceColour {
            family,
            page: context.page,
            place,
            what,
            via,
            default,
            blending,
        });
    }

    /// Looks one named resource up in the resources in force, recording a miss.
    ///
    /// ISO 32000-2 §7.8.3 makes a resource dictionary a table of subdictionaries keyed by
    /// category, so a name that reaches no entry is a name the content stream referenced and its
    /// resources did not define — which is the whole of ISO 19005-4 section 6.2.2's third sentence.
    pub(super) fn look_up(
        &mut self,
        category: &'static str,
        name: &[u8],
        context: &Context<'_>,
        what: &'static str,
    ) -> Option<Object> {
        // The name was referenced whether or not the resources define it, which is the fact
        // ISO 19005 section 6.2.2's second requirement turns on.
        if let Some(record) = context.record
            && let Some(opened) = self.survey.streams.get_mut(record)
        {
            opened.referenced = true;
        }
        let found = self.peek(category, name, context);
        self.note_selection(category, name, found.as_ref());
        if found.is_none() {
            let printed = String::from_utf8_lossy(name).into_owned();
            if self.first_time(Observation::Missing(
                context.page,
                category,
                printed.clone(),
            )) {
                self.survey.missing.push(MissingResource {
                    page: context.page,
                    category,
                    name: printed,
                    what,
                });
            }
        }
        found
    }

    /// Whether an observation is new, and worth the entry it would cost.
    pub(super) fn first_time(&mut self, observation: Observation) -> bool {
        if self.seen.len() >= MAX_OBSERVATIONS {
            self.survey.stopped_short = true;
            return false;
        }
        self.seen.insert(observation)
    }

    /// One named resource as the resources in force define it, with no side effect.
    ///
    /// [`Walk::look_up`] is this plus the two facts ISO 19005 section 6.2.2 reads — that the
    /// stream referenced a name, and that the name reached nothing — and the `Tf` record above
    /// wants the entry without either.
    pub(super) fn peek(
        &self,
        category: &'static str,
        name: &[u8],
        context: &Context<'_>,
    ) -> Option<Object> {
        let table = self.document.get_key(context.resources, category);
        table
            .as_dict()
            .and_then(|table| table.get_by_name(&Name::new(name.to_vec())))
            .cloned()
    }

    /// Tells the ledger what a lookup found, where [`super::Survey::ledgered`] asked.
    pub(super) fn note_selection(
        &mut self,
        category: &'static str,
        name: &[u8],
        found: Option<&Object>,
    ) {
        if let Some(ledger) = &mut self.ledger {
            ledger.select(category, name, found);
        }
    }
}
