//! ISO 32000-2 §14.11.5's output intents: which one is in force for a page, and what it is called.
//!
//! **One selection, two readers.** The interpreter asks which profile says what a page's device
//! colours mean (`crate::content`'s colour conversion, ADR 0272); a print asks which press the
//! document was prepared for, so that it can say what a print through this program does not do
//! with it (ADR 1932). Both are answered by [`selected`], so the press a print names is always the
//! press whose profile the page was drawn through, and never a second reading of the same array
//! that could disagree with the first.
//!
//! **What a print does not do with it** is Table 401's own distinction. Its `/DestOutputProfile`
//! entry names two directions:
//!
//! > The output transformation uses the profile's "from CIE" information (BToA in ICC
//! > terminology); the "to CIE" (AToB) information may optionally be used to remap source colour
//! > values to some other destination colour space, such as for screen preview or hardcopy
//! > proofing.
//!
//! This tree evaluates the second and has no evaluator for the first (ADR 0009), so a page goes to
//! paper as the screen's RGB, colour-managed by the print system, rather than converted for the
//! press the intent describes. That is the clause's permission exercised — "PDF processors are free
//! to disregard it" — and a reader about to print a document prepared for a press is owed the
//! sentence (RFC 0009 §2.6, `doc/questions/A254`).

use pdf_syntax::{Dictionary, Document, Object};

use crate::colour::ColourSpace;
use crate::page::Pages;

/// The output intent the colour of a page is drawn through, as a person would recognise it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputIntent {
    /// Table 401's `/S`, the output intent subtype — `GTS_PDFX`, `GTS_PDFA1`, `ISO_PDFE1` or an
    /// extension's — where the dictionary states it as a name.
    pub subtype: Option<String>,
    /// What to call the intended output condition, where the dictionary says, chosen in the order
    /// Table 401 ranks its strings for presentation; [`Self::named`] is the reading.
    pub condition: Option<String>,
}

impl OutputIntent {
    /// Reads the presentable half of one output intent dictionary.
    ///
    /// Table 401 ranks three strings. `/OutputCondition` is "the preferred method of defining such
    /// a string for presentation to the user"; `/OutputConditionIdentifier`, where it is
    /// human-readable, may stand in for that string; and `/Info` "shall be used for further human-readable identification". `Custom` is the
    /// table's own word for a condition that is not a registered standard, which names nothing to
    /// a person, so `/Info` is taken past it — wherever a producer wrote it, since one corpus
    /// document puts it in `/OutputCondition` too.
    fn named(document: &Document, dictionary: &Dictionary) -> Self {
        let text = |key: &str| {
            document
                .get_key(dictionary, key)
                .as_string()
                .map(pdf_syntax::text_string::text_string)
                .map(|text| text.trim().to_owned())
                .filter(|text| !text.is_empty() && text != "Custom")
        };
        Self {
            subtype: document
                .get_key(dictionary, "S")
                .as_name()
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
            condition: text("OutputCondition")
                .or_else(|| text("OutputConditionIdentifier"))
                .or_else(|| text("Info")),
        }
    }
}

/// The output intent in force for one page: its dictionary, and the colour space its profile is.
pub(crate) struct Selected {
    /// The output intent dictionary the space was read from.
    pub(crate) dictionary: Dictionary,
    /// Its `/DestOutputProfile`, parsed.
    pub(crate) space: ColourSpace,
}

/// Reads the output intent in force for this page, where one names a usable profile.
///
/// Only a profile whose own space is one a PDF can name is useful here; an output intent
/// for a device with some other colourant model says nothing about `DeviceCMYK`.
///
/// **The array has two homes and the page's wins**, which is §14.11.5 in its own words:
///
/// > The optional OutputIntents entry in the document catalog dictionary (see 7.7.2, "Document
/// > catalog dictionary") or a Page dictionary (see 7.7.3.3, "Page objects") holds an array of
/// > output intent dictionaries, each describing the colour reproduction characteristics of a
/// > possible output device.
///
/// and, two paragraphs later, the sentence that decides between them:
///
/// > The data in an output intent dictionary shall be for informational purposes only, and PDF
/// > processors are free to disregard it. If a PDF processor chooses to respect output intents,
/// > then when processing a page that has an associated (page-level) output intent, that
/// > page-level output intent shall be used.
///
/// This tree does choose to respect them — that is what this function is — so the `shall` binds it,
/// and both homes are read (ADR 0821). A page-level array is rare — `doc/pdf.js` holds none and
/// `doc/corpora`'s PDF 2.0 examples hold one; what the second home gives is that a document *can* say per page what its
/// device colours mean, which is the construction `pdf_transform::merge` writes when two sources'
/// catalog intents disagree.
pub(crate) fn selected(document: &Document, page: Option<&Dictionary>) -> Option<Selected> {
    if let Some(page) = page
        && let Some(selected) = first_usable(document, &document.get_key(page, "OutputIntents"))
    {
        return Some(selected);
    }
    let catalog = document.catalog().ok()?;
    first_usable(document, &document.get_key(&catalog, "OutputIntents"))
}

/// The first output intent of an array that names a profile this tree can parse.
///
/// §14.11.5 NOTE 1:
///
/// > The choice of which output intent to use in a given workflow is outside the scope of ISO
/// > 32000-2 and therefore PDF intentionally does not include a selector for choosing a
/// > particular output intent from within the PDF file.
///
/// With no selector stated, the first usable one is taken.
fn first_usable(document: &Document, intents: &Object) -> Option<Selected> {
    for intent in intents.as_array()? {
        let intent = document.resolve(intent);
        let Some(dict) = intent.as_dict() else {
            continue;
        };
        let profile = document.get_key(dict, "DestOutputProfile");
        let Some(stream) = profile.as_stream() else {
            continue;
        };
        if let Some(data) = document.decoded_stream_data(stream)
            && let Some(parsed) = crate::icc::Profile::parse(&data)
        {
            return Some(Selected {
                dictionary: dict.clone(),
                space: ColourSpace::Icc {
                    profile: std::sync::Arc::new(parsed),
                },
            });
        }
    }
    None
}

/// Every output intent some page of the document is drawn through, each once, in page order.
///
/// Empty for a document whose pages draw through none, which is every document that states no
/// `/OutputIntents` with a readable profile. A page-level array is rare (`doc/corpora`'s PDF 2.0
/// examples hold one) and wins for its own page only, so the catalog's answer is read once and a
/// page is asked its own question only where it states an array of its own.
///
/// **Off every launch path**: this reads each page's dictionary, which a print does once before it
/// starts and an open never does.
#[must_use]
pub fn in_force_on_pages(document: &Document) -> Vec<OutputIntent> {
    let catalog = document
        .catalog()
        .ok()
        .and_then(|catalog| first_usable(document, &document.get_key(&catalog, "OutputIntents")))
        .map(|selected| OutputIntent::named(document, &selected.dictionary));
    let pages = Pages::new(document);
    let mut found: Vec<OutputIntent> = Vec::new();
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let own = first_usable(document, &document.get_key(&page.dict, "OutputIntents"))
            .map(|selected| OutputIntent::named(document, &selected.dictionary));
        if let Some(intent) = own.or_else(|| catalog.clone())
            && !found.contains(&intent)
        {
            found.push(intent);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{OutputIntent, in_force_on_pages, selected};
    use pdf_syntax::Document;

    /// The sRGB profile this project ships for `pdf_transform::archive` (ADR 1153), so that the
    /// selection below reads a real profile and not one shaped to pass it.
    fn profile() -> Vec<u8> {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/icc/sRGB2014.icc"
        ))
        .to_vec()
    }

    /// Builds a document from object bodies numbered from 1; object 1 is the catalog. A body
    /// containing `PROFILE` becomes a stream holding [`profile`]'s bytes.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let number = index.saturating_add(1);
            if *body == "PROFILE" {
                let data = profile();
                out.extend_from_slice(
                    format!(
                        "{number} 0 obj\n<< /N 3 /Length {} >>\nstream\n",
                        data.len()
                    )
                    .as_bytes(),
                );
                out.extend_from_slice(&data);
                out.extend_from_slice(b"\nendstream\nendobj\n");
            } else {
                out.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
            }
        }
        let xref_at = out.len();
        let mut tail = String::new();
        let _ = write!(
            tail,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(tail, "{offset:010} 00000 n ");
        }
        let _ = write!(
            tail,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        out.extend_from_slice(tail.as_bytes());
        Document::open(out).expect("a valid file")
    }

    /// The fixture's profile parses, so that the tests below are about the selection and not about
    /// a profile this tree would refuse — the control trap 13 asks for.
    #[test]
    fn the_fixture_profile_is_one_this_tree_reads() {
        assert!(crate::icc::Profile::parse(&profile()).is_some());
    }

    /// Table 401's ranking of its strings for presentation: `/OutputCondition` first, then a
    /// human-readable identifier, then `/Info` past the table's own `Custom`.
    #[test]
    fn the_condition_is_named_in_the_order_table_401_ranks_its_strings() {
        let named = |intent: &str| {
            let document = document(&[
                &format!(
                    "<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent \
                     /DestOutputProfile 3 0 R {intent} >>] >>"
                ),
                "<< /Type /Pages /Kids [4 0 R] /Count 1 >>",
                "PROFILE",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            ]);
            in_force_on_pages(&document)
        };
        assert_eq!(
            named(
                "/S /GTS_PDFX /OutputCondition (Coated FOGRA39) /OutputConditionIdentifier (FOGRA39)"
            ),
            [OutputIntent {
                subtype: Some("GTS_PDFX".to_owned()),
                condition: Some("Coated FOGRA39".to_owned()),
            }]
        );
        assert_eq!(
            named("/S /GTS_PDFA1 /OutputConditionIdentifier (sRGB IEC61966-2.1)")[0].condition,
            Some("sRGB IEC61966-2.1".to_owned())
        );
        assert_eq!(
            named("/S /GTS_PDFA1 /OutputConditionIdentifier (Custom) /Info (House press)")[0]
                .condition,
            Some("House press".to_owned())
        );
        assert_eq!(
            named("/S /GTS_PDFA1 /OutputConditionIdentifier (Custom)")[0].condition,
            None
        );
    }

    /// An intent with no readable profile is not the one in force — the interpreter would not draw
    /// through it — and a later one in the array is (NOTE 1's first usable one).
    #[test]
    fn an_intent_without_a_readable_profile_is_passed_over_as_the_interpreter_passes_it() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OutputIntents [\
             << /S /GTS_PDFX /OutputCondition (Unread) >> \
             << /S /GTS_PDFX /OutputCondition (Read) /DestOutputProfile 3 0 R >>] >>",
            "<< /Type /Pages /Kids [4 0 R] /Count 1 >>",
            "PROFILE",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        let found = in_force_on_pages(&document);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].condition.as_deref(), Some("Read"));
        let catalog = document.catalog().expect("a catalog");
        let page = document
            .get_key(&catalog, "Pages")
            .as_dict()
            .map(|pages| document.get_key(pages, "Kids"))
            .and_then(|kids| kids.as_array().and_then(|kids| kids.first().cloned()))
            .map(|kid| document.resolve(&kid))
            .and_then(|page| page.as_dict().cloned())
            .expect("the page");
        assert!(selected(&document, Some(&page)).is_some());
    }

    /// §14.11.5's "that page-level output intent shall be used": a page stating its own array is
    /// drawn through it, the others through the catalog's, and each is named once.
    #[test]
    fn a_page_level_intent_wins_for_its_page_and_each_intent_is_named_once() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OutputIntents [\
             << /S /GTS_PDFX /OutputCondition (Catalog press) /DestOutputProfile 3 0 R >>] >>",
            "<< /Type /Pages /Kids [4 0 R 5 0 R 6 0 R] /Count 3 >>",
            "PROFILE",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /OutputIntents [\
             << /S /GTS_PDFX /OutputCondition (Page press) /DestOutputProfile 3 0 R >>] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        let names: Vec<_> = in_force_on_pages(&document)
            .into_iter()
            .filter_map(|intent| intent.condition)
            .collect();
        assert_eq!(names, ["Catalog press", "Page press"]);
    }

    /// A document that states no output intent draws through none and names none.
    #[test]
    fn a_document_with_no_output_intent_names_none() {
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        assert!(in_force_on_pages(&document).is_empty());
    }
}
