//! The version a file states, from §7.5.2's header and Table 29's `/Version`.
//!
//! Two kinds of place say it and the standard ranks them, so reading one is reading neither.
//! Table 29, in §7.7.2:
//!
//! > If the header specifies a later version, or if this entry is absent, the document shall
//! > conform to the version specified in the header.
//!
//! The entry counts only "if later than the version specified in the file's header", so the
//! numbers are read and the later one wins. **There may be more than two of them**: §7.5.6 lets
//! each incremental update carry a catalog of its own and forbids any of them to reduce what an
//! earlier one said, so the file states one version per revision and the document's is the
//! latest. [`Document::version`] has that sentence and what follows from it for a reader.
//!
//! **Why a reader wants it at all**: Annex I is normative, and it asks for a warning rather than
//! a behaviour — "[i]f a PDF processor opens a PDF file with a version number newer than the
//! version that it supports … it should warn the user that it is unlikely to be able to read the
//! document successfully". The number was located and thrown away here for three hundred and
//! sixty sessions: `xref::read` searches for `%PDF-` to fix the byte offsets and reads no digits.
//! ADR 0206 is where that was found; the annex had no ledger row before it.

use std::fmt;

use crate::Document;
use crate::object::{Object, ObjectId};
use crate::xref::Location;

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
    ///
    /// Public because the same two digits are what a *name* spells where a dictionary states a
    /// version — Table 29's `/Version` and Table 245's `/Version` are both read through this, one
    /// crate up, so the rule for what counts as a version lives in one place rather than in each
    /// clause that needs one.
    #[must_use]
    pub fn parse(after: &[u8]) -> Option<Self> {
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
                && let Some(version) = Version::parse(after.get(..3).unwrap_or(after))
            {
                return Some(version);
            }
        }
        None
    }

    /// The version the document conforms to: the latest of the header's and every revision's.
    ///
    /// `None` where the header states none and no catalog states one, which is a file that never
    /// said — not a file that said 1.0.
    ///
    /// **The chain is walked, not only its newest link**, because §7.5.6 makes the entry
    /// cumulative. The clause's sentence about it is one Errata Collection 3 rewrote (Issue
    /// #399, `/State` `Review` `Accepted`), and the amended wording is not quoted here because
    /// it is not in the published text this tree checks its quotations against: it says that an
    /// update's catalog `/Version` *upgrades* the version the document conforms to, considering
    /// the header and any catalog entry already present together, and that the catalog of an
    /// incremental update shall not reduce that version by the entry's value or by its absence.
    ///
    /// So the version at any revision is the version before it or higher, never lower, and the
    /// document's is the latest thing any revision said. Reading the newest catalog alone gets
    /// two files wrong in the same direction — one whose later update states a lower `/Version`,
    /// and one whose later update, rewriting the catalog, simply left the entry out — and the
    /// second is what the amended sentence's *absence* is for.
    ///
    /// The cost is one forwards walk of the `/Prev` chain and one parse of each distinct copy of
    /// the catalog, which is why this is a question a caller asks and not something
    /// [`Document::open`] computes: nothing on the launch path needs the number, and Annex I —
    /// the one clause that does — asks for a warning rather than a behaviour.
    #[must_use]
    pub fn version(&self) -> Option<Version> {
        let mut latest = self.header_version();
        for stated in self.stated_versions() {
            latest = Some(latest.map_or(stated, |so_far| so_far.max(stated)));
        }
        latest
    }

    /// Table 29's `/Version` as each revision of the file stated it, oldest first.
    ///
    /// Two things keep this to one catalog parse for the ordinary file that was written once and
    /// never updated, and to one per *rewritten catalog* for a file that was. A revision whose
    /// catalog stands where the previous revision's stood says nothing new and is not read again.
    /// And a revision whose catalog is the copy this document's own table names is read through
    /// *this* document, cache and all, rather than through a second one built over a copy of the
    /// table — which is the newest revision always, and every earlier one that did not touch the
    /// catalog.
    fn stated_versions(&self) -> Vec<Version> {
        let mut stated = Vec::new();
        let mut previous: Option<(ObjectId, Option<Location>)> = None;
        crate::xref::walk_revisions(self.bytes(), self.limits(), |table| {
            let Some(root) = table.trailer().get("Root") else {
                return;
            };
            let catalog = match root.as_reference() {
                Some(id) => {
                    let at = table.location(id.number);
                    let here = (id, at);
                    if previous.replace(here) == Some(here) {
                        return;
                    }
                    if at.is_some() && at == self.xref().location(id.number) {
                        let Some(dict) = self.get(id).as_dict().cloned() else {
                            return;
                        };
                        self.get_key(&dict, "Version")
                    } else {
                        let revision = self.as_of(table.clone());
                        let Some(dict) = revision.get(id).as_dict().cloned() else {
                            return;
                        };
                        revision.get_key(&dict, "Version")
                    }
                }
                // A `/Root` written as a direct dictionary is not what §7.5.5 asks for, and it
                // is read anyway: the entry is in front of us, and the revision's table adds
                // nothing to a value that is already there.
                None => root
                    .as_dict()
                    .map_or(Object::Null, |dict| self.get_key(dict, "Version")),
            };
            if let Some(version) = as_version(&catalog) {
                stated.push(version);
            }
        });
        if stated.is_empty()
            && let Ok(catalog) = self.catalog()
            && let Some(version) = as_version(&self.get_key(&catalog, "Version"))
        {
            // The chain said nothing, and the document still has a catalog. Two files reach
            // here and both are the same shape: one whose `/Prev` chain cannot be read at all,
            // and one whose chain reads but leads nowhere, which is the document `xref::rebuild`
            // recovered by scanning. Asking this document's own catalog can only add what the
            // file itself states, because a chain that did state a version never gets here.
            stated.push(version);
        }
        stated
    }
}

/// Table 29's `/Version` as a version, where the object is one.
///
/// "The value of this entry shall be a name object, not a number", and a document that writes a
/// number instead has not stated the entry the table defines.
fn as_version(stated: &Object) -> Option<Version> {
    Version::parse(stated.as_name()?.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::Version;
    use crate::Document;

    /// A document of one revision per catalog, each update rewriting object 1.
    ///
    /// §7.5.6's own shape, built by hand so that the reader is tested against the file rather
    /// than against this crate's writer: the objects, a cross-reference section covering exactly
    /// them, a trailer chaining to the section before through `/Prev`, and an `%%EOF` of its own
    /// for each.
    fn updated(header: &str, catalogs: &[&str]) -> Document {
        use std::fmt::Write as _;

        let mut out = String::new();
        let _ = writeln!(out, "{header}");
        let mut previous: Option<usize> = None;
        for (index, catalog) in catalogs.iter().enumerate() {
            let catalog_at = out.len();
            let _ = write!(
                out,
                "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog} >>\nendobj\n"
            );
            let pages_at = out.len();
            if index == 0 {
                out.push_str("2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
            }
            let section_at = out.len();
            out.push_str("xref\n");
            if index == 0 {
                out.push_str("0 3\n0000000000 65535 f \n");
                let _ = write!(out, "{catalog_at:010} 00000 n \n{pages_at:010} 00000 n \n");
            } else {
                let _ = write!(out, "1 1\n{catalog_at:010} 00000 n \n");
            }
            out.push_str("trailer\n<< /Root 1 0 R /Size 3");
            if let Some(previous) = previous {
                let _ = write!(out, " /Prev {previous}");
            }
            let _ = write!(out, " >>\nstartxref\n{section_at}\n%%EOF\n");
            previous = Some(section_at);
        }
        Document::open(out.into_bytes()).expect("the fixture is a document")
    }

    /// One revision is the case every other test in this module builds, and it reads the same
    /// way through the chain walk as it did through the trailer alone.
    #[test]
    fn a_file_of_one_revision_states_the_version_its_catalog_states() {
        assert_eq!(
            updated("%PDF-1.4", &["/Version /2.0"]).version(),
            Some(Version { major: 2, minor: 0 })
        );
        assert_eq!(
            updated("%PDF-1.7", &[""]).version(),
            Some(Version { major: 1, minor: 7 })
        );
    }

    /// §7.5.6: "The catalog of an incremental update shall not reduce the version of the
    /// document with the value, or absence, of the Version entry." The value half: a second
    /// revision states 1.7 over a first that stated 2.0, and the document is still 2.0.
    #[test]
    fn a_later_update_cannot_reduce_the_version_an_earlier_one_reached() {
        assert_eq!(
            updated("%PDF-1.4", &["/Version /2.0", "/Version /1.7"]).version(),
            Some(Version { major: 2, minor: 0 })
        );
    }

    /// The same sentence's other half — "or absence" — which is the likelier file of the two: an
    /// update rewrote the catalog and did not carry the entry over.
    #[test]
    fn an_update_whose_catalog_states_no_version_reduces_nothing() {
        assert_eq!(
            updated("%PDF-1.4", &["/Version /2.0", ""]).version(),
            Some(Version { major: 2, minor: 0 })
        );
    }

    /// And the direction the entry exists for: §7.5.6's "upgrade the current version of the PDF
    /// specification to which the document conforms", which an update states and the header
    /// cannot, because an append may not rewrite the first line of the file.
    #[test]
    fn an_update_upgrades_the_version_the_header_states() {
        assert_eq!(
            updated("%PDF-1.4", &["", "/Version /2.0"]).version(),
            Some(Version { major: 2, minor: 0 })
        );
        assert_eq!(
            updated("%PDF-1.4", &["", "/Version /1.5", "/Version /2.0"]).version(),
            Some(Version { major: 2, minor: 0 })
        );
    }

    /// A document whose cross-references are unusable still states its version.
    ///
    /// The chain yields no revision at all here — `startxref` names bytes that are not a
    /// section — so the table is `xref::rebuild`'s scan and the catalog is this document's own.
    /// Falling back to it can only add what the file states, because a chain that did state a
    /// version never reaches the fallback.
    #[test]
    fn a_document_whose_chain_cannot_be_read_still_states_its_catalogs_version() {
        let whole = updated("%PDF-1.4", &["/Version /2.0"]);
        let mut bytes = whole.bytes().read(0..whole.bytes().len()).to_vec();
        let at = bytes
            .windows(10)
            .position(|window| window == b"startxref\n")
            .expect("the fixture writes one");
        let digits = at.saturating_add(10);
        for byte in &mut bytes[digits..] {
            if byte.is_ascii_digit() {
                *byte = b'9';
            }
        }
        let document = Document::open(bytes).expect("the scan recovers it");
        assert!(
            document.was_recovered(),
            "the fixture has to reach the scan for this to be the case it is about"
        );
        assert_eq!(document.version(), Some(Version { major: 2, minor: 0 }));
    }

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
