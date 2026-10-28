//! The header an output states where its source stated no version, and the source's header kept.
//!
//! ISO 32000-2 §7.5.2 makes the header one line naming a version, `%PDF-` followed by a single
//! digit, a point and a single digit, and asks a processor that writes a file conforming to the
//! document to identify that file's version as 2.0. A converted output is a file this program
//! writes, so its header states the version of the file written, as every output's does
//! (`doc/adr/2558`). A source whose header names no version this tree reads (`%PDF1.4`, `%PDF-1.`,
//! no header at all) still converts: the output's header is the target's, and the bytes the source
//! put where its header belonged are kept in the output's own `xmpMM:History` and in the report,
//! so the archive says what the producer wrote and does not pretend it named a version.

use std::fmt::Write as _;

use pdf_syntax::{Document, Version};

use pdf_archive::Target;

/// How many bytes of the source's first line the record keeps.
///
/// A header is eight bytes and its EOL marker, so 64 holds every header a producer meant and a
/// good deal of what it did not. The bound is what stops a file whose first line is its whole body
/// from writing that body into the packet: the record says how long the line was and stops.
pub(super) const MOST_HEADER_BYTES: usize = 64;

/// The version an output's header states where the source's header and catalog state none.
///
/// The version of the standard each part is built on: ISO 19005-2 on ISO 32000-1, which is PDF
/// 1.7, and ISO 19005-4 on ISO 32000-2, whose §7.5.2 names the version of a file conforming to it
/// as 2.0. Each is what the output *is*: the validator holds a part-2 output to ISO 32000-1's
/// requirements as ISO 19005-2 states them, and a part-4 output to ISO 32000-2's.
#[must_use]
pub(super) fn target_version(target: Target) -> Version {
    match target.part() {
        pdf_archive::Part::Two => Version { major: 1, minor: 7 },
        pdf_archive::Part::Four => Version { major: 2, minor: 0 },
    }
}

/// The `xmpMM:History` parameters recording the source's header, where it stated no version.
///
/// `None` where the source's header or a catalog of any revision states a version
/// ([`Document::version`]), since then the output's header follows what the producer said and
/// nothing was chosen for it.
#[must_use]
pub(super) fn history(source: &Document, target: Target) -> Option<String> {
    if source.version().is_some() {
        return None;
    }
    let written = target_version(target);
    let bytes = source.bytes();
    let window = bytes.read(0..MOST_HEADER_BYTES.saturating_add(1));
    Some(format!(
        "the header %PDF-{written} written for {target}, because the source's header states no \
         version ISO 32000-2 \u{a7}7.5.2 defines; the source's first line was {}",
        first_line(&window)
    ))
}

/// The bytes before the first EOL marker, escaped so that every byte the line held is legible.
///
/// A printable ASCII byte stands for itself, a backslash is doubled, and every other byte is
/// written `\xHH`, so the record reads back to the bytes byte for byte. Past
/// [`MOST_HEADER_BYTES`] the line is cut and the record says so.
fn first_line(window: &[u8]) -> String {
    let line_end = window
        .iter()
        .position(|byte| matches!(byte, b'\r' | b'\n'))
        .unwrap_or(window.len());
    let cut = line_end > MOST_HEADER_BYTES;
    let line = window.get(..line_end.min(MOST_HEADER_BYTES)).unwrap_or(&[]);
    let mut out = String::with_capacity(line.len().saturating_add(32));
    out.push('"');
    for &byte in line {
        match byte {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            0x20..=0x7e => out.push(char::from(byte)),
            _ => {
                let _ = write!(out, "\\x{byte:02X}");
            }
        }
    }
    out.push('"');
    if line.is_empty() {
        out.push_str(" (empty)");
    }
    if cut {
        let _ = write!(out, " (its first {MOST_HEADER_BYTES} bytes; it runs on)");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{MOST_HEADER_BYTES, first_line, history, target_version};
    use pdf_archive::{Flavour, Level, Target};
    use pdf_syntax::{Document, Version};

    /// A one-page file behind `header`, the rest of it well formed.
    fn with_header(header: &[u8]) -> Document {
        let mut bytes = header.to_vec();
        let objects: [&[u8]; 3] = [
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n",
        ];
        let mut offsets = Vec::new();
        for object in objects {
            offsets.push(bytes.len());
            bytes.extend_from_slice(object);
        }
        let xref_at = bytes.len();
        bytes.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n").as_bytes(),
        );
        Document::open(bytes).expect("the fixture is a well-formed file")
    }

    #[test]
    fn each_part_writes_the_version_of_the_standard_it_is_built_on() {
        assert_eq!(
            target_version(Target::Two(Level::B)),
            Version { major: 1, minor: 7 }
        );
        assert_eq!(
            target_version(Target::Four(Flavour::Plain)),
            Version { major: 2, minor: 0 }
        );
    }

    #[test]
    fn a_source_that_states_a_version_records_nothing() {
        let document = with_header(b"%PDF-1.4\n");
        assert_eq!(history(&document, Target::Four(Flavour::Plain)), None);
    }

    #[test]
    fn a_header_with_a_stray_byte_reads_back_byte_for_byte() {
        // PDFBOX-6040-nodeloop's shape: the `D` of `%PDF-` replaced by one byte above 7Fh.
        let document = with_header(b"%P\xB5F-1.4\n");
        let recorded =
            history(&document, Target::Four(Flavour::Plain)).expect("no version is stated");
        assert!(
            recorded.ends_with(r#"the source's first line was "%P\xB5F-1.4""#),
            "{recorded}"
        );
        assert!(
            recorded.contains("%PDF-2.0 written for PDF/A-4"),
            "{recorded}"
        );
    }

    #[test]
    fn a_file_with_no_header_records_its_first_line() {
        let document = with_header(b"%\xE2\xE3\xCF\xD3\n");
        let recorded = history(&document, Target::Two(Level::B)).expect("no version is stated");
        assert!(recorded.contains("%PDF-1.7 written"), "{recorded}");
        assert!(recorded.ends_with(r#""%\xE2\xE3\xCF\xD3""#), "{recorded}");
    }

    #[test]
    fn the_line_is_cut_at_the_bound_and_says_so() {
        let at_the_bound = vec![b'a'; MOST_HEADER_BYTES];
        assert_eq!(
            first_line(&at_the_bound),
            format!("\"{}\"", "a".repeat(MOST_HEADER_BYTES))
        );
        let past = vec![b'a'; MOST_HEADER_BYTES + 1];
        assert_eq!(
            first_line(&past),
            format!(
                "\"{}\" (its first {MOST_HEADER_BYTES} bytes; it runs on)",
                "a".repeat(MOST_HEADER_BYTES)
            )
        );
    }

    #[test]
    fn a_backslash_and_a_quote_are_escaped_so_the_record_is_unambiguous() {
        assert_eq!(first_line(b"%\\x41\"\r\n"), r#""%\\x41\"""#);
        assert_eq!(first_line(b"\n%PDF-1.4"), r#""" (empty)"#);
    }
}
