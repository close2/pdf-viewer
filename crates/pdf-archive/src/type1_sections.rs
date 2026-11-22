//! A Type 1 font program's three portions, measured as ISO 32000-2 §9.9's Table 125 states them.
//!
//! Table 124 files under `/FontFile` a "Type 1 font program, in the original (noncompact) format
//! described in Adobe Type 1 Font Format", and Table 125 gives its stream three lengths, one per
//! portion: `/Length1` the clear-text portion, `/Length2` the encrypted portion and `/Length3` the
//! fixed-content portion, a `/Length3` of 0 saying that the 512 zeros and `cleartomark` were not
//! included. §9.9's NOTE 2 says what the portions are and in which form PDF carries the second:
//!
//! > Although the encrypted portion of a standard Type 1 font can be in binary or ASCII
//! > hexadecimal format, PDF supports only the binary format.
//!
//! So two shapes a producer writes are not the program those tables describe, and a reader that
//! takes the lengths at their word cannot read either (`doc/adr/3375`):
//!
//! - **A program framed in PFB segment headers** (Adobe Technical Note 5040's packaging for one
//!   operating system's font files): each portion is preceded by six bytes that are no part of the
//!   Type 1 format, so `/Length1` either counts them or does not reach the end of the clear text.
//! - **An encrypted portion in ASCII hexadecimal**, which NOTE 2 excludes: Adobe Type 1 Font
//!   Format section 7.2 tells the two forms apart by the first four bytes after `eexec` — all
//!   four hexadecimal digits is the hexadecimal form — and both encode the same ciphertext, so
//!   the binary bytes are the hexadecimal digits read in pairs.
//! - **Lengths that do not divide the program where its portions meet**: `issue4630`'s
//!   `/Length2 0` over an encrypted portion of thousands of bytes, which tells a reader the
//!   encrypted portion is empty.
//!
//! [`type1_sections`] reads where the portions meet from the program itself — after `eexec` and
//! the white space that ends its line, and at the first of the zeros before `cleartomark` — so a
//! validator can compare the stated lengths with them and a converter can state them.

/// The white space of PostScript's syntax, which Adobe Type 1 Font Format section 2.2 uses.
const WHITE_SPACE: &[u8] = b" \t\r\n\x0c\x00";

/// The PFB segment marker, the first byte of every segment header.
const PFB_MARKER: u8 = 0x80;

/// The operator that ends the clear-text portion.
const EEXEC: &[u8] = b"eexec";

/// The operator that ends the fixed-content portion.
const CLEARTOMARK: &[u8] = b"cleartomark";

/// How a Type 1 font program opens (Adobe Type 1 Font Format section 2.4).
const TYPE1: &[u8] = b"%!";

/// A Type 1 program, unframed, and where its three portions meet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type1Sections {
    /// The program as Table 124 files it: the PFB segment headers removed where it had them, every
    /// other byte as it was.
    pub program: Vec<u8>,
    /// Whether the program was framed in PFB segment headers.
    pub framed: bool,
    /// Whether the encrypted portion was written in ASCII hexadecimal, which [`Self::program`]
    /// holds as the binary bytes it encodes.
    pub hexadecimal: bool,
    /// The clear-text portion's length, through `eexec` and the white space after it: `/Length1`.
    pub clear: usize,
    /// The encrypted portion's length: `/Length2`.
    pub encrypted: usize,
    /// The fixed-content portion's length, from the first of its zeros to the end, or 0 where the
    /// program includes none: `/Length3`.
    pub fixed: usize,
    /// How many white-space bytes end the clear-text portion, inside which a stated `/Length1`
    /// still divides the portions where they meet.
    clear_slack: usize,
    /// How many white-space bytes end the encrypted portion, for the same reason.
    encrypted_slack: usize,
}

impl Type1Sections {
    /// Whether a stream stating these lengths divides the program where its portions meet.
    ///
    /// A boundary stated anywhere in the white space at a portion's end is the same division:
    /// producers differ on which side of that line ending they count, and no reader of the
    /// encrypted portion is misled by it.
    #[must_use]
    pub fn stated_by(&self, length1: Option<usize>, length2: Option<usize>) -> bool {
        let (Some(length1), Some(length2)) = (length1, length2) else {
            return false;
        };
        let clear_end = self.clear;
        let encrypted_end = self.clear.saturating_add(self.encrypted);
        let first = clear_end.saturating_sub(self.clear_slack)..=clear_end;
        let second = encrypted_end.saturating_sub(self.encrypted_slack)..=encrypted_end;
        first.contains(&length1) && second.contains(&length1.saturating_add(length2))
    }
}

/// The program's portions, or `None` where it is no Type 1 program whose clear text ends in
/// `eexec`, or a PFB framing that does not read to its end.
#[must_use]
pub fn type1_sections(data: &[u8]) -> Option<Type1Sections> {
    let (program, framed) = if data.first() == Some(&PFB_MARKER) {
        (unframed(data)?, true)
    } else {
        (data.to_vec(), false)
    };
    if !program.starts_with(TYPE1) {
        return None;
    }
    let eexec = find(&program, EEXEC, 0)?.saturating_add(EEXEC.len());
    let clear = eexec.saturating_add(white_space_from(&program, eexec));
    let clear_slack = clear.saturating_sub(eexec);
    let fixed_start = fixed_start(&program, clear).unwrap_or(program.len());
    let encrypted = program.get(clear..fixed_start).unwrap_or_default();
    let hexadecimal = encrypted
        .get(..4)
        .is_some_and(|first| first.iter().all(u8::is_ascii_hexdigit));
    if hexadecimal {
        let binary = from_hexadecimal(encrypted)?;
        let mut written = Vec::with_capacity(program.len());
        written.extend_from_slice(program.get(..clear).unwrap_or_default());
        written.extend_from_slice(&binary);
        written.extend_from_slice(program.get(fixed_start..).unwrap_or_default());
        return Some(Type1Sections {
            clear,
            encrypted: binary.len(),
            fixed: program.len().saturating_sub(fixed_start),
            clear_slack,
            encrypted_slack: 0,
            program: written,
            framed,
            hexadecimal,
        });
    }
    // Without the fixed-content portion nothing in the program marks where the encrypted portion
    // ends — `bug1157493` states a `/Length3` of 532 over a program that holds no zeros at all —
    // so any end past its first byte is one the program does not contradict.
    let encrypted_slack = if fixed_start == program.len() {
        encrypted.len().saturating_sub(1)
    } else {
        encrypted
            .iter()
            .rev()
            .take_while(|byte| WHITE_SPACE.contains(byte))
            .count()
    };
    Some(Type1Sections {
        clear,
        encrypted: fixed_start.saturating_sub(clear),
        fixed: program.len().saturating_sub(fixed_start),
        clear_slack,
        encrypted_slack,
        program,
        framed,
        hexadecimal,
    })
}

/// The bytes a hexadecimal encrypted portion encodes, its white space skipped; `None` where it
/// holds anything else or an odd number of digits, which no reading of section 7.2 completes.
fn from_hexadecimal(digits: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(digits.len() / 2);
    let mut high: Option<u8> = None;
    for byte in digits {
        if WHITE_SPACE.contains(byte) {
            continue;
        }
        let value = u8::try_from(char::from(*byte).to_digit(16)?).ok()?;
        match high.take() {
            None => high = Some(value),
            Some(first) => out.push(first.wrapping_mul(16).wrapping_add(value)),
        }
    }
    high.is_none().then_some(out)
}

/// The program inside PFB segments: each segment's six-byte header (marker, type, little-endian
/// length) removed and the segments concatenated, up to the end-of-file segment of type 3.
///
/// Two damaged framings are read as the bytes they hold, because the program is the same either
/// way and only the framing is wrong: a segment whose stated length runs past the stream ends with
/// the stream (`issue14462_reduced`'s last segment states 532 bytes and holds fewer), and bytes
/// after a segment that open no header are the program's own continuation (`bug860632` frames an
/// empty third segment and writes its 512 zeros and `cleartomark` after it unframed).
fn unframed(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len());
    let mut at = 0;
    while at < data.len() {
        if data.get(at) != Some(&PFB_MARKER) {
            if at == 0 {
                return None;
            }
            out.extend_from_slice(data.get(at..).unwrap_or_default());
            return Some(out);
        }
        match data.get(at.saturating_add(1))? {
            1 | 2 => {
                let header = data.get(at.saturating_add(2)..at.saturating_add(6))?;
                let length = usize::try_from(u32::from_le_bytes(header.try_into().ok()?)).ok()?;
                let start = at.saturating_add(6);
                let end = start.saturating_add(length).min(data.len());
                out.extend_from_slice(data.get(start..end).unwrap_or_default());
                at = end;
            }
            3 => return Some(out),
            _ => return None,
        }
    }
    Some(out)
}

/// Where the fixed-content portion begins: the first of the zeros before the last `cleartomark`
/// past the clear text, where the program includes one.
fn fixed_start(program: &[u8], clear: usize) -> Option<usize> {
    let mark = rfind(program, CLEARTOMARK, clear)?;
    let mut start = mark;
    while start > clear
        && program
            .get(start.saturating_sub(1))
            .is_some_and(|byte| *byte == b'0' || WHITE_SPACE.contains(byte))
    {
        start = start.saturating_sub(1);
    }
    let zeros = program.get(start..mark)?;
    let first_zero = zeros.iter().position(|byte| *byte == b'0')?;
    Some(start.saturating_add(first_zero))
}

/// How many white-space bytes begin `data[from..]`.
fn white_space_from(data: &[u8], from: usize) -> usize {
    data.get(from..)
        .unwrap_or_default()
        .iter()
        .take_while(|byte| WHITE_SPACE.contains(byte))
        .count()
}

/// The first place `needle` occurs in `data` at or after `from`.
fn find(data: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    data.get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| at.saturating_add(from))
}

/// The last place `needle` occurs in `data` at or after `from`.
fn rfind(data: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    data.get(from..)?
        .windows(needle.len())
        .rposition(|window| window == needle)
        .map(|at| at.saturating_add(from))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program of the three portions, the encrypted one binary bytes that include a `0` and a
    /// line feed, so that its end is found by the zeros rather than by the first white space.
    fn program() -> Vec<u8> {
        let mut out = b"%!FontType1-1.0: F\n/F findfont currentfile eexec\r\n".to_vec();
        out.extend_from_slice(&[0xD9, 0x30, 0x0A, 0xFF, 0x80, 0x01]);
        out.push(b'\n');
        for _ in 0..8 {
            out.extend_from_slice(&[b'0'; 64]);
            out.push(b'\n');
        }
        out.extend_from_slice(b"cleartomark\n");
        out
    }

    #[test]
    fn the_portions_meet_after_eexec_and_at_the_first_zero() {
        let data = program();
        let sections = type1_sections(&data).expect("a Type 1 program");
        assert!(!sections.framed);
        assert_eq!(sections.clear, 50, "through eexec and its CR LF");
        assert_eq!(sections.encrypted, 7, "six binary bytes and the line feed");
        assert_eq!(sections.fixed, 8 * 65 + 12);
        assert!(sections.stated_by(Some(50), Some(7)));
        assert!(
            sections.stated_by(Some(48), Some(8)),
            "inside the white space either side"
        );
        assert!(
            !sections.stated_by(Some(50), Some(0)),
            "issue4630's /Length2 0"
        );
        assert!(!sections.stated_by(None, Some(7)));
    }

    #[test]
    fn pfb_segment_headers_are_removed_and_the_program_measured_without_them() {
        let data = program();
        let mut framed = Vec::new();
        for (kind, part) in [(1u8, &data[..50]), (2, &data[50..57]), (1, &data[57..])] {
            framed.extend_from_slice(&[PFB_MARKER, kind]);
            framed.extend_from_slice(&u32::try_from(part.len()).expect("small").to_le_bytes());
            framed.extend_from_slice(part);
        }
        framed.extend_from_slice(&[PFB_MARKER, 3]);
        let sections = type1_sections(&framed).expect("a framed Type 1 program");
        assert!(sections.framed);
        assert_eq!(sections.program, data);
        assert_eq!((sections.clear, sections.encrypted), (50, 7));

        // Damaged framings hold the same program: an empty segment followed by the rest unframed,
        // and a last segment stating more than the stream holds.
        let mut continued = framed[..6 + 50 + 6 + 7].to_vec();
        continued.extend_from_slice(&[PFB_MARKER, 1, 0, 0, 0, 0]);
        continued.extend_from_slice(&data[57..]);
        assert_eq!(
            type1_sections(&continued).map(|it| it.program),
            Some(data.clone())
        );
        let mut overclaimed = framed[..6 + 50 + 6 + 7].to_vec();
        overclaimed.extend_from_slice(&[PFB_MARKER, 1, 0xFF, 0xFF, 0, 0]);
        overclaimed.extend_from_slice(&data[57..]);
        assert_eq!(
            type1_sections(&overclaimed).map(|it| it.program),
            Some(data)
        );
    }

    #[test]
    fn a_hexadecimal_encrypted_portion_is_held_as_the_binary_it_encodes() {
        let mut data = b"%!PS-AdobeFont-1.0: F\ncurrentfile eexec\nD9D6 6F\n63\n".to_vec();
        data.extend_from_slice(&[b'0'; 512]);
        data.extend_from_slice(b"\ncleartomark\n");
        let sections = type1_sections(&data).expect("a Type 1 program");
        assert!(sections.hexadecimal);
        assert_eq!((sections.clear, sections.encrypted), (40, 4));
        assert_eq!(
            sections.program.get(40..44),
            Some(&[0xD9, 0xD6, 0x6F, 0x63][..])
        );
        assert_eq!(sections.fixed, 512 + 13);
    }

    #[test]
    fn a_program_without_its_fixed_content_states_none_and_an_unknown_frame_is_not_read() {
        let data = b"%!PS-AdobeFont-1.0: F\ncurrentfile eexec\n\xD9\x10".to_vec();
        let sections = type1_sections(&data).expect("a Type 1 program");
        assert_eq!(
            (sections.clear, sections.encrypted, sections.fixed),
            (40, 2, 0)
        );
        assert!(
            sections.stated_by(Some(40), Some(1)),
            "no fixed content marks the end"
        );
        assert!(
            !sections.stated_by(Some(40), Some(0)),
            "but the portion is not empty"
        );
        assert!(
            type1_sections(&[PFB_MARKER, 9, 1, 0, 0, 0, b'%']).is_none(),
            "no segment type 9"
        );
        assert!(
            type1_sections(b"\x00\x01\x00\x00").is_none(),
            "no Type 1 program"
        );
    }
}
