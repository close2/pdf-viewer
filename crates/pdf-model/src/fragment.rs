//! ISO 32000-2 Annex O: the fragment identifier that says where a document opens.
//!
//! A fragment identifier is the text after `#` in a URI, and Annex O is the normative list of the
//! ones a PDF understands — `report.pdf#page=12`, `report.pdf#nameddest=Chapter3`. §O.1:
//!
//! > These fragment identifiers anchor onto specific content or characteristics of the document to
//! > which the URI refers.
//!
//! Eleven parameters in two tables. Table Annex O.3's five name an *object* in the document —
//! `page`, `nameddest`, `structelem`, `comment`, `ef` — and Table Annex O.4's six are what §O.2.2
//! calls open parameters: `zoom`, `view`, `viewrect`, `highlight`, `search`, `fdf`. Every one of
//! them is a `shall` addressed to "the PDF processor".
//!
//! # This module reads; it does not act
//!
//! [`Fragment::parse`] needs no document and no window: a fragment is a string that arrives with
//! the *request* rather than with the file — a document cannot contain one — so the grammar and
//! the eleven argument forms stand on their own. What a parameter then names in a particular file
//! is answered by whatever already reads that clause: §12.3.2.4's named destinations by
//! [`crate::destination::Destination::read`], §14.7.2's `/ID` by
//! [`crate::structure::Tree::element_by_id`], Table 166's `/NM` by [`annotation_named`] below,
//! which is the one reading this annex needed and nothing else had. Turning any of that into a
//! page, a magnification and a scroll needs a *window*, and that is
//! `viewer_core::Open::apply_fragment`.
//!
//! Reading and acting are also where the two halves of this annex divide honestly:
//! [`Parameter::unhonoured`] says which of the eleven this *program* cannot carry out and why,
//! which is a claim about this tree rather than about the standard, and it decays the way a ledger
//! row does. It names none of them today — four parameters have stood on that list and all four
//! came off it — and the function is kept because the decay runs both ways.
//!
//! # The grammar, and the two things the annex does not state
//!
//! §O.2 states the separators and the order, and nothing else:
//!
//! > The fragment identifier is comprised of one or more parameters which are appended to a single
//! > URI, each of which is separated by the AMPERSAND (28h) character.
//!
//! > Arguments to those parameters, which represent information needed to qualify the action or
//! > object, shall be COMMA (2Ch) separated when more than one is required.
//!
//! > Fragment identifiers shall be processed and (if required) executed from left to right as they
//! > appear in the character string that makes up the fragment identifier.
//!
//! **The annex prints the wrong code for its own separator, and this reader takes the name.**
//! `&` is 26h; 28h is `(`. The standard settles it against itself: Table D.2 gives `AMPERSAND` the
//! code 0x26, and a fragment whose parameters were separated by an opening parenthesis would be
//! neither the URI convention §O.1 points at nor anything any producer writes. So the separator is
//! the character the sentence *names*.
//!
//! **The annex never states how a parameter is joined to its arguments.** It gives the two
//! separators above, two tables of parameter names and their arguments, and not one example — so
//! `=` is this reader's choice rather than a reading, and it is written down as one. What it rests
//! on: §O.1 defines these as fragment identifiers of a *URI*, where a list of `&`-separated
//! `name=value` pairs is the one shape the surrounding syntax already has, and no other spelling
//! makes the two separators the annex *does* state do any work.
//!
//! A parameter's name is matched exactly as the tables print it, which is lowercase. Table 149's
//! keywords, which the `view` parameter defers to, are name objects and case-sensitive; treating
//! the parameter names the same way is the one rule that needs no second decision. A name that
//! matches nothing is reported by name rather than dropped — [`Unread`].
//!
//! # Percent-encoding, decoded once and after splitting
//!
//! `structelem`'s argument is "a byte string with URI encoding", so an argument reaches the
//! document decoded. Decoding happens *after* the fragment is split on `&` and `,`, never before:
//! a `%26` inside an argument is data and must not become a separator, which is why RFC 3986
//! section 2.4 requires a component to be separated before its octets are decoded.
//!
//! # Numbers
//!
//! The annex asks for "an integer or floating point value" and states no syntax for one, so this
//! reader takes the standard's own — §7.3.3:
//!
//! > An integer shall be written as one or more decimal digits optionally preceded by a sign.
//!
//! > A real value shall be written as one or more decimal digits with an optional sign and a
//! > leading, trailing, or embedded PERIOD (2Eh) (decimal point).
//!
//! which rejects the three spellings Rust's own parser accepts and the standard does not have: an
//! exponent — the clause bans it outright, "in exponential format (such as 6.02E23)" — and the
//! words `inf` and `NaN`.
//!
//! `pdf_syntax::Lexer` reads numbers too, and deliberately reads them *leniently*: a content
//! stream that says `1.2.3` still has to draw, so the lexer salvages what it can and falls back to
//! zero. A fragment has no page to keep drawing. An argument that is not a number is a person's
//! mistyped URI, and turning it into a zero would silently open the document somewhere nobody
//! asked for, so this reader refuses it and says which parameter it was.

use crate::destination::View;

/// One fragment identifier, read.
///
/// The two lists keep the fragment's own order, which §O.2 makes normative: "[f]ragment
/// identifiers shall be processed and (if required) executed from left to right".
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fragment {
    /// The parameters this reader understood, in the order the fragment states them.
    pub parameters: Vec<Parameter>,
    /// What it did not, in the same order. Never silently dropped.
    pub unread: Vec<Unread>,
}

impl Fragment {
    /// Reads a fragment identifier — the text after `#`, as the URI spells it.
    ///
    /// A URI is ASCII by construction, so this takes `&str`; a host holding bytes has already had
    /// to decide what they mean before it had a URI at all.
    ///
    /// Never fails. Every parameter either lands in [`Self::parameters`] or is named in
    /// [`Self::unread`], because a fragment is a person's instruction and the useful answer to
    /// half of one is the half that was understood plus the name of the half that was not.
    ///
    /// No budget is stated and the reason is amplification: this reader's output is bounded by a
    /// constant times the length of the string it was given, where every budget elsewhere in this
    /// tree guards an input that can *expand*. A bound here would be a number nobody could derive.
    #[must_use]
    pub fn parse(fragment: &str) -> Self {
        let mut read = Self::default();
        // §O.2's AMPERSAND, by the name the sentence gives it rather than by the code it prints.
        for part in fragment.split('&') {
            // Two separators with nothing between them state no parameter, so there is nothing to
            // report the name of. Not an error: `page=1&` is a trailing separator and means what
            // `page=1` means.
            if part.is_empty() {
                continue;
            }
            let Some((name, arguments)) = part.split_once('=') else {
                read.unread.push(Unread {
                    name: part.to_owned(),
                    reason: Reason::Unknown,
                });
                continue;
            };
            // §O.2's COMMA, and it separates arguments before anything is decoded.
            let arguments: Vec<Vec<u8>> = arguments.split(',').map(percent_decode).collect();
            match Parameter::read(name, &arguments) {
                Some(parameter) => read.parameters.push(parameter),
                None => read.unread.push(Unread {
                    name: name.to_owned(),
                    reason: if KNOWN.contains(&name) {
                        Reason::Arguments
                    } else {
                        Reason::Unknown
                    },
                }),
            }
        }
        read
    }
}

/// Every parameter name Table Annex O.3 and Table Annex O.4 define.
///
/// Kept beside the reader so that a parameter the annex *does* define can be told from one it does
/// not when its arguments fail to read: the first is a URI this program could not carry out and
/// the second is a URI meant for something else, and a person needs to know which they wrote.
const KNOWN: [&str; 11] = [
    "page",
    "nameddest",
    "structelem",
    "comment",
    "ef",
    "zoom",
    "view",
    "viewrect",
    "highlight",
    "search",
    "fdf",
];

/// One of Annex O's eleven parameters, with its arguments read.
///
/// The byte strings are bytes rather than text on purpose: each is matched against something *in
/// the file* — a name tree's key, a structure element's `/ID`, an annotation's `/NM` — and the
/// annex states no character encoding for any of them, so decoding one here would be this reader
/// inventing a rule the standard does not have.
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    /// Table Annex O.3's `page`, one-based — §O.2.1.
    ///
    /// > A specified page number of the document. The argument shall be a positive integer number.
    /// > The first page in the document has a pageNum value of 1.
    ///
    /// so a zero is not a page and is [`Reason::Arguments`] rather than page one.
    Page(u32),
    /// Table Annex O.3's `nameddest`: §12.3.2.4's key, stated in §O.2.1.
    ///
    /// > The argument provided is a string which shall correspond to the name of a destination in
    /// > the target document.
    NamedDestination(Vec<u8>),
    /// Table Annex O.3's `structelem`: §14.7.2's `/ID`, through Table 354's `/IDTree`. §O.2.1.
    ///
    /// > The structID is a byte string with URI encoding that matches the ID key within a
    /// > StructElem dictionary of the document.
    StructureElement(Vec<u8>),
    /// Table Annex O.3's `comment`: Table 166's `/NM`, stated in §O.2.1.
    ///
    /// > The commentID shall be the value of an annotation name, which is defined by the NM key in
    /// > the corresponding annotation dictionary
    ///
    /// The table also states where this parameter has to appear, and why: "[i]f the comment
    /// parameter is combined with another parameter that defines a specific page to be identified,
    /// then the comment parameter shall appear after that in the URI", because "[t]he NM key is
    /// unique to a specific page, but is not guaranteed to be unique to a document".
    Comment(Vec<u8>),
    /// Table Annex O.3's `ef`: §7.11.4's `/EmbeddedFiles` key, stated in §O.2.1.
    ///
    /// > The name argument shall be a byte string used to match a file specification dictionary in
    /// > the EmbeddedFiles name tree. Any remaining parameters after this parameter apply to the
    /// > selected embedded file.
    ///
    /// The second sentence is not about this parameter but about the ones after it: everything
    /// following it names something in *another* document, and applying it to this one would
    /// follow a URI to the wrong place. So a reader that carries this one out still stops here —
    /// which is `viewer_core::Open::apply_fragment`'s business rather than [`Self::unhonoured`]'s,
    /// and the difference between the two is what this variant was refused for until the
    /// four-hundred-and-seventy-fifth session.
    EmbeddedFile(Vec<u8>),
    /// Table Annex O.4's `zoom` — a percentage, and optionally where to put the page. §O.2.2.
    ///
    /// > The scale argument shall be either an integer or floating point value representing the
    /// > percentage to which the document should be zoomed, where a value of 100 would correspond
    /// > to a zoom of 100%.
    ///
    /// > The left and top arguments are optional, but shall both be specified if either is
    /// > included.
    ///
    /// A *percentage*, where Table 149's `/XYZ` — which [`Self::View`] carries — states the same
    /// magnification as a factor. Both are in this annex, three rows apart, and the conversion is
    /// the caller's.
    Zoom {
        /// The magnification, as the percentage the fragment states.
        percent: f32,
        /// The offset from the page's left and top edges, where the fragment states one.
        offset: Option<(f32, f32)>,
    },
    /// Table Annex O.4's `view`: §12.3.2.2's explicit destination, spelled in text. §O.2.2.
    ///
    /// > The arguments shall correspond to those found in 12.3.2.2, "Explicit destinations". The
    /// > keyword shall correspond to one of the keywords defined in "Table 149 - Destination
    /// > syntax" with appropriate position values.
    ///
    /// which is [`View`] and no second reading of Table 149.
    View(View),
    /// Table Annex O.4's `viewrect` — the rectangle the window shows. §O.2.2.
    ///
    /// > The left and top arguments shall be integer or floating point values representing the
    /// > offset from the left and top of the page in a coordinate system where 0,0 represents the
    /// > top left corner of the page. The width and height arguments shall be integer or floating
    /// > point values representing the width and height of the view.
    ViewRect {
        /// The offset from the page's left edge.
        left: f32,
        /// The offset from the page's top edge, downward.
        top: f32,
        /// How wide the view is.
        width: f32,
        /// How tall the view is.
        height: f32,
    },
    /// Table Annex O.4's `highlight` — a rectangle, and no rule for what to do with it. §O.2.2.
    ///
    /// > Each argument shall be an integer or floating point value representing the rectangle
    /// > measured from the top left corner of the page. The nature of the highlighting is
    /// > implementation-dependent.
    Highlight {
        /// The left edge, from the page's left.
        left: f32,
        /// The right edge, from the page's left.
        right: f32,
        /// The top edge, from the page's top, downward.
        top: f32,
        /// The bottom edge, from the page's top, downward.
        bottom: f32,
    },
    /// Table Annex O.4's `search` — the words to look for. §O.2.2.
    ///
    /// > The wordList argument defines the search words and shall be a string enclosed within
    /// > quotation marks comprised of individual words separated by space characters.
    ///
    /// The quotation marks are the syntax and not part of a word, so they are removed here; a
    /// wordList written without them is still a list of words and is read as one, because the
    /// alternative is refusing a URI over punctuation the annex asks a *writer* for.
    Search(Vec<Vec<u8>>),
    /// Table Annex O.4's `fdf`: §12.7.8's form data, named from outside the document. §O.2.2.
    ///
    /// > The URI shall be either a relative or absolute URI to an FDF or XFDF file.
    ///
    /// A URI rather than a file name, and this reader keeps it as the bytes it was written as: a
    /// *relative* one is resolved against the document's own URI and an absolute one names
    /// somewhere else entirely, and both of those are decisions about a machine rather than about
    /// a document. `viewer_core::Event::NeedsFile` is where the name goes and a host is what
    /// answers it — the same division §12.7.6.4's import action already takes, whose file
    /// specification is equally a name this crate never resolves.
    Fdf(Vec<u8>),
}

impl Parameter {
    /// The name the annex prints for this parameter.
    ///
    /// What a report says out loud, so that a person reading it sees the word they typed.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Page(_) => "page",
            Self::NamedDestination(_) => "nameddest",
            Self::StructureElement(_) => "structelem",
            Self::Comment(_) => "comment",
            Self::EmbeddedFile(_) => "ef",
            Self::Zoom { .. } => "zoom",
            Self::View(_) => "view",
            Self::ViewRect { .. } => "viewrect",
            Self::Highlight { .. } => "highlight",
            Self::Search(_) => "search",
            Self::Fdf(_) => "fdf",
        }
    }

    /// What this program cannot do with this parameter, and why.
    ///
    /// `None` means it is carried out. **A claim about this tree rather than about the standard**,
    /// in the shape [`crate::requirements::Kind::unmet`] already uses: a sentence rather than a
    /// boolean, one reason per arm, and it decays — which it has now done for all four of the
    /// parameters that were ever on this list. `Search` came off
    /// this list in the four-hundred-and-fourteenth session, where it had read "this program has no
    /// document-wide search" and `viewer_core::Command::Find` is one; `EmbeddedFile` came off it in
    /// the four-hundred-and-seventy-fifth, where it had read "opening an embedded file is the
    /// host's decision, and every parameter after this one applies to that file rather than to this
    /// document" — two claims in one coat, and only the second was ever a blocker. Table Annex O.3
    /// in §O.2.1 makes this parameter the annex's one `shall` on a *processor* that has nothing to
    /// do with where the window points:
    ///
    /// > When used as part of a PDF open parameter, the PDF processor shall open the embedded file
    /// > contained within the EmbeddedFiles name tree identified by name .
    ///
    /// §7.11.4's bytes are inside the document and `viewer_core::Event::Extracted` is how they
    /// reach a host, which is precisely what "the host's decision" describes rather than what
    /// stands in its way. ADR 0310.
    ///
    /// **The last two came off in the five-hundred-and-twenty-second, and every one of the eleven
    /// is carried out**, which is what this function now answers and why it has no arm left that
    /// names a reason. `Highlight` had read that the annex's rectangle "is a shape of its own, and
    /// no host has asked for one to draw": the shape is `viewer_core::Query::Highlight` and the
    /// annex is what asks for it, on the same argument ADR 0316 took for §12.4.4.2 — a clause may
    /// ask for a message where a host cannot see what the question is about, and no host sees this
    /// URI's fragment. `Fdf` had read that "fetching a URI is the host's, and no host supplies one
    /// yet", of which the first half is a description of the division rather than a blocker and the
    /// second stopped being true when `Event::NeedsFile` reached three hosts: §12.7.6.4's file
    /// arrives that way already, and `fdf` names one for the same purpose.
    ///
    /// **The shape stays although nothing uses it**, and that is the point rather than an
    /// oversight: the answer decays in both directions, so a parameter this program stops being
    /// able to carry out — a format withdrawn, a dependency lost — has somewhere to say so, and
    /// `tools/state.sh annex-o` reads this function whichever way it answers.
    #[must_use]
    #[expect(
        clippy::needless_return,
        reason = "`tools/state.sh annex-o` reads this function as text: the variants named before                   `return None` are the ones carried out and each arm after it is a parameter with                   its reason. The keyword is what keeps that contract readable now that there is                   nothing after it, and what the next refusal will be written in front of."
    )]
    pub fn unhonoured(&self) -> Option<&'static str> {
        match self {
            Self::Page(_)
            | Self::NamedDestination(_)
            | Self::StructureElement(_)
            | Self::Comment(_)
            | Self::EmbeddedFile(_)
            | Self::Zoom { .. }
            | Self::View(_)
            | Self::ViewRect { .. }
            | Self::Highlight { .. }
            | Self::Search(_)
            | Self::Fdf(_) => return None,
        }
    }

    /// Reads one parameter's arguments, or `None` where they are not what the table states.
    ///
    /// **The count is part of what the table states**, so a parameter given more arguments than its
    /// row names is not read. Nothing in the annex defines a fourth argument to `zoom` or a second
    /// to `nameddest`, and taking the ones that fit would mean looking up a destination whose name
    /// is not the one that was written. `view` is the exception and not by choice: its arguments are
    /// Table 149's, and how many a keyword takes is that table's business — which is
    /// [`View::from_keyword`], reading them exactly as it reads a destination array's.
    fn read(name: &str, arguments: &[Vec<u8>]) -> Option<Self> {
        let text = |count: usize| {
            (arguments.len() == count)
                .then(|| arguments.first())
                .flatten()
                .filter(|argument| !argument.is_empty())
                .cloned()
        };
        let number = |index: usize| arguments.get(index).and_then(|bytes| number(bytes));
        let counted = |count: usize| (arguments.len() == count).then_some(());
        Some(match name {
            "page" => Self::Page(page_number(&text(1)?)?),
            "nameddest" => Self::NamedDestination(text(1)?),
            "structelem" => Self::StructureElement(text(1)?),
            "comment" => Self::Comment(text(1)?),
            "ef" => Self::EmbeddedFile(text(1)?),
            "zoom" => Self::Zoom {
                percent: number(0)?,
                // "The left and top arguments are optional, but shall both be specified if either
                // is included", so one of the two is a malformed parameter rather than a
                // magnification with half a position.
                offset: match arguments.len() {
                    1 => None,
                    3 => Some((number(1)?, number(2)?)),
                    _ => return None,
                },
            },
            // Table 149's keyword and its position values, read where Table 149 already lives.
            "view" => Self::View(View::from_keyword(
                arguments.first().filter(|keyword| !keyword.is_empty())?,
                &(1..arguments.len()).map(&number).collect::<Vec<_>>(),
            )?),
            "viewrect" => {
                counted(4)?;
                Self::ViewRect {
                    left: number(0)?,
                    top: number(1)?,
                    width: number(2)?,
                    height: number(3)?,
                }
            }
            "highlight" => {
                counted(4)?;
                Self::Highlight {
                    left: number(0)?,
                    right: number(1)?,
                    top: number(2)?,
                    bottom: number(3)?,
                }
            }
            // "a string ... comprised of individual words separated by space characters", so the
            // whole argument is one string and the words are inside it. A comma in it would have
            // split it into several by §O.2's rule, which is the writer's business; every part is
            // taken, in order, so nothing typed is lost.
            "search" => Self::Search({
                let words: Vec<Vec<u8>> = arguments
                    .iter()
                    .flat_map(|argument| words(argument))
                    .collect();
                if words.is_empty() {
                    return None;
                }
                words
            }),
            "fdf" => Self::Fdf(text(1)?),
            _ => return None,
        })
    }
}

/// The annotation on `page` that Table 166's `/NM` names — Annex O's `comment` parameter.
///
/// §12.5.2 makes the entry "[t]he annotation name, a text string uniquely identifying it among all
/// the annotations on its page", which is the sentence behind the annex's own restriction that a
/// `comment` parameter must follow whatever chose the page: §O.2.1 says "[t]he NM key is unique to
/// a specific page, but is not guaranteed to be unique to a document", so this asks one page.
///
/// The comparison is between the fragment's bytes and §7.9.2's *decoded* text string, as UTF-8.
/// Annex O gives its argument no character encoding and the file's own string states one, so
/// decoding the half that says what it is is the only comparison either side defines.
///
/// An annotation written directly into `/Annots` rather than as a reference is skipped: what a
/// caller does with the answer is address that annotation, and an object with no identity cannot
/// be addressed. The first match wins, because the clause makes the name unique and a file that
/// broke that has not said which one it meant.
#[must_use]
pub fn annotation_named(
    document: &pdf_syntax::Document,
    page: &crate::Page,
    name: &[u8],
) -> Option<pdf_syntax::ObjectId> {
    let annotations = document.get_key(&page.dict, "Annots");
    let annotations = annotations.as_array()?;
    for annotation in annotations {
        let Some(id) = annotation.as_reference() else {
            continue;
        };
        let annotation = document.resolve(annotation);
        let Some(annotation) = annotation.as_dict() else {
            continue;
        };
        if let pdf_syntax::Object::String(stated) = document.get_key(annotation, "NM")
            && pdf_syntax::text_string(&stated).as_bytes() == name
        {
            return Some(id);
        }
    }
    None
}

/// A parameter that was not read, named rather than dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unread {
    /// The parameter as the fragment spells it.
    pub name: String,
    /// Why it was not read.
    pub reason: Reason,
}

/// Why a parameter was not read.
///
/// Two, and they mean opposite things to whoever wrote the URI: the first is a fragment written
/// for some other reader, the second is one of Annex O's own eleven with something wrong in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Neither Table Annex O.3 nor Table Annex O.4 defines a parameter with this name.
    Unknown,
    /// The annex defines it; its arguments are not the ones the table states.
    Arguments,
}

impl Reason {
    /// What this reason says, for a report a person reads.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "not one of Annex O's eleven parameters",
            Self::Arguments => "its arguments are not what Annex O states for it",
        }
    }
}

/// §7.3.3's number syntax, over one already-decoded argument.
///
/// Deliberately stricter than [`pdf_syntax::Lexer`]'s reader, which salvages a malformed number
/// because a content stream still has to draw. Nothing has to draw here.
fn number(bytes: &[u8]) -> Option<f32> {
    let text = std::str::from_utf8(bytes).ok()?;
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    // "one or more decimal digits with an optional sign and a leading, trailing, or embedded
    // PERIOD (2Eh)": at most one period, at least one digit, and nothing else at all.
    if digits.is_empty()
        || digits.matches('.').count() > 1
        || !digits.contains(|character: char| character.is_ascii_digit())
        || !digits
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
    {
        return None;
    }
    // Rejected above: an exponent, `inf` and `NaN` — the three forms `f32::from_str` accepts and
    // §7.3.3 does not have. What is left can still overflow to infinity on a number with three
    // hundred digits, which is a value rather than a syntax and is refused here.
    text.parse::<f32>().ok().filter(|value| value.is_finite())
}

/// Table Annex O.3's `pageNum`: "a positive integer number", counting from one.
fn page_number(bytes: &[u8]) -> Option<u32> {
    let text = std::str::from_utf8(bytes).ok()?;
    let digits = text.strip_prefix('+').unwrap_or(text);
    if digits.is_empty() || !digits.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok().filter(|page| *page > 0)
}

/// The words of a `wordList`, with the quotation marks §O.2.2 asks a writer for removed.
fn words(argument: &[u8]) -> Vec<Vec<u8>> {
    let unquoted = match argument {
        [b'"', inner @ .., b'"'] => inner,
        whole => whole,
    };
    unquoted
        .split(u8::is_ascii_whitespace)
        .filter(|word| !word.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}

/// Percent-decoding, applied to one argument after the fragment has been split.
///
/// A `%` that is not followed by two hexadecimal digits is kept as itself. RFC 3986 calls that
/// sequence invalid and states no recovery; a byte string on its way to being compared with the
/// document's own bytes is better off carrying what was typed than losing it.
fn percent_decode(argument: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(argument.len());
    let mut rest = argument.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'%'
            && let (Some(high), Some(low)) = (
                tail.first().copied().and_then(hex_value),
                tail.get(1).copied().and_then(hex_value),
            )
        {
            out.push(high.saturating_mul(16).saturating_add(low));
            rest = tail.get(2..).unwrap_or_default();
            continue;
        }
        out.push(byte);
        rest = tail;
    }
    out
}

/// The value of one hexadecimal digit.
///
/// Four lines rather than a shared helper: `pdf_syntax`'s is §7.3.4.3's hexadecimal *string*, and
/// a percent-escape is not one — the two happen to spell a byte the same way and are governed by
/// different documents.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "each arm's range guarantees the subtraction is in range and the sum is at most 15"
)]
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Fragment, Parameter, Reason};
    use crate::destination::View;

    /// The fragments in these tests are written by hand, and that is not this tree's usual habit.
    /// A fragment identifier cannot come from a corpus document: it arrives with the *request*,
    /// which is exactly why the ledger's row for Annex O was `silent` and why no gate in this
    /// project could ever have found it. What the fragments are applied *to* is a real document,
    /// and those tests are `viewer-core`'s.
    fn parse(fragment: &str) -> Fragment {
        Fragment::parse(fragment)
    }

    #[test]
    fn every_parameter_the_two_tables_define_is_read() {
        let read = parse(
            "page=12&nameddest=Chapter3&structelem=Sec1.1&comment=note-4&ef=data.xml\
             &zoom=150,72,720&view=FitH,700&viewrect=10,20,300,400&highlight=1,2,3,4\
             &search=\"ordered pair\"&fdf=values.fdf",
        );
        assert_eq!(read.unread, Vec::new());
        assert_eq!(
            read.parameters,
            vec![
                Parameter::Page(12),
                Parameter::NamedDestination(b"Chapter3".to_vec()),
                Parameter::StructureElement(b"Sec1.1".to_vec()),
                Parameter::Comment(b"note-4".to_vec()),
                Parameter::EmbeddedFile(b"data.xml".to_vec()),
                Parameter::Zoom {
                    percent: 150.0,
                    offset: Some((72.0, 720.0)),
                },
                Parameter::View(View::FitH { top: Some(700.0) }),
                Parameter::ViewRect {
                    left: 10.0,
                    top: 20.0,
                    width: 300.0,
                    height: 400.0,
                },
                Parameter::Highlight {
                    left: 1.0,
                    right: 2.0,
                    top: 3.0,
                    bottom: 4.0,
                },
                Parameter::Search(vec![b"ordered".to_vec(), b"pair".to_vec()]),
                Parameter::Fdf(b"values.fdf".to_vec()),
            ]
        );
    }

    /// §O.2: "[f]ragment identifiers shall be processed and (if required) executed from left to
    /// right as they appear in the character string". The order is the requirement, so it is what
    /// the reader has to preserve — the `comment` row's own NOTE turns it into behaviour.
    #[test]
    fn the_order_the_fragment_states_is_the_order_that_comes_back() {
        let forwards = parse("page=3&comment=x");
        let backwards = parse("comment=x&page=3");
        assert_eq!(
            forwards.parameters,
            vec![Parameter::Page(3), Parameter::Comment(b"x".to_vec())]
        );
        assert_eq!(
            backwards.parameters,
            vec![Parameter::Comment(b"x".to_vec()), Parameter::Page(3)]
        );
    }

    #[test]
    fn a_parameter_no_table_defines_is_named_rather_than_dropped() {
        let read = parse("pagemode=bookmarks&page=2&Page=3&toolbar");
        assert_eq!(read.parameters, vec![Parameter::Page(2)]);
        assert_eq!(
            read.unread
                .iter()
                .map(|unread| (unread.name.as_str(), unread.reason))
                .collect::<Vec<_>>(),
            vec![
                ("pagemode", Reason::Unknown),
                // Case-sensitive, like Table 149's own keywords: the annex prints `page`.
                ("Page", Reason::Unknown),
                // A parameter with no arguments at all is not one of the eleven either — every
                // row of both tables states arguments.
                ("toolbar", Reason::Unknown),
            ]
        );
    }

    /// The distinction the two reasons exist for: a fragment written for another reader, against
    /// one of Annex O's own eleven with something wrong inside it.
    #[test]
    fn a_defined_parameter_with_bad_arguments_is_not_an_unknown_one() {
        let read = parse("page=one&viewrect=1,2,3&zoom=&view=Sideways,1");
        assert_eq!(read.parameters, Vec::new());
        assert!(
            read.unread
                .iter()
                .all(|unread| unread.reason == Reason::Arguments),
            "{:?}",
            read.unread
        );
        assert_eq!(read.unread.len(), 4);
    }

    /// The count is part of what the table states. Nothing in the annex defines a fourth argument
    /// to `zoom` or a second to `nameddest`, and reading the ones that fit would look up a
    /// destination whose name is not the one that was written.
    #[test]
    fn a_parameter_given_more_arguments_than_its_row_names_is_not_read() {
        for fragment in [
            "zoom=100,1,2,3",
            "nameddest=a,b",
            "page=1,2",
            "viewrect=1,2,3,4,5",
            "highlight=1,2,3,4,5",
        ] {
            let read = parse(fragment);
            assert_eq!(read.parameters, Vec::new(), "{fragment}");
            assert_eq!(read.unread.len(), 1, "{fragment}");
            assert_eq!(read.unread[0].reason, Reason::Arguments, "{fragment}");
        }
        // `view` is the exception, and it is Table 149's rule rather than this reader's: how many
        // numbers a keyword takes is that table's business, and a destination array with more
        // entries than its form uses is read the same way in §12.3.2.2.
        assert_eq!(
            parse("view=Fit,1,2").parameters,
            vec![Parameter::View(View::Fit)]
        );
    }

    /// `pdf_syntax::Lexer` reads `12pt` as 12 and `x` as zero, because a content stream still has
    /// to draw. A fragment has nothing to keep drawing, and a page number silently read as zero
    /// would open the document somewhere nobody asked for.
    #[test]
    fn a_malformed_number_is_refused_rather_than_salvaged() {
        for fragment in ["page=12pt", "page=0", "page=-3", "zoom=1e5", "zoom=inf"] {
            let read = parse(fragment);
            assert_eq!(read.parameters, Vec::new(), "{fragment}");
            assert_eq!(read.unread.len(), 1, "{fragment}");
            assert_eq!(read.unread[0].reason, Reason::Arguments, "{fragment}");
        }
    }

    /// §7.3.3's syntax, in the forms EXAMPLE 2 gives: a leading, trailing or embedded period, and
    /// an optional sign.
    #[test]
    fn the_numbers_of_the_standards_own_syntax_are_read() {
        assert_eq!(
            parse("viewrect=-.002,4.,+123.6,0").parameters,
            vec![Parameter::ViewRect {
                left: -0.002,
                top: 4.0,
                width: 123.6,
                height: 0.0,
            }]
        );
    }

    /// RFC 3986 section 2.4: a component is split before its octets are decoded. A `%26` inside a
    /// name is data, and decoding first would have made it a second parameter.
    #[test]
    fn a_percent_escape_is_decoded_after_the_separators_and_not_before() {
        assert_eq!(
            parse("nameddest=A%26B%2Cc").parameters,
            vec![Parameter::NamedDestination(b"A&B,c".to_vec())]
        );
        // §O.2.1 makes `structelem`'s argument "a byte string with URI encoding", so this is the
        // one whose decoding the annex states outright.
        assert_eq!(
            parse("structelem=Chapter%201").parameters,
            vec![Parameter::StructureElement(b"Chapter 1".to_vec())]
        );
        // A `%` that begins no escape is kept, because a byte string on its way to a comparison
        // with the file's own bytes is better off carrying what was typed.
        assert_eq!(
            parse("nameddest=100%25%2&ef=a%zz").parameters,
            vec![
                Parameter::NamedDestination(b"100%%2".to_vec()),
                Parameter::EmbeddedFile(b"a%zz".to_vec()),
            ]
        );
    }

    /// "The left and top arguments are optional, but shall both be specified if either is
    /// included" — so one of them is a malformed parameter rather than half a position.
    #[test]
    fn zoom_takes_both_offsets_or_neither() {
        assert_eq!(
            parse("zoom=200").parameters,
            vec![Parameter::Zoom {
                percent: 200.0,
                offset: None,
            }]
        );
        assert_eq!(parse("zoom=200,10").parameters, Vec::new());
    }

    /// Table 149 is read in one place, and this is the second caller reaching it: every keyword
    /// the destination array spells, spelled in a URI instead.
    #[test]
    fn view_reads_table_149_where_the_destination_array_does() {
        let forms = [
            ("view=Fit", View::Fit),
            ("view=FitB", View::FitB),
            ("view=FitV,10", View::FitV { left: Some(10.0) }),
            ("view=FitBH,20", View::FitBH { top: Some(20.0) }),
            (
                "view=FitR,0,0,612,792",
                View::FitR {
                    rect: [0.0, 0.0, 612.0, 792.0],
                },
            ),
            (
                "view=XYZ,72,720,1.5",
                View::Xyz {
                    left: Some(72.0),
                    top: Some(720.0),
                    zoom: Some(1.5),
                },
            ),
        ];
        for (fragment, view) in forms {
            assert_eq!(
                parse(fragment).parameters,
                vec![Parameter::View(view)],
                "{fragment}"
            );
        }
        // Table 149's own null, which §12.3.2.2 makes "retained unchanged" rather than absent —
        // and a zoom of zero, which the same clause makes null as well.
        assert_eq!(
            parse("view=XYZ,,,0").parameters,
            vec![Parameter::View(View::Xyz {
                left: None,
                top: None,
                zoom: None,
            })]
        );
    }

    /// "The wordList argument … shall be a string enclosed within quotation marks comprised of
    /// individual words separated by space characters."
    #[test]
    fn a_word_list_is_its_words() {
        assert_eq!(
            parse("search=%22two%20words%22").parameters,
            vec![Parameter::Search(vec![b"two".to_vec(), b"words".to_vec()])]
        );
        assert_eq!(
            parse("search=single").parameters,
            vec![Parameter::Search(vec![b"single".to_vec()])]
        );
        assert_eq!(parse("search=\"\"").parameters, Vec::new());
    }

    /// All eleven are carried out, and this is where a host learns that it has nothing to report.
    ///
    /// **Four have moved sides**, which is what [`Parameter::unhonoured`]'s own comment says it is
    /// for: `search` in the four-hundred-and-fourteenth session, when
    /// `viewer_core::Command::Find` became a document-wide search; `ef` in the
    /// four-hundred-and-seventy-fifth, when its refusal turned out to be about the parameters
    /// *after* it rather than about itself (ADR 0310); and `highlight` and `fdf` in the
    /// five-hundred-and-twenty-second, the first because the annex is what asks for the shape a
    /// host draws and the second because `viewer_core::Event::NeedsFile` reached three hosts
    /// (ADR 0357).
    #[test]
    fn every_parameter_the_annex_defines_is_carried_out() {
        let read = parse(
            "page=1&nameddest=a&structelem=b&comment=c&zoom=100&view=Fit&viewrect=0,0,1,1\
             &ef=d&highlight=0,1,2,3&search=x&fdf=f.fdf",
        );
        let refused: Vec<_> = read
            .parameters
            .iter()
            .filter(|parameter| parameter.unhonoured().is_some())
            .map(Parameter::name)
            .collect();
        assert_eq!(refused, Vec::<&str>::new());
        assert_eq!(
            read.parameters
                .iter()
                .map(Parameter::name)
                .collect::<Vec<_>>(),
            [
                "page",
                "nameddest",
                "structelem",
                "comment",
                "zoom",
                "view",
                "viewrect",
                "ef",
                "highlight",
                "search",
                "fdf"
            ]
        );
    }

    /// A fragment is a person's instruction, and half of one is still worth carrying out. Nothing
    /// here is an error and nothing here refuses the document.
    #[test]
    fn a_fragment_that_says_nothing_reads_as_nothing() {
        for fragment in ["", "&", "&&&", "="] {
            let read = parse(fragment);
            assert_eq!(read.parameters, Vec::new(), "{fragment:?}");
            assert!(read.unread.len() <= 1, "{fragment:?}");
        }
    }
}
