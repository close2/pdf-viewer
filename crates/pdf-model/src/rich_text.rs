//! Rich text: the formatted text ISO 32000-2 hands to XFA 3.3, read and laid out.
//!
//! Five entries carry it. Table 228's `/RV` is a variable text field's "rich text string" and its
//! `/DS` "[a] default style string", both "as described in Adobe XML Architecture, XML Forms
//! Architecture (XFA) Specification, version 3.3"; Table 249 carries an `/RV` into an FDF file;
//! Table 172's `/RC` is the text a markup annotation's popup window shows; and Table 177's `/RC`
//! and `/DS` are a free text annotation's, where the rich text string "shall be used to generate
//! the appearance of the annotation". The specification they name is held (`doc/third-party-data.md`,
//! ADR 1197 for why `CLAUDE.md`'s XFA exclusion does not reach it), and its chapter 27, the Rich
//! Text Reference, is what this module is written against — cited by section and page, never
//! quoted, as ADR 0187 holds every text but ISO 32000-2.
//!
//! # The parts
//!
//! - `style` — the CSS2 declarations a style attribute and a `/DS` state, and what each computes
//!   to.
//! - `markup` — the XHTML subset chapter 27 names, walked into paragraphs of styled runs.
//! - `layout` — those runs set into §12.7.4.3's appearance stream, each in its own face, size and
//!   colour.
//! - [`span`] — the same runs as the flat list of styled spans a script holds, and written back
//!   (ADR 1762).
//!
//! This file is where the five entries meet them: which string a field or a note is drawn from
//! ([`for_field`], [`for_free_text`]), and the `/RV` a value this program set is saved with
//! ([`written`]). ADR 1634 is the subset and the mapping; ADR 1635 is which entry wins when two
//! disagree and what a changed value regenerates.

mod layout;
mod markup;
pub mod span;
mod style;

use pdf_syntax::{Dictionary, Document, Object};

pub(crate) use layout::{Request, lay_out, one_style, root_style};
pub(crate) use markup::RichText;
pub(crate) use style::Character;

use markup::{ListIndent, Paragraph, Piece};
use style::{Block, Declarations, Unapplied};

/// The names a reader outside this module matches a read string's pieces by: §12.5.6.14's popup
/// window, which hands Table 172's `/RC` to a host as runs rather than laying it out (ADR 1642).
pub(crate) mod parts {
    pub(crate) use super::markup::{ListIndent, Piece};
    pub(crate) use super::style::{
        Align, Leader, LeaderPattern, Linear, RuleStyle, Spacing, TabAlign,
    };
}

/// What a field or a note is drawn from, once its entries are read.
pub(crate) struct Chosen {
    /// The text, with its formatting.
    pub(crate) rich: RichText,
    /// Why the formatting drawn is not the rich text string the file states, where it is not.
    pub(crate) disagrees: Option<crate::variable_text::Owed>,
    /// The text's natural language, from [`language`].
    pub(crate) language: pdf_font::pairs::Language,
}

/// The natural language of the text an annotation shows, as the language systems it selects
/// for pair kerning (ADR 1708).
///
/// The natural language specification decides it, and two of its places reach a field's or a
/// note's text. The annotation's own entry, Table 166 of §12.5.2:
///
/// > A language identifier overriding the document's language identifier to specify the natural
/// > language for all text in the annotation except where overridden by other explicit language
/// > specifications
///
/// and beneath it the catalog's, §14.9.2.3:
///
/// > The Lang entry in the document catalog dictionary shall specify the default natural
/// > language for all text in the document.
///
/// A widget's field states no language of its own — Table 226 has no `/Lang` and Table 166's
/// is not inheritable — so the walk is the annotation, then the document. An empty identifier is
/// §14.9.2.2's unknown language, and one that is not a BCP 47 tag is treated as unknown as
/// [`crate::structure::document_language`] treats the catalog's; both select the default
/// language system. XFA 3.3's `locale` property does not reach here: it belongs to a template's
/// draw, field or subform (chapter 4, *Localization and Canonicalization*), which this tree does
/// not read, and an interactive form field carries no such entry. What is not carried is the language a
/// §7.9.2.2.2 escape sequence states inside the value, which the text string's decoding removes,
/// and a structure element's `/Lang` over the annotation.
pub(crate) fn language(document: &Document, annotation: &Dictionary) -> pdf_font::pairs::Language {
    let own = match document.get_key(annotation, "Lang") {
        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
        _ => None,
    };
    own.or_else(|| crate::structure::document_language(document))
        .filter(|tag| crate::structure::well_formed_language_tag(tag))
        .map_or_else(pdf_font::pairs::Language::default, |tag| {
            pdf_font::pairs::Language::of(&tag)
        })
}

/// A text string or text stream entry, decoded: §7.9.2.2's text string, or §7.9.3's stream whose
/// decoded bytes are one.
fn entry_text(document: &Document, value: &Object) -> Option<String> {
    crate::variable_text::value_text(document, value)
}

/// The nearest dictionary of a field's own chain that states an entry.
///
/// Table 228 marks `/DA` and `/Q` inheritable and neither `/RV` nor `/DS`, so this is the walk for
/// a widget merged with its field, or one whose field sits above it in `/Kids` — the dictionaries
/// that *are* the field — and not §12.7.4.1's inheritance.
fn nearest(document: &Document, chain: &[Dictionary], key: &str) -> Option<(usize, Object)> {
    chain.iter().enumerate().find_map(|(at, dict)| {
        let value = document.get_key(dict, key);
        (!value.is_null()).then_some((at, value))
    })
}

/// Whether two texts state the same characters, white space compared the way chapter 27 lays it
/// out: any run of it is one space, and none counts at either end (page 1194).
///
/// A rich text string's paragraphs are line ends in its plain twin, and its compressed spaces are
/// the plain text's runs of them, so this is the comparison under which a producer's `/V` and its
/// `/RV` — or a note's `/Contents` and its `/RC` — are one text.
pub(crate) fn same_characters(rich: &str, plain: &str) -> bool {
    let words = |text: &str| -> Vec<String> {
        text.split(|character: char| character.is_whitespace())
            .filter(|word| !word.is_empty())
            .map(str::to_owned)
            .collect()
    };
    words(rich) == words(plain)
}

/// What a rich text field's current value comes with, beside its characters.
#[derive(Clone, Copy)]
pub(crate) struct Stated<'a> {
    /// The value's text as it stands, which Table 231 bit 26 makes a rich text string.
    pub(crate) markup: Option<&'a str>,
    /// Table 249's `/RV`, where an import stated one beside its value.
    pub(crate) imported_rich: Option<&'a str>,
    /// Whether the value is the file's own `/V` or an import's, as it stands: an edit or a reset
    /// replaces it with a value no `/RV` describes.
    pub(crate) stored: bool,
}

/// A rich text field's text, where Table 231 bit 26 is set and the file states formatting.
///
/// `chain` is the field's own `/Parent` chain, nearest first; `plain` the characters §12.7.5.3
/// makes the field's text, already read from whichever value is current; `stated` what came with
/// that value. An edit or a reset replaces the value, and the `/RV` the file states then describes
/// a value the field no longer has; an import replaces it **and** the `/RV`, where the FDF field
/// states one — §12.7.8.3.2's "importing a field causes the values of the entries in the FDF
/// field dictionary to replace those of the corresponding entries in the field", and Table 249's
/// `/RV` corresponds to Table 228's (ADR 1648). `root` is the style of text nothing styles, from
/// the `/DA`'s face.
///
/// `None` is a field the plain layout draws: the flag clear, or nothing in the file stating
/// formatting — no `/RV`, no `/DS`, no markup in the value.
///
/// # Which entry wins
///
/// Table 231 bit 26 makes the value "a rich text string" and has `/RV` "specify the rich text
/// string" where the field has a value; §12.7.5.3 makes `/V` hold the field's text. So `/RV` is
/// drawn where its characters are the field's, a `/V` that is itself markup is drawn as the rich
/// text string it is (ADR 1197), and where `/RV` states other characters `/V` wins, in the
/// field's default style, with [`crate::variable_text::Owed::RichTextDisagrees`] beside it (ADR
/// 1635).
pub(crate) fn for_field(
    document: &Document,
    chain: &[Dictionary],
    plain: &str,
    stated: Stated<'_>,
    root: &Character,
) -> Option<Chosen> {
    let default_style =
        nearest(document, chain, "DS").and_then(|(_, value)| entry_text(document, &value));
    let default_style = default_style.as_deref();
    // The chain's nearest dictionary is the widget, the annotation the text is shown in.
    let language = chain.first().map_or_else(
        || language(document, &Dictionary::new()),
        |widget| language(document, widget),
    );
    if stated.stored {
        let rich_value = match stated.imported_rich {
            Some(imported) => Some(imported.to_owned()),
            None => {
                nearest(document, chain, "RV").and_then(|(_, value)| entry_text(document, &value))
            }
        };
        if let Some(rich) =
            rich_value.and_then(|markup| markup::parse(&markup, default_style, root.clone()))
        {
            if same_characters(&rich.text(), plain) {
                return Some(Chosen {
                    rich,
                    disagrees: None,
                    language,
                });
            }
            return Some(Chosen {
                rich: plain_text(plain, default_style, root),
                disagrees: Some(crate::variable_text::Owed::RichTextDisagrees {
                    rich: "/RV",
                    plain: "/V",
                    clause: "§12.7.5.3 makes /V hold the field's text",
                }),
                language,
            });
        }
        if let Some(rich) = stated
            .markup
            .and_then(|markup| markup::parse(markup, default_style, root.clone()))
        {
            return Some(Chosen {
                rich,
                disagrees: None,
                language,
            });
        }
    }
    default_style.map(|default_style| Chosen {
        rich: plain_text(plain, Some(default_style), root),
        disagrees: None,
        language,
    })
}

/// A free text annotation's text, where it states `/RC` or `/DS`.
///
/// `shared` is the dictionary §12.5.6.2's group attributes come from — "Contents (or RC and DS )"
/// are among them — and `retyped` what a person typed in place of the file's text, which the
/// note's default style still styles. `None` is a note the plain layout draws.
///
/// **`/Contents` wins where the two disagree.** §12.5.6.2 says the annotation's `/Contents`
/// "specifies the displayed text" of a free text annotation, and its NOTE 1 says the two are
/// expected to be textually equivalent; Table 177 makes `/RC` what the appearance is generated
/// from. So `/RC` is drawn wherever its characters are the note's, and `/Contents` otherwise, in
/// the note's default style (ADR 1635).
pub(crate) fn for_free_text(
    document: &Document,
    shared: &Dictionary,
    contents: Option<&str>,
    retyped: Option<&str>,
    root: &Character,
) -> Option<Chosen> {
    let default_style = entry_text(document, &document.get_key(shared, "DS"));
    let default_style = default_style.as_deref();
    // The text shown is the group's, so its language is the annotation that holds it.
    let language = language(document, shared);
    if let Some(retyped) = retyped {
        return default_style.map(|default_style| Chosen {
            rich: plain_text(retyped, Some(default_style), root),
            disagrees: None,
            language,
        });
    }
    let rich = entry_text(document, &document.get_key(shared, "RC"))
        .and_then(|markup| markup::parse(&markup, default_style, root.clone()));
    // A `/RC` that is not well formed still has the characters read before the fault, which the
    // popup's reader keeps (ADR 0224); those are the note's text where it states no `/Contents`.
    let lenient = crate::popup::rich_text(document, shared);
    match (rich, contents.filter(|contents| !contents.is_empty())) {
        (Some(rich), None) => Some(Chosen {
            rich,
            disagrees: None,
            language,
        }),
        (Some(rich), Some(contents)) if same_characters(&rich.text(), contents) => Some(Chosen {
            rich,
            disagrees: None,
            language,
        }),
        (Some(_), Some(contents)) => Some(Chosen {
            rich: plain_text(contents, default_style, root),
            disagrees: Some(crate::variable_text::Owed::RichTextDisagrees {
                rich: "/RC",
                plain: "/Contents",
                clause: "§12.5.6.2 makes /Contents specify a free text annotation's displayed text",
            }),
            language,
        }),
        (None, contents) => default_style.map(|default_style| Chosen {
            rich: plain_text(
                contents.or(lenient.as_deref()).unwrap_or_default(),
                Some(default_style),
                root,
            ),
            disagrees: None,
            language,
        }),
    }
}

/// A rich text string's characters, read by the walk that lays it out: paragraphs as carriage
/// returns, white space as chapter 27 compresses it.
///
/// `None` for a string that is not well formed. What an XFDF `<value-richtext>` with no
/// `<value>` beside it makes the field's value (`crate::xfdf`, ADR 1648).
pub(crate) fn characters(markup: &str) -> Option<String> {
    markup::parse(markup, None, Character::root()).map(|rich| rich.text())
}

/// Plain characters as a rich text string with nothing but a default style: one paragraph per
/// line, every character kept as it stands.
///
/// What a field's value is drawn as where the file states no rich text string for it — an edit, a
/// reset, a value `/RV` disagrees with — and what a note's `/Contents` is, under the `/DS` the
/// file does state. The characters are not white-space-compressed: they are a text string, not
/// markup, and §12.5.6.2's carriage return is the only structure they have.
pub(crate) fn plain_text(text: &str, default_style: Option<&str>, root: &Character) -> RichText {
    let mut character = root.clone();
    let mut block = Block::root();
    let mut unapplied = Unapplied::default();
    if let Some(stated) = default_style {
        style::apply(
            &Declarations::parse(stated),
            &mut character,
            &mut block,
            &mut unapplied,
        );
    }
    character.spacerun = true;
    character.tab_count = 0;
    let mut paragraphs = Vec::new();
    for line in text.split("\r\n").flat_map(|line| line.split(['\r', '\n'])) {
        let pieces = if line.is_empty() {
            vec![Piece::Break]
        } else {
            vec![Piece::Text(line.to_owned(), character.clone())]
        };
        paragraphs.push(Paragraph {
            block: block.inherited(),
            strut: character.clone(),
            pieces,
            tag: None,
            list: ListIndent::None,
        });
    }
    RichText {
        body: block,
        paragraphs,
        unapplied,
    }
}

/// Table 228's `/RV` for a value this program set on a rich text field.
///
/// Table 231 bit 26: "If the field has a value, the RV entry of the field dictionary ("Table 228 -
/// Additional entries common to all fields containing variable text") shall specify the rich text
/// string." A person typing into such a field sets plain characters, so the string written is
/// those characters with no formatting of its own — one `p` per line, a run of spaces chapter 27
/// would compress kept by `xfa-spacerun:yes` (page 1220) — and the field's `/DS` styles it, which
/// is what the appearance written beside it shows. The `body` names XHTML's namespace and XFA's,
/// and `xfa:spec` the version of chapter 27 the string is written to (*Version Specification*,
/// page 1222); `xfa:APIVersion` is left out, which the same section reads as the latest revision.
/// ADR 1635.
pub(crate) fn written(text: &str) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\"?><body xmlns=\"http://www.w3.org/1999/xhtml\" \
         xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\" xfa:spec=\"3.3\">",
    );
    for line in text.split("\r\n").flat_map(|line| line.split(['\r', '\n'])) {
        out.push_str("<p>");
        if line.is_empty() {
            out.push_str("<br/>");
        } else {
            let collapses = line.starts_with([' ', '\t'])
                || line.ends_with([' ', '\t'])
                || line.contains("  ")
                || line.contains('\t');
            if collapses {
                out.push_str("<span style=\"xfa-spacerun:yes\">");
            }
            for character in line.chars() {
                match character {
                    '&' => out.push_str("&amp;"),
                    '<' => out.push_str("&lt;"),
                    '>' => out.push_str("&gt;"),
                    other => out.push(other),
                }
            }
            if collapses {
                out.push_str("</span>");
            }
        }
        out.push_str("</p>");
    }
    out.push_str("</body>");
    out
}

/// Writes Table 228's `/RV` beside a value this program set, on the dictionary `/V` goes on.
///
/// Only for a field Table 231 bit 26 makes rich text: the entry is "[o]ptional" otherwise, and a
/// plain field's value has no rich text string to specify. A value set is written as [`written`]
/// spells it; a value cleared takes its `/RV` with it, because bit 26's `shall` is conditioned on
/// the field having a value and an `/RV` left behind would describe one it no longer has (ADR
/// 1635).
pub(crate) fn write_beside(
    document: &Document,
    widget: &Dictionary,
    value: Option<&Object>,
    holder: &mut Dictionary,
) {
    let field = crate::appearance::Field::read(document, widget, crate::view::FieldValue::Stored);
    if field.kind != Some(crate::appearance::FieldKind::Text)
        || field.flags & crate::appearance::FLAG_RICH_TEXT == 0
    {
        return;
    }
    let name = pdf_syntax::Name::new(b"RV".to_vec());
    match value.and_then(|value| entry_text(document, value)) {
        Some(text) => {
            holder.insert(
                name,
                Object::String(pdf_syntax::text_string::encode_text_string(&written(&text)).into()),
            );
        }
        None => {
            holder.remove("RV");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{plain_text, same_characters, written};
    use crate::rich_text::markup;
    use crate::rich_text::style::Character;

    /// The string a save writes reads back as the characters that were typed — a run of spaces,
    /// an empty line and a markup character included — which is what makes the `/RV` beside the
    /// `/V` one value in two entries.
    #[test]
    fn the_rv_a_save_writes_reads_back_as_the_value() {
        let value = "a  <b> & c\r\rlast ";
        let markup = written(value);
        let rich = markup::parse(&markup, None, Character::root()).expect("well formed");
        assert_eq!(rich.text(), "a  <b> & c\r\rlast ");
        assert!(same_characters(&rich.text(), value));
    }

    /// White space compares as chapter 27 lays it out.
    #[test]
    fn characters_compare_with_white_space_compressed() {
        assert!(same_characters("Hi there", " Hi\r\nthere "));
        assert!(!same_characters("Hi there", "Hi where"));
    }

    /// A plain value keeps its characters as they stand, one paragraph a line.
    #[test]
    fn a_plain_value_is_a_paragraph_a_line() {
        let rich = plain_text(
            "one\rtwo  spaced\r\rfour",
            Some("font-size:9pt"),
            &Character::root(),
        );
        assert_eq!(rich.paragraphs.len(), 4);
        assert_eq!(rich.text(), "one\rtwo  spaced\r\rfour");
    }
}
