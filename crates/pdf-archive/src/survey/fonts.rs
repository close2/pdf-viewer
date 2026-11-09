//! A text-showing operator, a font selection, and a Type 3 font's glyph procedures.

use pdf_model::content::ledger::Route as Via;
use pdf_syntax::Dictionary;
use pdf_syntax::Object;
use pdf_syntax::object::Name;
use std::collections::BTreeMap;

use std::collections::BTreeSet;

use pdf_syntax::object::ObjectId;

use super::{
    Context, FontKey, Glyph, GlyphsShown, Mark, Operands, Replacement, SelectedFont, State, Walk,
    widen,
};

impl Walk<'_> {
    /// A text-showing operator: the font it ran with, and the marks it made.
    pub(super) fn show(&mut self, state: &State, operands: &Operands<'_>, context: &Context<'_>) {
        let Some(name) = state.font.clone() else {
            return;
        };
        // Mode 3 alone is exempt: mode 7 adds the glyphs to the clipping path, which is a mark
        // on the page by way of everything drawn afterwards.
        let font = self.select(&name, context, state.mode != 3, operands.strings);
        if font.is_none() {
            self.select_undefined(&name, context, state.mode != 3, operands.strings);
            // Whether an empty font would have done more than the reader did without one: see
            // [`MovingShow`]. A clipping mode adds the glyphs to the clip, and an empty set of
            // glyphs is not the clip the reader kept, so it is recorded at once; a spacing moves
            // the position, which is a mark only if a later glyph is placed from it, and the
            // stream's loop decides that.
            if !(0..=3).contains(&state.mode) {
                if let Some(show) = self.moving_show(&name, context) {
                    self.record_moving_show(show);
                }
            } else if state.char_spaced || state.word_spaced {
                self.displacing = self.moving_show(&name, context);
            }
        }
        // ISO 32000-2 §9.3.6: modes 0, 2, 4 and 6 fill and modes 1, 2, 5 and 6 stroke, so the
        // colour space in force is used exactly as a path-painting operator would use it.
        let fills = matches!(state.mode, 0 | 2 | 4 | 6);
        let strokes = matches!(state.mode, 1 | 2 | 5 | 6);
        self.paint(fills, strokes, state, context);
        if let Some((font, id)) = font {
            if let Some(id) = id {
                self.note_type3_glyphs(&font, id, operands, context.page);
            }
            self.type3(&font, state, context);
        }
    }

    /// Records that a content stream selected the font a resource name stands for.
    fn select(
        &mut self,
        name: &[u8],
        context: &Context<'_>,
        rendered: bool,
        strings: &[Vec<u8>],
    ) -> Option<(Dictionary, Option<ObjectId>)> {
        let entry = self.look_up("Font", name, context, context.what)?;
        let id = entry.as_reference();
        let Object::Dictionary(dict) = self.document.resolve(&entry) else {
            return None;
        };
        let key = id.map_or_else(
            || FontKey::Direct(context.page, name.to_vec()),
            FontKey::Indirect,
        );
        let entry = self
            .survey
            .fonts
            .entry(key)
            .or_insert_with(|| SelectedFont {
                dict: dict.clone(),
                id,
                page: context.page,
                name: String::from_utf8_lossy(name).into_owned(),
                rendered: false,
                shown: BTreeMap::new(),
                shown_complete: true,
            });
        note_shown(
            entry,
            rendered,
            strings,
            &self.marks,
            context.page,
            &mut self.shown_budget,
        );
        Some((dict, id))
    }

    /// Records the text shown under a font name nothing defines, where the name is one of
    /// §9.6.2.2's fourteen, which this reader draws from its own faces (`doc/adr/2500`).
    ///
    /// Kept apart from [`super::Survey::fonts`], which holds the fonts a file defines: the entry's
    /// dictionary is [`standard_font_definition`], the one a conversion would write for the name,
    /// so that the codes drawn are known before it is written. One entry for each distinct name,
    /// whichever page or stream made the miss, because every miss of a name is defined as the same
    /// font.
    pub(super) fn select_undefined(
        &mut self,
        name: &[u8],
        context: &Context<'_>,
        rendered: bool,
        strings: &[Vec<u8>],
    ) {
        if self.peek("Font", name, context).is_some() {
            return;
        }
        let Some(dict) = standard_font_definition(name) else {
            return;
        };
        let entry = self
            .survey
            .undefined_fonts
            .entry(name.to_vec())
            .or_insert_with(|| SelectedFont {
                dict,
                id: None,
                page: context.page,
                name: String::from_utf8_lossy(name).into_owned(),
                rendered: false,
                shown: BTreeMap::new(),
                shown_complete: true,
            });
        note_shown(
            entry,
            rendered,
            strings,
            &self.marks,
            context.page,
            &mut self.shown_budget,
        );
    }

    /// Records which glyph procedures of a Type 3 font stating no `/Resources` this show
    /// selected on this page (`doc/adr/2746`).
    ///
    /// ISO 32000-2 §9.6.4's Table 110: a Type 3 font is simple, so each byte is a code, and its
    /// `Encoding` "shall" be a dictionary whose `Differences` array specifies the complete
    /// character encoding, mapping each code to a name that `CharProcs` maps to its procedure.
    /// Anything that does not follow that path is [`GlyphsShown::Unknown`], which counts every
    /// procedure as shown.
    fn note_type3_glyphs(
        &mut self,
        font: &Dictionary,
        id: ObjectId,
        operands: &Operands<'_>,
        page: usize,
    ) {
        if !is_type3_without_resources(self.document, font) {
            return;
        }
        let document = self.document;
        let codes = self
            .type3_codes
            .entry(id)
            .or_insert_with(|| type3_codes(document, font));
        let mut shown = BTreeSet::new();
        let mut unknown = !operands.strings_complete;
        for code in operands.strings.iter().flatten() {
            // A code is a byte and the table has 256 entries, so the index is always in it.
            match codes[usize::from(*code)] {
                Glyph::Procedure(procedure) => {
                    shown.insert(procedure);
                }
                Glyph::Absent => {}
                Glyph::Unknown => unknown = true,
            }
        }
        let entry = self
            .survey
            .type3_glyphs
            .entry((page, id))
            .or_insert_with(|| GlyphsShown::These(BTreeSet::new()));
        match entry {
            GlyphsShown::These(_) if unknown => *entry = GlyphsShown::Unknown,
            GlyphsShown::These(set) => set.extend(shown),
            GlyphsShown::Unknown => {}
        }
    }

    /// ISO 32000-2 §9.6.4: a Type 3 font's glyphs are content streams of their own.
    ///
    /// **The number used to be §9.6.5, which is a subclause of both editions and is neither of
    /// the ones meant.** It is *Type 3 fonts* in ISO 32000-1:2008 — the edition PDF/A-2 adheres
    /// to — and *Character encoding* in ISO 32000-2, whose Type 3 subclause is §9.6.4;
    /// `crate::table::fonts`'s module comment carries the whole mapping.
    ///
    /// Annex Q.5 says the same thing for transparency, and section 6.2.4.3's corpus says it for
    /// colour: what a glyph procedure paints is painted on the page that showed the glyph.
    fn type3(&mut self, font: &Dictionary, state: &State, context: &Context<'_>) {
        let subtype = self.document.get_key(font, "Subtype");
        if subtype
            .as_name()
            .is_none_or(|name| name.as_bytes() != b"Type3")
        {
            return;
        }
        // §9.6.4: a Type 3 font's own `/Resources` are what its glyph procedures are read
        // against, and a font that states none is read against the **page's**. §7.8.3's search
        // ends there — the glyph description's own dictionary, the font's, then the page and
        // what §7.7.3.4 gave it (Errata Collection 3, Issue #128) — and not at the stream whose
        // text-showing operator reached the glyph, which is a different dictionary whenever that
        // stream is a form with `/Resources` of its own. The same reading `pdf_model`'s
        // interpreter has since ADR 1059, and the cross-check is calibrated on the place the two
        // part. A glyph description's own `/Resources`, that search's first step, is read where
        // the glyph is opened.
        let own = self.document.get_key(font, "Resources");
        let stated = own.as_dict().is_some();
        let resources = own
            .as_dict()
            .cloned()
            .unwrap_or_else(|| (*self.page_resources).clone());
        let procedures = self.document.get_key(font, "CharProcs");
        let Some(procedures) = procedures.as_dict() else {
            return;
        };
        for (_, procedure) in procedures.iter() {
            let id = procedure.as_reference();
            let Object::Stream(stream) = self.document.resolve(procedure) else {
                continue;
            };
            self.nested(
                &stream,
                id,
                &resources,
                context.page,
                context.depth,
                state,
                "a Type 3 glyph procedure",
                stated,
                Via::Type3Glyph,
                false,
            );
        }
    }
}

/// Whether a font is a Type 3 font that states no `/Resources`, whose glyph procedures ISO 32000-2
/// §9.6.4 reads against the resources of the page showing them.
fn is_type3_without_resources(document: &pdf_syntax::Document, font: &Dictionary) -> bool {
    document
        .get_key(font, "Subtype")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"Type3")
        && document.get_key(font, "Resources").as_dict().is_none()
}

/// What each of a Type 3 font's 256 codes selects, from its `Encoding`'s `Differences` and its
/// `CharProcs` (ISO 32000-2 §9.6.4, Table 110; §9.6.5.1, Table 112's `Differences`).
fn type3_codes(document: &pdf_syntax::Document, font: &Dictionary) -> Vec<Glyph> {
    let mut codes = vec![Glyph::Unknown; 256];
    let encoding = document.get_key(font, "Encoding");
    let Some(encoding) = encoding.as_dict() else {
        return codes;
    };
    let procedures = document.get_key(font, "CharProcs");
    let procedures = procedures.as_dict();
    let differences = document.get_key(encoding, "Differences");
    let Some(differences) = differences.as_array() else {
        return codes;
    };
    let mut next: Option<usize> = None;
    for item in differences {
        match document.resolve(item) {
            Object::Integer(code) => next = usize::try_from(code).ok(),
            Object::Name(name) => {
                if let Some(slot) = next.and_then(|code| codes.get_mut(code)) {
                    *slot = match procedures.and_then(|table| table.get_by_name(&name)) {
                        None => Glyph::Absent,
                        Some(Object::Reference(procedure)) => Glyph::Procedure(*procedure),
                        Some(_) => Glyph::Unknown,
                    };
                }
                next = next.map(|code| code.saturating_add(1));
            }
            _ => next = None,
        }
    }
    codes
}

/// The font dictionary a conversion writes for a resource name that is one of §9.6.2.2's fourteen
/// and that no resource dictionary defines: a plain `Type1` font of that `BaseFont`.
///
/// ISO 32000-2 §9.6.2.1's Table 109 lets such a font leave out `FirstChar`, `LastChar`, `Widths`
/// and `FontDescriptor`, and the reader draws the name unaided from its own faces, so this is the
/// font the page already shows. The leave-out holds for PDF 1.0 to 1.7 only: at part 4 those entries
/// are required, so this definition is never what a part 4 output keeps, and the embedding route
/// (`doc/adr/2522`) replaces it with a font that states its widths and its descriptor. `None` for
/// any other name.
#[must_use]
pub fn standard_font_definition(name: &[u8]) -> Option<Dictionary> {
    let base = std::str::from_utf8(name)
        .ok()
        .filter(|name| pdf_font::standard::is_standard_name(name))?;
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Font"[..])),
    );
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"Type1"[..])),
    );
    dict.insert(
        Name::new(&b"BaseFont"[..]),
        Object::Name(Name::new(base.as_bytes().to_vec())),
    );
    Some(dict)
}

/// Adds one text-showing operator's strings to a font's record of what it drew.
///
/// Read before the entry is borrowed, and once rather than per string: every string a single
/// text-showing operator draws stands inside the same sequences. The stack is lent to `widen`
/// rather than collected into a list, because collecting one would be a heap allocation per
/// text-showing operator on a tagged page — which is most of them.
fn note_shown(
    entry: &mut SelectedFont,
    rendered: bool,
    strings: &[Vec<u8>],
    marks: &[Mark],
    page: usize,
    budget: &mut usize,
) {
    let settled = marks.iter().any(|mark| mark.settles);
    let bare = !settled && !marks.iter().any(|mark| mark.mcid.is_some());
    let enclosing: &[Mark] = if settled { &[] } else { marks };
    entry.rendered |= rendered;
    for text in strings {
        if let Some(replacement) = entry.shown.get_mut(text) {
            widen(replacement, bare, enclosing, page);
            continue;
        }
        if text.len() > *budget {
            // The font is not reported on a prefix of its text; see `SHOWN_BUDGET`.
            entry.shown_complete = false;
            continue;
        }
        *budget = budget.saturating_sub(text.len());
        let mut replacement = Replacement::default();
        widen(&mut replacement, bare, enclosing, page);
        entry.shown.insert(text.clone(), replacement);
    }
}
