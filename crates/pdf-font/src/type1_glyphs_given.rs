//! A bare Type 1 program with glyphs appended to its `CharStrings` dictionary — the Type 1
//! counterpart of [`crate::cff::with_glyphs_given`] (`doc/adr/2620`).
//!
//! A Type 1 program is two sections and a trailer, which ISO 32000-2 §9.9.1's Table 125 measures
//! as `/Length1`, `/Length2` and `/Length3`: a cleartext header ending in `eexec`, a section
//! encrypted under Adobe Type 1 Font Format section 7.2's key 55665, and the zeros and
//! `cleartomark` after it. The `CharStrings` dictionary is inside the encrypted section (section
//! 2.4), and each charstring is encrypted again under section 7.3's key 4330, after as many
//! leading bytes as the Private dictionary's `lenIV` states (section 5.12; four where it states
//! none). So a glyph is given by decrypting the section, appending one `/name len RD <bytes> ND`
//! entry per glyph before the dictionary's `end`, raising the dictionary's stated size by as many,
//! and encrypting the section again; the header and the trailer are copied — a carriage return
//! in the white space after `eexec` written as a line feed, for the reason
//! `Sections::header_written` gives — so `/Length1` and `/Length3` keep their values and
//! `/Length2` is the new section's length.
//!
//! **The reading is proved twice before a program is returned** (trap 301): this module's own
//! scan of the rewritten section must find every entry the program held, byte for byte, and each
//! new entry decrypting to the charstring it was written from under the program's `lenIV`; and
//! `read-fonts`, through [`Program`], must read the glyph count, each new glyph's name at its
//! index, its advance, and its outline — `.notdef`'s, or none.

use skrifa::outline::OutlinePen;

use crate::cff::{AddedGlyph, GivenOutline, GlyphToGive, MOST_GLYPHS_GIVEN};
use crate::type1::{Program, Type1Error};

/// Adobe Type 1 Font Format section 7.2: the `eexec` section's initial key.
const EEXEC_KEY: u16 = 55665;
/// Section 7.3: a charstring's initial key.
const CHARSTRING_KEY: u16 = 4330;
/// Section 7.2: the leading bytes of the `eexec` section, which carry no text.
const EEXEC_LEADING: usize = 4;
/// Section 5.12: the leading bytes of a charstring where the Private dictionary states no `lenIV`.
const DEFAULT_LEN_IV: i64 = 4;
/// Section 6.4's `hsbw`, and `sbw` after the escape byte 12.
const HSBW: u8 = 13;
/// The escape byte of a two-byte charstring command (section 6.3).
const ESCAPE: u8 = 12;
/// `sbw`'s second byte (section 6.4).
const SBW: u8 = 7;
/// `endchar` (section 6.4).
const ENDCHAR: u8 = 14;

/// The longest charstring a given glyph is written as, its `lenIV` leading bytes included: Adobe
/// Type 1 Font Format chapter 10 (page 83) rules that no charstring of a Type 1 program exceeds
/// 65 535 bytes, so a copy of a `.notdef` longer than that is refused rather than written. It
/// bounds what one call adds to the program at [`MOST_GLYPHS_GIVEN`] such charstrings
/// (`doc/adr/2650`).
pub const MOST_CHARSTRING_BYTES: usize = 65_535;

/// The most bytes one call writes: every given glyph copies `.notdef`'s charstring, and a
/// hostile program's `.notdef` times [`MOST_GLYPHS_GIVEN`] would otherwise be the allocation.
/// It is checked before the section is built.
pub const MOST_BYTES_WRITTEN: usize = 64 << 20;

/// A program [`with_glyphs_given`] rewrote, with the three lengths §9.9.1's Table 125 states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphsGiven {
    /// The rewritten program: the header, the re-encrypted section and the trailer.
    pub program: Vec<u8>,
    /// `/Length1`, `/Length2` and `/Length3` of [`Self::program`].
    pub lengths: [usize; 3],
    /// The glyphs appended, by the index `read-fonts` gives them and their name.
    pub added: Vec<AddedGlyph>,
}

/// A bare Type 1 program with the glyphs asked for appended to its `CharStrings` dictionary, each
/// under its own name, where `length1` and `length2` are the stream's `/Length1` and `/Length2`.
///
/// A [`GivenOutline::NotdefsOwn`] glyph is `.notdef`'s charstring with its `hsbw` or `sbw`
/// width operand restated to the advance asked for, and every other byte kept; a
/// [`GivenOutline::Nothing`] glyph is `0 <advance> hsbw endchar`. A glyph named `None` takes the
/// first of `notdef`, `notdef.1`, … the program does not hold, as
/// [`crate::cff::with_glyphs_given`] names it. Every glyph the program held keeps its name and
/// its charstring byte for byte, and the cleartext header — the built-in `/Encoding` with it — is
/// not touched but for a carriage return after `eexec`, which is written as a line feed.
///
/// # Errors
///
/// [`Type1Error::NotWritten`] where the lengths do not divide the program into a header ending
/// in `eexec` and a binary section, the section's `CharStrings` dictionary does not read as
/// `read-fonts` reads it, the program has no `.notdef`, a name is held already or is not a
/// PostScript name, more than [`MOST_GLYPHS_GIVEN`] glyphs or more than [`MOST_BYTES_WRITTEN`]
/// bytes are asked for, `.notdef` states its width where it cannot be restated, or the rewritten
/// program does not read back as written; [`Type1Error::Malformed`] where `read-fonts` does not
/// read the program.
pub fn with_glyphs_given(
    data: &[u8],
    (length1, length2): (usize, usize),
    glyphs: &[GlyphToGive],
) -> Result<GlyphsGiven, Type1Error> {
    if glyphs.len() > MOST_GLYPHS_GIVEN {
        return Err(not_written(
            "more glyphs are asked for than a simple font has codes",
        ));
    }
    let sections = Sections::of(data, length1, length2)?;
    let plain = decrypt(sections.encrypted, EEXEC_KEY);
    let scan =
        Scan::of(&plain).ok_or_else(|| not_written("the CharStrings dictionary does not read"))?;
    let before = Program::parse(data)?;
    agrees_with_reader(&scan, &plain, &before)?;
    let notdef = scan
        .entries
        .iter()
        .find(|entry| entry.name(&plain) == b".notdef")
        .ok_or_else(|| not_written("the program holds no .notdef charstring"))?;
    let notdef_plain = charstring_plain(&plain[notdef.data.clone()], scan.len_iv)
        .ok_or_else(|| not_written("the .notdef charstring is shorter than its lenIV"))?;

    let mut held: std::collections::BTreeSet<Vec<u8>> = scan
        .entries
        .iter()
        .map(|entry| entry.name(&plain).to_vec())
        .collect();
    let mut written = Vec::with_capacity(glyphs.len());
    let mut suffix = 0usize;
    let mut size = data.len();
    for asked in glyphs {
        let name = name_for(asked, &held, &mut suffix)?;
        let advance = i32::try_from(asked.advance)
            .map_err(|_| not_written("an advance does not fit a charstring number"))?;
        let body = body_of(asked.draws, advance, &notdef_plain)?;
        let stored = body
            .len()
            .saturating_add(usize::try_from(scan.len_iv.max(0)).unwrap_or(usize::MAX));
        if stored > MOST_CHARSTRING_BYTES {
            return Err(not_written(
                "a given glyph's charstring would pass the 65 535 bytes a charstring may hold",
            ));
        }
        size = size
            .saturating_add(body.len())
            .saturating_add(name.len())
            .saturating_add(usize::try_from(scan.len_iv.max(0)).unwrap_or(usize::MAX))
            .saturating_add(32);
        if size > MOST_BYTES_WRITTEN {
            return Err(not_written(
                "the rewritten program would pass the writer's bound",
            ));
        }
        // Held from here on so a later glyph of the same call cannot take the name again.
        held.insert(name.as_bytes().to_vec());
        written.push(Written {
            name,
            advance,
            draws: asked.draws,
        });
    }

    let first = u16::try_from(scan.entries.len())
        .map_err(|_| not_written("the program holds more glyphs than a glyph index reaches"))?;
    // The section is written once, into the program, and encrypted where it lies: a hostile
    // `.notdef` makes it the size of the writer's bound, and the program is the one copy held.
    let mut program = Vec::with_capacity(size);
    program.extend(sections.header_written());
    let section_at = program.len();
    section_with(
        &mut program,
        (&plain, &scan, notdef),
        &written,
        &notdef_plain,
    )?;
    encrypt_in_place(&mut program[section_at..], EEXEC_KEY);
    let section_length = program.len().saturating_sub(section_at);
    program.extend_from_slice(sections.trailer);
    let mut added = Vec::with_capacity(written.len());
    for (at, Written { name, .. }) in written.iter().enumerate() {
        let glyph = u16::try_from(at)
            .ok()
            .and_then(|at| first.checked_add(at))
            .ok_or_else(|| not_written("a given glyph's index passes a glyph index"))?;
        added.push(AddedGlyph {
            glyph,
            name: name.clone(),
        });
    }
    let given = GlyphsGiven {
        lengths: [section_at, section_length, sections.trailer.len()],
        program,
        added,
    };
    read_back(&given, (&plain, &scan), (&written, &notdef_plain), glyphs)?;
    Ok(given)
}

/// One glyph [`with_glyphs_given`] appends: its name, its advance and what it draws. Its
/// charstring is built from these by [`body_of`] where it is written and again where it is read
/// back, rather than held, since each can be a copy of a `.notdef` as long as the program.
struct Written {
    name: String,
    advance: i32,
    draws: GivenOutline,
}

/// The charstring a glyph is given, before charstring encryption: `0 <advance> hsbw endchar`,
/// or `.notdef`'s own with its width restated.
fn body_of(draws: GivenOutline, advance: i32, notdef_plain: &[u8]) -> Result<Vec<u8>, Type1Error> {
    match draws {
        GivenOutline::Nothing => {
            let mut body = number(0);
            body.extend(number(advance));
            body.extend([HSBW, ENDCHAR]);
            Ok(body)
        }
        GivenOutline::NotdefsOwn => with_width(notdef_plain, advance)
            .ok_or_else(|| not_written("the .notdef charstring's width cannot be restated")),
    }
}

/// The program divided by its stated lengths.
struct Sections<'a> {
    header: &'a [u8],
    /// Where the white space after `eexec` begins in [`Self::header`].
    white: usize,
    encrypted: &'a [u8],
    trailer: &'a [u8],
}

impl<'a> Sections<'a> {
    /// The header, which ends in `eexec` and the white space after it; a binary section, which
    /// the header's PFB segment marker or a hexadecimal section would rule out (section 7.2
    /// distinguishes the two forms by the first four bytes); and the trailer.
    fn of(data: &'a [u8], length1: usize, length2: usize) -> Result<Self, Type1Error> {
        let end = length1
            .checked_add(length2)
            .filter(|end| *end <= data.len())
            .ok_or_else(|| not_written("Length1 and Length2 pass the program's end"))?;
        let header = &data[..length1];
        let encrypted = &data[length1..end];
        let trailer = &data[end..];
        let stated = header
            .iter()
            .rposition(|byte| !is_white(*byte))
            .map_or(&header[..0], |last| &header[..=last]);
        if !stated.ends_with(b"eexec") || header.first() == Some(&0x80) {
            return Err(not_written("Length1 does not end the header at eexec"));
        }
        if encrypted.len() < EEXEC_LEADING
            || encrypted[..EEXEC_LEADING].iter().all(u8::is_ascii_hexdigit)
        {
            return Err(not_written("the eexec section is not in binary form"));
        }
        Ok(Self {
            header,
            white: stated.len(),
            encrypted,
            trailer,
        })
    }

    /// The header as it is written: every byte copied, but a carriage return in the white space
    /// after `eexec`, which is written as a line feed.
    ///
    /// Adobe Type 1 Font Format section 7.2 rules that the first ciphertext byte is not white
    /// space, so every white space byte after `eexec` comes before the section and either one
    /// reads the same to the standard. `read-fonts` follows `FreeType`'s rule instead and skips a
    /// carriage return there only where a line feed occurs somewhere later in the data: for a
    /// header ending `eexec` and a carriage return, where the section starts would then depend on
    /// whether the re-encrypted bytes happen to hold a line feed. A line feed it skips whatever
    /// follows, and the substitution keeps the header's length, so `/Length1` is unchanged
    /// (`doc/adr/2650`).
    fn header_written(&self) -> impl Iterator<Item = u8> + '_ {
        let (stated, white) = self.header.split_at(self.white);
        stated.iter().copied().chain(
            white
                .iter()
                .map(|&byte| if byte == b'\r' { b'\n' } else { byte }),
        )
    }
}

/// One `CharStrings` entry of the decrypted section, by byte range.
#[derive(Debug, Clone)]
struct Entry {
    /// The name, without its `/`.
    name: std::ops::Range<usize>,
    /// The charstring's bytes, still under charstring encryption.
    data: std::ops::Range<usize>,
    /// The token that reads the bytes (`RD` or `-|`).
    reader: std::ops::Range<usize>,
    /// The token after the bytes that defines the entry (`ND`, `|-`), where one follows.
    definer: Option<std::ops::Range<usize>>,
}

impl Entry {
    fn name<'p>(&self, plain: &'p [u8]) -> &'p [u8] {
        &plain[self.name.clone()]
    }
}

/// What the decrypted section states about its charstrings.
#[derive(Debug)]
struct Scan {
    /// The `lenIV` in force where `CharStrings` is read.
    len_iv: i64,
    /// The size `CharStrings <n> dict` states, by byte range.
    size: std::ops::Range<usize>,
    /// The size's value.
    stated: u64,
    /// Every entry, in the order the section holds them.
    entries: Vec<Entry>,
    /// Where the dictionary's closing `end` or `def` begins, where new entries go.
    close: usize,
}

/// A token of the section, by kind and byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// `/name`, the range without the `/`.
    Name(std::ops::Range<usize>),
    /// An integer.
    Int(i64, std::ops::Range<usize>),
    /// `<n> RD <n bytes>`: the bytes, and the reader token's range.
    Binary(std::ops::Range<usize>, std::ops::Range<usize>),
    /// Any other run, a string, a procedure brace or a bracket.
    Other(std::ops::Range<usize>),
}

/// The tokens of a decrypted section, read by the rules `read-fonts` reads them by (white space,
/// the delimiters, `%` comments, strings, and the `<n> RD` blob); its `Program` is asked to agree
/// before anything this reads is trusted.
struct Tokens<'p> {
    plain: &'p [u8],
    at: usize,
}

// Every position below is an index into `plain`, a slice in memory, and is at most one past a
// byte `get` has just returned (a string's depth is at most the bytes read), so no step of one
// and no length measured inside `plain` can reach `usize::MAX`: the saturating forms state that
// bound and never saturate.
impl Tokens<'_> {
    fn skip_white(&mut self) {
        while self.plain.get(self.at).is_some_and(|byte| is_white(*byte)) {
            self.at = self.at.saturating_add(1);
        }
    }

    fn run_end(&self, from: usize) -> usize {
        self.plain[from..]
            .iter()
            .position(|byte| is_white(*byte) || is_delimiter(*byte))
            .map_or(self.plain.len(), |len| from.saturating_add(len))
    }

    /// The next token, or `None` at the end or at a string that never closes.
    fn next(&mut self) -> Option<Token> {
        loop {
            self.skip_white();
            let start = self.at;
            let byte = *self.plain.get(start)?;
            match byte {
                b'%' => {
                    self.at = self.plain[start..]
                        .iter()
                        .position(|byte| matches!(byte, b'\r' | b'\n'))
                        .map_or(self.plain.len(), |len| start.saturating_add(len));
                }
                b'(' => {
                    let mut depth = 0usize;
                    let mut at = start;
                    loop {
                        match *self.plain.get(at)? {
                            b'\\' => at = at.saturating_add(1),
                            b'(' => depth = depth.saturating_add(1),
                            b')' => {
                                // The opening `(` counted one, so a `)` finds `depth` at one or more.
                                depth = depth.saturating_sub(1);
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        at = at.saturating_add(1);
                    }
                    self.at = at.saturating_add(1);
                    return Some(Token::Other(start..self.at));
                }
                b'<' => {
                    let close = self.plain[start..].iter().position(|byte| *byte == b'>')?;
                    self.at = start.saturating_add(close).saturating_add(1);
                    return Some(Token::Other(start..self.at));
                }
                b'/' => {
                    let end = self.run_end(start.saturating_add(1));
                    self.at = end;
                    return Some(Token::Name(start.saturating_add(1)..end));
                }
                _ if is_delimiter(byte) => {
                    self.at = start.saturating_add(1);
                    return Some(Token::Other(start..self.at));
                }
                _ => {
                    let end = self.run_end(start);
                    self.at = end;
                    let text = std::str::from_utf8(&self.plain[start..end]).ok();
                    let Some(value) = text.and_then(|text| text.parse::<i64>().ok()) else {
                        return Some(Token::Other(start..end));
                    };
                    let mut ahead = Tokens {
                        plain: self.plain,
                        at: end,
                    };
                    ahead.skip_white();
                    let reader_start = ahead.at;
                    let reader_end = ahead.run_end(reader_start);
                    let reader = &self.plain[reader_start..reader_end];
                    if reader_start < self.plain.len() && (reader == b"RD" || reader == b"-|") {
                        // One space after the reader token, then the bytes (section 2.4).
                        let from = reader_end.checked_add(1)?;
                        let to = from.checked_add(usize::try_from(value).ok()?)?;
                        if to > self.plain.len() {
                            return None;
                        }
                        self.at = to;
                        return Some(Token::Binary(from..to, reader_start..reader_end));
                    }
                    return Some(Token::Int(value, start..end));
                }
            }
        }
    }
}

impl Scan {
    /// The section's `lenIV` and `CharStrings` dictionary, or `None` where it states none, or
    /// one whose entries do not each read as a name and a blob.
    fn of(plain: &[u8]) -> Option<Self> {
        let mut tokens = Tokens { plain, at: 0 };
        let mut len_iv = DEFAULT_LEN_IV;
        loop {
            match tokens.next()? {
                Token::Name(name) if &plain[name.clone()] == b"lenIV" => {
                    let Token::Int(value, _) = tokens.next()? else {
                        return None;
                    };
                    len_iv = value;
                }
                Token::Name(name) if &plain[name.clone()] == b"CharStrings" => break,
                _ => {}
            }
        }
        let Token::Int(stated, size) = tokens.next()? else {
            return None;
        };
        let stated = u64::try_from(stated).ok()?;
        let mut entries: Vec<Entry> = Vec::new();
        let close = loop {
            let before = tokens.at;
            match tokens.next()? {
                Token::Other(range)
                    if &plain[range.clone()] == b"end"
                        || (&plain[range.clone()] == b"def" && !entries.is_empty()) =>
                {
                    break range.start;
                }
                Token::Name(name) => {
                    let Token::Binary(data, reader) = tokens.next()? else {
                        return None;
                    };
                    let mut ahead = Tokens {
                        plain,
                        at: tokens.at,
                    };
                    let definer = match ahead.next() {
                        Some(Token::Other(range))
                            if &plain[range.clone()] != b"end"
                                && &plain[range.clone()] != b"def" =>
                        {
                            tokens.at = ahead.at;
                            Some(range)
                        }
                        _ => None,
                    };
                    entries.push(Entry {
                        name,
                        data,
                        reader,
                        definer,
                    });
                }
                _ => {
                    // `dict dup begin` and the like, which name no glyph.
                    debug_assert!(tokens.at > before, "a token always advances");
                }
            }
        };
        Some(Self {
            len_iv,
            size,
            stated,
            entries,
            close,
        })
    }
}

/// Whether the scan reads the glyphs `read-fonts` read: the same names in the same order, but
/// for `.notdef`, which `read-fonts` moves to index 0 by swapping it with the glyph there.
fn agrees_with_reader(scan: &Scan, plain: &[u8], program: &Program) -> Result<(), Type1Error> {
    let disagrees =
        || not_written("the CharStrings dictionary does not read as read-fonts reads it");
    let mut names: Vec<&[u8]> = scan.entries.iter().map(|entry| entry.name(plain)).collect();
    if let Some(at) = names.iter().position(|name| *name == b".notdef") {
        names.swap(0, at);
    }
    let read = program.glyph_names();
    if read.len() != names.len() {
        return Err(disagrees());
    }
    if read
        .iter()
        .zip(&names)
        .any(|(read, scanned)| read.as_bytes() != *scanned)
    {
        return Err(disagrees());
    }
    Ok(())
}

/// The section with `written` appended before the dictionary's close and its stated size raised
/// by as many; the leading bytes, and every byte of every entry held, kept.
fn section_with(
    section: &mut Vec<u8>,
    (plain, scan, notdef): (&[u8], &Scan, &Entry),
    written: &[Written],
    notdef_plain: &[u8],
) -> Result<(), Type1Error> {
    let size = scan
        .stated
        .checked_add(u64::try_from(written.len()).unwrap_or(u64::MAX))
        .ok_or_else(|| not_written("the CharStrings size cannot be raised"))?;
    let reader = &plain[notdef.reader.clone()];
    let definer = notdef
        .definer
        .as_ref()
        .map(|range| &plain[range.clone()])
        .ok_or_else(|| not_written("the .notdef entry has no defining token to copy"))?;
    section.extend_from_slice(&plain[..scan.size.start]);
    section.extend_from_slice(size.to_string().as_bytes());
    section.extend_from_slice(&plain[scan.size.end..scan.close]);
    for Written {
        name,
        advance,
        draws,
    } in written
    {
        let body = body_of(*draws, *advance, notdef_plain)?;
        let encrypted = charstring_encrypted(&body, scan.len_iv)
            .ok_or_else(|| not_written("lenIV does not fit a charstring"))?;
        section.push(b'/');
        section.extend_from_slice(name.as_bytes());
        section.extend_from_slice(format!(" {} ", encrypted.len()).as_bytes());
        section.extend_from_slice(reader);
        section.push(b' ');
        section.extend_from_slice(&encrypted);
        section.push(b' ');
        section.extend_from_slice(definer);
        section.push(b'\n');
    }
    section.extend_from_slice(&plain[scan.close..]);
    Ok(())
}

/// Whether `given` reads as it was written, by this module's scan and by `read-fonts`.
fn read_back(
    given: &GlyphsGiven,
    (plain, scan): (&[u8], &Scan),
    (written, notdef_plain): (&[Written], &[u8]),
    glyphs: &[GlyphToGive],
) -> Result<(), Type1Error> {
    let differs = || not_written("the rewritten program does not read back as it was written");
    let [header, section, _] = given.lengths;
    let stored = given
        .program
        .get(header..header.saturating_add(section))
        .ok_or_else(differs)?;
    // This scan's decrypted copy is dropped before `read-fonts` makes its own below.
    let section = decrypt(stored, EEXEC_KEY);
    let again = Scan::of(&section).ok_or_else(differs)?;
    let stated = u64::try_from(written.len())
        .ok()
        .and_then(|added| scan.stated.checked_add(added));
    // Both counts are lengths of vectors in memory, so their sum cannot saturate.
    if again.len_iv != scan.len_iv
        || again.entries.len() != scan.entries.len().saturating_add(written.len())
        || Some(again.stated) != stated
    {
        return Err(differs());
    }
    for (was, is) in scan.entries.iter().zip(&again.entries) {
        if was.name(plain) != is.name(&section)
            || plain[was.data.clone()] != section[is.data.clone()]
        {
            return Err(differs());
        }
    }
    for (glyph, is) in written.iter().zip(&again.entries[scan.entries.len()..]) {
        let read = charstring_plain(&section[is.data.clone()], again.len_iv);
        let body = body_of(glyph.draws, glyph.advance, notdef_plain)?;
        if is.name(&section) != glyph.name.as_bytes() || read != Some(body) {
            return Err(differs());
        }
    }
    drop(section);

    let program = Program::parse(&given.program)?;
    let names = program.glyph_names();
    if names.len() != scan.entries.len().saturating_add(written.len()) {
        return Err(differs());
    }
    let notdef_outline = outline(&program, 0)?;
    for (glyph, asked) in given.added.iter().zip(glyphs) {
        if names.get(usize::from(glyph.glyph)).map(String::as_str) != Some(glyph.name.as_str()) {
            return Err(differs());
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "the advance was checked to fit an i32, and a charstring's width is read \
                      exactly as a whole number below 2^24 or not at all"
        )]
        let wanted = asked.advance as f32;
        if program.advance(glyph.glyph)? != Some(wanted) {
            return Err(differs());
        }
        let drawn = outline(&program, glyph.glyph)?;
        let expected = match asked.draws {
            GivenOutline::Nothing => Vec::new(),
            GivenOutline::NotdefsOwn => notdef_outline.clone(),
        };
        if drawn != expected {
            return Err(differs());
        }
    }
    Ok(())
}

/// One glyph's outline as `read-fonts` draws it, each command's coordinates as bits.
fn outline(program: &Program, glyph: u16) -> Result<Vec<(u8, [u32; 6])>, Type1Error> {
    /// Records each command.
    struct Recorded(Vec<(u8, [u32; 6])>);
    impl OutlinePen for Recorded {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.push((0, [x.to_bits(), y.to_bits(), 0, 0, 0, 0]));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.push((1, [x.to_bits(), y.to_bits(), 0, 0, 0, 0]));
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            self.0.push((
                2,
                [cx.to_bits(), cy.to_bits(), x.to_bits(), y.to_bits(), 0, 0],
            ));
        }
        fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
            self.0.push((
                3,
                [
                    cx0.to_bits(),
                    cy0.to_bits(),
                    cx1.to_bits(),
                    cy1.to_bits(),
                    x.to_bits(),
                    y.to_bits(),
                ],
            ));
        }
        fn close(&mut self) {
            self.0.push((4, [0; 6]));
        }
    }
    let mut pen = Recorded(Vec::new());
    program.draw(glyph, &mut pen)?;
    Ok(pen.0)
}

/// The name `asked` is given: its own, refused where the program holds it or it is not a
/// PostScript name a `/name` token reads whole, or the first of `notdef`, `notdef.1`, … not held.
fn name_for(
    asked: &GlyphToGive,
    held: &std::collections::BTreeSet<Vec<u8>>,
    suffix: &mut usize,
) -> Result<String, Type1Error> {
    if let Some(name) = &asked.name {
        if held.contains(name.as_bytes()) {
            return Err(not_written(
                "a name asked for is one the program already holds",
            ));
        }
        let readable = !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && !is_delimiter(byte));
        if !readable {
            return Err(not_written(
                "a name asked for is not one a /name token reads",
            ));
        }
        return Ok(name.clone());
    }
    // At most one candidate past every name held, so the walk is bounded by them.
    loop {
        let candidate = if *suffix == 0 {
            "notdef".to_owned()
        } else {
            format!("notdef.{suffix}")
        };
        *suffix = suffix.saturating_add(1);
        if !held.contains(candidate.as_bytes()) {
            return Ok(candidate);
        }
    }
}

/// `charstring` with its first command's width operand — `hsbw`'s second, `sbw`'s third —
/// written as `advance`, or `None` where the charstring does not open with either command over
/// plain numbers (section 6.4: every charstring begins with one of them).
fn with_width(charstring: &[u8], advance: i32) -> Option<Vec<u8>> {
    let mut operands: Vec<std::ops::Range<usize>> = Vec::new();
    let mut at = 0;
    loop {
        let byte = *charstring.get(at)?;
        let len = match byte {
            32..=246 => 1,
            247..=254 => 2,
            255 => 5,
            _ => break,
        };
        // `at` is inside `charstring` (its byte was just read) and `len` is at most five, so the
        // sum cannot saturate; past the end it is refused here rather than sliced.
        let end = at.saturating_add(len);
        if end > charstring.len() || operands.len() == 4 {
            return None;
        }
        operands.push(at..end);
        at = end;
    }
    let width = match (
        charstring.get(at),
        charstring.get(at.saturating_add(1)),
        operands.len(),
    ) {
        (Some(&HSBW), _, 2) => operands.get(1)?.clone(),
        (Some(&ESCAPE), Some(&SBW), 4) => operands.get(2)?.clone(),
        _ => return None,
    };
    let mut out = Vec::with_capacity(charstring.len().saturating_add(4));
    out.extend_from_slice(&charstring[..width.start]);
    out.extend(number(advance));
    out.extend_from_slice(&charstring[width.end..]);
    Some(out)
}

/// One charstring number, in section 6.2's encoding.
fn number(value: i32) -> Vec<u8> {
    match value {
        // Each arm's arithmetic stays inside the byte its range was chosen for.
        -107..=107 => vec![u8::try_from(value.saturating_add(139)).unwrap_or(0)],
        108..=1131 => {
            let v = value.saturating_sub(108);
            vec![
                u8::try_from((v / 256).saturating_add(247)).unwrap_or(0),
                u8::try_from(v % 256).unwrap_or(0),
            ]
        }
        -1131..=-108 => {
            let v = value.saturating_neg().saturating_sub(108);
            vec![
                u8::try_from((v / 256).saturating_add(251)).unwrap_or(0),
                u8::try_from(v % 256).unwrap_or(0),
            ]
        }
        _ => {
            let mut out = vec![255];
            out.extend_from_slice(&value.to_be_bytes());
            out
        }
    }
}

/// A charstring's plain bytes: decrypted and its `len_iv` leading bytes dropped, or as stored
/// where `len_iv` is negative, which is how `read-fonts` reads an unencrypted charstring; `None`
/// where the charstring is shorter than its leading bytes.
fn charstring_plain(stored: &[u8], len_iv: i64) -> Option<Vec<u8>> {
    let Ok(leading) = usize::try_from(len_iv) else {
        return Some(stored.to_vec());
    };
    let plain = decrypt(stored, CHARSTRING_KEY);
    plain.get(leading..).map(<[u8]>::to_vec)
}

/// `body` as a charstring is stored: `len_iv` leading zero bytes and the whole encrypted
/// (section 7.3, which restricts no leading byte), or `body` itself where `len_iv` is negative;
/// `None` where `len_iv` passes the writer's bound.
fn charstring_encrypted(body: &[u8], len_iv: i64) -> Option<Vec<u8>> {
    let Ok(leading) = usize::try_from(len_iv) else {
        return Some(body.to_vec());
    };
    if leading > MOST_BYTES_WRITTEN {
        return None;
    }
    let mut plain = vec![0u8; leading];
    plain.extend_from_slice(body);
    Some(encrypt(&plain, CHARSTRING_KEY))
}

/// Section 7.1's encryption under the initial key `r`, of `bytes` where they lie.
fn encrypt_in_place(bytes: &mut [u8], mut r: u16) {
    for byte in bytes {
        let cipher = *byte ^ r.to_be_bytes()[0];
        r = step(cipher, r);
        *byte = cipher;
    }
}

/// Section 7.1's decryption under the initial key `r`.
fn decrypt(cipher: &[u8], mut r: u16) -> Vec<u8> {
    cipher
        .iter()
        .map(|&byte| {
            let plain = byte ^ r.to_be_bytes()[0];
            r = step(byte, r);
            plain
        })
        .collect()
}

/// Section 7.1's encryption under the initial key `r`.
fn encrypt(plain: &[u8], mut r: u16) -> Vec<u8> {
    plain
        .iter()
        .map(|&byte| {
            let cipher = byte ^ r.to_be_bytes()[0];
            r = step(cipher, r);
            cipher
        })
        .collect()
}

/// Section 7.1's key step after the ciphertext byte `cipher`: `r = (c + r) * c1 + c2`, modulo
/// 65536, with `c1` 52845 and `c2` 22719.
fn step(cipher: u8, r: u16) -> u16 {
    u16::from(cipher)
        .wrapping_add(r)
        .wrapping_mul(52845)
        .wrapping_add(22719)
}

/// White space as a Type 1 program's PostScript reads it.
fn is_white(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | 0x0c | 0)
}

/// PostScript's delimiters.
fn is_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

fn not_written(detail: &str) -> Type1Error {
    Type1Error::NotWritten {
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
#[path = "type1_glyphs_given_tests.rs"]
mod tests;
