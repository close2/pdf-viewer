//! The version a file states, from §7.5.2's header and Table 29's `/Version`.
//!
//! Two places say it and the standard ranks them, so reading one is reading neither. Table 29,
//! in §7.7.2:
//!
//! > If the header specifies a later version, or if this entry is absent, the document shall
//! > conform to the version specified in the header.
//!
//! The entry counts only "if later than the version specified in the file's header", so the two
//! numbers are read and the later one wins.
//!
//! **Why a reader wants it at all**: Annex I is normative, and it asks for a warning rather than
//! a behaviour — "[i]f a PDF processor opens a PDF file with a version number newer than the
//! version that it supports … it should warn the user that it is unlikely to be able to read the
//! document successfully". The number was located and thrown away here for three hundred and
//! sixty sessions: `xref::read` searches for `%PDF-` to fix the byte offsets and reads no digits.
//! ADR 0206 is where that was found; the annex had no ledger row before it.

use std::fmt;

use crate::Document;

/// The version of the specification a file says it conforms to: `1.7`, `2.0`.
///
/// Ordered by component, which is why this is a pair of numbers rather than the name the file
/// writes: Table 29's rule is *later than*, and `"1.10"` beside `"1.7"` is the string comparison
/// that gets that backwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// The part before the point: 1 or 2 for every version the standard defines.
    pub major: u8,
    /// The part after it, "a single digit number between 0 (30h) and 9 (39h)".
    pub minor: u8,
}

impl Version {
    /// The version of ISO 32000 this implementation is written against.
    ///
    /// §7.5.2's own sentence about what a 2.0 file says: "[a] PDF processor that writes a file
    /// that conforms to this document shall identify the version … as 2.0". A reader supports
    /// what it was written against, and this constant is what Annex I compares a file with.
    pub const SUPPORTED: Self = Self { major: 2, minor: 0 };

    /// Reads `n.m` from the bytes immediately after a `%PDF-` or `%FDF-` marker.
    ///
    /// §7.5.2: "[t]he file header shall consist of '%PDF1. n ' or '%PDF2. n ' followed by a
    /// single EOL marker, where ' n ' is a single digit number between 0 (30h) and 9 (39h)" —
    /// so exactly one digit either side of the point, and anything else is not a version this
    /// can name. Being liberal here would invent a number the file does not state.
    fn from_digits(after: &[u8]) -> Option<Self> {
        let [major, b'.', minor, ..] = *after else {
            return match *after {
                [major, b'.', minor] => digits(major, minor),
                _ => None,
            };
        };
        digits(major, minor)
    }
}

/// One version from two ASCII digits, or nothing.
fn digits(major: u8, minor: u8) -> Option<Version> {
    let major = char::from(major).to_digit(10)?;
    let minor = char::from(minor).to_digit(10)?;
    Some(Version {
        major: u8::try_from(major).ok()?,
        minor: u8::try_from(minor).ok()?,
    })
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl Document {
    /// The version the file's header states, if it states one this clause recognises.
    ///
    /// §7.5.2 puts the header on the first line and NOTE 1 licenses bytes in front of it, so the
    /// same window `xref::read` measures offsets from is the one searched here — one number, one
    /// rule, and a file whose header is preceded by junk keeps its version.
    #[must_use]
    pub fn header_version(&self) -> Option<Version> {
        let bytes = self.bytes();
        let window = bytes.len().min(crate::xref::HEADER_SEARCH_WINDOW);
        let start = bytes.read(0..window);
        for marker in [b"%PDF-", b"%FDF-"] {
            if let Some(at) = start
                .windows(marker.len())
                .position(|candidate| candidate == marker)
                && let Some(after) = start.get(at.saturating_add(marker.len())..)
                && let Some(version) = Version::from_digits(after.get(..3).unwrap_or(after))
            {
                return Some(version);
            }
        }
        None
    }

    /// The version the document conforms to: the header's, unless Table 29's `/Version` is later.
    ///
    /// `None` where the header states none and the catalog states none, which is a file that
    /// never said — not a file that said 1.0.
    #[must_use]
    pub fn version(&self) -> Option<Version> {
        let header = self.header_version();
        let Ok(catalog) = self.catalog() else {
            return header;
        };
        let stated = self.get_key(&catalog, "Version");
        // "The value of this entry shall be a name object, not a number", and a document that
        // writes a number instead has not stated the entry the table defines.
        let catalog_version = stated
            .as_name()
            .and_then(|name| Version::from_digits(name.as_bytes()));
        match (header, catalog_version) {
            (Some(header), Some(catalog)) => Some(header.max(catalog)),
            (header, catalog) => header.or(catalog),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Version;
    use crate::Document;

    /// A one-page document whose header and catalog can be varied.
    fn document(header: &str, catalog: &str) -> Document {
        let body = format!(
            "{header}\n\
             1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog} >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n\
             trailer\n<< /Root 1 0 R /Size 3 >>\n"
        );
        Document::open(body.into_bytes()).expect("the fixture is a document")
    }

    #[test]
    fn the_header_states_the_version() {
        let document = document("%PDF-1.7", "");
        assert_eq!(
            document.header_version(),
            Some(Version { major: 1, minor: 7 })
        );
        assert_eq!(document.version(), Some(Version { major: 1, minor: 7 }));
    }

    /// Table 29: the catalog's entry counts "if later than the version specified in the file's
    /// header", and "[i]f the header specifies a later version … the document shall conform to
    /// the version specified in the header".
    #[test]
    fn the_catalogs_version_counts_only_when_it_is_later() {
        assert_eq!(
            document("%PDF-1.4", "/Version /2.0").version(),
            Some(Version { major: 2, minor: 0 })
        );
        assert_eq!(
            document("%PDF-1.7", "/Version /1.4").version(),
            Some(Version { major: 1, minor: 7 })
        );
    }

    /// "The value of this entry shall be a name object, not a number."
    #[test]
    fn a_version_written_as_a_number_is_not_the_entry_the_table_defines() {
        assert_eq!(
            document("%PDF-1.4", "/Version 2.0").version(),
            Some(Version { major: 1, minor: 4 })
        );
    }

    /// NOTE 1 licenses arbitrary bytes before the header, and the version survives them.
    #[test]
    fn junk_before_the_header_does_not_hide_the_version() {
        let document = document("harmless preamble\n%PDF-2.0", "");
        assert_eq!(document.version(), Some(Version { major: 2, minor: 0 }));
    }

    /// Ordering is by component, which is the whole reason this is not a string: `1.10` is later
    /// than `1.7` as a version and earlier as text.
    #[test]
    fn a_version_orders_by_component() {
        assert!(Version { major: 1, minor: 7 } < Version { major: 2, minor: 0 });
        assert!(Version { major: 1, minor: 9 } > Version { major: 1, minor: 7 });
        assert_eq!(Version::SUPPORTED.to_string(), "2.0");
    }
}
