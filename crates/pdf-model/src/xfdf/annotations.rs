//! XFDF's `<annots>`: each annotation element, spelled back into the annotation dictionary
//! ISO 32000-2 §12.5 defines.
//!
//! # Two texts, and which one decides what
//!
//! The grammar is Adobe's *XML Forms Data Format Specification*, version 3.0 (August 2009), the
//! document ISO 19444-1 was made from, held at `doc/XFDF_Spec_3.0.pdf` and read as
//! `doc/md/XFDF_Spec_3.0.md`. It is licensed for reading and not for copying, so this module
//! cites it by chapter, heading and page and paraphrases it; nothing below is its wording
//! (`doc/third-party-data.md`, ADR 0187). Its divisions are never written with a `§`, which in
//! this tree means ISO 32000-2 alone.
//!
//! What that text supplies is the **spelling**: which element is which subtype (chapter 2,
//! *Annotation Elements*, pages 37 to 54), which attribute or child element is which dictionary
//! entry (*Annotation Subelements*, pages 55 to 68, and *Annotation attributes*, pages 69 to
//! 90), and the two mapping tables that state both directions at once (*Mapping Tables*, pages
//! 91 to 99). Chapter 1's *Implementation Notes* (pages 27 to 32) add the string, rich-text and
//! stream conventions.
//!
//! What the text does not decide is **what the entry means or whether it is allowed**. Its own
//! introduction (chapter 1, *How to Use This Specification*, page 17) sends the reader to the PDF
//! reference for every key it names, so the key an attribute lands in is ISO 32000-2's and so is
//! every rule about it — the subtype tables of §12.5.6, Table 166's common entries, Table 172's
//! markup ones. Where the two disagree, ISO 32000-2 wins, and each place it does is a comment
//! below and a row of ADR 1297.
//!
//! # What becomes of each element
//!
//! [`read`] returns the same [`FdfAnnotation`] an FDF file's `/Annots` is read into, with its
//! dictionary built here rather than carried, so `crate::view::ViewState::import` places both
//! formats' annotations through one path. An XFDF annotation's dictionary is the FDF form of it:
//! Table 254's `/Page` beside the rest, and Table 172's `/IRT` as the text string of the replied-to
//! annotation's `/NM`, which that table requires of an FDF file and the XFDF text states in the
//! same words for its `inreplyto` (page 72). The import turns both into what a PDF file states.
//!
//! A child `<popup>` becomes a second [`FdfAnnotation`] of subtype `Popup`, listed after the
//! annotation it belongs to and linked to it by position, because Table 172's `/Popup` and Table
//! 186's `/Parent` are indirect references and an import is where the objects to refer to exist.
//!
//! # What is refused, and why each is a refusal
//!
//! Every attribute or element the text defines that has nowhere to go is named on
//! `FormsData::owed`, once per kind; nothing is dropped silently. The kinds are a closed list:
//!
//! - an entry ISO 32000-2 states for no annotation at all (`rotation`, whose `/Rotate` no
//!   annotation table defines);
//! - an entry ISO 32000-2 states, but not for this subtype (a `/BS` on a text annotation, a `/DA`
//!   on a caret, a `/BE` on a polyline);
//! - a value whose construction the text does not state (`<appearance>`'s base 64, whose
//!   decoded bytes it never describes; `<overlayappearance>`'s form `XObject`, given only as a
//!   text string);
//! - a text this program excludes (`<ex_data>`, Table 173's external data, whose subtypes are
//!   clause 13's 3D; `<resource>`, Table 45's `/Mac`, deprecated in PDF 2.0 with no entry defined);
//! - an annotation ISO 32000-2 requires an entry of that the element does not supply, which is
//!   not placed at all, since a dictionary without a required entry is not an annotation of its
//!   subtype;
//! - `<link>`, which Table 246 excludes from an FDF file's `/Annots` — the array the text maps
//!   `<annots>` onto (page 37) — and which the import therefore refuses by name, as it refuses a
//!   Link in an FDF file; its destination and action elements are named rather than built.

use pdf_syntax::text_string::encode_text_string;
use pdf_syntax::{Dictionary, Name, Object};

use crate::forms_data::{FdfAnnotation, MAX_ANNOTATIONS};

/// Most bytes of `<data>` decoded out of one XFDF file, over all its annotations.
///
/// The standard states no number and nothing it requires a reader to carry does either (trap 38),
/// so this bounds *work*: the same sixteen mebibytes `forms_data` allows one carried entry, for a
/// file whose attachments and sounds together are past any comment and short of exhausting a
/// reader (`CLAUDE.md` principle 3).
const MAX_DATA_BYTES: usize = 16 * 1024 * 1024;

/// Most numbers read out of one coordinate attribute or element.
///
/// An ink annotation is the largest honest case, one gesture per stroke; a list longer than this
/// is making the reader work rather than describing a mark.
const MAX_NUMBERS: usize = 1 << 20;

/// One element of an annotation, captured whole so that its children can be read in any order.
///
/// XFDF's content models join an annotation's children with the text's *and* connector (chapter
/// 1, *XML content model syntax*, page 32), so `<popup>` may come before or after
/// `<contents-richtext>` and a streaming reader would have to hold the annotation open anyway.
#[derive(Debug, Default)]
pub(super) struct Node {
    /// The element's local name.
    pub(super) name: String,
    /// Its attributes, unescaped, in the order the file states them.
    pub(super) attributes: Vec<(String, String)>,
    /// Its character content, unescaped.
    pub(super) text: String,
    /// Where its content begins and ends in the file, for `<contents-richtext>`, whose rich text
    /// string is the markup inside it rather than its characters.
    pub(super) markup: Option<(usize, usize)>,
    /// Whether any element opened inside it, which is how a `<contents-richtext>` holding plain
    /// text is told from one holding markup.
    pub(super) has_elements: bool,
    /// Its child elements, in order.
    pub(super) children: Vec<Node>,
}

impl Node {
    /// The value of the attribute of this local name.
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(stated, _)| stated == name)
            .map(|(_, value)| value.as_str())
    }

    /// The first child element of this local name.
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|child| child.name == name)
    }
}

/// The text's annotation elements and the Table 171 subtype each one is.
///
/// Chapter 2's *Annotation Elements* (page 37) and the *Subtype* rows of the mapping tables (pages
/// 94 and 98). **One spelling is ISO 32000-2's and not the text's**: the mapping table writes the
/// polyline subtype `Polyline`, and Table 171 spells it `PolyLine`. The table is what a reader
/// matches on, so it is the one written.
const SUBTYPES: [(&str, &str); 19] = [
    ("text", "Text"),
    ("highlight", "Highlight"),
    ("underline", "Underline"),
    ("strikeout", "StrikeOut"),
    ("squiggly", "Squiggly"),
    ("line", "Line"),
    ("circle", "Circle"),
    ("square", "Square"),
    ("caret", "Caret"),
    ("polygon", "Polygon"),
    ("polyline", "PolyLine"),
    ("stamp", "Stamp"),
    ("ink", "Ink"),
    ("freetext", "FreeText"),
    ("fileattachment", "FileAttachment"),
    ("sound", "Sound"),
    ("link", "Link"),
    ("redact", "Redact"),
    ("projection", "Projection"),
];

/// Table 171's non-markup subtypes among [`SUBTYPES`], plus `Popup`.
///
/// Table 172's entries — `/T`, `/RC`, `/CreationDate`, `/IRT`, `/Subj`, `/RT`, `/IT` and `/Popup`
/// — belong to a markup annotation, and the table's third column says which those are.
const NOT_MARKUP: [&str; 2] = ["Link", "Popup"];

/// The subtypes whose §12.5.6 table states a `/BS` (Tables 176, 177, 178, 180, 181 and 185).
const BORDER_STYLE: [&str; 8] = [
    "Link", "FreeText", "Line", "Square", "Circle", "Polygon", "PolyLine", "Ink",
];

/// The subtypes whose §12.5.6 table states a `/BE`.
///
/// §12.5.4 names square, circle and polygon from PDF 1.5 and free text from PDF 1.6, and Table 181
/// makes the entry "meaningful only for polygon annotations". The text lists a polyline among the
/// elements taking a border effect (page 84); ISO 32000-2 does not, and wins.
const BORDER_EFFECT: [&str; 4] = ["Square", "Circle", "Polygon", "FreeText"];

/// The subtypes whose §12.5.6 table states an `/IC` (Tables 178, 180, 181 and 195).
const INTERIOR_COLOUR: [&str; 6] = ["Line", "Square", "Circle", "Polygon", "PolyLine", "Redact"];

/// The subtypes whose §12.5.6 table states an `/RD` (Tables 177, 180 and 183).
const DIFFERENCES: [&str; 4] = ["FreeText", "Square", "Circle", "Caret"];

/// The subtypes whose §12.5.6 table states `/QuadPoints` (Tables 176, 182 and 195).
const QUADRILATERALS: [&str; 6] = [
    "Highlight",
    "Underline",
    "Squiggly",
    "StrikeOut",
    "Link",
    "Redact",
];

/// The subtypes whose §12.5.6 table states an icon `/Name` (Tables 175, 184, 187 and 188).
const ICONS: [&str; 4] = ["Text", "Stamp", "FileAttachment", "Sound"];

/// The subtypes whose table makes one entry required, the entry, and the sentence that names an
/// annotation which cannot be placed without it.
const REQUIRED: [(&str, &str, &str); 8] = [
    (
        "Highlight",
        "QuadPoints",
        "<highlight> without coords: Table 182 requires /QuadPoints, so it is not placed",
    ),
    (
        "Underline",
        "QuadPoints",
        "<underline> without coords: Table 182 requires /QuadPoints, so it is not placed",
    ),
    (
        "Squiggly",
        "QuadPoints",
        "<squiggly> without coords: Table 182 requires /QuadPoints, so it is not placed",
    ),
    (
        "StrikeOut",
        "QuadPoints",
        "<strikeout> without coords: Table 182 requires /QuadPoints, so it is not placed",
    ),
    (
        "Line",
        "L",
        "<line> without both start and end: Table 178 requires /L, so it is not placed",
    ),
    (
        "Polygon",
        "Vertices",
        "<polygon> without vertices: Table 181 requires /Vertices, so it is not placed",
    ),
    (
        "PolyLine",
        "Vertices",
        "<polyline> without vertices: Table 181 requires /Vertices, so it is not placed",
    ),
    (
        "Ink",
        "InkList",
        "<ink> without an inklist: Table 185 requires /InkList, so it is not placed",
    ),
];

/// Reads every captured annotation element into the annotations an import places.
///
/// `source` is the whole file, for the rich text a `<contents-richtext>` holds as markup.
pub(super) fn read(
    nodes: &[Node],
    source: &str,
    owed: &mut Vec<&'static str>,
) -> Vec<FdfAnnotation> {
    let mut out = Vec::new();
    let mut budget = MAX_DATA_BYTES;
    for node in nodes {
        if out.len() >= MAX_ANNOTATIONS {
            owe(
                owed,
                "<annots>: more annotations than this reader places, and the list was cut",
            );
            break;
        }
        let Some(&(_, subtype)) = SUBTYPES.iter().find(|(element, _)| *element == node.name) else {
            owe(
                owed,
                "<annots>: a child element the XFDF text names no annotation for",
            );
            continue;
        };
        // Table 254's `/Page`, which §12.7.8.3.4 requires of every annotation in an FDF file and
        // the text's `page` attribute is (page 70). The redaction element's attribute list leaves
        // it out (page 54), and Table 254 does not, so it is read here for every element alike.
        let Some(page) = node
            .attribute("page")
            .and_then(|page| page.trim().parse::<usize>().ok())
        else {
            owe(
                owed,
                "<annots>: an annotation with no page attribute, which Table 254 requires, \
                 so it names no page to be placed on",
            );
            continue;
        };
        let mut builder = Builder {
            node,
            subtype,
            dict: Dictionary::new(),
            owed: &mut *owed,
            source,
            budget: &mut budget,
        };
        let Some(dict) = builder.annotation() else {
            continue;
        };
        let index = out.len();
        out.push(FdfAnnotation {
            page: Some(page),
            subtype: Some(subtype.to_owned()),
            dictionary: Some(dict),
            popup: None,
            parent: None,
        });
        let Some(popup) = node.child("popup") else {
            continue;
        };
        if NOT_MARKUP.contains(&subtype) {
            // Table 172's `/Popup` is a markup annotation's entry, and a link is not one.
            owe(
                owed,
                "<popup> under <link>: Table 172's /Popup belongs to a markup annotation",
            );
            continue;
        }
        let mut builder = Builder {
            node: popup,
            subtype: "Popup",
            dict: Dictionary::new(),
            owed: &mut *owed,
            source,
            budget: &mut budget,
        };
        let Some(dict) = builder.annotation() else {
            continue;
        };
        let popup_page = popup
            .attribute("page")
            .and_then(|page| page.trim().parse::<usize>().ok())
            .unwrap_or(page);
        out[index].popup = Some(out.len());
        out.push(FdfAnnotation {
            page: Some(popup_page),
            subtype: Some("Popup".to_owned()),
            dictionary: Some(dict),
            popup: None,
            parent: Some(index),
        });
    }
    out
}

/// Records one sentence on `FormsData::owed`, once however often the file earns it.
fn owe(owed: &mut Vec<&'static str>, what: &'static str) {
    if !owed.contains(&what) {
        owed.push(what);
    }
}

/// One annotation element being spelled into its dictionary.
struct Builder<'a, 'b> {
    /// The element.
    node: &'a Node,
    /// Its Table 171 subtype.
    subtype: &'static str,
    /// The dictionary so far.
    dict: Dictionary,
    /// `FormsData::owed`.
    owed: &'b mut Vec<&'static str>,
    /// The whole file.
    source: &'a str,
    /// What is left of [`MAX_DATA_BYTES`].
    budget: &'b mut usize,
}

impl Builder<'_, '_> {
    /// The dictionary, or `None` where an entry ISO 32000-2 requires of the subtype is absent.
    fn annotation(&mut self) -> Option<Dictionary> {
        self.put("Type", name("Annot"));
        self.put("Subtype", name(self.subtype));
        let mut border_style = Dictionary::new();
        let mut border_effect = Dictionary::new();
        for (attribute, value) in &self.node.attributes {
            self.attribute(attribute, value, &mut border_style, &mut border_effect);
        }
        for child in &self.node.children {
            self.element(child, &mut border_style);
        }
        self.borders(border_style, border_effect);
        self.pairs();
        self.required()
    }

    /// Inserts one entry, answering `Some` so that an attribute's arm reads as the entry it made.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "an attribute's arm answers None for a malformed value and this for the entry made"
    )]
    fn put(&mut self, key: &str, value: Object) -> Option<()> {
        self.dict.insert(Name::new(key.as_bytes()), value);
        Some(())
    }

    /// Whether the subtype is a markup annotation, which Table 171's third column decides.
    fn markup(&self) -> bool {
        !NOT_MARKUP.contains(&self.subtype)
    }

    /// One attribute, into the entry the text maps it to where ISO 32000-2 states that entry for
    /// this subtype.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per attribute of the text's list, which is the mapping itself"
    )]
    fn attribute(
        &mut self,
        attribute: &str,
        value: &str,
        border_style: &mut Dictionary,
        border_effect: &mut Dictionary,
    ) {
        let subtype = self.subtype;
        let markup = self.markup();
        let mapped = match attribute {
            // Read by `read`, as Table 254's `/Page`, and not an entry of the dictionary itself.
            "page" => return,
            // *Common annotation attributes*, page 70: Table 166's entries.
            "rect" => numbers(value, 4, 4).and_then(|rect| self.put("Rect", reals(&rect))),
            "color" => colour(value).map(|colour| {
                if let Colour::Rgb(colour) = colour {
                    self.put("C", colour);
                }
            }),
            "date" => self.put("M", text(value)),
            "flags" => flags(value).and_then(|flags| self.put("F", Object::Integer(flags))),
            "name" => self.put("NM", text(value)),
            // Table 172's `/T` is a markup annotation's; Table 186 names a popup's own `/T` as
            // one its parent's overrides, so a popup may state one too. A link may not.
            "title" if markup || subtype == "Popup" => self.put("T", text(value)),
            // *Markup annotation attributes*, pages 71 and 72: Table 172's entries, and `/CA`,
            // which Table 166 now states for every annotation.
            "opacity" => numbers(value, 1, 1).and_then(|opacity| self.put("CA", real(opacity[0]))),
            "creationdate" if markup => self.put("CreationDate", text(value)),
            "subject" if markup => self.put("Subj", text(value)),
            "intent" if markup => self.put("IT", name(intent(value))),
            "inreplyto" if markup => self.put("IRT", text(value)),
            "replyType" if markup => match value {
                "reply" => self.put("RT", name("R")),
                "group" => self.put("RT", name("Group")),
                _ => None,
            },
            // *Text annotation attributes*, page 73, and the other three icon names.
            "icon" if ICONS.contains(&subtype) => self.put("Name", name(value)),
            // Table 175 types both as text strings; the text calls them names (page 74). ISO
            // 32000-2 decides the type.
            "state" if subtype == "Text" => self.put("State", text(value)),
            "statemodel" if subtype == "Text" => self.put("StateModel", text(value)),
            // *Text markup annotation attributes* (page 72), and the same attribute on a
            // redaction (page 83).
            "coords" if QUADRILATERALS.contains(&subtype) => numbers(value, 8, MAX_NUMBERS)
                .and_then(|quads| {
                    quads
                        .len()
                        .is_multiple_of(8)
                        .then_some(())
                        .and_then(|()| self.put("QuadPoints", reals(&quads)))
                }),
            // *Line annotation attributes*, pages 74 to 76: Table 178's entries. `start` and
            // `end` are halves of one `/L` and are joined in `pairs`.
            "start" | "end" if subtype == "Line" => numbers(value, 2, 2).map(|_| ()),
            "head" | "tail" if subtype == "Line" || subtype == "PolyLine" => Some(()),
            "leaderLength" if subtype == "Line" => {
                numbers(value, 1, 1).and_then(|length| self.put("LL", real(length[0])))
            }
            "leaderExtend" if subtype == "Line" => {
                numbers(value, 1, 1).and_then(|length| self.put("LLE", real(length[0])))
            }
            "leader-offset" if subtype == "Line" => {
                numbers(value, 1, 1).and_then(|length| self.put("LLO", real(length[0])))
            }
            "caption" if subtype == "Line" => {
                yes_no(value).and_then(|caption| self.put("Cap", Object::Boolean(caption)))
            }
            "caption-style" if subtype == "Line" => match value {
                "Inline" | "Top" => self.put("CP", name(value)),
                _ => None,
            },
            "caption-offset-h" | "caption-offset-v" if subtype == "Line" => {
                numbers(value, 1, 1).map(|_| ())
            }
            "interior-color" if INTERIOR_COLOUR.contains(&subtype) => colour(value).map(|colour| {
                if let Colour::Rgb(colour) = colour {
                    self.put("IC", colour);
                }
            }),
            // *Circle and Square* and *Caret annotation attributes*, page 77.
            "fringe" if DIFFERENCES.contains(&subtype) => {
                numbers(value, 4, 4).and_then(|fringe| self.put("RD", reals(&fringe)))
            }
            "symbol" if subtype == "Caret" => match value {
                "none" => self.put("Sy", name("None")),
                "paragraph" => self.put("Sy", name("P")),
                _ => None,
            },
            // *Freetext annotation attributes* (page 79) and the redaction's (page 84).
            "justification" if subtype == "FreeText" || subtype == "Redact" => {
                justification(value).and_then(|quadding| self.put("Q", Object::Integer(quadding)))
            }
            // *Border style attributes*, pages 84 and 85: Table 168's `/BS`. `style` is shared
            // with Table 169's `/BE` by the text's own design (page 84), and `cloudy` is the one
            // value that belongs to the second.
            "style" if value == "cloudy" => {
                if BORDER_EFFECT.contains(&subtype) {
                    border_effect.insert(Name::new(&b"S"[..]), name("C"));
                    Some(())
                } else {
                    owe(
                        self.owed,
                        "style=\"cloudy\" on an annotation whose ISO 32000-2 table states no /BE",
                    );
                    return;
                }
            }
            "width" | "dashes" | "style" if BORDER_STYLE.contains(&subtype) => {
                border_style_entry(attribute, value, border_style)
            }
            "intensity" if BORDER_EFFECT.contains(&subtype) => numbers(value, 1, 1).map(|i| {
                border_effect.insert(Name::new(&b"I"[..]), real(i[0]));
            }),
            // *Redaction annotation attributes*, pages 83 and 84: Table 195's entries.
            "overlay-text" if subtype == "Redact" => self.put("OverlayText", text(value)),
            "overlay-text-repeat" if subtype == "Redact" => match value {
                "true" => self.put("Repeat", Object::Boolean(true)),
                "false" => self.put("Repeat", Object::Boolean(false)),
                _ => None,
            },
            // *Popup annotation attributes*, page 82: Table 186's `/Open`.
            "open" if subtype == "Popup" => {
                yes_no(value).and_then(|open| self.put("Open", Object::Boolean(open)))
            }
            // *Link annotation attributes*, page 82: Table 176's `/H`.
            "Highlight" if subtype == "Link" => match value {
                "None" => self.put("H", name("N")),
                "Invert" => self.put("H", name("I")),
                "Outline" => self.put("H", name("O")),
                "Push" => self.put("H", name("P")),
                _ => None,
            },
            // Read with the `<data>` element they describe, in `stream`.
            "file" | "size" | "modification" | "creation" | "checksum" | "mimetype"
                if subtype == "FileAttachment" =>
            {
                Some(())
            }
            "rate" | "bits" | "channels" | "encoding" if subtype == "Sound" => Some(()),
            // No annotation table of ISO 32000-2 states a `/Rotate`; the text maps the
            // attribute onto one (pages 79, 81 and 54).
            "rotation" => {
                owe(
                    self.owed,
                    "rotation: ISO 32000-2 states no /Rotate entry for any annotation",
                );
                return;
            }
            other => {
                owe(self.owed, unplaced(other));
                return;
            }
        };
        if mapped.is_none() {
            owe(self.owed, malformed(attribute));
        }
    }

    /// One child element.
    fn element(&mut self, child: &Node, border_style: &mut Dictionary) {
        let subtype = self.subtype;
        match child.name.as_str() {
            // Read by `read`, as an annotation of its own.
            "popup" => {}
            // *contents*, page 57: Table 166's `/Contents`.
            "contents" => {
                self.put("Contents", text(&child.text));
            }
            // *The contents and contents-richtext elements in annotations*, page 30: markup goes
            // to Table 172's `/RC`, and plain text goes to `/Contents` unless `<contents>` says
            // that itself.
            "contents-richtext" => self.rich_text(child),
            // *vertices*, page 67: Table 181's `/Vertices`.
            "vertices" if subtype == "Polygon" || subtype == "PolyLine" => {
                match coordinates(&child.text) {
                    Some(vertices) => {
                        self.put("Vertices", reals(&vertices));
                    }
                    None => owe(self.owed, malformed("vertices")),
                }
            }
            // *inklist* and *gesture*, pages 59, 60 and 64: Table 185's `/InkList`, one array per
            // gesture.
            "inklist" if subtype == "Ink" => {
                let paths: Option<Vec<Object>> = child
                    .children
                    .iter()
                    .filter(|gesture| gesture.name == "gesture")
                    .map(|gesture| coordinates(&gesture.text).map(|path| reals(&path)))
                    .collect();
                match paths {
                    Some(paths) if !paths.is_empty() => {
                        self.put("InkList", Object::Array(paths));
                    }
                    _ => owe(self.owed, malformed("inklist")),
                }
            }
            // *defaultappearance*, page 58: `/DA`, a byte string, for the two subtypes whose
            // table states one. The text also lists it under a caret (page 45) and maps it to
            // the free text dictionary's key; Table 183 states no `/DA` for a caret.
            "defaultappearance" if subtype == "FreeText" || subtype == "Redact" => {
                self.put("DA", Object::String(bytes(&child.text).into()));
            }
            "defaultappearance" if subtype == "Caret" => owe(
                self.owed,
                "<defaultappearance> under <caret>: Table 183 states no /DA for a caret",
            ),
            // *defaultstyle*, page 59: Table 177's `/DS`.
            "defaultstyle" if subtype == "FreeText" => {
                self.put("DS", text(&child.text));
            }
            // *The border element*, page 29: a legacy free text border, which the text maps onto
            // both Table 166's `/Border` and Table 168's `/BS`.
            "border" if subtype == "FreeText" => {
                match child.attribute("width").and_then(|w| numbers(w, 1, 1)) {
                    Some(width) => {
                        self.put("Border", reals(&[0.0, 0.0, width[0]]));
                        border_style.insert(Name::new(&b"W"[..]), real(width[0]));
                    }
                    None => owe(self.owed, malformed("border")),
                }
            }
            "data" if subtype == "FileAttachment" || subtype == "Sound" => self.stream(child),
            "appearance" => owe(
                self.owed,
                "<appearance>: the XFDF text gives base 64 and never says what the decoded \
                 bytes are, so there is no /AP to build from it",
            ),
            "overlayappearance" => owe(
                self.owed,
                "<overlayappearance>: Table 195's /RO is a form XObject, which the XFDF text \
                 states only as a text string",
            ),
            "resource" => owe(
                self.owed,
                "<resource>: Table 45's /Mac, deprecated in PDF 2.0, where ISO 32000-2 defines \
                 none of its entries",
            ),
            "ex_data" => owe(
                self.owed,
                "<ex_data>: Table 173's /ExData, whose subtypes are clause 13's 3D, excluded",
            ),
            "Dest" | "OnActivation" | "BorderStyleAlt" if subtype == "Link" => owe(
                self.owed,
                "<link>'s Dest, OnActivation and BorderStyleAlt: not built, because Table 246 \
                 excludes a Link from an FDF file's annotations and the import refuses it",
            ),
            _ => owe(
                self.owed,
                "an element inside an annotation that the XFDF text defines for no annotation \
                 of that subtype",
            ),
        }
    }

    /// `<contents-richtext>`: rich text into Table 172's `/RC`, plain text into `/Contents`.
    fn rich_text(&mut self, child: &Node) {
        if !self.markup() {
            owe(
                self.owed,
                "<contents-richtext> on an annotation that is not markup: Table 172's /RC \
                 belongs to a markup annotation",
            );
            return;
        }
        if child.has_elements {
            let markup = child
                .markup
                .and_then(|(start, end)| self.source.get(start..end))
                .unwrap_or_default()
                .trim();
            self.put("RC", text(markup));
        } else if self.node.child("contents").is_none() {
            self.put("Contents", text(&child.text));
        }
    }

    /// `<data>` and the attributes that describe it, into Table 187's file specification or
    /// Table 188's sound object.
    ///
    /// *Stream encoding* (page 31) states two pairings of `mode` and `encoding`, and *Stream
    /// attributes* (pages 86 and 87) makes `length` and `filter` the stream's own `/Length` and
    /// `/Filter`. So the content is the stream's bytes as it stood in the file — `filtered` names
    /// the escaping of XML's delimiters in an ASCII rendering, `raw` a hexadecimal one — and a
    /// length that disagrees with what was decoded is a damaged element and is refused.
    fn stream(&mut self, data: &Node) {
        let decoded = match (data.attribute("mode"), data.attribute("encoding")) {
            (Some("raw"), Some("hex")) => hexadecimal(&data.text),
            (Some("filtered"), Some("ascii")) => {
                data.text.is_ascii().then(|| data.text.as_bytes().to_vec())
            }
            _ => None,
        };
        let length = data
            .attribute("length")
            .and_then(|length| length.trim().parse::<usize>().ok());
        let Some(decoded) = decoded.filter(|bytes| length.is_none_or(|n| n == bytes.len())) else {
            owe(self.owed, malformed("data"));
            return;
        };
        let Some(left) = self.budget.checked_sub(decoded.len()) else {
            owe(
                self.owed,
                "<data>: more embedded bytes than this reader decodes out of one XFDF file",
            );
            return;
        };
        *self.budget = left;
        let mut dict = Dictionary::new();
        dict.insert(
            Name::new(&b"Length"[..]),
            Object::Integer(i64::try_from(decoded.len()).unwrap_or(i64::MAX)),
        );
        if let Some(filters) = data.attribute("filter").map(str::trim)
            && !filters.is_empty()
        {
            let filters: Vec<Object> = filters.split(',').map(|f| name(f.trim())).collect();
            let filter = match <[Object; 1]>::try_from(filters) {
                Ok([one]) => one,
                Err(many) => Object::Array(many),
            };
            dict.insert(Name::new(&b"Filter"[..]), filter);
        }
        if self.subtype == "Sound" {
            self.sound(dict, decoded);
        } else {
            self.embedded_file(dict, decoded);
        }
    }

    /// Table 305's sound object, from the `<sound>` element's attributes (page 81).
    fn sound(&mut self, mut dict: Dictionary, decoded: Vec<u8>) {
        let node = self.node;
        dict.insert(Name::new(&b"Type"[..]), name("Sound"));
        let Some(rate) = node.attribute("rate").and_then(|rate| numbers(rate, 1, 1)) else {
            owe(
                self.owed,
                "<sound> without a rate: Table 305 requires /R, so there is no sound object",
            );
            return;
        };
        dict.insert(Name::new(&b"R"[..]), real(rate[0]));
        for (attribute, key) in [("bits", "B"), ("channels", "C")] {
            if let Some(value) = node.attribute(attribute) {
                match value.trim().parse::<i64>() {
                    Ok(count) => {
                        dict.insert(Name::new(key.as_bytes()), Object::Integer(count));
                    }
                    Err(_) => owe(self.owed, malformed(attribute)),
                }
            }
        }
        match node.attribute("encoding") {
            None => {}
            Some("raw") => {
                dict.insert(Name::new(&b"E"[..]), name("Raw"));
            }
            Some("signed") => {
                dict.insert(Name::new(&b"E"[..]), name("Signed"));
            }
            Some("mulaw") => {
                dict.insert(Name::new(&b"E"[..]), name("muLaw"));
            }
            // The text lists a fourth, `alaw` (page 82); Table 305 as this tree holds it lists
            // Raw, Signed and muLaw, so there is no name to write for it.
            Some(_) => owe(
                self.owed,
                "encoding on <sound>: a value Table 305's /E does not list",
            ),
        }
        self.put("Sound", stream(dict, decoded));
    }

    /// Table 187's `/FS`: a file specification whose `/EF` holds the file (§7.11.4).
    fn embedded_file(&mut self, mut dict: Dictionary, decoded: Vec<u8>) {
        let node = self.node;
        dict.insert(Name::new(&b"Type"[..]), name("EmbeddedFile"));
        // *Miscellaneous attributes*, page 90: Table 44's `/Subtype`, a media type.
        if let Some(media) = node.attribute("mimetype") {
            dict.insert(Name::new(&b"Subtype"[..]), name(media));
        }
        // *Embedded file parameter attributes*, page 86: Table 45's entries.
        let mut params = Dictionary::new();
        if let Some(size) = node.attribute("size") {
            match size.trim().parse::<i64>() {
                Ok(size) => {
                    params.insert(Name::new(&b"Size"[..]), Object::Integer(size));
                }
                Err(_) => owe(self.owed, malformed("size")),
            }
        }
        for (attribute, key) in [("creation", "CreationDate"), ("modification", "ModDate")] {
            if let Some(date) = node.attribute(attribute) {
                params.insert(Name::new(key.as_bytes()), text(date));
            }
        }
        // Table 45 makes `/CheckSum` "[a] 16-byte string"; the text states its XML form only
        // through the byte-string convention of page 28, and a value that is not sixteen bytes
        // under it is not a checksum.
        if let Some(sum) = node.attribute("checksum") {
            let sum = bytes(sum);
            if sum.len() == 16 {
                params.insert(Name::new(&b"CheckSum"[..]), Object::String(sum.into()));
            } else {
                owe(self.owed, malformed("checksum"));
            }
        }
        if !params.is_empty() {
            dict.insert(Name::new(&b"Params"[..]), Object::Dictionary(params));
        }
        // *File specification attributes*, page 87: Table 43's `/F`. Table 43 recommends a
        // `/UF` wherever there is an `/F`, and the name is the same one in a text string.
        let Some(file) = node.attribute("file") else {
            owe(
                self.owed,
                "<fileattachment> without a file: Table 43 requires a file name, so there is no \
                 file specification",
            );
            return;
        };
        let mut embedded = Dictionary::new();
        embedded.insert(Name::new(&b"F"[..]), stream(dict, decoded));
        let mut spec = Dictionary::new();
        spec.insert(Name::new(&b"Type"[..]), name("Filespec"));
        spec.insert(Name::new(&b"F"[..]), Object::String(bytes(file).into()));
        spec.insert(Name::new(&b"UF"[..]), text(file));
        spec.insert(Name::new(&b"EF"[..]), Object::Dictionary(embedded));
        self.put("FS", Object::Dictionary(spec));
    }

    /// Table 168's `/BS` and Table 169's `/BE`, where anything was stated for them.
    fn borders(&mut self, border_style: Dictionary, mut border_effect: Dictionary) {
        if !border_style.is_empty() {
            self.put("BS", Object::Dictionary(border_style));
        }
        // Table 169: `/I` is "valid only if the value of S is C".
        let cloudy = border_effect
            .get("S")
            .and_then(Object::as_name)
            .is_some_and(|style| style.as_bytes() == b"C");
        if border_effect.get("I").is_some() && !cloudy {
            border_effect.remove("I");
            owe(
                self.owed,
                "intensity without style=\"cloudy\": Table 169 makes /I valid only where /S is /C",
            );
        }
        if !border_effect.is_empty() {
            self.put("BE", Object::Dictionary(border_effect));
        }
    }

    /// The entries two attributes make together: Table 178's `/L` and `/CO`, and the `/LE` of
    /// Tables 178 and 181.
    fn pairs(&mut self) {
        let node = self.node;
        let point = |attribute: &str| node.attribute(attribute).and_then(|v| numbers(v, 2, 2));
        if self.subtype == "Line"
            && let (Some(start), Some(end)) = (point("start"), point("end"))
        {
            self.put("L", reals(&[start[0], start[1], end[0], end[1]]));
        }
        // Table 178's `/CO` defaults to no offset in either direction, so one half stated is the
        // other half zero.
        let offset = |attribute: &str| node.attribute(attribute).and_then(|v| numbers(v, 1, 1));
        let (horizontal, vertical) = (offset("caption-offset-h"), offset("caption-offset-v"));
        if self.subtype == "Line" && (horizontal.is_some() || vertical.is_some()) {
            let at = |half: Option<Vec<f64>>| half.map_or(0.0, |half| half[0]);
            self.put("CO", reals(&[at(horizontal), at(vertical)]));
        }
        // Tables 178 and 181 default each end of `/LE` to `/None`.
        let (head, tail) = (node.attribute("head"), node.attribute("tail"));
        if (self.subtype == "Line" || self.subtype == "PolyLine")
            && (head.is_some() || tail.is_some())
        {
            self.put(
                "LE",
                Object::Array(vec![
                    name(head.unwrap_or("None")),
                    name(tail.unwrap_or("None")),
                ]),
            );
        }
    }

    /// The dictionary, checked against the entries ISO 32000-2 requires of the subtype and the
    /// entries it makes conditional on one another.
    fn required(&mut self) -> Option<Dictionary> {
        // Table 166: `/Rect` is required of every annotation.
        if self.dict.get("Rect").is_none() {
            owe(
                self.owed,
                "<annots>: an annotation with no rect, which Table 166 requires, so it is not \
                 placed",
            );
            return None;
        }
        for (subtype, key, why) in REQUIRED {
            if self.subtype == subtype && self.dict.get(key).is_none() {
                owe(self.owed, why);
                return None;
            }
        }
        let missing: Option<&'static str> = match self.subtype {
            // Table 177: `/DA` is required, and the text's content model agrees (page 50).
            "FreeText" if self.dict.get("DA").is_none() => Some(
                "<freetext> without a defaultappearance: Table 177 requires /DA, so it is not \
                 placed",
            ),
            "FileAttachment" if self.dict.get("FS").is_none() => Some(
                "<fileattachment> without its file: Table 187 requires /FS, so it is not placed",
            ),
            "Sound" if self.dict.get("Sound").is_none() => {
                Some("<sound> without its sound: Table 188 requires /Sound, so it is not placed")
            }
            _ => None,
        };
        if let Some(why) = missing {
            owe(self.owed, why);
            return None;
        }
        // The conditional requirements, each of which leaves the dependent entry out rather than
        // the annotation: the annotation is whole without it.
        for (dependent, on, why) in [
            (
                "State",
                "StateModel",
                "state without statemodel: Table 175 requires /StateModel wherever /State is",
            ),
            (
                "RT",
                "IRT",
                "replyType without inreplyto: Table 172 requires /IRT wherever /RT is",
            ),
            (
                "LLE",
                "LL",
                "leaderExtend without leaderLength: Table 178 requires /LL wherever /LLE is",
            ),
            (
                "OverlayText",
                "DA",
                "overlay-text without defaultappearance: Table 195 requires /DA wherever \
                 /OverlayText is",
            ),
        ] {
            if self.dict.get(dependent).is_some() && self.dict.get(on).is_none() {
                self.dict.remove(dependent);
                owe(self.owed, why);
            }
        }
        Some(std::mem::take(&mut self.dict))
    }
}

/// One of Table 168's three entries.
fn border_style_entry(attribute: &str, value: &str, border_style: &mut Dictionary) -> Option<()> {
    let (key, entry) = match attribute {
        "width" => ("W", real(numbers(value, 1, 1)?[0])),
        "dashes" => ("D", reals(&numbers(value, 1, MAX_NUMBERS)?)),
        // *Border style attributes*, page 85: the text's names for Table 168's five styles.
        _ => (
            "S",
            name(match value {
                "solid" => "S",
                "dash" => "D",
                "bevelled" => "B",
                "inset" => "I",
                "underline" => "U",
                _ => return None,
            }),
        ),
    };
    border_style.insert(Name::new(key.as_bytes()), entry);
    Some(())
}

/// A markup annotation's `/IT`, with the text's two spellings of Table 181's dimension intents
/// written as the table spells them.
///
/// The text lists `polygon-dimension` and `polyline-dimension` (page 79); Table 181 states
/// `PolygonDimension` and `PolyLineDimension`, and says those are the valid values. The two
/// lists name the same intents, so this is a spelling and not a choice.
fn intent(value: &str) -> &str {
    match value {
        "polygon-dimension" => "PolygonDimension",
        "polyline-dimension" => "PolyLineDimension",
        other => other,
    }
}

/// Table 167's flags, from the text's comma-separated names (page 70).
fn flags(value: &str) -> Option<i64> {
    const NAMES: [&str; 9] = [
        "invisible",
        "hidden",
        "print",
        "nozoom",
        "norotate",
        "noview",
        "readonly",
        "locked",
        "togglenoview",
    ];
    let mut flags = 0_i64;
    for stated in value
        .split(',')
        .map(str::trim)
        .filter(|stated| !stated.is_empty())
    {
        // Table 167 numbers its bits from 1 at the low-order end, and the text's nine names are
        // its first nine in order.
        let bit = NAMES.iter().position(|name| *name == stated)?;
        flags |= 1 << bit;
    }
    Some(flags)
}

/// Table 177's and Table 195's `/Q`, from either of the text's spellings of it.
fn justification(value: &str) -> Option<i64> {
    match value.trim() {
        "left" | "0" => Some(0),
        "centered" | "1" => Some(1),
        "right" | "2" => Some(2),
        _ => None,
    }
}

/// The text's `yes` and `no` (pages 76 and 82).
fn yes_no(value: &str) -> Option<bool> {
    match value.trim() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

/// What a colour attribute states.
enum Colour {
    /// The empty string the text gives as the interior colour's default (page 77), which is the
    /// same as an absent `/IC`.
    Transparent,
    /// The three `DeviceRGB` numbers Table 166's `/C` and the subtype tables' `/IC` hold.
    Rgb(Object),
}

/// A colour the text writes as `#RRGGBB` (page 70), or `None` for a value in no such form.
fn colour(value: &str) -> Option<Colour> {
    let value = value.trim();
    if value.is_empty() {
        return Some(Colour::Transparent);
    }
    let digits = value.strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    let components: Option<Vec<f64>> = digits
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16)
                .ok()
                .map(|component| f64::from(component) / 255.0)
        })
        .collect();
    Some(Colour::Rgb(reals(&components?)))
}

/// Comma-separated numbers, between `least` and `most` of them.
fn numbers(value: &str, least: usize, most: usize) -> Option<Vec<f64>> {
    let parsed: Option<Vec<f64>> = value
        .split(',')
        .take(most.saturating_add(1))
        .map(|number| number.trim().parse::<f64>().ok().filter(|n| n.is_finite()))
        .collect();
    parsed.filter(|numbers| (least..=most).contains(&numbers.len()))
}

/// The text's point lists — pairs separated by commas, pairs by semicolons (pages 60 and 67) — as
/// the flat run of alternating coordinates Tables 181 and 185 hold.
fn coordinates(value: &str) -> Option<Vec<f64>> {
    let flat: Option<Vec<f64>> = value
        .split([',', ';'])
        .map(str::trim)
        .filter(|number| !number.is_empty())
        .take(MAX_NUMBERS.saturating_add(1))
        .map(|number| number.parse::<f64>().ok().filter(|n| n.is_finite()))
        .collect();
    flat.filter(|flat| !flat.is_empty() && flat.len() <= MAX_NUMBERS && flat.len() % 2 == 0)
}

/// Hexadecimal stream data, with the line feeds the text allows between its digits (page 31).
fn hexadecimal(value: &str) -> Option<Vec<u8>> {
    let digits: Vec<u32> = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .map(|character| character.to_digit(16))
        .collect::<Option<Vec<u32>>>()?;
    if !digits.len().is_multiple_of(2) {
        return None;
    }
    digits
        .chunks_exact(2)
        .map(|pair| u8::try_from(pair[0].saturating_mul(16).saturating_add(pair[1])).ok())
        .collect()
}

/// A PDF byte string from the text's convention for one (chapter 1, *String encoding
/// conventions*, pages 28 and 29).
///
/// Each character is one ISO Latin-1 byte, and a backslash followed by three octal digits is the
/// byte those digits spell — the escape the convention writes for the control characters. The
/// convention's own enhancement lets a writer translate UTF-8 characters rather than bytes, so a
/// string holding a character Latin-1 has not got is taken as UTF-8.
fn bytes(value: &str) -> Vec<u8> {
    if value.chars().any(|character| u32::from(character) > 0xFF) {
        return value.as_bytes().to_vec();
    }
    let characters: Vec<char> = value.chars().collect();
    let mut out = Vec::with_capacity(characters.len());
    let mut at = 0;
    while at < characters.len() {
        let octal = characters
            .get(at.saturating_add(1)..at.saturating_add(4))
            .filter(|digits| {
                characters[at] == '\\' && digits.iter().all(|digit| ('0'..='7').contains(digit))
            })
            .and_then(|digits| {
                let text: String = digits.iter().collect();
                u8::from_str_radix(&text, 8).ok()
            });
        if let Some(byte) = octal {
            out.push(byte);
            at = at.saturating_add(4);
        } else {
            out.push(u8::try_from(u32::from(characters[at])).unwrap_or(b'?'));
            at = at.saturating_add(1);
        }
    }
    out
}

/// A text string (§7.9.2.2), which is how every entry whose type is one is written.
fn text(value: &str) -> Object {
    Object::String(encode_text_string(value).into())
}

/// A name object.
fn name(value: &str) -> Object {
    Object::Name(Name::new(value.as_bytes()))
}

/// A real number.
fn real(value: f64) -> Object {
    Object::Real(value)
}

/// An array of real numbers.
fn reals(values: &[f64]) -> Object {
    Object::Array(values.iter().copied().map(real).collect())
}

/// A direct stream, which the import makes an indirect object when it writes the annotation.
fn stream(dict: Dictionary, data: Vec<u8>) -> Object {
    Object::Stream(std::sync::Arc::new(pdf_syntax::Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    }))
}

/// Generates, for every attribute name the text defines on an annotation element, the two
/// sentences that name it: one for a value not in the text's form, one for an attribute on an
/// element whose subtype ISO 32000-2 states no entry for it.
macro_rules! sentences {
    ($($attribute:literal),* $(,)?) => {
        /// The sentence naming an attribute whose value is not in the form the text states.
        fn malformed(attribute: &str) -> &'static str {
            match attribute {
                $($attribute => concat!(
                    $attribute,
                    ": a value not in the form the XFDF text states, so its entry is left out"
                ),)*
                _ => "an XFDF value not in the form the XFDF text states, so its entry is left out",
            }
        }

        /// The sentence naming an attribute this subtype's ISO 32000-2 table has no entry for.
        fn unplaced(attribute: &str) -> &'static str {
            match attribute {
                $($attribute => concat!(
                    $attribute,
                    ": on an annotation whose ISO 32000-2 table states no entry for it"
                ),)*
                _ => "an attribute the XFDF text defines for no annotation element",
            }
        }
    };
}

sentences!(
    "rect",
    "color",
    "flags",
    "title",
    "opacity",
    "creationdate",
    "subject",
    "intent",
    "inreplyto",
    "replyType",
    "icon",
    "state",
    "statemodel",
    "coords",
    "start",
    "end",
    "head",
    "tail",
    "leaderLength",
    "leaderExtend",
    "leader-offset",
    "caption",
    "caption-style",
    "caption-offset-h",
    "caption-offset-v",
    "interior-color",
    "fringe",
    "symbol",
    "justification",
    "width",
    "dashes",
    "style",
    "intensity",
    "overlay-text",
    "overlay-text-repeat",
    "open",
    "Highlight",
    "file",
    "size",
    "modification",
    "creation",
    "checksum",
    "mimetype",
    "rate",
    "bits",
    "channels",
    "encoding",
    "vertices",
    "inklist",
    "border",
    "data",
);
