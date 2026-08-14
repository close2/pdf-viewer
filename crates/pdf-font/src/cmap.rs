//! A composite font's `CMap`: the mapping from character codes to CIDs.
//!
//! ISO 32000-2 §9.7.5.1 states what one is:
//!
//! > A CMap shall specify the mapping from character codes to character selectors. In PDF,
//! > the character selectors shall be CIDs in a CIDFont
//!
//! and §9.7.1 for why the same module answers "how long is a code":
//!
//! > When the current font is composite, the text-showing operators shall behave differently
//! > than with simple fonts. For simple fonts, each byte of a string to be shown selects one
//! > glyph, whereas for composite fonts, a sequence of one or more bytes are decoded to
//! > select a glyph from the descendant CIDFont.
//!
//! So a `CMap` decides two separate things, and both are here because both are stated in
//! terms of its codespace ranges: how many bytes the next code occupies (§9.7.6.2), and
//! which CID that code selects (§9.7.6.2 and §9.7.6.3).
//!
//! # Why the code's length travels with its value
//!
//! §9.7.6.2 looks a code up "in the character code mappings **for codes of that length**", so
//! the one-byte code `20` and the two-byte code `0020` are different codes that happen to
//! share a numeric value. A `CMap` that maps both is legal. [`Code`] therefore carries the
//! length, and every table here is indexed by it — which is also what §9.3.3's word spacing
//! needs, since that rule is about a code's encoded length rather than its value.
//!
//! # This is not [`crate::tounicode`]
//!
//! Both are `CMap` files and both are read with the same lexer, and they answer different
//! questions: this one maps a code to a *CID*, which selects a glyph, and that one maps a
//! code to *text*, which is what the code means. A `bfchar` destination is UTF-16BE text
//! there and a character selector here — see [`CMap::take_chars`] for the clause that
//! decides it.

use std::collections::BTreeMap;

use pdf_syntax::{Lexer, Token};

/// The longest code a `CMap` may define, in bytes.
///
/// ISO 32000-2 §9.7.6.2: "The code length shall not be greater than 4."
const MAX_CODE_BYTES: usize = 4;

/// Bounds the individually-listed mappings one `CMap` may contribute, per code length.
const MAX_SINGLES: usize = 1 << 16;

/// Bounds the ranges, which are cheap individually but not unbounded.
const MAX_RANGES: usize = 1 << 14;

/// Bounds the codespace ranges, which are scanned per code decoded.
const MAX_CODESPACE: usize = 1 << 8;

/// A character code taken from a string shown by a text operator.
///
/// Its length is part of its identity, not a decoding detail: see the module comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code {
    /// The code's value, its bytes read most significant first.
    value: u32,
    /// How many bytes of the string it occupied, from 1 to 4.
    length: u8,
    /// Whether it matched a codespace range.
    ///
    /// ISO 32000-2 §9.7.6.3 calls a code that matched none *invalid* and sends it straight
    /// to a substitute glyph rather than to the character mappings, so the two cases cannot
    /// be told apart by the value alone.
    in_codespace: bool,
}

impl Code {
    /// A simple font's code, which is one byte and always valid (§9.7.1: "each byte of a
    /// string to be shown selects one glyph").
    #[must_use]
    pub const fn single_byte(byte: u8) -> Self {
        Self {
            // `u32::from` is not a const function on this toolchain; the widening is exact.
            value: byte as u32,
            length: 1,
            in_codespace: true,
        }
    }

    /// The code's value.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.value
    }

    /// How many bytes of the string the code occupied (§9.7.6.2), from 1 to 4.
    #[must_use]
    pub const fn length(self) -> u8 {
        self.length
    }

    /// Whether ISO 32000-2 §9.3.3's word spacing applies to this code.
    ///
    /// > Word spacing shall be applied to every occurrence of the single-byte character code
    /// > 32 in a string when using a simple font (including Type 3) or a composite font that
    /// > defines code 32 as a single-byte code. It shall not apply to occurrences of the byte
    /// > value 32 in multiple-byte codes.
    ///
    /// The rule is about the code's *encoded length*, which is why that length is carried
    /// here rather than answered per font: an embedded `CMap` may define codes of several
    /// lengths in one font, and four of the corpus's do.
    #[must_use]
    pub const fn takes_word_spacing(self) -> bool {
        self.length == 1 && self.value == 32
    }
}

/// One codespace range: a pair of bounds of equal length (§9.7.6.2).
#[derive(Debug, Clone, Copy)]
struct Codespace {
    /// How many bytes the bounds have, from 1 to 4.
    length: u8,
    low: [u8; MAX_CODE_BYTES],
    high: [u8; MAX_CODE_BYTES],
}

impl Codespace {
    /// Whether one byte of a code lies inside this range's corresponding byte.
    ///
    /// ISO 32000-2 §9.7.6.2 makes the test per byte rather than on the code's value:
    ///
    /// > A code shall be considered to match the range if it is the same length as the
    /// > bounding codes and the value of each of its bytes lies between the corresponding
    /// > bytes of the lower and upper bounds.
    ///
    /// That is not the same as comparing the whole code numerically, and the difference is
    /// load-bearing on a UTF-8-shaped `CMap`: `<C280> <DFBF>` admits `C2 80` but not
    /// `C2 C0`, which a numeric comparison against `0xC280..=0xDFBF` would accept.
    fn byte_matches(self, index: usize, byte: u8) -> bool {
        let (Some(&low), Some(&high)) = (self.low.get(index), self.high.get(index)) else {
            return false;
        };
        (low..=high).contains(&byte)
    }

    /// Whether a code of this range's own length matches it.
    fn matches(self, bytes: &[u8]) -> bool {
        usize::from(self.length) == bytes.len()
            && bytes
                .iter()
                .enumerate()
                .all(|(index, &byte)| self.byte_matches(index, byte))
    }

    /// How many leading bytes of a string this range accepts, up to its own length.
    ///
    /// §9.7.6.3's recovery for an invalid code is stated in terms of this "partial match".
    fn partial_match(self, bytes: &[u8]) -> usize {
        let limit = usize::from(self.length).min(bytes.len());
        (0..limit)
            .take_while(|&index| {
                bytes
                    .get(index)
                    .is_some_and(|&byte| self.byte_matches(index, byte))
            })
            .count()
    }
}

/// The mappings for codes of one length: individual codes and ranges of them.
#[derive(Debug, Default, Clone)]
struct Mapping {
    /// Codes given a selector one at a time, by `cidchar` or `notdefchar`.
    singles: BTreeMap<u32, u32>,
    /// Spans of codes mapping to consecutive CIDs: low, high, and the first CID.
    ///
    /// Kept as ranges rather than expanded, because a single `<0000> <FFFF>` line would
    /// otherwise allocate sixty-five thousand entries — the same decoding bomb
    /// [`crate::tounicode`] avoids the same way.
    ranges: Vec<(u32, u32, u32)>,
}

impl Mapping {
    /// The selector a code maps to, or `None` where this mapping says nothing about it.
    ///
    /// `consecutive` is what separates the two kinds of range a `CMap` may state. A
    /// `cidrange` numbers its codes upward from the CID it names — §9.7.5.4's example maps
    /// `<20> <7D>` to 231 and following — while a `notdefrange` gives *every* code in the
    /// range the one CID it names. ISO 32000-2 does not spell the second out; §9.7.5.3 hands
    /// the file's syntax to Adobe Technical Note #5014, which does, and §9.7.6.3's purpose for
    /// a notdef mapping says the same thing in different words: it exists "to obtain a
    /// substitute character selector", and a run of undefined codes wants one substitute
    /// rather than a run of consecutive CIDs the `CIDFont` has no reason to carry.
    fn get(&self, code: u32, consecutive: bool) -> Option<u32> {
        if let Some(&cid) = self.singles.get(&code) {
            return Some(cid);
        }
        // First match in file order wins. Ranges in a `CMap` are not supposed to overlap,
        // and where a malformed one does, taking the earliest is the same rule §9.7.4.3
        // states for a repeated CID in `/W`: "the first specification is the one that shall
        // be used".
        for &(low, high, first) in &self.ranges {
            if (low..=high).contains(&code) {
                return Some(if consecutive {
                    first.saturating_add(code.saturating_sub(low))
                } else {
                    first
                });
            }
        }
        None
    }

    /// Whether this mapping says nothing at all.
    fn is_empty(&self) -> bool {
        self.singles.is_empty() && self.ranges.is_empty()
    }
}

/// Which kind of section a `CMap` file's buffered operands belong to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    /// `begincodespacerange`: pairs of bounds.
    Codespace,
    /// `begincidrange` or `beginnotdefrange`: two bounds and a CID.
    Ranges { notdef: bool },
    /// `begincidchar` or `beginnotdefchar`: a code and a CID.
    Chars { notdef: bool },
    /// `beginbfchar`, which §9.7.5.4 c) forbids here and one corpus document writes anyway —
    /// see [`CMap::take_chars`].
    BfChars,
    /// `beginbfrange`, for the same reason.
    BfRanges,
}

/// A `CMap`: how a composite font's codes are extracted from a string and what they select.
#[derive(Debug, Clone)]
pub struct CMap {
    /// The codespace ranges, in the order the file gave them.
    codespace: Vec<Codespace>,
    /// The character mappings, indexed by code length − 1 (§9.7.6.2).
    mappings: [Mapping; MAX_CODE_BYTES],
    /// The notdef mappings, indexed the same way (§9.7.6.3).
    notdef: [Mapping; MAX_CODE_BYTES],
    /// `WMode` from the `CMap` file: 0 horizontal, 1 vertical (§9.7.5.1).
    wmode: u8,
    /// Whether the file contains a `usecmap` reference.
    ///
    /// §9.7.5.4 a) requires such a reference to be named by the stream dictionary's
    /// `/UseCMap` as well, so this is what lets the loader notice a file whose mappings are
    /// incomplete because the `CMap` it builds on was never identified.
    references_another: bool,
}

impl CMap {
    /// The `Identity-H` and `Identity-V` mapping of §9.7.5.2 Table 116.
    ///
    /// > It maps 2-byte character codes ranging from 0 to 65,535 to the same 2-byte CID value
    ///
    /// Held as one range rather than 65 536 entries, which is also what the sentence says.
    #[must_use]
    pub fn identity() -> Self {
        let mut mappings: [Mapping; MAX_CODE_BYTES] = Default::default();
        if let Some(two_byte) = mappings.get_mut(1) {
            two_byte.ranges.push((0, 0xFFFF, 0));
        }
        Self {
            codespace: vec![Codespace {
                length: 2,
                low: [0x00, 0x00, 0, 0],
                high: [0xFF, 0xFF, 0, 0],
            }],
            mappings,
            notdef: Default::default(),
            wmode: 0,
            references_another: false,
        }
    }

    /// `Identity-V`, which §9.7.5.2's Table 116 makes `Identity-H`'s vertical twin.
    ///
    /// > Vertical version of Identity-H . The mapping is the same as for Identity-H .
    ///
    /// Same codespace and same mapping; the writing mode is the whole difference, and what it
    /// changes is which of §9.7.4.3's two sets of metrics places the next glyph.
    #[must_use]
    pub fn identity_vertical() -> Self {
        Self {
            wmode: 1,
            ..Self::identity()
        }
    }

    /// Parses an embedded `CMap` file (§9.7.5.3), optionally over the one it builds on.
    ///
    /// The `used` argument is §9.7.5.3's `/UseCMap`: "If this entry is present, the
    /// referencing `CMap` shall specify only the character mappings that differ from the
    /// referenced `CMap`." So the referenced map supplies the starting point — its codespace
    /// ranges included, since a file that states only its differences states no codespace
    /// ranges either — and this file's own entries are consulted first.
    ///
    /// A malformed file yields whatever was understood before the damage, as
    /// [`crate::tounicode::ToUnicode::parse`] does: the loader then refuses it for having no
    /// codespace ranges or no mappings, rather than a truncated file silently mapping a few
    /// codes and sending the rest to `.notdef`.
    #[must_use]
    pub fn parse(bytes: &[u8], used: Option<&Self>) -> Self {
        let mut map = used.cloned().unwrap_or_else(Self::empty);
        // A referenced map's own `usecmap` is already resolved; only this file's matters.
        map.references_another = false;
        // Anything this file defines is consulted before what it inherited, which is what
        // "only the character mappings that differ" asks for.
        let mut own = Self {
            wmode: map.wmode,
            ..Self::empty()
        };

        let mut lexer = Lexer::new(bytes);
        // Operands seen since the last keyword. A section is introduced by a count followed
        // by a keyword, and the count is not trustworthy — the terminating keyword is what
        // ends a section — so operands are simply buffered.
        let mut operands: Vec<Token> = Vec::new();
        let mut section: Option<Section> = None;
        // Whether the next integer is `/WMode`'s value.
        let mut expecting_wmode = false;

        while let Some(token) = lexer.next_token() {
            match token {
                Token::Keyword(word) => {
                    match word.as_slice() {
                        b"begincodespacerange" => section = Some(Section::Codespace),
                        b"begincidrange" => section = Some(Section::Ranges { notdef: false }),
                        b"beginnotdefrange" => section = Some(Section::Ranges { notdef: true }),
                        b"begincidchar" => section = Some(Section::Chars { notdef: false }),
                        b"beginnotdefchar" => section = Some(Section::Chars { notdef: true }),
                        b"beginbfchar" => section = Some(Section::BfChars),
                        b"beginbfrange" => section = Some(Section::BfRanges),
                        b"endcodespacerange" | b"endcidrange" | b"endnotdefrange"
                        | b"endcidchar" | b"endnotdefchar" | b"endbfchar" | b"endbfrange" => {
                            own.take(section, &operands);
                            section = None;
                        }
                        b"usecmap" => own.references_another = true,
                        _ => section = None,
                    }
                    operands.clear();
                    expecting_wmode = false;
                }
                Token::Name(name) => {
                    expecting_wmode = name.as_slice() == b"WMode";
                    operands.clear();
                }
                Token::Integer(value) if expecting_wmode => {
                    // §9.7.5.1: "A CMap shall specify the writing mode — horizontal or
                    // vertical". Table 118 requires the stream dictionary's `/WMode` to
                    // agree with the file's; the loader reads both and refuses if either
                    // asks for vertical writing, because agreeing is the producer's duty
                    // and being drawn horizontally is not a near miss.
                    own.wmode = u8::try_from(value).unwrap_or(0);
                    expecting_wmode = false;
                    operands.clear();
                }
                other => {
                    expecting_wmode = false;
                    if operands.len() < MAX_SINGLES.saturating_mul(4) {
                        operands.push(other);
                    }
                }
            }
        }
        // A file that ends mid-section still described every mapping before the damage.
        own.take(section, &operands);

        map.absorb(own);
        map
    }

    /// A `CMap` that states nothing, which every parse starts from.
    fn empty() -> Self {
        Self {
            codespace: Vec::new(),
            mappings: Default::default(),
            notdef: Default::default(),
            wmode: 0,
            references_another: false,
        }
    }

    /// Puts this file's entries in front of whatever it inherited.
    fn absorb(&mut self, own: Self) {
        self.wmode = own.wmode;
        self.references_another = own.references_another;
        // Codespace ranges are scanned by length rather than in order, so appending is
        // enough; a shorter code is always tried first (§9.7.6.2).
        for range in own.codespace {
            if self.codespace.len() < MAX_CODESPACE {
                self.codespace.push(range);
            }
        }
        for (into, from) in self.mappings.iter_mut().zip(own.mappings) {
            into.singles.extend(from.singles);
            // In front, so a referencing file's range wins over the one it replaces.
            let mut ranges = from.ranges;
            ranges.append(&mut into.ranges);
            into.ranges = ranges;
        }
        for (into, from) in self.notdef.iter_mut().zip(own.notdef) {
            into.singles.extend(from.singles);
            let mut ranges = from.ranges;
            ranges.append(&mut into.ranges);
            into.ranges = ranges;
        }
    }

    /// Reads one section's buffered operands.
    fn take(&mut self, section: Option<Section>, operands: &[Token]) {
        match section {
            Some(Section::Codespace) => self.take_codespace(operands),
            Some(Section::Ranges { notdef }) => self.take_ranges(operands, notdef, false),
            Some(Section::Chars { notdef }) => self.take_chars(operands, notdef, false),
            Some(Section::BfChars) => self.take_chars(operands, false, true),
            Some(Section::BfRanges) => self.take_ranges(operands, false, true),
            None => {}
        }
    }

    /// Reads a `begincodespacerange` section: pairs of bounds of equal length.
    fn take_codespace(&mut self, operands: &[Token]) {
        for pair in operands.chunks(2) {
            let [Token::String(low), Token::String(high)] = pair else {
                continue;
            };
            if low.len() != high.len() || low.is_empty() || low.len() > MAX_CODE_BYTES {
                continue;
            }
            if self.codespace.len() >= MAX_CODESPACE {
                return;
            }
            let mut range = Codespace {
                // Checked non-empty and at most four just above.
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the length is bounded by MAX_CODE_BYTES, which is 4"
                )]
                length: low.len() as u8,
                low: [0; MAX_CODE_BYTES],
                high: [0; MAX_CODE_BYTES],
            };
            for (index, (&lo, &hi)) in low.iter().zip(high.iter()).enumerate() {
                if let Some(slot) = range.low.get_mut(index) {
                    *slot = lo;
                }
                if let Some(slot) = range.high.get_mut(index) {
                    *slot = hi;
                }
            }
            self.codespace.push(range);
        }
    }

    /// Reads a `begincidchar` or `beginnotdefchar` section: a code and a CID each.
    ///
    /// `bf` marks a `beginbfchar` section, whose destination is a hex string rather than an
    /// integer. §9.7.5.4 c) says those "shall not appear in a `CMap` that is used as the
    /// `Encoding` entry of a Type 0 font" — and one corpus document writes an `Encoding`
    /// `CMap` whose only mappings are `bfchar` lines. §9.7.6.2 settles what to do with it,
    /// because its own account of the decoding algorithm names them:
    ///
    /// > The code extracted from the string shall be looked up in the character code mappings
    /// > for codes of that length. (These are the mappings defined by `beginbfchar` … and
    /// > corresponding operators for ranges.)
    ///
    /// So the two subclauses disagree, and this one describes what a processor *does*: the
    /// destination is a character selector, which in PDF is a CID (§9.7.5.1), read from the
    /// hex string most significant byte first. Reading it as [`crate::tounicode`] would —
    /// UTF-16BE text — is the other clause's question about the same bytes.
    fn take_chars(&mut self, operands: &[Token], notdef: bool, bf: bool) {
        for pair in operands.chunks(2) {
            let [Token::String(source), target] = pair else {
                continue;
            };
            let (Some(code), Some(length)) = (code_of(source), code_length(source)) else {
                continue;
            };
            let Some(cid) = selector_of(target, bf) else {
                continue;
            };
            let Some(mapping) = self.mapping_mut(notdef, length) else {
                continue;
            };
            if mapping.singles.len() < MAX_SINGLES {
                mapping.singles.insert(code, cid);
            }
        }
    }

    /// Reads a `begincidrange` or `beginnotdefrange` section: two bounds and a CID.
    fn take_ranges(&mut self, operands: &[Token], notdef: bool, bf: bool) {
        for triple in operands.chunks(3) {
            let [Token::String(low), Token::String(high), target] = triple else {
                continue;
            };
            let (Some(low_code), Some(high_code), Some(length)) =
                (code_of(low), code_of(high), code_length(low))
            else {
                continue;
            };
            if low.len() != high.len() || low_code > high_code {
                continue;
            }
            let Some(cid) = selector_of(target, bf) else {
                continue;
            };
            let Some(mapping) = self.mapping_mut(notdef, length) else {
                continue;
            };
            if mapping.ranges.len() < MAX_RANGES {
                mapping.ranges.push((low_code, high_code, cid));
            }
        }
    }

    /// The table a mapping of the given code length belongs in.
    fn mapping_mut(&mut self, notdef: bool, length: usize) -> Option<&mut Mapping> {
        let index = length.checked_sub(1)?;
        if notdef {
            self.notdef.get_mut(index)
        } else {
            self.mappings.get_mut(index)
        }
    }

    /// Extracts the next character code from a string (§9.7.6.2).
    ///
    /// > A sequence of one or more bytes shall be extracted from the string and matched
    /// > against the codespace ranges in the CMap. That is, the first byte shall be matched
    /// > against 1-byte codespace ranges; if no match is found, a second byte shall be
    /// > extracted, and the 2-byte code shall be matched against 2-byte codespace ranges.
    /// > This process continues for successively longer codes until a match is found or all
    /// > codespace ranges have been tested.
    ///
    /// Returns a code of at least one byte for any non-empty string, so a caller advancing
    /// by the code's length always terminates.
    #[must_use]
    pub fn next_code(&self, bytes: &[u8]) -> Code {
        for length in 1..=MAX_CODE_BYTES.min(bytes.len()) {
            let Some(head) = bytes.get(..length) else {
                break;
            };
            if self.codespace.iter().any(|range| range.matches(head)) {
                return Code {
                    value: value_of(head),
                    // Bounded by MAX_CODE_BYTES.
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "length is at most MAX_CODE_BYTES, which is 4"
                    )]
                    length: length as u8,
                    in_codespace: true,
                };
            }
        }
        self.recover(bytes)
    }

    /// Chooses how many bytes an invalid code consumes (§9.7.6.3).
    ///
    /// > The character mapping algorithm shall be reset to its original position in the
    /// > string, and a modified mapping algorithm chooses the best partially matching
    /// > codespace range … The length of the codes in the chosen codespace range determines
    /// > the total number of bytes to consume from the string for the current mapping
    /// > operation.
    ///
    /// The two rules the clause states are a) no first-byte match at all, take the range
    /// with the shortest codes, and b) otherwise the longest partial match, ties going to
    /// the shortest codes.
    fn recover(&self, bytes: &[u8]) -> Code {
        let shortest = self
            .codespace
            .iter()
            .map(|range| usize::from(range.length))
            .min();
        // A `CMap` with no codespace ranges cannot decode at all, and the loader refuses one
        // before it reaches here; one byte keeps this function total either way.
        let mut chosen = shortest.unwrap_or(1);
        let mut best_partial = 0usize;
        for range in &self.codespace {
            let matched = range.partial_match(bytes);
            if matched == 0 {
                continue;
            }
            let length = usize::from(range.length);
            if matched > best_partial || (matched == best_partial && length < chosen) {
                best_partial = matched;
                chosen = length;
            }
        }
        // Never zero, so a caller advancing by the length always makes progress even on a
        // string that has run out mid-code.
        let taken = chosen.min(bytes.len()).clamp(1, MAX_CODE_BYTES);
        let head = bytes.get(..taken).unwrap_or(bytes);
        Code {
            value: value_of(head),
            #[expect(
                clippy::cast_possible_truncation,
                reason = "clamped to MAX_CODE_BYTES, which is 4"
            )]
            length: taken as u8,
            in_codespace: false,
        }
    }

    /// The CID a code maps to, or `None` where the `CMap` defines none (§9.7.6.2).
    #[must_use]
    pub fn cid(&self, code: Code) -> Option<u32> {
        if !code.in_codespace {
            // §9.7.6.3: an invalid code goes straight to a substitute glyph. Looking it up
            // would let a code the codespace excludes borrow a mapping meant for a code of
            // the same value and a different length.
            return None;
        }
        self.mappings
            .get(usize::from(code.length).checked_sub(1)?)?
            .get(code.value, true)
    }

    /// The CID a code's notdef mapping supplies, if the `CMap` has one (§9.7.6.3).
    ///
    /// > If a code maps to a CID for which no such glyph exists in the descendant CIDFont,
    /// > the notdef mappings in the CMap shall be consulted to obtain a substitute character
    /// > selector.
    #[must_use]
    pub fn notdef_cid(&self, code: Code) -> Option<u32> {
        self.notdef
            .get(usize::from(code.length).checked_sub(1)?)?
            .get(code.value, false)
    }

    /// The writing mode: 0 horizontal, 1 vertical (§9.7.5.1).
    #[must_use]
    pub fn wmode(&self) -> u8 {
        self.wmode
    }

    /// Whether the file names a `CMap` to build on (§9.7.5.4 a)).
    #[must_use]
    pub fn references_another(&self) -> bool {
        self.references_another
    }

    /// Whether any codespace range was stated, without which no code can be extracted.
    #[must_use]
    pub fn has_codespace(&self) -> bool {
        !self.codespace.is_empty()
    }

    /// Whether any character mapping was stated.
    #[must_use]
    pub fn has_mappings(&self) -> bool {
        self.mappings.iter().any(|mapping| !mapping.is_empty())
    }

    /// Visits every code this `CMap` states a mapping for that its own codespace admits.
    ///
    /// The inverse direction of [`Self::next_code`], and the only way to ask a composite font
    /// which codes exist: §12.7.4.3 has a processor *write* a string in a font's own codes, and
    /// a code is a length as well as a value (§9.7.6.2), so nothing outside the `CMap` can
    /// produce one.
    ///
    /// **A code is offered only if a decoder reading its own bytes back would extract exactly
    /// it**, which is [`Self::extracted`] and is the whole of what inverting the codespace
    /// ranges amounts to. A mapping alone does not make a code usable: a `CMap` stating
    /// `<00> <FF>` as its codespace and a two-byte `cidrange` maps codes no string can ever
    /// contain, because §9.7.6.2 matches the first byte against the one-byte ranges and stops
    /// there. Writing such a code would produce a string that draws two other glyphs.
    ///
    /// **Visited shortest length first**, which is the order §9.7.6.2 extracts codes in, and
    /// within one length the individually stated codes ascending and then the ranges in file
    /// order — [`Mapping::get`]'s own precedence. So a caller keeping the first code that means
    /// a character keeps the one this `CMap` answers with.
    ///
    /// Returns `false` without visiting anything where the mappings state more than `limit`
    /// codes. All or nothing on purpose: a partial inverse would tell a caller that the font has
    /// no code for a character it has a code for, and the report would then name the wrong
    /// reason (`CLAUDE.md` trap 5).
    #[must_use]
    pub fn each_addressable_code(&self, limit: u64, mut visit: impl FnMut(Code)) -> bool {
        if self.stated_code_count() > limit {
            return false;
        }
        for (index, mapping) in self.mappings.iter().enumerate() {
            let Ok(length) = u8::try_from(index.saturating_add(1)) else {
                continue;
            };
            let mut offer = |value: u32| {
                if let Some(code) = self.extracted(value, length) {
                    visit(code);
                }
            };
            for &value in mapping.singles.keys() {
                offer(value);
            }
            for &(low, high, _) in &mapping.ranges {
                for value in low..=high {
                    offer(value);
                }
            }
        }
        true
    }

    /// How many codes the character mappings state, as the walk above would visit them.
    ///
    /// An upper bound on the work rather than a count of usable codes — the codespace test is
    /// applied per code and cannot be summed — which is exactly what a bound against a limit
    /// needs to be. Saturating throughout: a single `cidrange` may span the whole four-byte
    /// space, and a total that overflowed would report a small number for the largest input
    /// there is.
    fn stated_code_count(&self) -> u64 {
        self.mappings.iter().fold(0_u64, |total, mapping| {
            let singles = u64::try_from(mapping.singles.len()).unwrap_or(u64::MAX);
            let ranges = mapping.ranges.iter().fold(0_u64, |sum, &(low, high, _)| {
                sum.saturating_add(u64::from(high.saturating_sub(low)).saturating_add(1))
            });
            total.saturating_add(singles).saturating_add(ranges)
        })
    }

    /// The code [`Self::next_code`] extracts from a value's own bytes, where it extracts that one.
    ///
    /// `None` where the codespace ranges give those bytes a different length or reject them
    /// outright, which is the whole of the test described above.
    fn extracted(&self, value: u32, length: u8) -> Option<Code> {
        let wanted = Code {
            value,
            length,
            in_codespace: true,
        };
        let bytes = value.to_be_bytes();
        // A value wider than the length it is claimed for keeps its low bytes here and is then
        // rejected below, because what those bytes decode to is not the value asked about.
        let skip = bytes.len().checked_sub(usize::from(length))?;
        let head = bytes.get(skip..)?;
        (self.next_code(head) == wanted).then_some(wanted)
    }
}

/// Reads a code from a `CMap` hex string, which is one to four bytes most significant first.
fn code_of(bytes: &[u8]) -> Option<u32> {
    (!bytes.is_empty() && bytes.len() <= MAX_CODE_BYTES).then(|| value_of(bytes))
}

/// A code's length in bytes, which is part of its identity (§9.7.6.2).
fn code_length(bytes: &[u8]) -> Option<usize> {
    (!bytes.is_empty() && bytes.len() <= MAX_CODE_BYTES).then_some(bytes.len())
}

/// Reads up to four bytes as one value, most significant byte first.
fn value_of(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .take(MAX_CODE_BYTES)
        .fold(0u32, |value, &byte| (value << 8_u32) | u32::from(byte))
}

/// Reads a mapping's destination, which is a CID either way.
///
/// A `cidchar` or `cidrange` states it as an integer; a `bfchar` or `bfrange` states it as a
/// hex string, and [`CMap::take_chars`] has the clause that says to read it as a selector.
fn selector_of(target: &Token, bf: bool) -> Option<u32> {
    match target {
        Token::Integer(value) => u32::try_from(*value).ok(),
        Token::String(bytes) if bf => code_of(bytes),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{CMap, Code};

    /// The example of §9.7.5.4, reduced to the parts that decide a mapping.
    const SHIFT_JIS: &[u8] = b"
        /CIDInit /ProcSet findresource begin
        12 dict begin begincmap
        /CMapName /90ms-RKSJ-H def
        /WMode 0 def
        4 begincodespacerange
        <00> <80>
        <8140> <9FFC>
        <A0> <DF>
        <E040> <FCFC>
        endcodespacerange
        1 beginnotdefrange
        <00> <1F> 231
        endnotdefrange
        2 begincidrange
        <20> <7D> 231
        <8140> <817E> 633
        endcidrange
        endcmap
    ";

    #[test]
    fn identity_maps_two_byte_codes_to_themselves() {
        let map = CMap::identity();
        let code = map.next_code(&[0x12, 0x34, 0x56]);
        assert_eq!(code.value(), 0x1234);
        assert_eq!(code.length, 2);
        assert_eq!(map.cid(code), Some(0x1234));
    }

    /// §9.7.6.2's algorithm tries the shortest codes first, so a one-byte range and a
    /// two-byte range starting with a different byte coexist.
    #[test]
    fn code_length_comes_from_the_codespace_ranges() {
        let map = CMap::parse(SHIFT_JIS, None);
        let one = map.next_code(&[0x41, 0x42]);
        assert_eq!((one.value(), one.length), (0x41, 1));
        let two = map.next_code(&[0x81, 0x45, 0x00]);
        assert_eq!((two.value(), two.length), (0x8145, 2));
    }

    #[test]
    fn a_cid_range_counts_from_its_first_cid() {
        let map = CMap::parse(SHIFT_JIS, None);
        let code = map.next_code(&[0x21]);
        assert_eq!(map.cid(code), Some(232), "231 for <20>, so 232 for <21>");
    }

    /// §9.7.6.2 tests each byte against the corresponding bound rather than comparing the
    /// code's value, which is what makes a UTF-8-shaped codespace work.
    #[test]
    fn a_codespace_range_is_matched_byte_by_byte() {
        let map = CMap::parse(
            b"2 begincodespacerange <00> <7F> <C280> <DFBF> endcodespacerange
              1 begincidrange <C280> <DFBF> 100 endcidrange",
            None,
        );
        let good = map.next_code(&[0xC2, 0x80]);
        assert_eq!((good.value(), good.length), (0xC280, 2));
        assert_eq!(map.cid(good), Some(100));

        // 0xC2C0 lies inside 0xC280..=0xDFBF numerically and outside it byte by byte,
        // because 0xC0 is above the second bound's 0xBF.
        let bad = map.next_code(&[0xC2, 0xC0]);
        assert!(!bad.in_codespace, "the second byte is outside 80..BF");
        assert_eq!(map.cid(bad), None);
    }

    /// §9.7.6.3 a): no first-byte match, so the shortest codes are chosen.
    #[test]
    fn an_unmatched_first_byte_consumes_the_shortest_code() {
        let map = CMap::parse(SHIFT_JIS, None);
        // 0xFF matches no one-byte range and no two-byte range's first byte.
        let code = map.next_code(&[0xFF, 0x41]);
        assert!(!code.in_codespace);
        assert_eq!(code.length, 1, "the shortest codespace range is one byte");
    }

    /// §9.7.6.3 b): a partial match consumes the partially matching range's length.
    #[test]
    fn a_partial_match_consumes_that_ranges_length() {
        let map = CMap::parse(SHIFT_JIS, None);
        // 0x81 begins the two-byte range <8140> <9FFC>; 0x00 is below its second bound.
        let code = map.next_code(&[0x81, 0x00, 0x41]);
        assert!(!code.in_codespace);
        assert_eq!(code.length, 2, "two bytes are consumed even though invalid");
        assert_eq!(code.value(), 0x8100);
    }

    /// A code shorter than the range it partially matches must still make progress.
    #[test]
    fn a_truncated_code_consumes_what_is_left() {
        let map = CMap::parse(SHIFT_JIS, None);
        let code = map.next_code(&[0x81]);
        assert_eq!(code.length, 1);
    }

    #[test]
    fn notdef_ranges_are_kept_apart_from_character_mappings() {
        let map = CMap::parse(SHIFT_JIS, None);
        let code = map.next_code(&[0x05]);
        assert_eq!(map.cid(code), None, "<05> has no character mapping");
        assert_eq!(
            map.notdef_cid(code),
            Some(231),
            "and the notdef range covers it"
        );
    }

    /// §9.7.6.2 looks a code up "in the character code mappings for codes of that length",
    /// so two codes of the same value and different lengths are different codes.
    #[test]
    fn a_codes_length_is_part_of_its_identity() {
        let map = CMap::parse(
            b"2 begincodespacerange <00> <7F> <8000> <FFFF> endcodespacerange
              1 begincidchar <41> 7 endcidchar
              1 begincidchar <8041> 9 endcidchar",
            None,
        );
        let one = map.next_code(&[0x41]);
        let two = map.next_code(&[0x80, 0x41]);
        assert_eq!(map.cid(one), Some(7));
        assert_eq!(map.cid(two), Some(9));
    }

    /// §9.3.3's rule is about the code's encoded length, and a `CMap` may mix lengths.
    #[test]
    fn word_spacing_follows_the_codes_length() {
        let map = CMap::parse(
            b"2 begincodespacerange <00> <7F> <8000> <FFFF> endcodespacerange
              1 begincidrange <00> <7F> 0 endcidrange",
            None,
        );
        assert!(map.next_code(&[0x20]).takes_word_spacing());
        assert!(
            !map.next_code(&[0x80, 0x20]).takes_word_spacing(),
            "the byte value 32 inside a two-byte code takes none"
        );
        assert!(
            !CMap::identity()
                .next_code(&[0x00, 0x20])
                .takes_word_spacing(),
            "Identity-H's codes are all two bytes"
        );
    }

    /// §9.7.5.4 c) forbids `bfchar` in an `Encoding` `CMap` and §9.7.6.2 names it among the
    /// character mappings anyway; one corpus document writes exactly this.
    #[test]
    fn a_bfchar_destination_is_read_as_a_selector() {
        let map = CMap::parse(
            b"1 begincodespacerange <0000> <FFFF> endcodespacerange
              2 beginbfchar <0020> <0003> <0021> <0004> endbfchar",
            None,
        );
        let code = map.next_code(&[0x00, 0x20]);
        assert_eq!(map.cid(code), Some(3));
        assert_eq!(map.cid(map.next_code(&[0x00, 0x21])), Some(4));
    }

    /// §9.7.5.3: "the referencing `CMap` shall specify only the character mappings that
    /// differ from the referenced `CMap`", so the codespace comes from the referenced one too.
    #[test]
    fn a_referencing_cmap_inherits_and_overrides() {
        let base = CMap::parse(
            b"1 begincodespacerange <00> <FF> endcodespacerange
              2 begincidchar <41> 1 <42> 2 endcidchar",
            None,
        );
        let over = CMap::parse(
            b"/Base usecmap 1 begincidchar <42> 99 endcidchar",
            Some(&base),
        );
        assert!(over.references_another());
        assert!(over.has_codespace(), "inherited from the referenced CMap");
        assert_eq!(over.cid(over.next_code(&[0x41])), Some(1), "inherited");
        assert_eq!(over.cid(over.next_code(&[0x42])), Some(99), "overridden");
    }

    #[test]
    fn wmode_is_read_from_the_file() {
        assert_eq!(CMap::parse(b"/WMode 0 def", None).wmode(), 0);
        assert_eq!(CMap::parse(b"/WMode 1 def", None).wmode(), 1);
        assert_eq!(
            CMap::parse(b"/CMapName /X def", None).wmode(),
            0,
            "the default is horizontal"
        );
    }

    #[test]
    fn an_empty_cmap_states_nothing() {
        let map = CMap::parse(b"begincmap endcmap", None);
        assert!(!map.has_codespace());
        assert!(!map.has_mappings());
    }

    #[test]
    fn damage_leaves_what_was_understood_before_it() {
        let map = CMap::parse(
            b"1 begincodespacerange <00> <FF> endcodespacerange 2 begincidchar <41> 5 <42",
            None,
        );
        assert_eq!(map.cid(map.next_code(&[0x41])), Some(5));
    }

    /// A range covering the whole two-byte space must not expand into entries.
    #[test]
    fn a_huge_range_stays_a_range() {
        let map = CMap::parse(
            b"1 begincodespacerange <0000> <FFFF> endcodespacerange
              1 begincidrange <0000> <FFFF> 0 endcidrange",
            None,
        );
        let mapping = map.mappings.get(1).expect("two-byte mappings");
        assert!(mapping.singles.is_empty());
        assert_eq!(mapping.ranges.len(), 1);
        assert_eq!(map.cid(map.next_code(&[0x12, 0x34])), Some(0x1234));
    }

    #[test]
    fn a_simple_fonts_code_is_one_byte() {
        let code = Code::single_byte(32);
        assert_eq!(code.value(), 32);
        assert!(code.takes_word_spacing());
    }

    /// Every code offered for inversion is one this same `CMap` would extract from its bytes.
    ///
    /// The property in its general form, over the three shapes a `CMap` states codes in — a
    /// `cidchar`, a `cidrange`, and two codespace ranges of different lengths in one file.
    /// §12.7.4.3 writes a string in these codes, and §9.7.6.2 is what reads it back.
    #[test]
    fn every_addressable_code_decodes_back_to_itself() {
        let map = CMap::parse(
            b"2 begincodespacerange <00> <7F> <8140> <9FFC> endcodespacerange
              1 begincidchar <20> 1 endcidchar
              2 begincidrange <41> <5A> 2 <8140> <8150> 40 endcidrange",
            None,
        );
        let mut seen = Vec::new();
        assert!(map.each_addressable_code(1 << 16, |code| seen.push(code)));
        assert!(!seen.is_empty(), "the fixture offers no codes at all");
        for code in seen {
            let bytes: Vec<u8> = code
                .value
                .to_be_bytes()
                .into_iter()
                .skip(4 - usize::from(code.length))
                .collect();
            assert_eq!(map.next_code(&bytes), code, "{code:?} does not round-trip");
        }
    }

    /// A mapping the codespace does not admit offers nothing (§9.7.6.2).
    ///
    /// The codespace is one byte wide, so a decoder matches the first byte and stops; the
    /// two-byte `cidrange` beside it maps codes no string can ever contain. Writing one would
    /// produce two one-byte codes that select two other glyphs — which is the failure this
    /// filter exists for, and which no corpus document can show because none has a `/DA` naming
    /// a composite font at all.
    #[test]
    fn a_code_the_codespace_excludes_is_never_offered() {
        let map = CMap::parse(
            b"1 begincodespacerange <00> <FF> endcodespacerange
              2 begincidrange <41> <42> 1 <0041> <0042> 9 endcidrange",
            None,
        );
        let mut seen = Vec::new();
        assert!(map.each_addressable_code(1 << 16, |code| seen.push(code)));
        assert_eq!(
            seen,
            vec![
                Code {
                    value: 0x41,
                    length: 1,
                    in_codespace: true
                },
                Code {
                    value: 0x42,
                    length: 1,
                    in_codespace: true
                }
            ],
            "only the codes a one-byte codespace admits"
        );
    }

    /// A `CMap` stating more codes than the limit is declined whole, and visits nothing.
    ///
    /// All or nothing, because a partial inverse would tell a caller that the font has no code
    /// for a character it has a code for.
    #[test]
    fn a_cmap_beyond_the_limit_offers_nothing_at_all() {
        let map = CMap::identity();
        let mut seen = 0_u32;
        assert!(!map.each_addressable_code(1 << 8, |_| seen = seen.saturating_add(1)));
        assert_eq!(seen, 0);
        // The same map inside a limit that admits it: Table 116's own 65 536 codes.
        assert!(map.each_addressable_code(1 << 16, |_| seen = seen.saturating_add(1)));
        assert_eq!(seen, 1 << 16);
    }
}
