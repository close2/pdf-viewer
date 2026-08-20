//! An image written into the content stream itself: ISO 32000-2 §8.9.7.
//!
//! An inline image is a stream object with its syntax taken away. `BI` opens a dictionary
//! written as bare key–value pairs, `ID` begins the data and `EI` ends it, so the two things
//! an ordinary stream carries in the file — the braces around its dictionary and its
//! `/Length` — are exactly what this clause has to replace.
//!
//! What comes out of [`scan`] is the [`Stream`] the same image would have been as an image
//! `XObject`, key for key, so `crate::image::decode` reads it by the route every other image
//! takes. A second decode path is the shape trap 6 in `doc/HANDOVER.md` is about.
//!
//! # The three things this clause has that an `XObject` does not
//!
//! **Abbreviations.** Table 91 abbreviates the dictionary's keys and Table 92 the colour
//! space and filter names they may hold. Both are expanded here, once, so that nothing
//! downstream needs to know an inline image from any other.
//!
//! **A colour space that may be a resource.** `/CS /DeviceRGB` and its abbreviation `/RGB`
//! always mean the device space — NOTE 3 says so in as many words — but any other name is a
//! key into the resource dictionary's `/ColorSpace`, which is a lookup an image `XObject`
//! never has to do because its own dictionary carries the space outright.
//!
//! **No length, unless the file is PDF 2.0.** `/L` was added in ISO 32000-2 and older files
//! do not have it, so where the data ends has to be *derived*. Three answers, in the order
//! this module tries them, and each is checked against the `EI` that must follow it:
//!
//! 1. `/L` (or `/Length`), which the clause requires of a PDF 2.0 file and defines exactly:
//!    "the length of the data between the ID and EI operators excluding the white-space
//!    delimiting those operators".
//! 2. For unfiltered data, arithmetic: §8.9.3 fixes the layout of samples, so the width, the
//!    height, the bit depth and the colour space's component count give the byte count with
//!    nothing left to guess.
//! 3. Failing both, a search for the first `EI` that stands as its own token. This is the
//!    one guess in the module, it is only reached for *filtered* data with no `/L`, and it is
//!    wrong exactly when the compressed bytes contain a whitespace-`EI`-delimiter sequence.
//!
//! The order matters: it puts the two answers the file states or implies ahead of the one
//! that reads the data looking for something that might not be a token at all.
//!
//! **And the order only holds if the first two can be checked**, which is why [`scan`] is told
//! whether its bytes are all there are. Each derived end is verified against the `EI` it
//! predicts, and a page's `/Contents` arrives through a window — so an end past the window is a
//! request for more bytes ([`InlineImageError::Truncated`]) rather than a failed check that lets
//! the search run. Until the six-hundred-and-nineteenth session it let the search run, and the
//! claim above about answer 3 was false for every unfiltered image larger than a window.

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Lexer, Name, Object, Parser, Stream, Token};

/// Why an inline image could not be read.
///
/// Every one of these leaves the image undrawn and reported. None of them stops the page:
/// [`Scan::resume`] says where the content stream continues either way, because the
/// alternative — tokenising image data as operators — is how binary becomes drawing
/// commands.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum InlineImageError {
    /// The content stream ended before `ID` or before `EI`.
    #[error("the content stream ends inside the image")]
    Truncated,
    /// Something between `BI` and `ID` was not a key–value pair.
    #[error("{detail}")]
    Malformed {
        /// What was met instead.
        detail: String,
    },
    /// Where the data ends could not be established.
    #[error("no EI ends the image data")]
    NoTerminator,
    /// The data is longer than the lookahead a window-fed reader can hold.
    ///
    /// **Kept apart from [`Self::NoTerminator`] because the two are opposite statements**: that
    /// one is about the file, which states no end to its data, and this one is about this
    /// reader, which declined to buffer any more of it looking for one. A page's `/Contents` is
    /// read through a window rather than decoded whole (`crate::content::reader`), and an
    /// inline image is the one thing in a content stream whose extent a window cannot know in
    /// advance — so the lookahead grows to `crate::content::reader::LOOKAHEAD`, which is
    /// sixteen mebibytes against a measured largest of 9.01 MiB and a clause that recommends
    /// 4096 bytes, and says so here rather than reading the image short.
    #[error("the image data is longer than the {bound}-byte lookahead this reader holds")]
    Unbuffered {
        /// How many bytes past `ID` were buffered before the search was given up.
        bound: usize,
    },
}

/// One `BI` … `ID` … `EI` sequence, read.
#[derive(Debug)]
pub struct Scan {
    /// Where interpretation of the content stream resumes.
    ///
    /// Past the `EI`, or at the end of the content when no `EI` was found. Set on every
    /// path, including every error, because image data is not a program and must never be
    /// handed back to the lexer.
    pub resume: usize,
    /// The image, in the form an image `XObject` would have taken, or why it could not be
    /// read.
    pub image: Result<Stream, InlineImageError>,
}

/// Reads the inline image whose `BI` has just been consumed, from `at`.
///
/// `resources` supplies the `/ColorSpace` subdictionary a `/CS` name may refer into
/// (§8.9.7, and §7.8.3 for what it is a key into).
///
/// `complete` says whether `content` is all of the content stream that is left, and it is part
/// of the question rather than a convenience. A page's `/Contents` reaches the interpreter
/// through a window (`crate::content::reader`), and the two answers this module derives — `/L`
/// and the arithmetic below — are *checked* against the `EI` they predict. A derived end past
/// the bytes held cannot be checked, and where more bytes exist the honest answer is
/// [`InlineImageError::Truncated`], which asks the caller for them. Answering it with the
/// forward search instead is what made this module's own "only for filtered data" claim false,
/// and it cost a crawled drawing 63% of an image (ADR 0454).
pub fn scan(
    document: &Document,
    content: &[u8],
    at: usize,
    resources: &Dictionary,
    complete: bool,
) -> Scan {
    let mut lexer = Lexer::at(content, at);
    let dict = match read_dictionary(document, &mut lexer, resources) {
        Ok(dict) => dict,
        Err(error) => {
            // The dictionary is what says where the data ends, so without it there is
            // nothing to do but look for the terminator and carry on from there.
            return Scan {
                resume: search_for_terminator(content, at).map_or(content.len(), |found| found.1),
                image: Err(error),
            };
        }
    };

    // §8.9.7: "the ID operator shall be followed by a single white-space character, and the
    // next character shall be interpreted as the first byte of image data". One character,
    // not any run of them — for unfiltered data every further byte is a sample.
    //
    // A carriage return followed by a line feed is *one* character for this purpose, because
    // §7.2.3 defines an end-of-line marker as "a CARRIAGE RETURN, a LINE FEED, or a CARRIAGE
    // RETURN followed immediately by a LINE FEED" — the same pair the `stream` keyword takes
    // as one marker in §7.3.8.1. `bug1065245.pdf` writes it, and reading the LINE FEED as
    // the first byte of its JPEG makes the codestream start one byte late.
    let mut start = lexer.position();
    if content
        .get(start)
        .copied()
        .is_some_and(pdf_syntax::lexer::is_whitespace)
    {
        let carriage_return = content.get(start) == Some(&b'\r');
        start = start.saturating_add(1);
        if carriage_return && content.get(start) == Some(&b'\n') {
            start = start.saturating_add(1);
        }
    }

    let (end, resume) = match data_extent(document, &dict, content, start, complete) {
        Extent::At { end, resume } => (end, resume),
        Extent::PastTheBuffer => {
            return Scan {
                resume: content.len(),
                image: Err(InlineImageError::Truncated),
            };
        }
        Extent::Missing => {
            return Scan {
                resume: content.len(),
                image: Err(InlineImageError::NoTerminator),
            };
        }
    };

    let data = content.get(start..end).unwrap_or_default();
    Scan {
        resume,
        image: Ok(Stream {
            dict,
            data: Arc::from(data),
            // §7.6.2: strings and streams "inside streams … which themselves are
            // encrypted" are not encrypted again, and an inline image lives inside a
            // content stream.
            decryption_failed: false,
        }),
    }
}

/// Reads the key–value pairs between `BI` and `ID`, expanded into an image dictionary.
///
/// The lexer is left immediately after the `ID`.
fn read_dictionary(
    document: &Document,
    lexer: &mut Lexer<'_>,
    resources: &Dictionary,
) -> Result<Dictionary, InlineImageError> {
    let content = lexer.input();
    let limits = document.limits();
    let mut dict = Dictionary::new();
    // Which entries were filled from Table 91's abbreviated spelling.
    let mut by_abbreviation: Vec<&'static str> = Vec::new();

    loop {
        let Some(token) = lexer.next_token() else {
            return Err(InlineImageError::Truncated);
        };
        let key = match token {
            Token::Keyword(word) if word == b"ID" => break,
            Token::Name(bytes) => bytes,
            other => {
                return Err(InlineImageError::Malformed {
                    detail: format!("{other:?} where a key was expected"),
                });
            }
        };

        let mut parser = Parser::at(content, lexer.position(), limits);
        let value = parser
            .parse_object()
            .map_err(|error| InlineImageError::Malformed {
                detail: format!("/{}: {error}", String::from_utf8_lossy(&key)),
            })?;
        lexer.seek(parser.position());

        if dict.len() >= limits.max_dict_len {
            return Err(InlineImageError::Malformed {
                detail: "more entries than max_dict_len allows".to_owned(),
            });
        }
        // §8.9.7, of Table 91's keys: "Entries other than those listed shall be ignored."
        // Dropping them here rather than downstream is what makes the result a dictionary an
        // image `XObject` could have had: `/SMask` is not on that list, so an inline image
        // has no soft mask however it spells one.
        if let Some(full) = expand_key(&key) {
            let abbreviated = key.as_slice() != full.as_bytes();
            // Two rules, and only the first is anybody's convention. A key stated *twice the
            // same way* keeps the first, which is what `pdf-syntax`'s parser does for every
            // other dictionary in a file. A key stated once each way keeps the abbreviated
            // one whichever came first; see the note on `expand_key` for why.
            let already = dict.get(full).is_some();
            if !already || (abbreviated && !by_abbreviation.contains(&full)) {
                dict.insert(Name::new(full.as_bytes().to_vec()), value);
                if abbreviated {
                    by_abbreviation.push(full);
                }
            }
        }
    }

    expand_names(document, &mut dict, resources);
    Ok(dict)
}

/// Expands Table 91's abbreviated key, or passes a full name through.
///
/// Returns `None` for a key that is neither, which is what "shall be ignored" means.
///
/// # When a file writes both spellings, the abbreviation wins — and that is a choice
///
/// §8.9.7 says only that "the abbreviations shown in Table 91 … and Table 92 … may be used
/// in place of the full names", so the two spellings are one entry and a file writing both
/// with different values has written the same key twice. The standard states no rule for
/// that, here or in §7.3.7, and this is therefore a decision rather than a reading.
///
/// It is decided by the one file that tests it, and by that file's *bytes* rather than by
/// its comments. `issue14256.pdf` is a `SafeDocs` conformance document whose eight inline
/// images are the same picture written eight ways, five of them pairing a correct
/// abbreviation with a contradicting full name. Two of those five can be settled without
/// anybody's opinion:
///
/// - `#4` writes `/F [/AHx] /Filter [/A85]` over data that is plainly ASCII hex. Under the
///   full name the stream does not decode to an image at all.
/// - `#8` writes `/DP [null << /Predictor 15 … >>] /DecodeParms [null null]` over a Flate
///   stream that *was* PNG-predicted. Under the full name every row is off by its predictor.
///
/// So the only rule under which that file decodes at all is "the abbreviation wins", and the
/// three cases it cannot settle — a colour space, a `/Decode` array, an `/Interpolate` flag —
/// are then decided the same way for consistency rather than separately. The alternative,
/// which this crate did before and `poppler` still does, is "the later spelling wins", and it
/// is exactly as defensible from the clause: nothing.
fn expand_key(key: &[u8]) -> Option<&'static str> {
    Some(match key {
        b"BPC" | b"BitsPerComponent" => "BitsPerComponent",
        b"CS" | b"ColorSpace" => "ColorSpace",
        b"D" | b"Decode" => "Decode",
        b"DP" | b"DecodeParms" => "DecodeParms",
        b"F" | b"Filter" => "Filter",
        b"H" | b"Height" => "Height",
        b"IM" | b"ImageMask" => "ImageMask",
        // Table 91 gives `Intent` no abbreviation at all, and `I` is `Interpolate`'s.
        b"Intent" => "Intent",
        b"I" | b"Interpolate" => "Interpolate",
        b"L" | b"Length" => "Length",
        b"W" | b"Width" => "Width",
        _ => return None,
    })
}

/// Expands Table 92's abbreviated colour space and filter names.
///
/// Table 92's abbreviations "are valid only in inline images; they shall not be used in image
/// `XObject`s", so this is the one place in the tree that has to know them, and after it the
/// dictionary says what a file written the long way would have said.
fn expand_names(document: &Document, dict: &mut Dictionary, resources: &Dictionary) {
    if let Some(filter) = dict.get("Filter").cloned() {
        let expanded = match filter {
            Object::Name(name) => Object::Name(expand_filter(&name)),
            Object::Array(items) => Object::Array(
                items
                    .into_iter()
                    .map(|item| match item {
                        Object::Name(name) => Object::Name(expand_filter(&name)),
                        other => other,
                    })
                    .collect(),
            ),
            other => other,
        };
        dict.insert(Name::new(b"Filter".to_vec()), expanded);
    }

    if let Some(space) = dict.get("ColorSpace").cloned() {
        let expanded = match space {
            // §8.9.7 NOTE 3, as Errata Collection 3 rewrites it (Issue #19, `/State` `Review`
            // `Completed`): the names `DeviceGray`, `DeviceRGB` and `DeviceCMYK` and their
            // abbreviations `G`, `RGB` and `CMYK` "never refer to resources in the ColorSpace
            // subdictionary; they always identify the corresponding colour spaces either
            // directly or via a default colour space (see 8.6.5.6 "Default colour spaces")".
            // Any other name does refer to the subdictionary — which is the whole reason this
            // function needs the resources at all.
            //
            // **This comment quoted the unamended NOTE until the four-hundred-and-nineteenth
            // session**, whose sweep of a fourth population of quotation reached `//` comments
            // for the first time (ADR 0255). The amendment costs no code: the expanded name is
            // parsed by `ColourSpace::parse`, which asks `/DefaultGray`, `/DefaultRGB` and
            // `/DefaultCMYK` before it answers with a device space — so the half the erratum
            // adds was already true here, and only the sentence was out of date.
            Object::Name(name) => match expand_device_space(&name) {
                Some(device) => Object::Name(device),
                // Probed with the operand's own bytes: §7.3.5 makes two names one object only
                // when "the resulting sequences of bytes are … an exact binary match", and this
                // key is the *document's* name rather than one ISO 32000-2 states.
                None => document
                    .get_key(resources, "ColorSpace")
                    .as_dict()
                    .and_then(|table| table.get_by_name(&name).cloned())
                    .unwrap_or(Object::Name(name)),
            },
            Object::Array(items) => Object::Array(
                items
                    .into_iter()
                    .map(|item| match item {
                        Object::Name(name) => Object::Name(expand_space(&name)),
                        other => other,
                    })
                    .collect(),
            ),
            other => other,
        };
        dict.insert(Name::new(b"ColorSpace".to_vec()), expanded);
    }
}

/// Expands one of Table 92's filter abbreviations.
fn expand_filter(name: &Name) -> Name {
    let full: &[u8] = match name.as_bytes() {
        b"AHx" => b"ASCIIHexDecode",
        b"A85" => b"ASCII85Decode",
        b"LZW" => b"LZWDecode",
        b"Fl" => b"FlateDecode",
        b"RL" => b"RunLengthDecode",
        b"CCF" => b"CCITTFaxDecode",
        b"DCT" => b"DCTDecode",
        other => other,
    };
    Name::new(full.to_vec())
}

/// Expands one of Table 92's device colour space abbreviations, or a full device name.
fn expand_device_space(name: &Name) -> Option<Name> {
    let full: &[u8] = match name.as_bytes() {
        b"G" | b"DeviceGray" => b"DeviceGray",
        b"RGB" | b"DeviceRGB" => b"DeviceRGB",
        b"CMYK" | b"DeviceCMYK" => b"DeviceCMYK",
        _ => return None,
    };
    Some(Name::new(full.to_vec()))
}

/// Expands a colour space name inside an array, where `I` means `Indexed`.
///
/// The same letter is `Interpolate` as a *key* and `Indexed` as a colour space family, which
/// is why the two expansions are separate functions rather than one table.
fn expand_space(name: &Name) -> Name {
    if name.as_bytes() == b"I" {
        return Name::new(b"Indexed".to_vec());
    }
    expand_device_space(name).unwrap_or_else(|| name.clone())
}

/// Where an inline image's data ends, as far as the bytes in hand can say.
enum Extent {
    /// The data ends at `end` and the content stream resumes at `resume`.
    At {
        /// End of the data, before the white space delimiting `EI`.
        end: usize,
        /// The offset after the terminator.
        resume: usize,
    },
    /// A length the file states or implies reaches past the bytes held.
    ///
    /// Nothing here can be checked, and the caller holds only a window: more bytes exist and
    /// the answer is one of the other two once they arrive.
    PastTheBuffer,
    /// No `EI` ends the data.
    Missing,
}

/// Finds where the image data ends, and where the content stream resumes past `EI`.
///
/// See this module's own documentation for why there are three answers and why they are tried
/// in this order. `complete` is what makes the order hold through a window: without it, a
/// derived end the buffer is merely too short to *check* falls through to the search, which is
/// the one answer this module calls a guess.
fn data_extent(
    document: &Document,
    dict: &Dictionary,
    content: &[u8],
    start: usize,
    complete: bool,
) -> Extent {
    // A derived end past the bytes held is unanswerable rather than wrong. Where `content` is
    // the whole of what is left, it *is* wrong — the file's own arithmetic overruns its content
    // stream — and the search below is what draws the samples that did arrive.
    let mut past_the_buffer = false;
    let mut check = |end: usize| -> Option<usize> {
        let found = terminator_at(content, end);
        if found.is_none() {
            past_the_buffer |= !complete && end > content.len();
        }
        found
    };

    // §8.9.7: `/L` "shall be present on all inline images" and is "the length of the data
    // between the ID and EI operators excluding the white-space delimiting those operators".
    // It is still checked against the `EI` it predicts rather than believed: a wrong length
    // would swallow the rest of the page's content stream as image data, and a file old
    // enough to have no `/L` at all is handled below anyway.
    let stated = document
        .get_key(dict, "Length")
        .as_integer()
        .and_then(|value| usize::try_from(value).ok());
    if let Some(length) = stated {
        let end = start.saturating_add(length);
        if let Some(resume) = check(end) {
            return Extent::At { end, resume };
        }
    }

    // Unfiltered data has exactly one possible length, and §8.9.3 gives it: samples run in
    // row order, each row padded to a byte boundary, so nothing about where it ends depends
    // on reading the data.
    if matches!(document.get_key(dict, "Filter"), Object::Null)
        && let Some(length) = unfiltered_length(document, dict)
    {
        let end = start.saturating_add(length);
        if let Some(resume) = check(end) {
            return Extent::At { end, resume };
        }
    }

    if past_the_buffer {
        return Extent::PastTheBuffer;
    }

    match search_for_terminator(content, start) {
        Some((end, resume)) => Extent::At { end, resume },
        None => Extent::Missing,
    }
}

/// The byte count of unfiltered sample data, from §8.9.3's layout.
fn unfiltered_length(document: &Document, dict: &Dictionary) -> Option<usize> {
    let width = usize::try_from(document.get_key(dict, "Width").as_integer()?).ok()?;
    let height = usize::try_from(document.get_key(dict, "Height").as_integer()?).ok()?;

    // §8.9.6.2, of a stencil mask: `/BitsPerComponent` "shall be 1", and no colour space is
    // consulted, so the one bit per sample is the whole of it.
    let is_mask = matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true));
    let (bits, components) = if is_mask {
        (1usize, 1usize)
    } else {
        let bits = usize::try_from(
            document
                .get_key(dict, "BitsPerComponent")
                .as_integer()
                .unwrap_or(8),
        )
        .ok()?;
        let space = document.get_key(dict, "ColorSpace");
        // The resource lookup has already happened, so nothing here names anything outside
        // itself and an empty resource dictionary is the honest argument to pass.
        let components =
            crate::colour::ColourSpace::parse(document, &space, &Dictionary::new())?.components();
        (bits, components)
    };

    let row_bits = width.checked_mul(components)?.checked_mul(bits)?;
    let row_bytes = row_bits.checked_add(7)? / 8;
    row_bytes.checked_mul(height)
}

/// Checks that `EI` stands at `at`, past any white space, and returns the offset after it.
fn terminator_at(content: &[u8], at: usize) -> Option<usize> {
    let mut cursor = at;
    while content
        .get(cursor)
        .copied()
        .is_some_and(pdf_syntax::lexer::is_whitespace)
    {
        cursor = cursor.saturating_add(1);
    }
    if content.get(cursor..cursor.checked_add(2)?)? != b"EI" {
        return None;
    }
    let after = cursor.saturating_add(2);
    // `EI` is a keyword, so what follows it must end the token — otherwise this is the
    // start of something else that merely begins with those two letters.
    if content
        .get(after)
        .copied()
        .is_none_or(|byte| !pdf_syntax::lexer::is_regular(byte))
    {
        Some(after)
    } else {
        None
    }
}

/// Searches for the `EI` that ends the data, from `start`.
///
/// Returns the end of the data — before the white space delimiting `EI`, as §8.9.7 defines
/// the length — and the offset after the terminator.
///
/// This is the module's only guess, and it is reached only for filtered data in a file with
/// no `/L`. Compressed bytes can contain white space, `E`, `I` and a delimiter in that order;
/// nothing in the format prevents it, which is why the two answers above are tried first.
///
/// Two candidates, one walk. The clause's own sentence — "the bytes between the ID operator
/// and a white-space token, but before the EI operator" — puts white space before `EI`, so a
/// white-space-delimited one is taken as soon as it is seen, and demanding it is what keeps an
/// `EI` inside the data from ending the image early. One without white space before it is
/// remembered and used only if the walk reaches the end without finding a better answer:
/// `issue19532.pdf` ends thirteen inline images with the `EI` hard against the last data byte,
/// and refusing them loses thirteen images to a delimiter the producer omitted.
///
/// The preference is why both are collected in a single walk rather than by searching twice.
/// This runs once per inline image and can reach the end of the stream, so a second pass would
/// double the cost of the one input that provokes it — a content stream full of unterminated
/// `BI` operators, which is a shape a hostile file can write cheaply.
fn search_for_terminator(content: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut at = start;
    let mut undelimited: Option<(usize, usize)> = None;

    while let Some(found) = content
        .get(at..)
        .and_then(|rest| rest.windows(2).position(|window| window == b"EI"))
    {
        let candidate = at.saturating_add(found);
        let preceded_by_space = candidate
            .checked_sub(1)
            .and_then(|index| content.get(index))
            .is_some_and(|&byte| pdf_syntax::lexer::is_whitespace(byte));
        if let Some(resume) = terminator_at(content, candidate) {
            if preceded_by_space {
                // The delimiting white space is not part of the data.
                return Some((candidate.saturating_sub(1), resume));
            }
            undelimited.get_or_insert((candidate, resume));
        }
        at = candidate.saturating_add(2);
    }
    undelimited
}
