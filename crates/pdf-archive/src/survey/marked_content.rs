//! Marked-content sequences: a `BDC` operand read as a property list, and the language and replacement text it states.

use pdf_syntax::Dictionary;
use pdf_syntax::Object;
use std::borrow::Cow;

use super::{
    Context, MAX_MARK_DEPTH, Mark, MarkedLanguage, Observation, Operands, PropertyList, Walk,
};

impl Walk<'_> {
    /// Opens one of §14.6.1's marked-content sequences, with what its property list says.
    pub(super) fn begin_marked(&mut self, operands: &Operands<'_>, context: &Context<'_>) {
        let mark = match self.property_list(operands, context) {
            // `BMC`, whose sequence carries no property list at all: §14.6.1 gives it a tag and
            // nothing else, so nothing it encloses is covered by anything.
            PropertyList::Absent => Mark::default(),
            PropertyList::Unreadable => Mark {
                settles: true,
                mcid: None,
                suppressed: 0,
            },
            PropertyList::Read(list) => {
                self.note_language(&list, context.page);
                Mark {
                    // §14.9.4 puts the entry in the property list of a sequence, and what it
                    // replaces is everything the sequence encloses. The *value* is not read
                    // here: whether it is a text string or a name is that rule's question, and
                    // this one is only whether the entry stands.
                    settles: !self.document.get_key(&list, "ActualText").is_null(),
                    mcid: self.document.get_key(&list, "MCID").as_integer(),
                    suppressed: 0,
                }
            }
        };
        if self.marks.len() < MAX_MARK_DEPTH {
            self.marks.push(mark);
        } else if let Some(top) = self.marks.last_mut() {
            top.suppressed = top.suppressed.saturating_add(1);
            top.settles = true;
        }
    }

    /// §14.6.2's property list, written into the operands or named through the resources.
    ///
    /// **A named list is read through [`Walk::look_up`]**, so a name the `/Properties`
    /// subdictionary does not define is recorded as a miss and the stream as having referenced a
    /// resource — the two facts ISO 19005 section 6.2.2's rows read. ISO 32000-2 §7.8.3's
    /// Table 34 lists `/Properties` among a resource dictionary's categories, and §14.6.2 makes
    /// the name form of a property list "a named resource" of it, so the clause's "all named
    /// resources referenced by this content stream" includes it. `issue14438`'s annotation
    /// appearance, `/OC /MC0 BDC` with no `/Properties` at all, is the document this was missing
    /// (`doc/adr/2684`).
    pub(super) fn property_list<'o>(
        &mut self,
        operands: &Operands<'o>,
        context: &Context<'_>,
    ) -> PropertyList<'o> {
        if let Some(inline) = operands.property {
            return PropertyList::Read(Cow::Borrowed(inline));
        }
        // §14.6.2's second form: the tag and then a name, so the name is the last of the two.
        if operands.names.len() < 2 {
            return PropertyList::Absent;
        }
        let Some(name) = operands.names.last() else {
            return PropertyList::Absent;
        };
        let found = self.look_up("Properties", name, context, context.what);
        let Some(found) = found else {
            return PropertyList::Unreadable;
        };
        match self.document.resolve(&found) {
            Object::Dictionary(dict) => PropertyList::Read(Cow::Owned(dict)),
            _ => PropertyList::Unreadable,
        }
    }

    /// Keeps a `/Lang` a marked-content property list stated, once per page and value.
    ///
    /// ISO 19005-2 section 6.7.4 requires a `/Lang` that is present to be a language identifier
    /// wherever it stands, and a property list is one of the three places it can stand.
    pub(super) fn note_language(&mut self, list: &Dictionary, page: usize) {
        let stated = self.document.get_key(list, "Lang");
        if stated.is_null() {
            return;
        }
        let value = stated
            .as_string()
            .map(<[u8]>::to_vec)
            .ok_or_else(|| stated.type_name());
        if !self.first_time(Observation::Language(page, value.clone())) {
            return;
        }
        self.survey.languages.push(MarkedLanguage { page, value });
    }
}
