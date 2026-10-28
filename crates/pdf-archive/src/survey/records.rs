//! Recording an observation, once: a colour, a space or a profile, and the resource lookups that feed them.

use crate::finding::Where;
use pdf_syntax::Name;
use pdf_syntax::Object;

use super::{
    Context, DeviceColour, DeviceFamily, IccProfile, IccSelection, MAX_OBSERVATIONS,
    MissingInStream, MissingResource, MovingShow, Observation, Route, SpaceKind, Walk, device_uses,
    icc_uses,
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
        let mut holder = (None, false);
        if let Some(record) = context.record
            && let Some(opened) = self.survey.streams.get_mut(record)
        {
            opened.referenced = true;
            holder = (opened.object, opened.own_resources);
        }
        let found = self.peek(category, name, context);
        self.note_selection(category, name, found.as_ref());
        if found.is_none() {
            if self.first_time(Observation::MissingIn(
                context.page,
                category,
                name.to_vec(),
                holder.0,
            )) {
                self.survey.missing_in.push(MissingInStream {
                    page: context.page,
                    category,
                    name: name.to_vec(),
                    what,
                    object: holder.0,
                    own_resources: holder.1,
                });
            }
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

    /// Records that a text-showing operator ran under a font name nothing defines, and did more
    /// than show glyphs (`doc/adr/2500`).
    pub(super) fn note_moving_show(&mut self, name: &[u8], context: &Context<'_>) {
        if self.peek("Font", name, context).is_some() {
            return;
        }
        let object = context
            .record
            .and_then(|record| self.survey.streams.get(record))
            .and_then(|opened| opened.object);
        if self.first_time(Observation::MovingShow(context.page, name.to_vec(), object)) {
            self.survey.moving_shows.push(MovingShow {
                page: context.page,
                name: name.to_vec(),
                object,
            });
        }
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
    ///
    /// An entry whose value is null, written so or as a reference to an object the file does
    /// not define, defines nothing. ISO 32000-2 §7.3.9:
    ///
    /// > An indirect object reference (see 7.3.10, "Indirect objects") to a nonexistent object
    /// > shall be treated the same as a null object. Specifying the null object as the value of
    /// > a dictionary entry (7.3.7, "Dictionary objects") shall be equivalent to omitting the
    /// > entry entirely.
    ///
    /// So the value is resolved before it counts as found, and the entry is returned as the
    /// file wrote it, the reference kept, for the caller that follows it (`doc/adr/2550`).
    pub(super) fn peek(
        &self,
        category: &'static str,
        name: &[u8],
        context: &Context<'_>,
    ) -> Option<Object> {
        let name = Name::new(name.to_vec());
        // A category table written directly is read where it stands. `get_key` hands back an
        // owned copy of it, and a copy of a table of n names for each of n lookups makes a walk
        // quadratic in the table (`doc/adr/2500`).
        let entry = if let Some(Object::Dictionary(table)) = context.resources.get(category) {
            table.get_by_name(&name).cloned()
        } else {
            let table = self.document.get_key(context.resources, category);
            table
                .as_dict()
                .and_then(|table| table.get_by_name(&name))
                .cloned()
        }?;
        (!self.document.resolve(&entry).is_null()).then_some(entry)
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
