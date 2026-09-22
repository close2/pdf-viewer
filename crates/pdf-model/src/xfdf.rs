//! ISO 19444-1's XML Forms Data Format: §12.7.8's data, spelled in XML.
//!
//! ISO 32000-2 names this format three times and defines it nowhere. §12.7.6.2's Table 240 bit 6
//! says "field names and values shall be submitted as XFDF"; §12.7.6.4's Table 243 makes `/F`
//! "[t]he FDF, XFDF or any other data format file from which to import the data"; and the list of
//! submission formats calls it "XFDF, a version of FDF based on XML as defined by ISO 19444-1".
//! So the grammar is that standard's, and this module is written against it: the second edition,
//! `doc/md/ISO-19444-1-2019-preview.md`, whose divisions are cited here as ISO 19444-1 section
//! 5.6.3 and never with a `§`, which in this tree means ISO 32000-2 and nothing else.
//!
//! # What XFDF is, in the standard's own division of it
//!
//! ISO 19444-1:2019 section 5.4.1 draws the boundary this module works inside. XFDF is a subset of
//! FDF holding forms and annotations, and the section names exactly four FDF dictionary entries it
//! has an equivalent for: `/Annots`, `/Fields`, `/F` and `/ID`. The other seven of Table 246 —
//! `/Status`, `/Encoding`, `/JavaScript`, `/EmbeddedFDFs`, `/Differences`, `/Target` and `/Pages`
//! — have none, which the section says in as many words.
//!
//! (**Paraphrase throughout, with the section cited.** ISO 19444-1 is a licensed text like ISO
//! 16684-1's and ISO 15076-1's previews, and this tree's rule for those is to cite and paraphrase
//! and never to quote in committed source. Nothing below is verbatim; where the exact words matter,
//! read the section. `doc/third-party-data.md` carries the rule and the provenance.)
//!
//! Four entries, and [`read`] answers three of them into the very same [`FormsData`] that
//! [`FormsData::read`] fills from an FDF file — `<f>` into its `source`, `<ids>` into its
//! `identifier`, `<fields>` into its `fields`. That is the point of returning that type rather
//! than one of this module's own: §12.7.6.4's import has one meaning, `ViewState::import` applies
//! it once, and a second path into it could disagree with the first about what a fully qualified
//! name is.
//!
//! **`<annots>` is the fourth and is counted rather than read**, which ADR 1108 argues and
//! [`FormsData::owed`] says out loud. The clause makes it owed — ISO 19444-1:2019 section 5.7.1
//! says an import may create a *new* annotation, which is the one thing it says a form import may
//! not do — and what it would take is that standard's sections 6.4 and 6.6, its per-subtype
//! elements and its PDF-to-XFDF
//! attribute mapping, which the preview this tree holds does not carry. Inventing them from
//! another reader or from sample files is what `CLAUDE.md` principle 5 forbids, so the count and
//! the sentence are the honest answer and the field data is imported beside them.
//!
//! # The grammar, as far as this reads it
//!
//! ISO 19444-1:2019 section 5.5.2 requires the file to be UTF-8, requires the namespace to be
//! `http://ns.adobe.com/xfdf/` — [`NAMESPACE`] here — requires `xml:space="preserve"`, and states
//! the two lines an XFDF document begins with. Section 5.4.1 adds that XFDF conforms to the XML
//! standard, which is what makes XML 1.0's five predefined entities and its numeric character
//! references the escaping here — [`crate::xmp`]'s reader already expands exactly those, for a
//! packet that is XML for the same reason.
//!
//! A form's field data is sections 5.6.2 and 5.6.3, and the two examples differ in one way that
//! decides the whole reader: a flat form writes `<field name="Street"><value>…</value></field>`,
//! and a hierarchical one nests the elements. Section 5.6.3 says that hierarchical field names are
//! conventionally written in a dot notation and that XFDF represents them as nested `field`
//! elements. So a `<field>`'s
//! fully qualified name is its ancestors' `name` attributes joined with §12.7.4.2's full stop,
//! which is the same name [`FormsData::read`] builds by concatenating `/T` down `/Kids`. The two
//! formats meet at that string and nowhere earlier.
//!
//! # What an import may and may not do, which the format states itself
//!
//! Section 5.6.1, and it is why nothing here creates a field: importing XFDF updates the values of
//! form fields the target document already has, and the section states outright that XFDF can
//! neither create a form field nor change anything about an existing one except its value.
//!
//! That is §12.7.8.3.2's "replace" with a narrower reach: an FDF field may also carry `/Ff`,
//! `/SetF` and the rest, and an XFDF field carries a value. [`FdfField::flags`] and
//! [`FdfField::annotation_flags`] therefore come back [`FlagChange::Unchanged`] from this reader,
//! which is not a gap — it is the sentence above.
//!
//! # Writing one
//!
//! [`crate::submission`] writes the same grammar for Table 240 bit 6, from the same field tree it
//! writes an FDF from, and the round trip through [`read`] is what the tests here and there
//! check together.

use crate::forms_data::{Encoding, FdfField, FlagChange, FormsData, MAX_FIELD_DEPTH, MAX_FIELDS};

/// The namespace ISO 19444-1:2019 section 5.5.2 requires of an XFDF document.
pub const NAMESPACE: &str = "http://ns.adobe.com/xfdf/";

/// Why an XFDF file could not be read at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum XfdfError {
    /// The bytes are not UTF-8, which ISO 19444-1:2019 section 5.5.2 requires of an XFDF file.
    #[error("the file is not UTF-8, which ISO 19444-1 section 5.5.2 requires of XFDF")]
    NotUtf8,
    /// The XML is malformed, with what the tokenizer could not get past.
    #[error("the file is not well-formed XML, which ISO 19444-1 section 5.4.1 requires: {0}")]
    Malformed(String),
    /// No `<xfdf>` element, which is how a file handed to this function identifies itself as one
    /// — the same position [`crate::forms_data::FormsDataError::NotFormsData`] takes for FDF.
    #[error("the file states no <xfdf> element, so it is not XML Forms Data Format")]
    NotXfdf,
}

/// Reads one XFDF file's `<xfdf>` element and everything under it.
///
/// The result is a [`FormsData`] so that §12.7.6.4's two named formats reach
/// [`crate::view::ViewState::import`] through one value: what this fills is `source`, `identifier`
/// and `fields`, and what it leaves at their empty defaults is everything ISO 19444-1 section
/// 5.4.1 says XFDF has no equivalent for.
///
/// # Errors
///
/// [`XfdfError::NotUtf8`] for bytes section 5.5.2 forbids, [`XfdfError::Malformed`] for XML the
/// tokenizer refuses, and [`XfdfError::NotXfdf`] for well-formed XML that is some other document.
pub fn read(bytes: &[u8]) -> Result<FormsData, XfdfError> {
    let text = std::str::from_utf8(bytes).map_err(|_| XfdfError::NotUtf8)?;
    let mut reader = Reader::default();
    for token in xmlparser::Tokenizer::from(text) {
        // A malformed file stops here rather than keeping what was read, which is the opposite of
        // `popup::rich_text`'s choice and for the opposite reason: that one is a window's text and
        // half of it is better than none, and this one replaces the values of a form. Half an
        // import is a form that says something nobody wrote.
        let token = token.map_err(|error| XfdfError::Malformed(error.to_string()))?;
        reader.token(&token);
    }
    // `xmlparser` is a tokenizer and not a parser: it will hand back the tokens of a truncated
    // file without complaint, so a document that stops in the middle of its `<fields>` reaches
    // here looking like a short one. A partial import is a form saying something nobody wrote, so
    // the balance is checked here — it is the one well-formedness rule this reader needs and the
    // one the tokenizer does not enforce.
    if !reader.open.is_empty() || reader.unbalanced {
        return Err(XfdfError::Malformed(
            "an element is left open or closed without being opened".to_owned(),
        ));
    }
    if !reader.saw_xfdf {
        return Err(XfdfError::NotXfdf);
    }
    Ok(reader.finish())
}

/// One element the walk is inside, by local name.
///
/// The prefix is dropped deliberately: ISO 19444-1 section 5.5.2 states the namespace and not a
/// prefix for it, and its own examples bind it as the default. A file that binds it to `xfdf:`
/// instead spells the same document, so the local name is what this matches on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Element {
    /// The `<xfdf>` root.
    Xfdf,
    /// `<fields>`, and every `<field>` under it.
    Field,
    /// `<value>`, whose character content is the field's value.
    Value,
    /// `<f>`, whose `href` names the document the fields came from.
    File,
    /// `<ids>`, whose two attributes are §14.4's file identifier.
    Ids,
    /// `<annots>`, counted rather than read.
    Annots,
    /// Anything else. Its children are `Other` too, so nothing inside an element this reader does
    /// not know can be mistaken for field data.
    Other,
}

/// One element the walk is inside.
struct Frame {
    /// Which element it is.
    element: Element,
    /// Whether it pushed a partial name onto [`Reader::path`].
    named: bool,
    /// Where its [`FdfField`] sits in [`Reader::fields`], for a `<field>` that has one.
    field: Option<usize>,
}

/// The walk's state: what is open, what has been named, and what the file owes.
#[derive(Default)]
struct Reader {
    /// Whether an `<xfdf>` element was seen at all.
    saw_xfdf: bool,
    /// The elements currently open, innermost last.
    open: Vec<Frame>,
    /// The `name` attributes of the `<field>` elements currently open, for section 5.6.3's
    /// nesting.
    path: Vec<String>,
    /// Character content collected since the innermost `<value>` opened.
    value: Option<String>,
    /// ISO 19444-1:2019 section 5.6.2's `<f href>`: the PDF document holding the form fields.
    source: Option<String>,
    /// `<ids original modified>`, which section 5.4.1 maps to the FDF `/ID` entry.
    ids: [Option<Vec<u8>>; 2],
    /// The fields, in the order the file states them.
    fields: Vec<FdfField>,
    /// How many `<annots>` children were counted, for the sentence ADR 1108 owes.
    annotations: usize,
    /// What this file states and this reader does not apply, in the order it was first met.
    ///
    /// A list rather than a flag apiece because each entry is a sentence for a person and not a
    /// condition anything branches on; [`Self::owe`] is what keeps one file from saying the same
    /// thing twice.
    owed: Vec<&'static str>,
    /// Whether a close tag arrived with nothing open, which is the other half of [`read`]'s
    /// balance check.
    unbalanced: bool,
}

impl Reader {
    /// One token of the walk.
    fn token(&mut self, token: &xmlparser::Token<'_>) {
        match *token {
            xmlparser::Token::ElementStart { local, .. } => self.start(local.as_str()),
            xmlparser::Token::Attribute { local, value, .. } => {
                self.attribute(local.as_str(), value.as_str());
            }
            xmlparser::Token::ElementEnd { end, .. } => match end {
                xmlparser::ElementEnd::Open => self.opened(),
                // An empty element opens and closes at once: `<field name="x"/>` is a field with
                // no value, which §12.7.8.3.2's "replace" makes a value removed.
                xmlparser::ElementEnd::Empty => {
                    self.opened();
                    self.close();
                }
                xmlparser::ElementEnd::Close(..) => self.close(),
            },
            // Character content, collected only while a `<value>` is open. CDATA carries no
            // entity references by definition, which is why only the first is unescaped.
            xmlparser::Token::Text { text } => {
                if let Some(into) = self.value.as_mut() {
                    crate::xmp::unescape(text.as_str(), into);
                }
            }
            xmlparser::Token::Cdata { text, .. } => {
                if let Some(into) = self.value.as_mut() {
                    into.push_str(text.as_str());
                }
            }
            _ => {}
        }
    }

    /// An element's name has been read; its attributes have not.
    fn start(&mut self, local: &str) {
        let parent = self.open.last().map(|frame| frame.element);
        let element = match parent {
            // Section 5.7.1's annotations: each child of `<annots>` is one, counted at the point
            // it opens and read by nothing. What reading it would take is ISO 19444-1 sections
            // 6.4 and 6.6, which the text this tree holds does not carry (ADR 1108).
            Some(Element::Annots) => {
                self.annotations = self.annotations.saturating_add(1);
                self.owe(
                    "<annots>: ISO 19444-1 section 5.7.1's annotations, whose elements and \
                     attributes are that standard's sections 6.4 and 6.6",
                );
                Element::Other
            }
            // Nothing below an unread element is read, whatever it is called.
            Some(Element::Other) => Element::Other,
            _ => match local {
                "xfdf" => {
                    self.saw_xfdf = true;
                    Element::Xfdf
                }
                "fields" | "field" => Element::Field,
                "value" if parent == Some(Element::Field) => Element::Value,
                "value-richtext" if parent == Some(Element::Field) => {
                    self.owe("<value-richtext>: Table 249's /RV, XFA rich text, excluded");
                    Element::Other
                }
                "f" => Element::File,
                "ids" => Element::Ids,
                "annots" => Element::Annots,
                _ => Element::Other,
            },
        };
        self.open.push(Frame {
            element,
            named: false,
            field: None,
        });
    }

    /// One attribute of the element that is being opened.
    fn attribute(&mut self, local: &str, value: &str) {
        let Some(frame) = self.open.last_mut() else {
            return;
        };
        let mut text = String::with_capacity(value.len());
        crate::xmp::unescape(value, &mut text);
        match (frame.element, local) {
            // ISO 19444-1:2019 section 5.6.2 explains this attribute as pointing at the PDF
            // document holding the form fields. A name for a person, never a path this crate
            // opens — exactly the position Table 246's `/F` takes in `forms_data`.
            (Element::File, "href") => {
                self.source.get_or_insert(text);
            }
            // Section 5.4.1 maps `<ids original modified>` onto the FDF `/ID` array, whose
            // elements are §14.4's byte strings written here as hexadecimal.
            (Element::Ids, "original") => self.ids[0] = hex(&text),
            (Element::Ids, "modified") => self.ids[1] = hex(&text),
            (Element::Field, "name") => {
                frame.named = true;
                self.path.push(text);
            }
            _ => {}
        }
    }

    /// The element's attributes are done and its content begins.
    ///
    /// A `<field>` becomes a [`FdfField`] here rather than at its close, so that a field stating
    /// no value is recorded exactly once and in the order the file writes it — the same order
    /// `forms_data::read_fields` records an FDF's `/Kids` in.
    fn opened(&mut self) {
        let Some(frame) = self.open.last() else {
            return;
        };
        match frame.element {
            Element::Value => self.value = Some(String::new()),
            Element::Field if frame.named => {
                let index = self.record();
                if let Some(frame) = self.open.last_mut() {
                    frame.field = index;
                }
            }
            _ => {}
        }
    }

    /// The innermost element closes.
    fn close(&mut self) {
        let Some(frame) = self.open.pop() else {
            self.unbalanced = true;
            return;
        };
        if frame.element == Element::Value
            && let Some(text) = self.value.take()
        {
            // The value belongs to the nearest enclosing `<field>`, which is the one whose name
            // is innermost on the path.
            if let Some(index) = self
                .open
                .iter()
                .rev()
                .find_map(|enclosing| enclosing.field)
                .filter(|index| *index < self.fields.len())
            {
                self.fields[index].value = Some(pdf_syntax::Object::String(
                    pdf_syntax::text_string::encode_text_string(&text).into(),
                ));
            }
        }
        if frame.named {
            self.path.pop();
        }
    }

    /// Records the open `<field>` under §12.7.4.2's fully qualified name, with no value yet.
    ///
    /// `None` where the bounds `forms_data` sets for an FDF file are reached, which is the same
    /// pair of numbers for the same reason: a file claiming more fields or deeper nesting than
    /// these is making a reader work rather than describing a form.
    fn record(&mut self) -> Option<usize> {
        if self.fields.len() >= MAX_FIELDS || self.path.len() > MAX_FIELD_DEPTH {
            self.owe("the field list is longer than this reader walks and was cut");
            return None;
        }
        let index = self.fields.len();
        self.fields.push(FdfField {
            name: self.path.join("."),
            value: None,
            // ISO 19444-1:2019 section 5.6.1 allows an import to change nothing about an existing
            // field except its value, so there are no flag entries in XFDF to read.
            flags: FlagChange::Unchanged,
            annotation_flags: FlagChange::Unchanged,
            options: None,
            // The same section, and the same sentence: an icon fit dictionary is not a value, so
            // XFDF states no element for one. Nor an appearance, nor an action — a change to
            // either is a change to something other than the field's value.
            icon_fit: None,
            appearance: None,
            appearance_reference: Vec::new(),
            actions: None,
            owed: Vec::new(),
        });
        Some(index)
    }

    /// Records one sentence about what this file states and this reader does not apply.
    ///
    /// Once per kind, however many elements of that kind the file holds: a person reading the
    /// list wants to know that annotations were not read, not how many times to be told.
    fn owe(&mut self, what: &'static str) {
        if !self.owed.contains(&what) {
            self.owed.push(what);
        }
    }

    /// The file, with what it states this program does not apply named on `owed`.
    fn finish(self) -> FormsData {
        let identifier = match self.ids {
            [Some(original), Some(modified)] => Some([original, modified]),
            // Half an identifier compared against a whole one would answer, which is
            // `forms_data::identifier`'s rule and is this one for the same reason.
            _ => None,
        };
        FormsData {
            // The IANA registration of `application/xfdf`, written by the committee that owns ISO
            // 19444-1, states the reason there is no number to read: "XFDF does not define an
            // explicitly encoded version number for XFDF into the XML data."
            version: None,
            conforms_to: None,
            source: self.source,
            identifier,
            fields: self.fields,
            // Section 5.4.1: there is no XFDF equivalent for the Status, Encoding, Target,
            // Pages or EmbeddedFDFs keys, so their absence here is that sentence rather than a
            // gap — the format states no element carrying another XFDF file inside one.
            status: None,
            // Section 5.5.2 makes the file UTF-8, so its values arrive as characters rather than
            // as bytes in some registered character set. `Unicode` is the entry that says so.
            encoding: Encoding::Unicode,
            annotations: Vec::new(),
            pages: Vec::new(),
            target: None,
            embedded: Vec::new(),
            owed: self.owed,
        }
    }
}

/// An `<ids>` attribute's hexadecimal, as the bytes §14.4's identifier is made of.
///
/// `None` for anything that is not an even number of hexadecimal digits: section 5.4.1's example
/// writes each as 32 of them against an FDF `/ID` of two 16-byte strings, and half a byte is not
/// an identifier to compare with.
fn hex(text: &str) -> Option<Vec<u8>> {
    let digits: &[u8] = text.as_bytes();
    if digits.is_empty() || !digits.len().is_multiple_of(2) {
        return None;
    }
    digits
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16)?;
            let low = (pair[1] as char).to_digit(16)?;
            u8::try_from(high.saturating_mul(16).saturating_add(low)).ok()
        })
        .collect()
}

/// Text with the characters XML 1.0 section 2.4 makes markup replaced by its predefined entities.
///
/// Both of the places XFDF puts text need this and they need different sets of it — an attribute
/// value must not carry the delimiter it is quoted with — so `quotes` says which is being written.
/// The apostrophe is left alone: attribute values here are written with QUOTATION MARK, and a
/// character that is not markup in the context it appears in is not escaped in it.
pub(crate) fn escape(text: &str, quotes: bool, out: &mut String) {
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if quotes => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat form in the shape ISO 19444-1:2019 section 5.6.2 lays out: the prologue section
    /// 5.5.2 requires, then `<f>`, `<ids>` and `<fields>` in that order, with one `<value>` to a
    /// `<field>`.
    ///
    /// The *shape* is the specification's and the content is this test's. Every element and
    /// attribute here is the format's grammar, which is what the reader is being calibrated
    /// against; the names and values the standard's own figure uses are its expression, and this
    /// tree cites that text rather than copying it (`doc/third-party-data.md`).
    const FLAT_FORM: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <xfdf xmlns=\"http://ns.adobe.com/xfdf/\" xml:space=\"preserve\">\n\
        <f href=\"invoices/march.pdf\"/>\n\
        <ids original=\"0F1E2D3C4B5A69788796A5B4C3D2E1F0\" \
        modified=\"00112233445566778899AABBCCDDEEFF\"/>\n\
        <fields>\n\
        <field name=\"Name\">\n\
        <value>Ada Lovelace</value>\n\
        </field>\n\
        <field name=\"Street\">\n\
        <value>12 Dorset Street</value>\n\
        </field>\n\
        <field name=\"CityState\">\n\
        <value>London, W1U</value>\n\
        </field>\n\
        </fields>\n\
        </xfdf>\n";

    /// The text of one field's value, as a target document would read it.
    fn value(data: &FormsData, name: &str) -> Option<String> {
        let field = data.fields.iter().find(|field| field.name == name)?;
        let bytes = field.value.as_ref()?.as_string()?;
        Some(pdf_syntax::text_string(bytes))
    }

    /// Section 5.6.2's three parts, read: the `<f href>`, the `<ids>` pair and three flat fields.
    #[test]
    fn a_simple_form_states_its_file_its_identifier_and_its_fields() {
        let data = read(FLAT_FORM.as_bytes()).expect("a form in section 5.6.2's shape");
        assert_eq!(data.source.as_deref(), Some("invoices/march.pdf"));
        assert_eq!(data.fields.len(), 3);
        assert_eq!(value(&data, "Name").as_deref(), Some("Ada Lovelace"));
        assert_eq!(value(&data, "Street").as_deref(), Some("12 Dorset Street"));
        assert_eq!(value(&data, "CityState").as_deref(), Some("London, W1U"));
        // Section 5.4.1 maps the two attributes onto the FDF `/ID` array's two byte strings, which
        // §14.4 makes sixteen bytes apiece written as thirty-two hexadecimal digits.
        let identifier = data.identifier.expect("the <ids> element");
        assert_eq!(identifier[0][0], 0x0f);
        assert_eq!(identifier[0].len(), 16);
        assert_eq!(identifier[1].len(), 16);
    }

    /// ISO 19444-1:2019 section 5.6.3, which represents a hierarchical field name — conventionally
    /// written with full stops — as nested `field` elements.
    ///
    /// The same three fields as [`FLAT_FORM`] under one parent, and the assertion is §12.7.4.2's
    /// name rather than the nesting: what a target document is matched against is `Address.Name`,
    /// so a reader that walked the nesting and did not join it would import nothing and report no
    /// error.
    #[test]
    fn a_hierarchical_form_names_its_fields_with_the_full_stop() {
        let data = read(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <xfdf xmlns=\"http://ns.adobe.com/xfdf/\" xml:space=\"preserve\">\n\
             <fields>\n\
             <field name=\"Address\">\n\
             <field name=\"Name\"><value>Ada Lovelace</value></field>\n\
             <field name=\"Street\"><value>12 Dorset Street</value></field>\n\
             <field name=\"CityState\"><value>London, W1U</value></field>\n\
             </field>\n\
             </fields>\n\
             </xfdf>\n"
                .as_bytes(),
        )
        .expect("a form in section 5.6.3's shape");
        let names: Vec<&str> = data
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "Address",
                "Address.Name",
                "Address.Street",
                "Address.CityState"
            ],
            "section 5.6.3's nesting is §12.7.4.2's dot notation, and the ancestor is a field of \
             its own exactly as an FDF /Kids node with a /T is"
        );
        assert_eq!(
            value(&data, "Address.Street").as_deref(),
            Some("12 Dorset Street")
        );
    }

    /// ISO 19444-1:2019 section 5.4.1 makes XFDF conform to the XML standard, so XML 1.0's
    /// predefined entities are the escaping — a value carrying an ampersand arrives as one
    /// character.
    #[test]
    fn a_values_entities_are_expanded_and_its_cdata_is_not() {
        let data = read(
            "<xfdf xmlns=\"http://ns.adobe.com/xfdf/\"><fields>\
             <field name=\"a\"><value>Smith &amp; Sons &#65;</value></field>\
             <field name=\"b\"><value><![CDATA[raw &amp; <kept>]]></value></field>\
             </fields></xfdf>"
                .as_bytes(),
        )
        .expect("well-formed XFDF");
        assert_eq!(value(&data, "a").as_deref(), Some("Smith & Sons A"));
        assert_eq!(value(&data, "b").as_deref(), Some("raw &amp; <kept>"));
    }

    /// A `<field>` with no `<value>` is a field whose value is removed, which is §12.7.8.3.2's
    /// "replace" over a file that states nothing — the same answer an FDF `<< /T (name) >>` gives.
    #[test]
    fn a_field_with_no_value_is_a_field_whose_value_is_removed() {
        let data = read(
            "<xfdf xmlns=\"http://ns.adobe.com/xfdf/\"><fields><field name=\"empty\"/>\
             </fields></xfdf>"
                .as_bytes(),
        )
        .expect("well-formed XFDF");
        assert_eq!(data.fields.len(), 1);
        assert_eq!(data.fields[0].name, "empty");
        assert!(data.fields[0].value.is_none());
    }

    /// ISO 19444-1:2019 section 5.7.1 makes annotations importable and this reader does not read
    /// them,
    /// so the file says how many it carried rather than dropping them in silence (ADR 1108).
    #[test]
    fn annotations_are_counted_and_named_rather_than_read() {
        let data = read(
            "<xfdf xmlns=\"http://ns.adobe.com/xfdf/\">\
             <annots><text page=\"0\"/><highlight page=\"1\"/></annots>\
             <fields><field name=\"a\"><value>x</value></field></fields></xfdf>"
                .as_bytes(),
        )
        .expect("well-formed XFDF");
        assert_eq!(data.fields.len(), 1, "the field data is still imported");
        assert!(
            data.owed.iter().any(|owed| owed.starts_with("<annots>")),
            "an unread <annots> is named, not silent: {:?}",
            data.owed
        );
    }

    /// The two ways a file is not this format, told apart: XML that is some other document, and
    /// bytes that are not XML at all.
    #[test]
    fn a_file_that_is_not_xfdf_says_which_way_it_is_not() {
        assert_eq!(
            read(b"<html><body>no</body></html>"),
            Err(XfdfError::NotXfdf)
        );
        // `xmlparser` tokenizes a truncated file without complaint, so this is the case
        // [`read`]'s own balance check exists for: two elements opened and neither closed.
        assert!(matches!(
            read(b"<xfdf><fields>"),
            Err(XfdfError::Malformed(_))
        ));
        assert!(matches!(
            read(b"<xfdf><fields</xfdf>"),
            Err(XfdfError::Malformed(_))
        ));
        assert_eq!(read(&[0xff, 0xfe, 0x00]), Err(XfdfError::NotUtf8));
    }

    /// XML 1.0 section 2.4's markup characters, both ways round: escaped on the way out and
    /// expanded on the way back, so a value holding them survives a submission and an import.
    #[test]
    fn markup_characters_survive_being_written_and_read_again() {
        let mut written = String::new();
        escape("a < b & c > d \"q\"", false, &mut written);
        assert_eq!(written, "a &lt; b &amp; c &gt; d \"q\"");
        let mut attribute = String::new();
        escape("a \"b\"", true, &mut attribute);
        assert_eq!(attribute, "a &quot;b&quot;");
    }
}
