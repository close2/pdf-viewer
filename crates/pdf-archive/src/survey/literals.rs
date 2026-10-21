//! The values written inside a content stream, measured against ISO 19005-2 section 6.1.13's limits, and the operators no summary lists.

use crate::finding::Where;
use pdf_syntax::Dictionary;
use pdf_syntax::Name;
use pdf_syntax::Object;

use super::{Extent, Extreme, MAX_INLINE_DEPTH, Observation, Origin, UnlistedOperator, Walk};

impl Walk<'_> {
    /// Widens [`super::ContentLiterals`]' two integer extremes by one operand.
    ///
    /// Two comparisons per integer token, which is what putting this on the walk's hottest path
    /// costs; `examples/cost` is where that was checked rather than assumed.
    pub(super) fn note_integer(&mut self, value: i64, page: usize) {
        let literals = &mut self.survey.literals;
        if literals
            .largest_integer
            .is_none_or(|held| value > held.value)
        {
            literals.largest_integer = Some(Extreme { value, page });
        }
        if literals
            .smallest_integer
            .is_none_or(|held| value < held.value)
        {
            literals.smallest_integer = Some(Extreme { value, page });
        }
    }

    /// Widens [`super::ContentLiterals`]' two real extremes by one operand, as magnitudes.
    ///
    /// A magnitude that is not finite is left out: no PDF real *literal* spells one, so an
    /// infinity here is what the lexer did with a run of digits too long for a `f64` rather
    /// than what the file said, and reading a bound off it would report the reader's arithmetic.
    pub(super) fn note_real(&mut self, value: f64, page: usize) {
        let value = value.abs();
        if !value.is_finite() {
            return;
        }
        let literals = &mut self.survey.literals;
        if literals
            .largest_real_magnitude
            .is_none_or(|held| value > held.value)
        {
            literals.largest_real_magnitude = Some(Extreme { value, page });
        }
        if value > 0.0
            && literals
                .smallest_real_magnitude
                .is_none_or(|held| value < held.value)
        {
            literals.smallest_real_magnitude = Some(Extreme { value, page });
        }
    }

    /// Widens the string or name extreme of [`super::ContentLiterals`] by one operand's length.
    pub(super) fn note_length(&mut self, value: usize, page: usize, of: Extent) {
        let literals = &mut self.survey.literals;
        let held = match of {
            Extent::String => &mut literals.longest_string,
            Extent::Name => &mut literals.longest_name,
        };
        if held.is_none_or(|held| value > held.value) {
            *held = Some(Extreme { value, page });
        }
    }

    /// Records how deep a `q` stack stood, where it is deeper than anything seen before.
    pub(super) fn note_nesting(&mut self, depth: usize, page: usize) {
        if self.survey.nesting.is_none_or(|held| depth > held.value) {
            self.survey.nesting = Some(Extreme { value: depth, page });
        }
    }

    /// Keeps one `/ActualText` value, once per page and string, within its byte budget.
    ///
    /// The budget is spent rather than compared, so a document cannot make this vector large by
    /// stating many distinct entries any more than by stating one enormous one.
    fn note_actual_text(&mut self, text: &[u8], page: usize) {
        if text.len() > self.actual_text_budget
            || !self.first_time(Observation::ActualText(page, text.to_vec()))
        {
            return;
        }
        self.actual_text_budget = self.actual_text_budget.saturating_sub(text.len());
        self.survey.actual_texts.push((page, text.to_vec()));
    }

    /// Reads a dictionary a content stream wrote inline, for the three things it can hold.
    ///
    /// Its operands are operands: ISO 19005-2 section 6.1.13's limits are on the values written
    /// inside a content stream, and a number written inside a property list is one of them —
    /// which is why this walks the whole dictionary rather than its top level. §14.9.4's entry
    /// is here where a producer wrote the list inline, which is no object at all: nothing that
    /// walks the cross-reference table can see it. And the *name* `ActualText` is the wider
    /// fact [`super::Survey::names_actual_text`] keeps, which no budget may narrow.
    ///
    /// Each entry costs one token of the walk's budget. That is an under-count of what the
    /// tokens of a dictionary really were — a key and its value are at least two — and it is
    /// the direction that keeps a bound from ending a walk early over an estimate.
    pub(super) fn note_inline_dictionary(&mut self, dict: &Dictionary, page: usize) {
        self.note_inline_entries(dict, page, 0);
    }

    /// The entries of one dictionary written inline, at a stated nesting depth.
    fn note_inline_entries(&mut self, dict: &Dictionary, page: usize, depth: u32) {
        for (name, value) in dict.iter() {
            self.survey.names_actual_text |= name.as_bytes() == b"ActualText";
            self.note_length(name.as_bytes().len(), page, Extent::Name);
            self.note_inline_object(value, page, Some(name), depth.saturating_add(1));
        }
    }

    /// One value inside a dictionary a content stream wrote inline; see above.
    fn note_inline_object(&mut self, object: &Object, page: usize, key: Option<&Name>, depth: u32) {
        if depth >= MAX_INLINE_DEPTH || self.budget == 0 {
            return;
        }
        self.budget = self.budget.saturating_sub(1);
        match object {
            Object::Integer(value) => self.note_integer(*value, page),
            Object::Real(value) => self.note_real(*value, page),
            Object::Name(name) => self.note_length(name.as_bytes().len(), page, Extent::Name),
            Object::String(text) => {
                self.note_length(text.len(), page, Extent::String);
                if key.is_some_and(|key| key.as_bytes() == b"ActualText") {
                    self.note_actual_text(text, page);
                }
            }
            Object::Array(items) => {
                for item in items {
                    self.note_inline_object(item, page, None, depth.saturating_add(1));
                }
            }
            Object::Dictionary(dict) => self.note_inline_entries(dict, page, depth),
            _ => {}
        }
    }

    /// Records a keyword no operator summary lists, once per page, stream kind and spelling.
    pub(super) fn note_unlisted_operator(&mut self, word: &[u8], page: usize, origin: Origin) {
        if !self.first_time(Observation::Operator(page, origin.what, word.to_vec())) {
            return;
        }
        let place = origin
            .record
            .and_then(|at| self.survey.streams.get(at))
            .map_or_else(|| Where::page(page), |stream| stream.place.clone());
        self.survey.unlisted.push(UnlistedOperator {
            page,
            place,
            what: origin.what,
            spelling: word.to_vec(),
        });
    }
}
