//! ISO 32000-2 §14.3.3's document information dictionary: what a file says about itself.
//!
//! The trailer's `/Info`, Table 349. Nine entries, seven of them deprecated in PDF 2.0 and all
//! nine still written by real producers — 510 of the 974 corpus documents carry the dictionary,
//! and 474 of those a `/Producer`.
//!
//! # Deprecated is not absent, and it is not ignorable
//!
//! §14.3.3 is explicit: "[i]n PDF 2.0 such use is deprecated except for two entries,
//! `CreationDate` and `ModDate`. For any other document level metadata, a metadata stream …
//! should be used instead." That binds a *writer*. A reader's job is the file it was given, which
//! is the same argument §8.6.5.1's withdrawn `CalCMYK` and §12.2's deprecated `/ViewArea` are
//! read on.
//!
//! # What is here and what is in the metadata stream
//!
//! Each of Table 349's seven text entries carries a NOTE naming its XMP counterpart —
//! `dc:title`, `dc:creator`, `dc:description`, `pdf:Keywords`, `xmp:CreatorTool`,
//! `pdf:Producer`, `pdf:Trapped`. **This module reads none of them and [`crate::xmp`] reads all
//! of them**, which is the boundary rather than a gap: this is Table 349, that is §14.3.2, and
//! they are two tables a document may fill in inconsistently.
//!
//! §14.3.4 is the clause about the disagreement and it hands the question back: "it is at the
//! discretion of the PDF processor how to use this data". So nothing here reconciles anything.
//! What this answers with is the dictionary, said to be the dictionary; §12.2 is the one place
//! the standard *ranks* the two, for the title alone, and `pdf-viewer.rs` obeys it there.
//!
//! **This paragraph said "reading it is an XML parser and therefore a dependency decision this
//! tree has not taken" until the two-hundred-and-ninety-fourth session**, which took it. The
//! sweep that found the sentence still standing is `doc/todo/01`'s fourth, run over the noun
//! `XMP` in the same round that retired it — a correction leaves its neighbours lying even when
//! the neighbour is one file away.

use pdf_syntax::{Date, Dictionary, Document, Object};

/// Table 349's `/Trapped`, whose three values are **names and not booleans**.
///
/// The clause says so twice, once per value: "[t]his shall be the name `True`, not the boolean
/// value `true`". A file writing the boolean has not stated one of the three, and gets the
/// stated default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Trapped {
    /// `True`: "[t]he document has been fully trapped; no further trapping shall be needed."
    Fully,
    /// `False`: "[t]he document has not yet been trapped."
    NotYet,
    /// `Unknown`, which is also Table 349's stated default: "[e]ither it is unknown whether the
    /// document has been trapped or it has been partly but not yet fully trapped".
    #[default]
    Unknown,
}

/// The document information dictionary, with Table 349's own default for `/Trapped`.
///
/// Every text field is `None` for an absent entry rather than an empty string, because an entry
/// a producer did not write and one it wrote empty are different statements and a properties
/// panel shows them differently.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Information {
    /// `/Title`, "[t]he document's title".
    pub title: Option<String>,
    /// `/Author`, "[t]he name of the person who created the document".
    pub author: Option<String>,
    /// `/Subject`, "[t]he subject of the document".
    pub subject: Option<String>,
    /// `/Keywords`, "[k]eywords associated with the document".
    pub keywords: Option<String>,
    /// `/Creator`: the application the document was authored in, before any conversion.
    pub creator: Option<String>,
    /// `/Producer`: what converted it to PDF.
    pub producer: Option<String>,
    /// `/CreationDate`, as the §7.9.4 string the file wrote.
    ///
    /// The bytes rather than the parse, with [`Self::created`] beside it — the same pairing
    /// §7.11.4's attachment dates and §12.8's signature dates use, and for the same reason: a
    /// date a producer wrote wrongly is still what the file says, and a panel that showed
    /// nothing would be hiding it.
    pub created: Option<String>,
    /// `/ModDate`, likewise. "Required if `PieceInfo` is present in the document catalog
    /// dictionary; otherwise optional."
    pub modified: Option<String>,
    /// `/Trapped`, defaulted to [`Trapped::Unknown`] as Table 349 states.
    pub trapped: Trapped,
}

impl Information {
    /// Reads the trailer's `/Info`, or an empty record where there is none.
    ///
    /// An absent `/Info` is not an error — §14.3.3 opens by calling the entry optional, and 454
    /// of the 964 corpus documents that open state none.
    #[must_use]
    pub fn read(document: &Document) -> Self {
        let info = document.get_key(document.trailer(), "Info");
        let Some(info) = info.as_dict() else {
            return Self::default();
        };
        Self::in_dictionary(document, info)
    }

    /// The same, from a dictionary a caller already holds.
    ///
    /// §14.3.3's last sentence is why this is public: "[d]ocument information dictionaries are
    /// also used with `Threads`", so §12.4.3's thread `/I` is the same table read from somewhere
    /// else.
    #[must_use]
    pub fn in_dictionary(document: &Document, info: &Dictionary) -> Self {
        // "Where a document information dictionary contains keys other than CreationDate and
        // ModDate, the value associated with any such key shall be a text string." So a value
        // that is not a string has not stated the entry, which is what `text` answers.
        let text = |key: &str| match document.get_key(info, key) {
            Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        };
        Self {
            title: text("Title"),
            author: text("Author"),
            subject: text("Subject"),
            keywords: text("Keywords"),
            creator: text("Creator"),
            producer: text("Producer"),
            created: text("CreationDate"),
            modified: text("ModDate"),
            trapped: match document.get_key(info, "Trapped").as_name() {
                Some(name) => match name.as_bytes() {
                    b"True" => Trapped::Fully,
                    b"False" => Trapped::NotYet,
                    _ => Trapped::Unknown,
                },
                None => Trapped::Unknown,
            },
        }
    }

    /// `/CreationDate` parsed as §7.9.4's date, where the producer wrote a conforming one.
    #[must_use]
    pub fn created_date(&self) -> Option<Date> {
        Date::parse(self.created.as_deref()?)
    }

    /// `/ModDate` parsed as §7.9.4's date, likewise.
    #[must_use]
    pub fn modified_date(&self) -> Option<Date> {
        Date::parse(self.modified.as_deref()?)
    }

    /// Whether the document stated anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Whether the document carries §14.3.2's metadata stream, without decoding it.
///
/// The cheap half of the question [`crate::xmp::Xmp::document`] answers expensively: a caller
/// listing what a document holds may want to know that a packet is there without inflating 78
/// KiB to find out. Measured: 319 of 964 corpus catalogs carry one, and 18 of the 22 that set
/// `/DisplayDocTitle` do.
///
/// **It is not the way to get a title.** §12.2 names `dc:title` and this answers `bool`, so a
/// host that titles a window asks `viewer_core::Query::Properties` and reads the packet.
#[must_use]
pub fn has_metadata_stream(document: &Document) -> bool {
    document
        .catalog()
        .is_ok_and(|catalog| !document.get_key(&catalog, "Metadata").is_null())
}

#[cfg(test)]
mod tests {
    use super::{Information, Trapped, has_metadata_stream};
    use pdf_syntax::Document;

    /// A document whose trailer names an `/Info`, built here.
    fn document(info: &str) -> Document {
        use std::fmt::Write as _;
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\n\
             3 0 obj\n{info}\nendobj\n"
        );
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for object in body.split_inclusive("endobj\n") {
            offsets.push(out.len());
            out.push_str(object);
        }
        let at = out.len();
        let size = offsets.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("the fixture is a valid file")
    }

    /// Table 349, whole, with the two rules the clause states as prose.
    ///
    /// `/Trapped` is a **name** — "[t]his shall be the name `True`, not the boolean value
    /// `true`" — and an entry that is not a text string has not stated a value, which is the
    /// clause's own "the value associated with any such key shall be a text string".
    #[test]
    fn table_349_is_read_and_its_two_prose_rules_hold() {
        let doc = document(
            "<< /Title (Annual report) /Author (John Doe) /Subject (Money) \
             /Keywords (annual, report) /Creator (My Word Processor) /Producer (An exporter) \
             /CreationDate (D:20140314124211+01'00') /ModDate (D:20140924212303+02'00') \
             /Trapped /True >>",
        );
        let info = Information::read(&doc);
        assert_eq!(info.title.as_deref(), Some("Annual report"));
        assert_eq!(info.author.as_deref(), Some("John Doe"));
        assert_eq!(info.subject.as_deref(), Some("Money"));
        assert_eq!(info.keywords.as_deref(), Some("annual, report"));
        assert_eq!(info.creator.as_deref(), Some("My Word Processor"));
        assert_eq!(info.producer.as_deref(), Some("An exporter"));
        assert_eq!(info.trapped, Trapped::Fully);
        assert_eq!(
            info.created_date().map(|date| date.year),
            Some(2014),
            "§7.9.4's date parses out of the string the file wrote"
        );
        assert!(!info.is_empty());
        assert!(
            !has_metadata_stream(&doc),
            "the fixture states no /Metadata"
        );

        // The boolean the clause forbids is not one of the three names, so the entry falls to
        // its default — and a `/Title` that is not a string has not stated a title.
        let doc = document("<< /Trapped true /Title /NotAString >>");
        let info = Information::read(&doc);
        assert_eq!(info.trapped, Trapped::Unknown);
        assert_eq!(info.title, None);

        // No `/Info` at all is a document that said nothing, not an error.
        let bare = Document::open(
            b"%PDF-2.0\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
              2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\n\
              trailer\n<< /Size 3 /Root 1 0 R >>\n%%EOF\n"
                .to_vec(),
        )
        .expect("a valid file");
        assert!(Information::read(&bare).is_empty());
    }
}
