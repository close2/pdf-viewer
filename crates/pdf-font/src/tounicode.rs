//! Reading a font's `/ToUnicode` `CMap`.
//!
//! A content stream shows text as character codes, whose meaning is font-specific and
//! frequently arbitrary — a subset font may put `A` at code 3. `/ToUnicode` is the
//! producer's own statement of what those codes *mean*, and it is the only source that is
//! reliable for every font, because it does not depend on glyph names existing or on the
//! encoding being a standard one.
//!
//! This is what makes text extraction possible, and it is also how a *composite* font gets
//! substituted: a CID has no meaning outside the font that defined it, so the only route
//! from a code to a substitute font's glyph runs through the character the code represents.
//!
//! # Shape of the data
//!
//! Two forms appear, and they are stored differently on purpose. `beginbfchar` lists codes
//! one at a time and is kept as a map. `beginbfrange` gives a contiguous span of codes
//! mapping to consecutive characters, and is kept *as a range* — expanding it would let a
//! single `<0000> <FFFF>` line allocate sixty-five thousand strings, which is a decoding
//! bomb rather than a font.
//!
//! # A `CMap` built on another
//!
//! §9.10.3 states it as the one dictionary entry that means anything here: `/UseCMap` "may be
//! used if the `CMap` is based on another `ToUnicode` `CMap`", and Table 118 says the
//! referencing `CMap` "shall specify only the character mappings that differ from the referenced
//! `CMap`". So
//! a map carries an optional base, consulted after its own mappings and never before them.

use std::collections::BTreeMap;

use pdf_syntax::{Lexer, Token};

/// One statement a `/ToUnicode` `CMap` makes, in the shape the file made it.
///
/// [`ToUnicode::append`] answers *per code*, which is what extraction wants. This is the same
/// content read the other way round — what the `CMap` **says**, rather than what it answers —
/// which is what a caller reasoning about the map's whole value set needs, and which asking it
/// code by code can only approximate: a code is one to four bytes (ISO 32000-2 §9.7.6.2), so
/// the space to ask about is four billion wide and any bound on it silently omits the tail.
///
/// The two forms stay apart rather than being flattened into one entry per code, for the reason
/// the module keeps them apart in storage: a single `<0000> <FFFF>` line would otherwise become
/// sixty-five thousand entries, and a four-byte one considerably worse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mapping<'a> {
    /// One code and the text it stands for.
    ///
    /// A `beginbfchar` entry, or a `beginbfrange` destination that could not be incremented —
    /// the two are the same statement about one code, and §9.10.3 lets the destination be a
    /// sequence rather than a single character.
    Single {
        /// The character code.
        code: u32,
        /// The text the code stands for.
        text: &'a str,
    },
    /// A span of codes standing for consecutive scalar values: one `beginbfrange` entry.
    Span {
        /// The lowest code the span covers.
        low: u32,
        /// The highest code it covers, inclusive.
        high: u32,
        /// The scalar value `low` stands for; each later code stands for the next one.
        first: u32,
    },
}

/// Bounds the entries one `/ToUnicode` `CMap` may contribute individually.
///
/// A `beginbfchar` entry and a `beginbfrange` whose destination is not a single scalar both
/// land here. The largest file this binary carries states 17 387 of them — `Adobe-Japan1-UCS2`,
/// which §9.10.2's third method reads for the whole Adobe-Japan1 collection — so the bound is
/// a little under four times the population it has to admit.
/// `no_carried_unicode_cmap_is_cut_by_these_bounds` is that measurement.
const MAX_SINGLES: usize = 1 << 16;

/// Bounds the ranges, which are cheap individually but not unbounded.
///
/// **The population this has to admit is not hypothetical**, and it is the reason this constant
/// carries a census the way [`crate::cmap`]'s does. §9.10.3 lets a producer's `/ToUnicode`
/// name another `CMap` in `/UseCMap`, and the ones it can name without carrying them are
/// Adobe's published files — 240 of which this binary compiles in (`data/cmaps/`). The largest
/// of those, read through this parser, states **13 291** `bfrange` entries (`UCS2-ETen-B5`),
/// which is 81% of this bound. Nothing is cut today; what would happen if a later edition of
/// those files crossed it is now said rather than silent (ADR 0971).
const MAX_RANGES: usize = 1 << 14;

/// What [`ToUnicode::truncated`] answers when [`MAX_SINGLES`] dropped a mapping.
///
/// These three are the words a report carries, so they are spelled as the bound a reader would
/// grep for rather than as prose — the vocabulary [`crate::cmap::CUT_BY_SINGLES`] established.
pub const CUT_BY_SINGLES: &str = "max_tounicode_singles";

/// What [`ToUnicode::truncated`] answers when [`MAX_RANGES`] dropped a `bfrange` entry.
pub const CUT_BY_RANGES: &str = "max_tounicode_ranges";

/// What [`ToUnicode::truncated`] answers when one section's operands outran the buffer.
pub const CUT_BY_OPERANDS: &str = "max_tounicode_operands";

/// Maps character codes to the text they represent.
#[derive(Debug, Default, Clone)]
pub struct ToUnicode {
    /// Codes given a mapping individually, including every multi-character one.
    singles: BTreeMap<u32, Box<str>>,
    /// Spans of codes mapping to consecutive characters: low, high, and the first scalar.
    ranges: Vec<(u32, u32, u32)>,
    /// The `CMap` this one only states its *differences* from (§9.10.3, Table 118).
    ///
    /// Boxed because the type is otherwise recursive, and owned rather than shared because a
    /// chain is at most a handful of maps deep and each is read once per font.
    base: Option<Box<ToUnicode>>,
    /// Which bound discarded a mapping the file stated, in the vocabulary of
    /// [`ToUnicode::truncated`]. The first bound to cut is kept, because a report names one
    /// limit.
    truncated: Option<&'static str>,
}

impl ToUnicode {
    /// Parses a `/ToUnicode` `CMap` stream that builds on nothing.
    ///
    /// A malformed `CMap` yields whatever was understood before the damage rather than
    /// nothing: text extraction degrades to missing characters, which is visible, instead
    /// of to silence.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        Self::parse_on(bytes, None)
    }

    /// Parses a `/ToUnicode` `CMap` stream that states only its differences from `base`.
    ///
    /// The base is what §9.10.3's `/UseCMap` — or the `usecmap` operator §9.7.5.4 a) requires
    /// that entry to agree with — names, resolved by the caller, which is the only party able
    /// to fetch a stream or a file.
    #[must_use]
    pub fn parse_on(bytes: &[u8], base: Option<Self>) -> Self {
        /// Which kind of section the operands currently belong to.
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Section {
            Chars,
            Ranges,
        }

        let mut map = Self {
            base: base.map(Box::new),
            ..Self::default()
        };
        let mut lexer = Lexer::new(bytes);
        // Operands seen since the last keyword. A CMap section is introduced by a count
        // followed by a keyword, and the count is not trustworthy — the terminating
        // keyword is what actually ends a section — so operands are simply buffered.
        let mut operands: Vec<Token<'_>> = Vec::new();
        let mut section: Option<Section> = None;

        while let Some(token) = lexer.next_token() {
            match token {
                Token::Keyword(word) => {
                    match word {
                        b"beginbfchar" => section = Some(Section::Chars),
                        b"beginbfrange" => section = Some(Section::Ranges),
                        b"endbfchar" => {
                            map.take_chars(&operands);
                            section = None;
                        }
                        b"endbfrange" => {
                            map.take_ranges(&operands);
                            section = None;
                        }
                        _ => section = None,
                    }
                    operands.clear();
                }
                // An array only appears as a `bfrange` destination list, and is kept
                // inline so the section handler sees it in order.
                other => {
                    if operands.len() < MAX_SINGLES.saturating_mul(4) {
                        operands.push(other);
                    } else {
                        map.cut_by(CUT_BY_OPERANDS);
                    }
                }
            }
        }

        // A stream that ends mid-section — truncated, or damaged — still described every
        // mapping before the damage, and those are worth keeping. Extraction then loses
        // characters, which shows, rather than losing the font's meaning entirely.
        match section {
            Some(Section::Chars) => map.take_chars(&operands),
            Some(Section::Ranges) => map.take_ranges(&operands),
            None => {}
        }

        // A `CMap` inherits what the one it builds on lost as well as what it holds: Table 118
        // makes the pair one statement in two files and [`Self::append`] reads it as one, so an
        // answer missing from either is missing. Absorbed after the parse rather than before it
        // so that a file which cut on its own bound reports its own — [`Self::cut_by`] keeps the
        // first — which is the one a reader of *this* stream can act on.
        if let Some(bound) = map.base.as_ref().and_then(|base| base.truncated) {
            map.cut_by(bound);
        }

        map.ranges.shrink_to_fit();
        map
    }

    /// Records that a bound discarded a mapping, keeping the first bound to do so.
    ///
    /// Called with the entry already in hand and nowhere to put it, never on reaching a count:
    /// the difference is a file whose last entry lands exactly on the bound, which lost nothing
    /// and has nothing to report (`doc/traps/instruments-and-reports.md` trap 11). The same
    /// property [`crate::cmap::CMap`]'s own `cut_by` has, for the same reason.
    fn cut_by(&mut self, bound: &'static str) {
        self.truncated.get_or_insert(bound);
    }

    /// Which bound discarded a mapping this `CMap` stated, or `None` for one read whole.
    ///
    /// `Some` names the bound in [`CUT_BY_SINGLES`]'s vocabulary and means the map answers
    /// nothing for codes the producer mapped. That is not only a text-extraction loss: a
    /// composite font whose program the document did not embed reaches its substitute's glyphs
    /// *through* this map (§9.7.4.2: "CIDs shall not participate in glyph selection"), so a
    /// mapping lost here is a glyph not drawn.
    #[must_use]
    pub fn truncated(&self) -> Option<&'static str> {
        self.truncated
    }

    /// Reads a `beginbfchar` section: pairs of source code and destination text.
    fn take_chars(&mut self, operands: &[Token<'_>]) {
        let mut index = 0usize;
        while index.saturating_add(1) < operands.len() {
            let (Some(Token::String(source)), Some(Token::String(target))) =
                (operands.get(index), operands.get(index.saturating_add(1)))
            else {
                index = index.saturating_add(1);
                continue;
            };
            if let (Some(code), Some(text)) = (code_of(source), text_of(target)) {
                self.insert_single(code, text);
            }
            index = index.saturating_add(2);
        }
    }

    /// Puts one code's text in the map, or records that [`MAX_SINGLES`] would not let it.
    ///
    /// The bound is asked *about this entry* rather than at the top of the loop, so a code the
    /// map already holds is still overwritten — §9.10.3 lets a later section restate a code, and
    /// a restatement discards nothing — and a section whose last entry lands exactly on the
    /// bound reports nothing.
    fn insert_single(&mut self, code: u32, text: String) {
        if self.singles.len() < MAX_SINGLES || self.singles.contains_key(&code) {
            self.singles.insert(code, text.into_boxed_str());
        } else {
            self.cut_by(CUT_BY_SINGLES);
        }
    }

    /// Reads a `beginbfrange` section: a low code, a high code, and a destination.
    fn take_ranges(&mut self, operands: &[Token<'_>]) {
        let mut index = 0usize;
        while index.saturating_add(2) < operands.len() {
            let (Some(Token::String(low)), Some(Token::String(high))) =
                (operands.get(index), operands.get(index.saturating_add(1)))
            else {
                index = index.saturating_add(1);
                continue;
            };
            let (Some(low), Some(high)) = (code_of(low), code_of(high)) else {
                index = index.saturating_add(2);
                continue;
            };

            match operands.get(index.saturating_add(2)) {
                // `<lo> <hi> <dst>`: consecutive characters from `dst`.
                Some(Token::String(target)) => {
                    if let Some(first) = scalar_of(target) {
                        // A `<hi>` below its `<lo>` states no span at all, which is the file
                        // being malformed rather than a bound of ours discarding anything —
                        // so it is dropped without a report, and only the bound reports.
                        if low <= high {
                            if self.ranges.len() < MAX_RANGES {
                                self.ranges.push((low, high, first));
                            } else {
                                self.cut_by(CUT_BY_RANGES);
                            }
                        }
                    } else if let Some(text) = text_of(target) {
                        // A destination that is not a single scalar — a ligature, say —
                        // cannot be incremented, so it applies to the low code only.
                        self.insert_single(low, text);
                    }
                    index = index.saturating_add(3);
                }
                // `<lo> <hi> [<d0> <d1> ...]`: one destination per code.
                Some(Token::ArrayOpen) => {
                    let mut at = index.saturating_add(3);
                    let mut code = low;
                    while let Some(token) = operands.get(at) {
                        match token {
                            Token::String(target) => {
                                // An array longer than the span it belongs to has said nothing
                                // about the codes past `high`, so stopping there discards
                                // nothing of the producer's; the bound below does.
                                if code > high {
                                    break;
                                }
                                if let Some(text) = text_of(target) {
                                    self.insert_single(code, text);
                                }
                                code = code.saturating_add(1);
                                at = at.saturating_add(1);
                            }
                            Token::ArrayClose => {
                                at = at.saturating_add(1);
                                break;
                            }
                            _ => break,
                        }
                    }
                    index = at;
                }
                _ => index = index.saturating_add(3),
            }
        }
    }

    /// Appends the text a character code represents, reporting whether any was found.
    ///
    /// Takes the destination by reference rather than returning a `String` because
    /// extraction calls this once per character on the page.
    pub fn append(&self, code: u32, out: &mut String) -> bool {
        if let Some(text) = self.singles.get(&code) {
            out.push_str(text);
            return true;
        }
        for &(low, high, first) in &self.ranges {
            if (low..=high).contains(&code) {
                let Some(scalar) = first.checked_add(code.saturating_sub(low)) else {
                    continue;
                };
                if let Some(ch) = char::from_u32(scalar) {
                    out.push(ch);
                    return true;
                }
            }
        }
        // Table 118: a referencing `CMap` "shall specify only the character mappings that
        // differ from the referenced CMap", so the base is what a code this file says nothing
        // about means — and it is consulted only here, after everything this file does say.
        self.base
            .as_ref()
            .is_some_and(|base| base.append(code, out))
    }

    /// Whether the `CMap` states anything at all about a code, whatever it states.
    ///
    /// The question ISO 32000-2 §9.10.2's ranking turns on: a method that answers is final for
    /// that code, and one that "fail[s] to produce a Unicode value" hands the code to the next.
    /// [`Self::char_for`] cannot answer it, because it says `None` both for a code this table
    /// omits and for one it maps to a sequence — and the second is an answer. The same walk as
    /// [`Self::append`], without the destination.
    #[must_use]
    pub fn states(&self, code: u32) -> bool {
        self.singles.contains_key(&code)
            || self
                .ranges
                .iter()
                .any(|&(low, high, _)| (low..=high).contains(&code))
            || self.base.as_ref().is_some_and(|base| base.states(code))
    }

    /// Returns the single character a code represents, when it represents exactly one.
    ///
    /// This is what font substitution needs: a `cmap` is keyed by character, so a code
    /// mapping to a ligature or a cluster has no single glyph to look up.
    #[must_use]
    pub fn char_for(&self, code: u32) -> Option<char> {
        if let Some(text) = self.singles.get(&code) {
            let mut chars = text.chars();
            let first = chars.next()?;
            return chars.next().is_none().then_some(first);
        }
        for &(low, high, first) in &self.ranges {
            if (low..=high).contains(&code) {
                return char::from_u32(first.checked_add(code.saturating_sub(low))?);
            }
        }
        self.base.as_ref().and_then(|base| base.char_for(code))
    }

    /// Every statement this `CMap` makes, the base it builds on included.
    ///
    /// The complement of [`Self::append`]: that answers "what does this code mean", this answers
    /// "what does this file say". A caller that needs the second and has only the first has to
    /// guess a code space and walk it, which is exact for no font and expensive for every one.
    ///
    /// # The base is enumerated too
    ///
    /// Table 118 makes a referencing `CMap` state "only the character mappings that differ from
    /// the referenced `CMap`", so the pair is one statement in two files and [`Self::append`]
    /// already reads it as one. Dropping the base here would make a code that `append` answers
    /// absent from the enumeration, which is the inconsistency a reader would trip over first.
    /// This map's own statements come before the base's, in the order `append` consults them,
    /// and a code both state therefore appears twice — the child's first. That is deliberate:
    /// both files said it, and which of the two a caller cares about is the caller's question.
    pub fn mappings(&self) -> impl Iterator<Item = Mapping<'_>> {
        // The chain is walked into a list rather than recursed into, because a recursive
        // `impl Iterator` cannot name its own opaque type. `base` is an owned tree built by the
        // caller, so the walk terminates without a visited set.
        let mut chain = Vec::new();
        let mut next = Some(self);
        while let Some(map) = next {
            chain.push(map);
            next = map.base.as_deref();
        }
        chain.into_iter().flat_map(|map| {
            map.singles
                .iter()
                .map(|(code, text)| Mapping::Single { code: *code, text })
                .chain(map.ranges.iter().map(|&(low, high, first)| Mapping::Span {
                    low,
                    high,
                    first,
                }))
        })
    }

    /// Whether the `CMap` said nothing at all.
    ///
    /// A file that states no mapping of its own but names a base has still said something,
    /// which is exactly the shape §9.10.3's `/UseCMap` sentence describes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.singles.is_empty()
            && self.ranges.is_empty()
            && self.base.as_ref().is_none_or(|base| base.is_empty())
    }
}

/// Reads a source code from a `CMap` hex string, which is one to four bytes big-endian.
fn code_of(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || bytes.len() > 4 {
        return None;
    }
    let mut code = 0u32;
    for &byte in bytes {
        code = code.checked_mul(256)?.checked_add(u32::from(byte))?;
    }
    Some(code)
}

/// Decodes a destination, which is UTF-16BE text rather than a code.
fn text_of(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();
    let text: String = char::decode_utf16(units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();
    (!text.is_empty()).then_some(text)
}

/// Decodes a destination that is exactly one character, as a scalar value.
fn scalar_of(bytes: &[u8]) -> Option<u32> {
    let text = text_of(bytes)?;
    let mut chars = text.chars();
    let first = chars.next()?;
    chars.next().is_none().then(|| u32::from(first))
}

#[cfg(test)]
mod tests {
    use super::{
        CUT_BY_OPERANDS, CUT_BY_RANGES, CUT_BY_SINGLES, MAX_RANGES, MAX_SINGLES, Mapping, ToUnicode,
    };

    /// One `beginbfrange` section of `entries` one-code spans, each mapping to U+0041.
    fn ranges(entries: usize) -> Vec<u8> {
        let mut source = format!("{entries} beginbfrange\n").into_bytes();
        for code in 0..entries {
            source.extend_from_slice(format!("<{code:08x}> <{code:08x}> <0041>\n").as_bytes());
        }
        source.extend_from_slice(b"endbfrange\n");
        source
    }

    /// A `/ToUnicode` that outruns [`MAX_RANGES`] says which bound took the rest.
    ///
    /// §9.10.2's first method is the producer's own table and a bound of ours discarding part
    /// of it leaves codes unnamed; for a composite font with no program §9.7.4.2 makes that
    /// the only route to a glyph, so it is a mark missed as well. ADR 0971.
    #[test]
    fn a_tounicode_past_the_range_bound_is_reported_by_name() {
        let map = ToUnicode::parse(&ranges(MAX_RANGES.saturating_add(1)));
        assert_eq!(map.truncated(), Some(CUT_BY_RANGES));
        // The entries before the bound are still there, which is what makes this a truncation
        // rather than a refusal.
        assert_eq!(map.char_for(0), Some('A'));
    }

    /// A `/ToUnicode` whose last entry lands exactly on the bound lost nothing and says nothing.
    ///
    /// Trap 11: a report fires on a *discarded entry*, never on reaching a count.
    #[test]
    fn a_tounicode_exactly_on_the_range_bound_reports_nothing_about_it() {
        let map = ToUnicode::parse(&ranges(MAX_RANGES));
        assert_eq!(map.truncated(), None);
        let last = u32::try_from(MAX_RANGES.saturating_sub(1)).expect("the bound fits a code");
        assert_eq!(map.char_for(last), Some('A'));
    }

    /// The same pair for the individual mappings, which `beginbfchar` states.
    #[test]
    fn a_tounicode_past_the_single_bound_is_reported_by_name() {
        let mut source = Vec::new();
        source.extend_from_slice(b"1 beginbfchar\n");
        for code in 0..=MAX_SINGLES {
            source.extend_from_slice(format!("<{code:08x}> <0041>\n").as_bytes());
        }
        source.extend_from_slice(b"endbfchar\n");
        let map = ToUnicode::parse(&source);
        assert_eq!(map.truncated(), Some(CUT_BY_SINGLES));

        // One fewer, and nothing was discarded.
        let mut fits = Vec::new();
        fits.extend_from_slice(b"1 beginbfchar\n");
        for code in 0..MAX_SINGLES {
            fits.extend_from_slice(format!("<{code:08x}> <0041>\n").as_bytes());
        }
        fits.extend_from_slice(b"endbfchar\n");
        assert_eq!(ToUnicode::parse(&fits).truncated(), None);
    }

    /// A restated code overwrites rather than being discarded, even at the bound.
    ///
    /// §9.10.3 does not forbid a later section from naming a code an earlier one named, and a
    /// restatement takes nothing away — so the bound is asked about the entry rather than about
    /// the count, and the map is full without this reporting anything.
    #[test]
    fn restating_a_code_at_the_bound_discards_nothing() {
        let mut source = Vec::new();
        source.extend_from_slice(b"1 beginbfchar\n");
        for code in 0..MAX_SINGLES {
            source.extend_from_slice(format!("<{code:08x}> <0041>\n").as_bytes());
        }
        source.extend_from_slice(b"endbfchar\n1 beginbfchar\n<00000000> <0042>\nendbfchar\n");
        let map = ToUnicode::parse(&source);
        assert_eq!(map.truncated(), None);
        assert_eq!(map.char_for(0), Some('B'));
    }

    /// A section whose operands outrun the buffer names that bound.
    #[test]
    fn a_tounicode_past_the_operand_buffer_is_reported_by_name() {
        let mut source = Vec::new();
        source.extend_from_slice(b"1 beginbfchar\n");
        // Four operands per entry is the buffer's own ratio, so twice as many entries as
        // `MAX_SINGLES` fills it with room to spare.
        for _ in 0..MAX_SINGLES.saturating_mul(2).saturating_add(1) {
            source.extend_from_slice(b"<0041> <0041>\n");
        }
        source.extend_from_slice(b"endbfchar\n");
        assert_eq!(ToUnicode::parse(&source).truncated(), Some(CUT_BY_OPERANDS));
    }

    /// A map inherits what the `CMap` it builds on lost, because the two are consulted as one.
    ///
    /// §9.10.3's `/UseCMap`, and Table 118's "shall specify only the character mappings that
    /// differ from the referenced CMap": an answer missing from the base is missing from the
    /// pair. The child's own bound wins where both cut, because that is the one a reader of
    /// this stream can act on.
    #[test]
    fn a_base_that_was_cut_is_carried_into_the_map_built_on_it() {
        let base = ToUnicode::parse(&ranges(MAX_RANGES.saturating_add(1)));
        assert_eq!(base.truncated(), Some(CUT_BY_RANGES));
        let built = ToUnicode::parse_on(b"1 beginbfchar\n<0001> <0041>\nendbfchar\n", Some(base));
        assert_eq!(built.truncated(), Some(CUT_BY_RANGES));
    }

    #[test]
    fn single_mappings_are_read() {
        let map = ToUnicode::parse(b"2 beginbfchar\n<0003> <0020>\n<0004> <0041>\nendbfchar\n");
        assert_eq!(map.char_for(3), Some(' '));
        assert_eq!(map.char_for(4), Some('A'));
        assert_eq!(map.char_for(5), None);
    }

    #[test]
    fn ranges_map_to_consecutive_characters() {
        let map = ToUnicode::parse(b"1 beginbfrange\n<0010> <0012> <0041>\nendbfrange\n");
        assert_eq!(map.char_for(0x10), Some('A'));
        assert_eq!(map.char_for(0x11), Some('B'));
        assert_eq!(map.char_for(0x12), Some('C'));
        assert_eq!(
            map.char_for(0x13),
            None,
            "the range must not run past its end"
        );
    }

    #[test]
    fn ranges_with_an_explicit_list_take_one_destination_per_code() {
        let map =
            ToUnicode::parse(b"1 beginbfrange\n<0020> <0022> [<0058> <0059> <005A>]\nendbfrange\n");
        assert_eq!(map.char_for(0x20), Some('X'));
        assert_eq!(map.char_for(0x21), Some('Y'));
        assert_eq!(map.char_for(0x22), Some('Z'));
    }

    /// A destination outside the basic plane arrives as a surrogate pair.
    #[test]
    fn surrogate_pairs_decode_to_one_character() {
        let map = ToUnicode::parse(b"1 beginbfchar\n<0001> <D83DDE00>\nendbfchar\n");
        assert_eq!(map.char_for(1), Some('\u{1F600}'));
    }

    /// A code may stand for several characters, which extraction must keep and
    /// substitution must decline.
    #[test]
    fn multi_character_destinations_are_text_but_not_a_single_character() {
        let map = ToUnicode::parse(b"1 beginbfchar\n<0007> <00660066>\nendbfchar\n");
        let mut out = String::new();
        assert!(map.append(7, &mut out));
        assert_eq!(out, "ff", "the ff ligature stands for two characters");
        assert_eq!(map.char_for(7), None, "no single glyph can be looked up");
    }

    /// A range covering the whole two-byte space must not expand into entries.
    #[test]
    fn a_huge_range_is_stored_as_a_range_rather_than_expanded() {
        let map = ToUnicode::parse(b"1 beginbfrange\n<0000> <FFFF> <0041>\nendbfrange\n");
        assert!(map.singles.is_empty(), "nothing should have been expanded");
        assert_eq!(map.ranges.len(), 1);
        assert_eq!(map.char_for(0), Some('A'));
    }

    #[test]
    fn damage_leaves_what_was_understood_before_it() {
        let map = ToUnicode::parse(b"1 beginbfchar\n<0003> <0020>\n<00");
        assert_eq!(map.char_for(3), Some(' '));
    }

    #[test]
    fn an_absent_cmap_is_empty_rather_than_wrong() {
        assert!(ToUnicode::parse(b"").is_empty());
        assert!(ToUnicode::parse(b"begincmap endcmap").is_empty());
    }

    /// Table 118: a referencing `CMap` "shall specify only the character mappings that differ
    /// from the referenced `CMap`". So a code the file states answers from the file, and one it
    /// does not answers from the base.
    #[test]
    fn a_base_answers_the_codes_the_file_states_nothing_about() {
        let base = ToUnicode::parse(b"1 beginbfrange\n<0000> <00FF> <0041>\nendbfrange\n");
        let map = ToUnicode::parse_on(b"1 beginbfchar\n<0002> <005A>\nendbfchar\n", Some(base));
        assert_eq!(map.char_for(2), Some('Z'), "the file's own mapping wins");
        assert_eq!(map.char_for(3), Some('D'), "and the base answers the rest");
    }

    /// The enumerator answers what the file *said*: a `bfchar` entry per code, a `bfrange` as
    /// one span, and neither expanded into the other.
    #[test]
    fn the_statements_come_back_in_the_form_they_were_written() {
        let map = ToUnicode::parse(
            b"1 beginbfchar\n<0003> <0020>\nendbfchar\n\
              1 beginbfrange\n<0010> <0012> <0041>\nendbfrange\n",
        );
        let seen: Vec<Mapping<'_>> = map.mappings().collect();
        assert_eq!(
            seen,
            vec![
                Mapping::Single { code: 3, text: " " },
                Mapping::Span {
                    low: 0x10,
                    high: 0x12,
                    first: 0x41
                },
            ]
        );
    }

    /// The whole two-byte space as one statement, which is the case an enumerator that expanded
    /// spans would turn into sixty-five thousand items.
    #[test]
    fn a_span_is_one_statement_however_wide_it_is() {
        let map = ToUnicode::parse(b"1 beginbfrange\n<0000> <FFFF> <0041>\nendbfrange\n");
        assert_eq!(
            map.mappings().count(),
            1,
            "one line of the CMap is one statement"
        );
    }

    /// A four-byte code is enumerated like any other, which is what asking per code over a
    /// guessed two-byte space could not do.
    #[test]
    fn a_code_wider_than_two_bytes_is_enumerated() {
        let map = ToUnicode::parse(b"1 beginbfchar\n<00A10001> <0041>\nendbfchar\n");
        assert_eq!(
            map.mappings().collect::<Vec<_>>(),
            vec![Mapping::Single {
                code: 0x00A1_0001,
                text: "A"
            }]
        );
    }

    /// Table 118 makes the pair one statement in two files, so the enumeration covers both —
    /// otherwise a code [`ToUnicode::append`] answers would be absent from it.
    #[test]
    fn the_base_is_enumerated_after_this_maps_own_statements() {
        let base = ToUnicode::parse(b"1 beginbfchar\n<0009> <0058>\nendbfchar\n");
        let map = ToUnicode::parse_on(b"1 beginbfchar\n<0002> <005A>\nendbfchar\n", Some(base));
        assert_eq!(
            map.mappings().collect::<Vec<_>>(),
            vec![
                Mapping::Single { code: 2, text: "Z" },
                Mapping::Single { code: 9, text: "X" },
            ]
        );
    }

    /// A file whose only statement is the base it builds on has still said something, which is
    /// what §9.10.3's `/UseCMap` sentence describes.
    #[test]
    fn a_file_that_only_names_a_base_is_not_empty() {
        let base = ToUnicode::parse(b"1 beginbfchar\n<0001> <0041>\nendbfchar\n");
        let map = ToUnicode::parse_on(b"begincmap endcmap", Some(base));
        assert!(!map.is_empty());
        assert_eq!(map.char_for(1), Some('A'));
        assert!(ToUnicode::parse_on(b"begincmap endcmap", Some(ToUnicode::default())).is_empty());
    }
}

#[cfg(test)]
mod states_tests {
    use super::ToUnicode;

    /// A code mapped to a sequence is *stated* and has no single character, and a code the file
    /// omits is neither. §9.10.2's ranking turns on the first distinction.
    #[test]
    fn a_stated_sequence_is_an_answer_and_an_omitted_code_is_not() {
        let table = ToUnicode::parse(
            b"1 begincodespacerange <00> <FF> endcodespacerange\n\
              2 beginbfchar <41> <00660066> <42> <0043> endbfchar\n\
              1 beginbfrange <50> <52> <0061> endbfrange\n",
        );
        assert!(table.states(0x41), "a ligature is stated");
        assert_eq!(table.char_for(0x41), None, "and addresses no single glyph");
        assert!(table.states(0x42));
        assert_eq!(table.char_for(0x42), Some('C'));
        assert!(table.states(0x51), "a range states each code it spans");
        assert!(!table.states(0x43), "a code the file omits is not stated");
        assert!(!table.states(0x53));
    }

    /// The base a `/ToUnicode` builds on (§9.10.3, `/UseCMap`) states codes for the pair.
    #[test]
    fn a_base_states_the_codes_the_child_leaves_to_it() {
        let base = ToUnicode::parse(b"1 beginbfchar <41> <0041> endbfchar\n");
        let child = ToUnicode::parse_on(b"1 beginbfchar <42> <0042> endbfchar\n", Some(base));
        assert!(child.states(0x41));
        assert!(child.states(0x42));
        assert!(!child.states(0x43));
    }
}
