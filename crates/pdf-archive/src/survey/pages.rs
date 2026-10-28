//! One page, its group, and its annotations' appearances: where a walk starts and what it records about a page as a whole.

use crate::finding::Where;
use pdf_model::content::ledger::Route as Via;
use pdf_model::content::reader::ContentReader;
use pdf_model::page::Page;
use pdf_syntax::Dictionary;
use pdf_syntax::Object;
use pdf_syntax::ObjectId;

use super::{
    Blending, DefaultSpace, GroupSpace, MAX_OBSERVATIONS, Observation, OpenedStream, Origin,
    SpaceKind, State, Walk, below_one, blends, classify, icc_profile,
};

impl Walk<'_> {
    /// One page: its group, its content, and its annotations' appearances.
    pub(super) fn page(&mut self, page: &Page, index: usize) {
        // §7.8.3: a page's content is associated with the dictionary its own `Resources` entry
        // designates *or* one it inherits, and only the first of those is explicit. Read the
        // same way a form XObject's entry is, so that an entry reaching no dictionary counts
        // as no association rather than as a broken one.
        let own = self
            .document
            .get_key(&page.dict, "Resources")
            .as_dict()
            .is_some();
        let blending = self.page_group(page, index, own);
        let state = State {
            blending: blending.clone(),
            ..State::default()
        };
        let origin = self.open_record(
            index,
            None,
            Where::page(index),
            "the page content",
            own,
            false,
        );
        self.page_resources = page.resources.clone();
        let mut reader = ContentReader::for_page(self.document, page);
        if let Some(ledger) = &mut self.ledger {
            ledger.enter(Via::Page, page.id);
        }
        self.run(&mut reader, &page.resources, index, 0, &state, origin);
        if let Some(ledger) = &mut self.ledger {
            ledger.leave();
        }
        self.annotations(page, index, &blending);
    }

    /// Notes that a content stream has been opened, and answers with what [`Walk::run`] needs.
    pub(super) fn open_record(
        &mut self,
        page: usize,
        object: Option<ObjectId>,
        place: Where,
        what: &'static str,
        own_resources: bool,
        through_page_table: bool,
    ) -> Origin {
        let record = (self.survey.streams.len() < MAX_OBSERVATIONS).then(|| {
            self.survey.streams.push(OpenedStream {
                page,
                object,
                place,
                what,
                referenced: false,
                own_resources,
                through_page_table,
            });
            self.survey.streams.len().saturating_sub(1)
        });
        Origin {
            what,
            record,
            explicitly_associated: own_resources,
        }
    }

    /// A page's `/Group`, which ISO 32000-2 §11.4.7 makes the page's blending colour space.
    ///
    /// The clause is explicit that a page group's `CS` is honoured whatever its `I` entry says,
    /// because a page imposed on the output medium is effectively isolated — so, unlike a form
    /// `XObject`'s group below, no isolation test guards this one.
    fn page_group(&mut self, page: &Page, index: usize, explicitly_associated: bool) -> Blending {
        let group = self.document.get_key(&page.dict, "Group");
        let Some(group) = group.as_dict() else {
            return Blending::default();
        };
        if !self.is_transparency_group(group) {
            return Blending::default();
        }
        self.record_group_space(
            group,
            &page.resources,
            index,
            Where::page(index),
            explicitly_associated,
        )
    }

    /// Whether a group attributes dictionary is a transparency group: §11.6.6, Table 147.
    pub(super) fn is_transparency_group(&self, group: &Dictionary) -> bool {
        self.document
            .get_key(group, "S")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Transparency")
    }

    /// Records one group's `CS` entry and answers with the blending space it establishes.
    pub(super) fn record_group_space(
        &mut self,
        group: &Dictionary,
        resources: &Dictionary,
        page: usize,
        place: Where,
        explicitly_associated: bool,
    ) -> Blending {
        let Some(stated) = group.get("CS") else {
            return Blending::default();
        };
        let stated = self.document.resolve(stated);
        let kind = classify(self.document, &stated, resources, 0);
        let in_force = match kind {
            SpaceKind::Device(family) => self.default_space(resources, family),
            _ => None,
        };
        let default = DefaultSpace::read(in_force, explicitly_associated);
        if self.first_time(Observation::Group(page, kind, default)) {
            self.survey.groups.push(GroupSpace {
                kind,
                page,
                place: place.named("CS"),
                default,
            });
        }
        Blending {
            kind: Some(kind),
            profile: icc_profile(self.document, &stated, resources, 0),
        }
    }

    /// Walks every appearance stream a page's annotations carry.
    ///
    /// ISO 32000-2 §12.5.5 makes an `/AP` entry either a stream or a dictionary of states, and
    /// each of the three entries is drawn under some condition, so all of them are content the
    /// document renders. §12.5.2's own `BM` entry is part of the graphics state the appearance
    /// is drawn in, so Annex Q's blend-mode test applies to it as it does to a `gs`.
    fn annotations(&mut self, page: &Page, index: usize, blending: &Blending) {
        let Object::Array(annotations) = self.document.get_key(&page.dict, "Annots") else {
            return;
        };
        let state = State {
            blending: blending.clone(),
            ..State::default()
        };
        for annotation in &annotations {
            let Object::Dictionary(annotation) = self.document.resolve(annotation) else {
                continue;
            };
            if self.annotation_is_transparent(&annotation) {
                self.survey.transparent.insert(index);
            }
            let appearances = self.document.get_key(&annotation, "AP");
            let Some(appearances) = appearances.as_dict() else {
                continue;
            };
            for (_, entry) in appearances.iter() {
                self.appearance(entry, index, &state);
            }
        }
    }

    /// Whether an annotation's own dictionary puts transparency in the graphics state.
    fn annotation_is_transparent(&self, annotation: &Dictionary) -> bool {
        blends(self.document, annotation) || below_one(&self.document.get_key(annotation, "CA"))
    }

    /// One `/AP` entry, which is either the stream itself or a dictionary of appearance states.
    fn appearance(&mut self, entry: &Object, index: usize, state: &State) {
        let id = entry.as_reference();
        match self.document.resolve(entry) {
            Object::Stream(stream) => {
                self.form(&stream, id, index, 0, state, Via::Appearance, false);
            }
            Object::Dictionary(states) => {
                for (_, one) in states.iter() {
                    let id = one.as_reference();
                    if let Object::Stream(stream) = self.document.resolve(one) {
                        self.form(&stream, id, index, 0, state, Via::Appearance, false);
                    }
                }
            }
            _ => {}
        }
    }
}
