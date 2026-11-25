//! The one channel every repair this reader makes is reported on (ADR 3477, ADR 3505).
//!
//! # Why a reader that repairs must say so
//!
//! This crate reads malformed files as far as it can: a cross-reference table rebuilt by
//! scanning, a stream's extent found by its `endstream` where `/Length` is wrong, a 34-digit
//! integer read as the nearest real. A viewer needs that. A validator reading the same file
//! through the same code judges the *repaired* reading rather than the bytes, and certifies a
//! file it never saw — which is how the PDF/A validator's misses were found, one at a time, by
//! another program (ADRs 3393, 3427). So every repair is a [`Repair`]: a [`RepairClass`] that
//! says what the file broke and what was read instead, and a [`Site`] that says where.
//!
//! # Where a repair is recorded
//!
//! On the [`crate::Document`] that read the file, in a log beside its object cache
//! ([`crate::Document::record_repair`], [`crate::Document::repairs`]). The log is about how the
//! bytes were read, not about what they say, which is why it sits beside the cache rather than
//! being a change to the document (`CLAUDE.md`, "`pdf_syntax::Document` stays immutable").
//! Two readers below the document carry their own until one exists to take them: the
//! [`crate::Lexer`] (and the [`crate::Parser`] over it), whose repairs a caller takes with
//! [`crate::Lexer::repairs`], and the [`crate::XrefTable`], which is read before the document
//! is built and whose repairs the document takes when it is.
//!
//! # The classes
//!
//! Every class this crate can record is a `pub static` here and is listed in [`CLASSES`], so a
//! validator can map each one to the rule it touches and a test can prove the map whole. The
//! crates above declare their own slices the same way. A class's [`RepairClass::name`] is stable:
//! it is what a validator's mapping table and a report name, so renaming one is a change to both.
//!
//! What is **not** a class is said beside the code that does it, with a `// not a repair:`
//! comment naming the clause that makes the reader's reading the specification's — an absent
//! entry's default, §7.3.4.2's ignored escape, §7.3.4.3's assumed final digit.

use crate::object::ObjectId;

/// One kind of repair: what the file broke, and what this reader read instead.
#[derive(Debug)]
pub struct RepairClass {
    /// The stable identifier, `crate/what-was-repaired`, which mappings and reports name.
    pub name: &'static str,
    /// The ISO 32000-2 clause the file departs from.
    pub clause: &'static str,
    /// One sentence: what the file wrote, and what was read in its place.
    pub read_instead: &'static str,
}

/// Where in the file a repair was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Site {
    /// The file as a whole: its cross-reference chain, its trailer, its header.
    WholeFile,
    /// A byte offset into the file.
    Offset(usize),
    /// An indirect object, as a whole.
    Object(ObjectId),
    /// A byte offset inside an indirect object's *data* — a decoded object stream, a content
    /// stream — rather than into the file.
    Within(ObjectId, usize),
    /// Inside a content stream: the one of the page or form `XObject` with this identifier, or one
    /// its reader holds without an object. A content stream's data crosses an archival conversion
    /// byte for byte, so a repair made in it is one a rewrite of the file does not undo, where the
    /// same class at [`Self::Object`] is one it does (ADR 3521).
    Content(Option<ObjectId>),
}

/// One repair: its class, and where it was made.
#[derive(Debug, Clone, Copy)]
pub struct Repair {
    /// What kind of repair it was.
    pub class: &'static RepairClass,
    /// Where it was made.
    pub at: Site,
}

impl Repair {
    /// A repair of `class` at `at`.
    #[must_use]
    pub const fn new(class: &'static RepairClass, at: Site) -> Self {
        Self { class, at }
    }

    /// This repair with an offset into a slice moved to the slice's place: an offset into
    /// bytes that begin at file offset `base` becomes a file offset, and one into an object's
    /// data becomes [`Site::Within`] that object. A site that names no offset is kept.
    #[must_use]
    pub fn placed(self, base: usize, object: Option<ObjectId>) -> Self {
        let at = match (self.at, object) {
            (Site::Offset(offset), Some(object)) => Site::Within(object, offset),
            (Site::Offset(offset), None) => Site::Offset(offset.saturating_add(base)),
            (other, _) => other,
        };
        Self { at, ..self }
    }
}

/// Two repairs are the same where their classes' names and their sites are.
impl PartialEq for Repair {
    fn eq(&self, other: &Self) -> bool {
        self.class.name == other.class.name && self.at == other.at
    }
}

impl Eq for Repair {}

impl PartialOrd for Repair {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// By class name, then by site: the order a report lists them in.
impl Ord for Repair {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.class
            .name
            .cmp(other.class.name)
            .then(self.at.cmp(&other.at))
    }
}

/// How many sites of one class a log keeps; the rest are counted.
///
/// A content stream of a million unmatched `)` is a million repairs of one class, and a log that
/// kept them all would be the decompression bomb this crate's [`crate::Limits`] exist to refuse.
/// What a validator needs of a class is that it happened and where, first; the count says how
/// often.
pub const SITES_KEPT_PER_CLASS: usize = 4096;

/// The repairs one reader has made, deduplicated, at most [`SITES_KEPT_PER_CLASS`] sites a class.
#[derive(Debug, Default, Clone)]
pub struct RepairLog {
    /// The repairs kept, in [`Repair`]'s order.
    kept: std::collections::BTreeSet<Repair>,
    /// Per class name, how many sites were kept and how many more were not.
    counts: std::collections::BTreeMap<&'static str, (usize, u64)>,
}

impl RepairLog {
    /// Records one repair; a repair already recorded is not recorded twice.
    pub fn record(&mut self, repair: Repair) {
        if self.kept.contains(&repair) {
            return;
        }
        let (kept, beyond) = self.counts.entry(repair.class.name).or_default();
        if *kept < SITES_KEPT_PER_CLASS {
            *kept = kept.saturating_add(1);
            self.kept.insert(repair);
        } else {
            *beyond = beyond.saturating_add(1);
        }
    }

    /// The repairs kept, in order.
    pub fn repairs(&self) -> impl Iterator<Item = Repair> + '_ {
        self.kept.iter().copied()
    }

    /// How many repairs of the class named `name` were recorded past the ones kept — counted as
    /// recorded, since a site past the log cannot be told from one recorded before it.
    #[must_use]
    pub fn beyond_the_log(&self, name: &str) -> u64 {
        self.counts.get(name).map_or(0, |(_, beyond)| *beyond)
    }

    /// Each class the log holds a repair of, once, in name order: what a reader is told a file
    /// needed, where the sites are the validator's business.
    pub fn classes(&self) -> impl Iterator<Item = &'static RepairClass> + '_ {
        let mut last: Option<&'static str> = None;
        self.kept.iter().filter_map(move |repair| {
            let new = last != Some(repair.class.name);
            last = Some(repair.class.name);
            new.then_some(repair.class)
        })
    }

    /// Whether nothing was repaired.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kept.is_empty()
    }
}

macro_rules! classes {
    ($($(#[$doc:meta])* $item:ident = $name:literal, $clause:literal, $instead:literal;)*) => {
        $(
            $(#[$doc])*
            pub static $item: RepairClass = RepairClass {
                name: $name,
                clause: $clause,
                read_instead: $instead,
            };
        )*
        /// Every class this crate records, in declaration order (ADR 3477).
        pub static CLASSES: &[&RepairClass] = &[$(&$item),*];
    };
}

classes! {
    // ---- the lexer, §7.2 and §7.3 ----------------------------------------------------------

    /// A `>` that is not half of `>>`, or a `)` that closes no string.
    DELIMITER_UNMATCHED = "syntax/delimiter-unmatched", "ISO 32000-2 §7.2.3",
        "a delimiter that opens or closes nothing was read as a one-byte keyword";
    /// A `#` in a name not followed by two hexadecimal digits.
    NAME_ESCAPE_INCOMPLETE = "syntax/name-escape-incomplete", "ISO 32000-2 §7.3.5",
        "a NUMBER SIGN in a name not followed by two hexadecimal digits was read as itself";
    /// A literal string whose parentheses never balance.
    STRING_UNTERMINATED = "syntax/string-unterminated", "ISO 32000-2 §7.3.4.2",
        "a literal string with no closing parenthesis was read to the end of the data";
    /// A hexadecimal string with no closing `>`.
    HEXADECIMAL_STRING_UNTERMINATED = "syntax/hexadecimal-string-unterminated",
        "ISO 32000-2 §7.3.4.3",
        "a hexadecimal string with no closing GREATER-THAN SIGN was read to the end of the data";
    /// A byte in a hexadecimal string that is neither a digit nor white space.
    HEXADECIMAL_STRING_STRAY_BYTE = "syntax/hexadecimal-string-stray-byte",
        "ISO 32000-2 §7.3.4.3",
        "a byte in a hexadecimal string that is neither a digit nor white space was skipped";
    /// An integer too large for 64 bits.
    INTEGER_PAST_I64 = "syntax/integer-past-i64", "ISO 32000-2 §7.3.3",
        "an integer too large for 64 bits was read as the nearest real";
    /// A number written with an exponent, or one too large for a real.
    NUMBER_WITH_AN_EXPONENT = "syntax/number-with-an-exponent", "ISO 32000-2 §7.3.3",
        "a number written in exponential form, which the clause does not admit, was read as \
         its value";
    /// A run like `1.2.3` or `12pt` whose front alone spells a number.
    NUMBER_SALVAGED = "syntax/number-salvaged", "ISO 32000-2 §7.3.3",
        "a run of regular characters spelling no number was read as the number off its front";

    // ---- the object parser, §7.3.7 to §7.3.10 ----------------------------------------------

    /// `N G R` with a number or generation outside its range.
    REFERENCE_OUT_OF_RANGE = "syntax/reference-out-of-range", "ISO 32000-2 §7.3.10",
        "an indirect reference whose object or generation number is out of range was read as \
         separate integers";
    /// A dictionary key written twice.
    DUPLICATE_KEY = "syntax/duplicate-key", "ISO 32000-2 §7.3.7",
        "a key written twice in one dictionary was read once, at its first entry";
    /// A dictionary entry whose key is not a name.
    KEY_NOT_A_NAME = "syntax/key-not-a-name", "ISO 32000-2 §7.3.7",
        "a dictionary entry whose key is not a name was skipped";
    /// The `stream` keyword's end-of-line marker padded, or a CARRIAGE RETURN alone.
    STREAM_KEYWORD_LINE_ENDING = "syntax/stream-keyword-line-ending", "ISO 32000-2 §7.3.8.1",
        "the stream keyword's end-of-line marker, padded or a CARRIAGE RETURN alone, was \
         taken as the start of the data";
    /// A stream whose stated `/Length` does not end it.
    STREAM_EXTENT_FROM_ENDSTREAM = "syntax/stream-extent-from-endstream", "ISO 32000-2 §7.3.8.2",
        "a stream whose /Length is absent, wrong or not an integer was read to its endstream";
    /// A stream with no `endstream` keyword.
    ENDSTREAM_MISSING = "syntax/endstream-missing", "ISO 32000-2 §7.3.8.1",
        "a stream with no endstream keyword was read without one";
    /// An indirect object with no `endobj` keyword.
    ENDOBJ_MISSING = "syntax/endobj-missing", "ISO 32000-2 §7.3.10",
        "an indirect object with no endobj keyword was read without one";
    /// A generation number in an object header too large for 16 bits.
    GENERATION_OUT_OF_RANGE = "syntax/generation-out-of-range", "ISO 32000-2 §7.3.10",
        "an object header's generation number past 65 535 was read as 0";

    // ---- objects, §7.3.10 and §7.5 -----------------------------------------------------------

    /// An object at its entry's offset that does not parse.
    OBJECT_UNPARSABLE = "syntax/object-unparsable", "ISO 32000-2 §7.3.10",
        "an indirect object that does not parse was read as null";
    /// An object whose loading reaches itself through references.
    REFERENCE_CYCLE = "syntax/reference-cycle", "ISO 32000-2 §7.3.10",
        "an object that its own loading refers back to was read as null at the second visit";
    /// A reference chain deeper than this reader follows.
    REFERENCE_CHAIN_TOO_DEEP = "syntax/reference-chain-too-deep", "ISO 32000-2 §7.3.10",
        "a reference chain deeper than this reader's bound was read as null";
    /// A file with no `%PDF-` or `%FDF-` header, read with its offsets counted from byte zero.
    HEADER_MISSING = "syntax/header-missing", "ISO 32000-2 §7.5.2",
        "a file with no header was read with its byte offsets counted from its first byte";
    /// A cross-reference entry whose offset is not its object's header.
    XREF_OFFSET_SEARCHED = "syntax/xref-offset-searched", "ISO 32000-2 §7.5.4",
        "an object whose entry's offset is not its header was found by its own header";
    /// A compressed object its object stream does not hold.
    COMPRESSED_OBJECT_ABSENT = "syntax/compressed-object-absent", "ISO 32000-2 §7.5.7",
        "an object its entry places in an object stream that does not hold it was read as null";
    /// A file too large to be held for a header scan.
    SCAN_REFUSED = "syntax/scan-refused", "ISO 32000-2 §7.5.4",
        "a repair needing the whole file was not made, because the file could not be held";

    // ---- the cross-reference information, §7.5.4 to §7.5.8 -------------------------------

    /// The cross-reference table rebuilt by scanning for object headers.
    XREF_REBUILT = "syntax/xref-rebuilt", "ISO 32000-2 §7.5.4",
        "cross-reference information that is absent, unreadable or wrong was replaced by a \
         scan for object headers";
    /// A `/Prev` chain that loops, points nowhere, or is not an offset.
    XREF_CHAIN_CUT = "syntax/xref-chain-cut", "ISO 32000-2 §7.5.6",
        "a /Prev or /XRefStm that loops, is not an offset, or reaches no section ended the \
         chain there";
    /// Entries a section declares that could not be read.
    XREF_ENTRIES_LOST = "syntax/xref-entries-lost", "ISO 32000-2 §7.5.4",
        "a cross-reference section's entries that do not read were declared unread";
    /// A subsection renumbered because its in-use entries agree on a displacement.
    XREF_SUBSECTION_REALIGNED = "syntax/xref-subsection-realigned", "ISO 32000-2 §7.5.4",
        "a subsection whose first object number is wrong was renumbered by its objects' headers";
    /// A trailer found by scanning rather than where the section ends.
    TRAILER_FOUND_BY_SCAN = "syntax/trailer-found-by-scan", "ISO 32000-2 §7.5.5",
        "a trailer dictionary absent or unreadable where the section ends was found by scanning";
    /// A `trailer` keyword followed by no dictionary, or none found at all.
    TRAILER_UNREADABLE = "syntax/trailer-unreadable", "ISO 32000-2 §7.5.5",
        "a trailer that does not parse as a dictionary, or is not found, was read as empty";
    /// A catalog chosen by its `/Type`, the trailer naming none.
    CATALOG_FOUND_BY_SCAN = "syntax/catalog-found-by-scan", "ISO 32000-2 §7.5.5",
        "a trailer naming no catalog was given the first object whose /Type is /Catalog";
    /// A cross-reference stream's `/W`, `/Size` or `/Index` of the wrong type or shape.
    XREF_STREAM_FIELDS = "syntax/xref-stream-fields", "ISO 32000-2 §7.5.8.2",
        "a cross-reference stream's /W, /Size or /Index of the wrong type or shape was read \
         as far as it goes";
    /// A cross-reference stream record of a type the clause does not define.
    XREF_STREAM_RECORD = "syntax/xref-stream-record", "ISO 32000-2 §7.5.8.3",
        "a cross-reference stream record of an undefined type or with an out-of-range field was \
         read as a free entry";
    /// `startxref` not where the file's end puts it, or not an offset.
    STARTXREF_SEARCHED = "syntax/startxref-searched", "ISO 32000-2 §7.5.5",
        "a startxref outside the file's last bytes, or not an offset, was searched for or \
         replaced by a scan";

    // ---- object streams, §7.5.7 --------------------------------------------------------------

    /// An object stream's header pairs, `/N` or `/First` missing, malformed or out of order.
    OBJECT_STREAM_HEADER = "syntax/object-stream-header", "ISO 32000-2 §7.5.7",
        "an object stream's /N, /First or header pairs, missing, malformed or out of order, \
         were read as far as they go";
    /// An object stream member that does not parse, or lies past the decoded data.
    OBJECT_STREAM_MEMBER_LOST = "syntax/object-stream-member-lost", "ISO 32000-2 §7.5.7",
        "an object stream member that does not parse or lies past the data was read as null";
    /// Compressed objects entered into a rebuilt table from their streams' own headers.
    OBJECT_STREAM_MEMBERS_ENUMERATED = "syntax/object-stream-members-enumerated",
        "ISO 32000-2 §7.5.7",
        "compressed objects no cross-reference stream listed were entered from the object \
         streams' own headers";

    // ---- text strings, §7.9.2.2 -----------------------------------------------------------

    /// A text string whose bytes contradict the encoding they are read in.
    TEXT_STRING_UNDECODABLE = "syntax/text-string-undecodable", "ISO 32000-2 §7.9.2.2",
        "a text string with an odd byte or an unpaired surrogate after the UTF-16BE prefix, a \
         malformed sequence after the UTF-8 one, or a byte Table D.3 leaves undefined, was read \
         with U+FFFD in place of each";

    // ---- stream data, §7.4 ---------------------------------------------------------------

    /// A filter that stopped short of its data's end, or found it corrupt.
    STREAM_DATA_DAMAGED = "syntax/stream-data-damaged", "ISO 32000-2 §7.4",
        "a stream whose filters stopped on damaged data was read as the prefix they decoded";
    /// A stream whose filters refused its data outright.
    STREAM_DATA_UNDECODABLE = "syntax/stream-data-undecodable", "ISO 32000-2 §7.4",
        "a stream whose filters refused its data was read as having none";
    /// `/DecodeParms` that does not match `/Filter` entry for entry.
    DECODE_PARMS_MISMATCHED = "syntax/decode-parms-mismatched", "ISO 32000-2 §7.3.8.2",
        "a /DecodeParms that does not match /Filter entry for entry was matched as far as it goes";

    // ---- encryption, §7.6 ------------------------------------------------------------------

    /// A trailer `/ID` missing or malformed in an encrypted file.
    ENCRYPTION_ID_MISSING = "syntax/encryption-id-missing", "ISO 32000-2 §7.6.4.3",
        "an encrypted file's /ID, missing or not a string, was read as the empty string";
    /// A string or stream that does not decrypt.
    DECRYPTION_FAILED = "syntax/decryption-failed", "ISO 32000-2 §7.6.3",
        "a string or stream that does not decrypt was read as empty";
    /// An encryption dictionary's `/V`, or a standard handler's `/R`, `/O`, `/U` or `/P`, absent
    /// or of the wrong type.
    ENCRYPTION_ENTRY = "syntax/encryption-entry", "ISO 32000-2 §7.6.4.2",
        "an encryption dictionary's /V, or a standard security handler's /R, /O, /U or /P, \
         absent or of the wrong type, was read as zero or empty";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Lexer};

    /// Every class's name is unique and names its crate, so a mapping keyed by name is a map.
    #[test]
    fn every_class_name_is_unique_and_names_its_crate() {
        let names: std::collections::BTreeSet<_> = CLASSES.iter().map(|class| class.name).collect();
        assert_eq!(names.len(), CLASSES.len());
        assert!(names.iter().all(|name| name.starts_with("syntax/")));
    }

    /// ISO 32000-2 §7.3.4.2: a literal string is balanced parentheses; one that never closes is
    /// read to the end, and the lexer says so where it opened.
    #[test]
    fn an_unterminated_string_is_a_repair_at_its_opening() {
        let mut lexer = Lexer::new(b"1 (abc");
        while lexer.next_token().is_some() {}
        let classes: Vec<_> = lexer
            .repairs()
            .iter()
            .map(|r| (r.class.name, r.at))
            .collect();
        assert_eq!(classes, [(STRING_UNTERMINATED.name, Site::Offset(2))]);
    }

    /// A well-formed run of tokens is no repair at all.
    #[test]
    fn well_formed_tokens_record_nothing() {
        let mut lexer = Lexer::new(b"<< /A (b) /C <0a> /D [1 2.5 -3] >>");
        while lexer.next_token().is_some() {}
        assert_eq!(lexer.repairs(), &[] as &[Repair]);
    }

    /// ISO 32000-2 §7.5.4: a file with no cross-reference table is read by a scan, and the
    /// document's log says the table was rebuilt.
    #[test]
    fn a_rebuilt_table_is_in_the_documents_log() {
        let bytes = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
                      2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n%%EOF\n";
        let document = Document::open(bytes.to_vec()).expect("a scan finds the objects");
        let names: Vec<_> = document.repairs().iter().map(|r| r.class.name).collect();
        assert!(names.contains(&XREF_REBUILT.name), "{names:?}");
    }

    /// The log keeps one site once, and at most [`SITES_KEPT_PER_CLASS`] sites a class.
    #[test]
    fn the_log_is_bounded_per_class() {
        let mut log = RepairLog::default();
        for offset in 0..SITES_KEPT_PER_CLASS {
            log.record(Repair::new(&DUPLICATE_KEY, Site::Offset(offset)));
            log.record(Repair::new(&DUPLICATE_KEY, Site::Offset(offset)));
        }
        for offset in SITES_KEPT_PER_CLASS..SITES_KEPT_PER_CLASS + 10 {
            log.record(Repair::new(&DUPLICATE_KEY, Site::Offset(offset)));
        }
        assert_eq!(log.repairs().count(), SITES_KEPT_PER_CLASS);
        assert_eq!(log.beyond_the_log(DUPLICATE_KEY.name), 10);
    }
}
