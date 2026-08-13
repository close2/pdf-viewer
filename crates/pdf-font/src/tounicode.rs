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
//! used if the CMap is based on another ToUnicode CMap", and Table 118 says the referencing
//! `CMap` "shall specify only the character mappings that differ from the referenced CMap". So
//! a map carries an optional base, consulted after its own mappings and never before them.

use std::collections::BTreeMap;

use pdf_syntax::{Lexer, Token};

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
        /// Bounds the entries a single `CMap` may contribute.
        const MAX_SINGLES: usize = 1 << 16;
        /// Bounds the ranges, which are cheap individually but not unbounded.
        const MAX_RANGES: usize = 1 << 14;

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
        let mut operands: Vec<Token> = Vec::new();
        let mut section: Option<Section> = None;

        while let Some(token) = lexer.next_token() {
            match token {
                Token::Keyword(word) => {
                    match word.as_slice() {
                        b"beginbfchar" => section = Some(Section::Chars),
                        b"beginbfrange" => section = Some(Section::Ranges),
                        b"endbfchar" => {
                            map.take_chars(&operands, MAX_SINGLES);
                            section = None;
                        }
                        b"endbfrange" => {
                            map.take_ranges(&operands, MAX_SINGLES, MAX_RANGES);
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
                    }
                }
            }
        }

        // A stream that ends mid-section — truncated, or damaged — still described every
        // mapping before the damage, and those are worth keeping. Extraction then loses
        // characters, which shows, rather than losing the font's meaning entirely.
        match section {
            Some(Section::Chars) => map.take_chars(&operands, MAX_SINGLES),
            Some(Section::Ranges) => map.take_ranges(&operands, MAX_SINGLES, MAX_RANGES),
            None => {}
        }

        map.ranges.shrink_to_fit();
        map
    }

    /// Reads a `beginbfchar` section: pairs of source code and destination text.
    fn take_chars(&mut self, operands: &[Token], limit: usize) {
        let mut index = 0usize;
        while index.saturating_add(1) < operands.len() {
            let (Some(Token::String(source)), Some(Token::String(target))) =
                (operands.get(index), operands.get(index.saturating_add(1)))
            else {
                index = index.saturating_add(1);
                continue;
            };
            if self.singles.len() >= limit {
                return;
            }
            if let (Some(code), Some(text)) = (code_of(source), text_of(target)) {
                self.singles.insert(code, text.into_boxed_str());
            }
            index = index.saturating_add(2);
        }
    }

    /// Reads a `beginbfrange` section: a low code, a high code, and a destination.
    fn take_ranges(&mut self, operands: &[Token], singles_limit: usize, range_limit: usize) {
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
                        if self.ranges.len() < range_limit && low <= high {
                            self.ranges.push((low, high, first));
                        }
                    } else if let Some(text) = text_of(target) {
                        // A destination that is not a single scalar — a ligature, say —
                        // cannot be incremented, so it applies to the low code only.
                        if self.singles.len() < singles_limit {
                            self.singles.insert(low, text.into_boxed_str());
                        }
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
                                if code > high || self.singles.len() >= singles_limit {
                                    break;
                                }
                                if let Some(text) = text_of(target) {
                                    self.singles.insert(code, text.into_boxed_str());
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
    use super::ToUnicode;

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
    /// from the referenced CMap". So a code the file states answers from the file, and one it
    /// does not answers from the base.
    #[test]
    fn a_base_answers_the_codes_the_file_states_nothing_about() {
        let base = ToUnicode::parse(b"1 beginbfrange\n<0000> <00FF> <0041>\nendbfrange\n");
        let map = ToUnicode::parse_on(b"1 beginbfchar\n<0002> <005A>\nendbfchar\n", Some(base));
        assert_eq!(map.char_for(2), Some('Z'), "the file's own mapping wins");
        assert_eq!(map.char_for(3), Some('D'), "and the base answers the rest");
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
