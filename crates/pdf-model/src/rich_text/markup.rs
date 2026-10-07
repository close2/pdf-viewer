//! A rich text string's XHTML, read into paragraphs of styled runs.
//!
//! XFA 3.3 section 27 names the elements a processor supports (*Summary of Supported XHTML and
//! CSS Attributes*, page 1187): the containers `html` and `body`, the paragraph `p`, the line
//! break `br`, the character elements `span`, `b`, `i`, `sub` and `sup`, the hyperlink `a`, and
//! since XFA 3.3 the lists `ol`, `ul` and `li` (*List Support*, page 1209). It states the rule for
//! everything else in the same place: a processor ignores markup it does not understand, and an
//! element it does not recognise is ignored together with its whole content — its own example is
//! XHTML's `head`. [`parse`] is that walk.
//!
//! # White space
//!
//! Chapter 27's *Paragraph* says consecutive white space is compressed to one space by default
//! (page 1194), which is CSS2 section 16.6.1's `normal`: a run of spaces, tabs and line ends is
//! one space, and none is kept at the start or the end of a line. *Retaining Consecutive Spaces*
//! is the one exception (page 1220): inside a span styled `xfa-spacerun:yes`, every space is kept
//! and every no-break space is read as an ordinary one.

use super::style::{self, Block, Character, Declarations, Unapplied};

/// The most markup this module reads, in bytes.
///
/// A field's value or a note's text, not a document; the same budget §12.5.6.2's popup applies to
/// the identical construct, so the two readers of one entry cannot disagree about whether it is
/// too long to read.
pub(crate) const MAX_MARKUP: usize = 64 << 10;

/// The deepest element nesting read.
///
/// Chapter 27's elements nest a few levels deep — a span in a paragraph in a body in an `html`,
/// and a list in a list — so a deeper document is not one a producer wrote for this grammar, and
/// the bound keeps the walk's stack the size of the markup's own sense rather than of its bytes.
const MAX_DEPTH: usize = 64;

/// How many pieces one string may become.
///
/// Bounds the one place the walk appends without consuming much input: an empty `br` is five
/// bytes and one piece, so a string of nothing else is the worst case, and 16 384 is far past
/// what any field or note shows.
const MAX_PIECES: usize = 1 << 14;

/// A rich text string, read.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RichText {
    /// The block properties the `body`, the `html` and the default style string state, whose
    /// margins frame the whole text.
    pub(crate) body: Block,
    /// The paragraphs, in order.
    pub(crate) paragraphs: Vec<Paragraph>,
    /// What chapter 27 names, the string states, and this tree does not carry out.
    pub(crate) unapplied: Unapplied,
}

impl RichText {
    /// The characters, as §12.5.6.2 asks a plain text to spell them: paragraphs separated by a
    /// carriage return, and a line break one too. A list item's tag is not among them: it is
    /// generated, not stated.
    pub(crate) fn text(&self) -> String {
        let mut out = String::new();
        for (index, paragraph) in self.paragraphs.iter().enumerate() {
            if index > 0 {
                out.push('\r');
            }
            let last = paragraph.pieces.len().saturating_sub(1);
            for (at, piece) in paragraph.pieces.iter().enumerate() {
                match piece {
                    Piece::Text(text, _) => out.push_str(text),
                    // A paragraph holding nothing but a break is an empty line, which the
                    // separator above already ends.
                    Piece::Break if at < last => out.push('\r'),
                    Piece::Break | Piece::Tab(_) => {}
                }
            }
        }
        out
    }
}

/// One paragraph: chapter 27's `p`, or the text a container holds outside one.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Paragraph {
    /// Its paragraph properties.
    pub(crate) block: Block,
    /// The character properties of the paragraph itself, which size a line holding no text.
    pub(crate) strut: Character,
    /// What it holds.
    pub(crate) pieces: Vec<Piece>,
    /// The item tag, where the paragraph is a list item (page 1209).
    pub(crate) tag: Option<Tag>,
    /// How far a list indents it.
    pub(crate) list: ListIndent,
}

/// How far a list indents a paragraph, from chapter 27's *List Layout* (page 1218).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListIndent {
    /// Not in a list.
    None,
    /// In this many nested lists: the single list indent each.
    Levels(u16),
    /// A list item outside any list, which the chapter gives the least list indent it can have:
    /// room for its tag and the tag gap, and no more.
    Minimal,
}

/// A list item's tag and the style it is set in.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tag {
    /// The characters: a number with its suffix, or a bullet.
    pub(crate) text: String,
    /// The item's own style.
    pub(crate) style: Character,
}

/// One piece of a paragraph.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Piece {
    /// Characters set in one style, white space already resolved.
    Text(String, Character),
    /// XHTML's `br`: the line ends here.
    Break,
    /// `xfa-tab-count`: advance by this many tab stops (*Tab Stops*, page 1206). It states no
    /// character of its own, so [`RichText::text`] spells nothing for it.
    Tab(u16),
}

/// Reads a rich text string, with a default style string applied beneath it.
///
/// `None` where the markup is not well formed, where it opens no element at all, or where it is
/// longer than [`MAX_MARKUP`]: a value that is not a rich text string is drawn as the characters
/// it is (ADR 1197), and a malformed one may not erase what the file states.
pub(crate) fn parse(
    markup: &str,
    default_style: Option<&str>,
    root: Character,
) -> Option<RichText> {
    if markup.len() > MAX_MARKUP {
        return None;
    }
    let mut walk = Walk::new(default_style, root);
    let mut pending: Option<Pending> = None;
    // Chapter 27 makes the `html` and `body` containers optional (page 1189), so a string may be
    // a run of paragraphs with no one element around them, which XML would refuse as a document.
    // It is read inside a `body` of the walk's own — a container that does not appear in the
    // output, which is what the chapter says the real one is — after the XML declaration a
    // producer may open with, which may not stand inside an element.
    let inner = markup
        .trim_start()
        .strip_prefix("<?xml")
        .and_then(|rest| rest.split_once("?>"))
        .map_or(markup, |(_, rest)| rest);
    let wrapped = format!("<body>{inner}</body>");
    // The wrapper is an element the string did not open, so it does not count as one.
    let mut elements = 0_usize;
    let mut wrapper_seen = false;
    for token in xmlparser::Tokenizer::from(wrapped.as_str()) {
        let token = token.ok()?;
        match token {
            xmlparser::Token::ElementStart { prefix, local, .. } => {
                if let Some(open) = pending.take() {
                    walk.open(&open, false);
                }
                if wrapper_seen {
                    elements = elements.saturating_add(1);
                }
                wrapper_seen = true;
                pending = Some(Pending {
                    name: local.as_str().to_ascii_lowercase(),
                    xhtml: prefix.as_str().is_empty(),
                    attributes: Vec::new(),
                });
            }
            xmlparser::Token::Attribute {
                prefix,
                local,
                value,
                ..
            } => {
                if let Some(open) = pending.as_mut() {
                    open.attributes.push((
                        prefix.as_str().to_owned(),
                        local.as_str().to_ascii_lowercase(),
                        unescaped(value.as_str()),
                    ));
                }
            }
            xmlparser::Token::ElementEnd { end, .. } => match end {
                xmlparser::ElementEnd::Open => {
                    if let Some(open) = pending.take() {
                        walk.open(&open, false);
                    }
                }
                xmlparser::ElementEnd::Empty => {
                    if let Some(open) = pending.take() {
                        walk.open(&open, true);
                    }
                }
                xmlparser::ElementEnd::Close(..) => walk.close(),
            },
            xmlparser::Token::Text { text } | xmlparser::Token::Cdata { text, .. } => {
                walk.text(&unescaped(text.as_str()));
            }
            _ => {}
        }
    }
    if elements == 0 {
        return None;
    }
    Some(walk.finish())
}

/// The text of a node or an attribute with XML's references resolved.
///
/// XML's five, and its numeric references, through the reader `crate::xmp` already has for the
/// identical grammar; and XHTML's `&nbsp;`, the one entity of XHTML's own that chapter 27's
/// examples lean on — its *Retaining Consecutive Spaces* is about exactly that character — and
/// which an XML reader without XHTML's document type would otherwise leave as six characters.
fn unescaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    crate::xmp::unescape(&text.replace("&nbsp;", "\u{a0}"), &mut out);
    out
}

/// An element whose start has been read and whose attributes are still arriving.
struct Pending {
    name: String,
    /// Whether it is in XHTML's namespace, which is the default one; a prefixed element is some
    /// other grammar's and is one chapter 27 does not name.
    xhtml: bool,
    /// `(prefix, local name, value)`.
    attributes: Vec<(String, String, String)>,
}

impl Pending {
    fn attribute(&self, prefix: &str, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(stated, local, _)| stated == prefix && local == name)
            .map(|(_, _, value)| value.as_str())
    }
}

/// What an open element is, for what its end undoes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `html` or `body`.
    Container,
    /// `p`.
    Paragraph,
    /// `span`, `b`, `i`, `sub`, `sup`, `a`.
    Character,
    /// `ol` or `ul`.
    List,
    /// `li`.
    Item,
    /// Anything chapter 27 does not name, whose whole content is ignored.
    Ignored,
}

/// One open element.
struct Frame {
    kind: Kind,
    character: Character,
    block: Block,
}

/// One open list.
struct List {
    style: ListStyle,
    /// The number the next item takes.
    next: i64,
    /// `xfa-list-item-tags:compound`, inherited from the enclosing list where unstated.
    compound: bool,
    /// The uncompounded tags of the items enclosing this list, outermost first.
    ancestry: Vec<String>,
    /// The uncompounded tag of the item open in this list, for a list nested inside it.
    current: Option<String>,
}

/// The walk's state.
struct Walk {
    stack: Vec<Frame>,
    lists: Vec<List>,
    /// Depth of an element being ignored with its content; zero when reading.
    ignoring: usize,
    paragraphs: Vec<Paragraph>,
    current: Option<Paragraph>,
    /// Whether the last character placed was a collapsible space, or nothing has been placed on
    /// the line yet — in both of which a space is not added.
    after_space: bool,
    pieces: usize,
    unapplied: Unapplied,
    body: Block,
    /// Whether XFA 3.3's list elements are read: `xfa:spec` names 3.3 or later, or nothing.
    lists_respected: bool,
    /// The `li` whose first paragraph is still to take its tag.
    tag_waiting: Option<(Tag, ListIndent)>,
}

impl Walk {
    fn new(default_style: Option<&str>, root: Character) -> Self {
        let mut character = root;
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
        // A count is an event at an element, and a default style string is not one.
        character.tab_count = 0;
        Self {
            body: block.clone(),
            stack: vec![Frame {
                kind: Kind::Container,
                character,
                block,
            }],
            lists: Vec::new(),
            ignoring: 0,
            paragraphs: Vec::new(),
            current: None,
            after_space: true,
            pieces: 0,
            unapplied,
            lists_respected: true,
            tag_waiting: None,
        }
    }

    fn top(&self) -> (Character, Block) {
        self.stack.last().map_or_else(
            || (Character::root(), Block::root()),
            |frame| (frame.character.clone(), frame.block.clone()),
        )
    }

    fn open(&mut self, element: &Pending, empty: bool) {
        if self.ignoring > 0 || self.stack.len() >= MAX_DEPTH {
            if !empty {
                self.ignoring = self.ignoring.saturating_add(1);
            }
            return;
        }
        let (mut character, parent_block) = self.top();
        let mut block = parent_block.inherited();
        let kind = self.kind_of(element);
        if kind == Kind::Ignored {
            if !empty {
                self.ignoring = 1;
            }
            return;
        }
        match element.name.as_str() {
            "b" => character.weight = 700,
            "i" => character.italic = true,
            // Chapter 27's *Subscript* and *Superscript* (pages 1204 and 1205): the baseline moves
            // by 15% and 31% of the current font height, down and up, and the new height is 66%
            // of the current one.
            "sub" => {
                character.rise = character.rise.plus(character.size.times(-0.15));
                character.size = character.size.times(0.66);
            }
            "sup" => {
                character.rise = character.rise.plus(character.size.times(0.31));
                character.size = character.size.times(0.66);
            }
            "a" => {
                // Chapter 27's *Hyperlink Support* recommends blue text with a single blue
                // underline for the link (page 1189), and that recommendation is taken; the
                // element's own style, applied below, outranks it as an author's style does.
                // Following the link is not: §12.7.4.3 brings in XFA 3.3 for a field's
                // "formatting information", and what a click on a widget does is §12.7's and
                // §12.6's, so the link is drawn as the formatting it is (ADR 1660).
                character.colour = Some([0.0, 0.0, 1.0]);
                character.underline = style::Underline::Single;
            }
            _ => {}
        }
        if let Some(stated) = element.attribute("", "style") {
            style::apply(
                &Declarations::parse(stated),
                &mut character,
                &mut block,
                &mut self.unapplied,
            );
        }
        // *Tab Stops* (page 1206): the element advances by its count of stops where it opens, and
        // nothing inside it inherits the count. The chapter recommends the element be empty and
        // lets a processor discard what it holds; what it holds is kept, after the advance.
        if character.tab_count > 0 {
            let count = std::mem::take(&mut character.tab_count);
            if self.current.is_none() {
                self.start_paragraph(block.clone(), character.clone());
            }
            self.push(Piece::Tab(count));
        }
        if element.attribute("xfa", "embed").is_some() {
            // *Embedded Object Specifications* (page 1222): a SOM expression names a node of a
            // form's XFA object model, which a PDF field does not have, and a URI is data from
            // outside the file, which a renderer with no network does not fetch. Neither is
            // formatting, which is what §12.7.4.3 brings XFA 3.3 in for, so the span is drawn as
            // what it holds — a choice — and the text it would have inserted is said (ADR 1660).
            self.unapplied.note("an embedded object (xfa:embed)");
        }
        match kind {
            Kind::Container => {
                if element.name == "body"
                    && let Some(spec) = element.attribute("xfa", "spec")
                {
                    self.lists_respected = version_at_least(spec, &[3, 3]);
                }
                // The body's margins frame the whole text; its inherited properties reach every
                // paragraph through the stack.
                if self.stack.len() <= 2 {
                    for (into, from) in self.body.margins.iter_mut().zip(block.margins) {
                        *into = into.plus(from);
                    }
                    self.body.align = block.align;
                    self.body.valign = block.valign;
                }
                block.margins = Block::root().margins;
            }
            Kind::Paragraph => {
                // Chapter 27's *Paragraph*: `p` elements cannot nest (page 1194), so one starting
                // ends whatever paragraph is open.
                self.end_paragraph();
                self.start_paragraph(block.clone(), character.clone());
            }
            Kind::List => self.open_list(element),
            Kind::Item => self.open_item(element, &character),
            Kind::Character | Kind::Ignored => {}
        }
        if element.name == "br" {
            self.line_break(&character, &block);
        }
        if !empty {
            self.stack.push(Frame {
                kind,
                character,
                block,
            });
        } else if kind == Kind::Paragraph {
            self.end_paragraph();
        } else if kind == Kind::List {
            self.lists.pop();
        }
    }

    fn kind_of(&self, element: &Pending) -> Kind {
        if !element.xhtml {
            return Kind::Ignored;
        }
        match element.name.as_str() {
            "html" | "body" => Kind::Container,
            "p" => Kind::Paragraph,
            "span" | "b" | "i" | "sub" | "sup" | "a" | "br" => Kind::Character,
            // *Legacy Handling of List Content* (page 1217): list content is respected only by a
            // grammar later than 3.2, and otherwise discarded with the elements.
            "ol" | "ul" if self.lists_respected => Kind::List,
            "li" if self.lists_respected => Kind::Item,
            _ => Kind::Ignored,
        }
    }

    fn close(&mut self) {
        if self.ignoring > 0 {
            self.ignoring = self.ignoring.saturating_sub(1);
            return;
        }
        let Some(frame) = self.stack.pop() else {
            return;
        };
        match frame.kind {
            Kind::Paragraph | Kind::Item => self.end_paragraph(),
            Kind::List => {
                self.end_paragraph();
                self.lists.pop();
            }
            Kind::Container | Kind::Character | Kind::Ignored => {}
        }
        if self.stack.is_empty() {
            // The root frame is never closed by markup; a stray end tag cannot remove it.
            self.stack.push(frame);
        }
    }

    fn text(&mut self, text: &str) {
        if self.ignoring > 0 || self.pieces >= MAX_PIECES {
            return;
        }
        let (character, block) = self.top();
        let mut resolved = String::with_capacity(text.len());
        if character.spacerun {
            // *Retaining Consecutive Spaces*: each space is kept, and each no-break space is a
            // space (page 1220). Line ends inside the span are still white space to compress.
            for letter in text.chars() {
                match letter {
                    ' ' | '\u{a0}' => resolved.push(' '),
                    '\t' | '\r' | '\n' => {
                        if !self.after_space {
                            resolved.push(' ');
                            self.after_space = true;
                        }
                        continue;
                    }
                    other => resolved.push(other),
                }
                self.after_space = false;
            }
        } else {
            for letter in text.chars() {
                if matches!(letter, ' ' | '\t' | '\r' | '\n') {
                    if !self.after_space {
                        resolved.push(' ');
                        self.after_space = true;
                    }
                } else {
                    resolved.push(letter);
                    self.after_space = false;
                }
            }
        }
        if resolved.is_empty() {
            return;
        }
        if self.current.is_none() {
            // Text outside a `p`: a paragraph of the container's own properties. White space
            // alone between two blocks starts nothing.
            if resolved.trim().is_empty() {
                self.after_space = true;
                return;
            }
            let trimmed = resolved.trim_start().to_owned();
            self.start_paragraph(block, character.clone());
            self.push(Piece::Text(trimmed, character));
            return;
        }
        self.push(Piece::Text(resolved, character));
    }

    fn line_break(&mut self, character: &Character, block: &Block) {
        if self.current.is_none() {
            self.start_paragraph(block.clone(), character.clone());
        }
        self.trim_end();
        self.push(Piece::Break);
        self.after_space = true;
    }

    fn push(&mut self, piece: Piece) {
        if let Some(paragraph) = self.current.as_mut() {
            // Two runs in one style side by side are one run, which keeps a line's pieces as few
            // as the styles on it.
            if let (Piece::Text(text, style), Some(Piece::Text(last, last_style))) =
                (&piece, paragraph.pieces.last_mut())
                && style == last_style
            {
                last.push_str(text);
                return;
            }
            paragraph.pieces.push(piece);
            self.pieces = self.pieces.saturating_add(1);
        }
    }

    fn start_paragraph(&mut self, block: Block, strut: Character) {
        let list = self
            .tag_waiting
            .as_ref()
            .map_or_else(|| self.list_indent(), |(_, indent)| *indent);
        let tag = self.tag_waiting.take().map(|(tag, _)| tag);
        self.current = Some(Paragraph {
            block,
            strut,
            pieces: Vec::new(),
            tag,
            list,
        });
        self.after_space = true;
    }

    fn list_indent(&self) -> ListIndent {
        match u16::try_from(self.lists.len()) {
            Ok(0) => ListIndent::None,
            Ok(levels) => ListIndent::Levels(levels),
            Err(_) => ListIndent::Levels(u16::MAX),
        }
    }

    /// Drops trailing collapsible spaces: CSS2 section 16.6.1 keeps none at a line's end.
    fn trim_end(&mut self) {
        let Some(paragraph) = self.current.as_mut() else {
            return;
        };
        while let Some(Piece::Text(text, style)) = paragraph.pieces.last_mut() {
            if style.spacerun {
                break;
            }
            let kept = text.trim_end_matches(' ').len();
            text.truncate(kept);
            if text.is_empty() {
                paragraph.pieces.pop();
            } else {
                break;
            }
        }
    }

    fn end_paragraph(&mut self) {
        self.trim_end();
        if let Some(mut paragraph) = self.current.take() {
            // A line break at the very end ends a line that has already ended (CSS2 section
            // 9.4.2's line boxes): chapter 27's Example 27.2 closes every line with one and shows
            // three lines, not four.
            if paragraph.pieces.len() > 1 && paragraph.pieces.last() == Some(&Piece::Break) {
                paragraph.pieces.pop();
            }
            if !paragraph.pieces.is_empty() || paragraph.tag.is_some() {
                self.paragraphs.push(paragraph);
            }
        }
        self.after_space = true;
    }

    fn open_list(&mut self, element: &Pending) {
        self.end_paragraph();
        let ordered = element.name == "ol";
        let declarations = element
            .attribute("", "style")
            .map(Declarations::parse)
            .unwrap_or_default();
        let stated = |name: &str| {
            declarations
                .0
                .iter()
                .rev()
                .find(|declaration| declaration.name == name)
                .map(|declaration| declaration.value.to_ascii_lowercase())
        };
        // The nesting depth counted from zero, which is how *List Support* numbers the levels its
        // default bullets are given by: Example 27.34's diagram draws a list nested one deep with
        // a disc (page 1219).
        let level = self.lists.len();
        let style = stated("list-style-type")
            .and_then(|value| ListStyle::named(&value))
            .or_else(|| element.attribute("", "type").and_then(ListStyle::html_type))
            .unwrap_or(if ordered {
                ListStyle::Decimal
            } else {
                // *List Support*: disc at levels zero and one, circle at two, square beyond
                // (page 1212).
                match level {
                    0 | 1 => ListStyle::Disc,
                    2 => ListStyle::Circle,
                    _ => ListStyle::Square,
                }
            });
        let compound = match stated("xfa-list-item-tags").as_deref() {
            Some("compound") => true,
            Some("simple") => false,
            _ => self.lists.last().is_some_and(|list| list.compound),
        };
        let start = element
            .attribute("", "start")
            .and_then(|start| start.trim().parse::<i64>().ok())
            .unwrap_or(1);
        let ancestry = self
            .lists
            .last()
            .map(|list| {
                let mut ancestry = list.ancestry.clone();
                ancestry.extend(list.current.clone());
                ancestry
            })
            .unwrap_or_default();
        self.lists.push(List {
            style,
            next: start,
            compound,
            ancestry,
            current: None,
        });
    }

    fn open_item(&mut self, element: &Pending, character: &Character) {
        self.end_paragraph();
        if self.lists.is_empty() {
            // *List in Combination of Other XHTML* (page 1217): an item outside any list is
            // read as if an unordered list held it, with minimal list indent.
            let tag = Tag {
                text: ListStyle::Disc.tag(1).unwrap_or_default(),
                style: character.clone(),
            };
            self.tag_waiting = Some((tag, ListIndent::Minimal));
            return;
        }
        let indent = self.list_indent();
        let Some(list) = self.lists.last_mut() else {
            return;
        };
        if let Some(value) = element
            .attribute("", "value")
            .and_then(|value| value.trim().parse::<i64>().ok())
        {
            list.next = value;
        }
        let number = list.next;
        list.next = list.next.saturating_add(1);
        let plain = list.style.tag(number).unwrap_or_default();
        let bare = list.style.bare(number);
        list.current.clone_from(&bare);
        let text = match (list.compound && list.style.numbered(), bare) {
            (true, Some(bare)) => {
                let mut parts = list.ancestry.clone();
                parts.push(bare);
                format!("{}.", parts.join("."))
            }
            _ => plain,
        };
        self.tag_waiting = Some((
            Tag {
                text,
                style: character.clone(),
            },
            indent,
        ));
    }

    fn finish(mut self) -> RichText {
        self.end_paragraph();
        if let Some((tag, list)) = self.tag_waiting.take() {
            // An empty item still shows its tag.
            let (character, block) = self.top();
            self.paragraphs.push(Paragraph {
                block,
                strut: character,
                pieces: Vec::new(),
                tag: Some(tag),
                list,
            });
        }
        RichText {
            body: self.body,
            paragraphs: self.paragraphs,
            unapplied: self.unapplied,
        }
    }
}

/// Whether a version identifier is at least `floor`, compared field by field with the shorter
/// padded with zeros — *Version Specification*'s rule for `xfa:APIVersion` (page 1223), applied
/// to `xfa:spec`, which the same section gives a version identifier as well.
fn version_at_least(stated: &str, floor: &[u64]) -> bool {
    let fields: Vec<u64> = stated
        .trim()
        .split('.')
        .map(|field| field.trim().parse::<u64>().unwrap_or(0))
        .collect();
    let length = fields.len().max(floor.len());
    for index in 0..length {
        let ours = fields.get(index).copied().unwrap_or(0);
        let theirs = floor.get(index).copied().unwrap_or(0);
        if ours != theirs {
            return ours > theirs;
        }
    }
    true
}

/// *List Support*'s alphabetic Kana and Hangul types (pages 1212 and 1213), whose letters the
/// chapter leaves to [CSS3-Lists]: the four Kana sets are *CSS Lists and Counters Module Level 3*'s
/// (W3C Working Draft, 24 May 2011, section 9.3), and the two Hangul sets that draft does not
/// name are those of *CSS3 module: Lists* (W3C Working Draft, 7 November 2002, section 4.4), each
/// in its own order — <https://www.w3.org/TR/2011/WD-css3-lists-20110524/> and
/// <https://www.w3.org/TR/2002/WD-css3-lists-20021107/>, which are not held. ADR 1660.
const HIRAGANA: &[char] = &[
    '\u{3042}', '\u{3044}', '\u{3046}', '\u{3048}', '\u{304a}', '\u{304b}', '\u{304d}', '\u{304f}',
    '\u{3051}', '\u{3053}', '\u{3055}', '\u{3057}', '\u{3059}', '\u{305b}', '\u{305d}', '\u{305f}',
    '\u{3061}', '\u{3064}', '\u{3066}', '\u{3068}', '\u{306a}', '\u{306b}', '\u{306c}', '\u{306d}',
    '\u{306e}', '\u{306f}', '\u{3072}', '\u{3075}', '\u{3078}', '\u{307b}', '\u{307e}', '\u{307f}',
    '\u{3080}', '\u{3081}', '\u{3082}', '\u{3084}', '\u{3086}', '\u{3088}', '\u{3089}', '\u{308a}',
    '\u{308b}', '\u{308c}', '\u{308d}', '\u{308f}', '\u{3092}', '\u{3093}',
];
/// The *iroha* order of the same syllables, which adds ゐ and ゑ.
const HIRAGANA_IROHA: &[char] = &[
    '\u{3044}', '\u{308d}', '\u{306f}', '\u{306b}', '\u{307b}', '\u{3078}', '\u{3068}', '\u{3061}',
    '\u{308a}', '\u{306c}', '\u{308b}', '\u{3092}', '\u{308f}', '\u{304b}', '\u{3088}', '\u{305f}',
    '\u{308c}', '\u{305d}', '\u{3064}', '\u{306d}', '\u{306a}', '\u{3089}', '\u{3080}', '\u{3046}',
    '\u{3090}', '\u{306e}', '\u{304a}', '\u{304f}', '\u{3084}', '\u{307e}', '\u{3051}', '\u{3075}',
    '\u{3053}', '\u{3048}', '\u{3066}', '\u{3042}', '\u{3055}', '\u{304d}', '\u{3086}', '\u{3081}',
    '\u{307f}', '\u{3057}', '\u{3091}', '\u{3072}', '\u{3082}', '\u{305b}', '\u{3059}', '\u{3093}',
];
/// [`HIRAGANA`] in Katakana.
const KATAKANA: &[char] = &[
    '\u{30a2}', '\u{30a4}', '\u{30a6}', '\u{30a8}', '\u{30aa}', '\u{30ab}', '\u{30ad}', '\u{30af}',
    '\u{30b1}', '\u{30b3}', '\u{30b5}', '\u{30b7}', '\u{30b9}', '\u{30bb}', '\u{30bd}', '\u{30bf}',
    '\u{30c1}', '\u{30c4}', '\u{30c6}', '\u{30c8}', '\u{30ca}', '\u{30cb}', '\u{30cc}', '\u{30cd}',
    '\u{30ce}', '\u{30cf}', '\u{30d2}', '\u{30d5}', '\u{30d8}', '\u{30db}', '\u{30de}', '\u{30df}',
    '\u{30e0}', '\u{30e1}', '\u{30e2}', '\u{30e4}', '\u{30e6}', '\u{30e8}', '\u{30e9}', '\u{30ea}',
    '\u{30eb}', '\u{30ec}', '\u{30ed}', '\u{30ef}', '\u{30f2}', '\u{30f3}',
];
/// [`HIRAGANA_IROHA`] in Katakana.
const KATAKANA_IROHA: &[char] = &[
    '\u{30a4}', '\u{30ed}', '\u{30cf}', '\u{30cb}', '\u{30db}', '\u{30d8}', '\u{30c8}', '\u{30c1}',
    '\u{30ea}', '\u{30cc}', '\u{30eb}', '\u{30f2}', '\u{30ef}', '\u{30ab}', '\u{30e8}', '\u{30bf}',
    '\u{30ec}', '\u{30bd}', '\u{30c4}', '\u{30cd}', '\u{30ca}', '\u{30e9}', '\u{30e0}', '\u{30a6}',
    '\u{30f0}', '\u{30ce}', '\u{30aa}', '\u{30af}', '\u{30e4}', '\u{30de}', '\u{30b1}', '\u{30d5}',
    '\u{30b3}', '\u{30a8}', '\u{30c6}', '\u{30a2}', '\u{30b5}', '\u{30ad}', '\u{30e6}', '\u{30e1}',
    '\u{30df}', '\u{30b7}', '\u{30f1}', '\u{30d2}', '\u{30e2}', '\u{30bb}', '\u{30b9}', '\u{30f3}',
];
/// The fourteen syllables 가 to 하.
const HANGUL: &[char] = &[
    '\u{ac00}', '\u{b098}', '\u{b2e4}', '\u{b77c}', '\u{b9c8}', '\u{bc14}', '\u{c0ac}', '\u{c544}',
    '\u{c790}', '\u{cc28}', '\u{ce74}', '\u{d0c0}', '\u{d30c}', '\u{d558}',
];
/// The fourteen consonants ㄱ to ㅎ.
const HANGUL_CONSONANT: &[char] = &[
    '\u{3131}', '\u{3134}', '\u{3137}', '\u{3139}', '\u{3141}', '\u{3142}', '\u{3145}', '\u{3147}',
    '\u{3148}', '\u{314a}', '\u{314b}', '\u{314c}', '\u{314d}', '\u{314e}',
];

/// The 2011 draft's `cjk-decimal`, which section 10.3 names as every CJK longhand style's
/// fallback outside its range: the ideographic zero and the nine digits, written positionally.
const CJK_DECIMAL: &[char] = &[
    '\u{3007}', '\u{4e00}', '\u{4e8c}', '\u{4e09}', '\u{56db}', '\u{4e94}', '\u{516d}', '\u{4e03}',
    '\u{516b}', '\u{4e5d}',
];

/// The 2011 draft's additive `hebrew` (section 9.6), defined from 1 up, whose 15 to 19 are
/// stated as their own pairs so that 15 and 16 take the forms the draft's comment names.
const HEBREW: &[(i64, &str)] = &[
    (400, "\u{5ea}"),
    (300, "\u{5e9}"),
    (200, "\u{5e8}"),
    (100, "\u{5e7}"),
    (90, "\u{5e6}"),
    (80, "\u{5e4}"),
    (70, "\u{5e2}"),
    (60, "\u{5e1}"),
    (50, "\u{5e0}"),
    (40, "\u{5de}"),
    (30, "\u{5dc}"),
    (20, "\u{5db}"),
    (19, "\u{5d9}\u{5d8}"),
    (18, "\u{5d9}\u{5d7}"),
    (17, "\u{5d9}\u{5d6}"),
    (16, "\u{5d8}\u{5d6}"),
    (15, "\u{5d8}\u{5d5}"),
    (10, "\u{5d9}"),
    (9, "\u{5d8}"),
    (8, "\u{5d7}"),
    (7, "\u{5d6}"),
    (6, "\u{5d5}"),
    (5, "\u{5d4}"),
    (4, "\u{5d3}"),
    (3, "\u{5d2}"),
    (2, "\u{5d1}"),
    (1, "\u{5d0}"),
];

/// The 2011 draft's additive `japanese-informal` (section 9.6), defined from 0 to 9999.
const JAPANESE_INFORMAL: &[(i64, &str)] = &[
    (9000, "\u{4e5d}\u{5343}"),
    (8000, "\u{516b}\u{5343}"),
    (7000, "\u{4e03}\u{5343}"),
    (6000, "\u{516d}\u{5343}"),
    (5000, "\u{4e94}\u{5343}"),
    (4000, "\u{56db}\u{5343}"),
    (3000, "\u{4e09}\u{5343}"),
    (2000, "\u{4e8c}\u{5343}"),
    (1000, "\u{5343}"),
    (900, "\u{4e5d}\u{767e}"),
    (800, "\u{516b}\u{767e}"),
    (700, "\u{4e03}\u{767e}"),
    (600, "\u{516d}\u{767e}"),
    (500, "\u{4e94}\u{767e}"),
    (400, "\u{56db}\u{767e}"),
    (300, "\u{4e09}\u{767e}"),
    (200, "\u{4e8c}\u{767e}"),
    (100, "\u{767e}"),
    (90, "\u{4e5d}\u{5341}"),
    (80, "\u{516b}\u{5341}"),
    (70, "\u{4e03}\u{5341}"),
    (60, "\u{516d}\u{5341}"),
    (50, "\u{4e94}\u{5341}"),
    (40, "\u{56db}\u{5341}"),
    (30, "\u{4e09}\u{5341}"),
    (20, "\u{4e8c}\u{5341}"),
    (10, "\u{5341}"),
    (9, "\u{4e5d}"),
    (8, "\u{516b}"),
    (7, "\u{4e03}"),
    (6, "\u{516d}"),
    (5, "\u{4e94}"),
    (4, "\u{56db}"),
    (3, "\u{4e09}"),
    (2, "\u{4e8c}"),
    (1, "\u{4e00}"),
    (0, "\u{3007}"),
];

/// The 2011 draft's additive `japanese-formal` (section 9.6), defined from 0 to 9999.
const JAPANESE_FORMAL: &[(i64, &str)] = &[
    (9000, "\u{4e5d}\u{9621}"),
    (8000, "\u{516b}\u{9621}"),
    (7000, "\u{4e03}\u{9621}"),
    (6000, "\u{516d}\u{9621}"),
    (5000, "\u{4f0d}\u{9621}"),
    (4000, "\u{56db}\u{9621}"),
    (3000, "\u{53c2}\u{9621}"),
    (2000, "\u{5f10}\u{9621}"),
    (1000, "\u{58f1}\u{9621}"),
    (900, "\u{4e5d}\u{767e}"),
    (800, "\u{516b}\u{767e}"),
    (700, "\u{4e03}\u{767e}"),
    (600, "\u{516d}\u{767e}"),
    (500, "\u{4f0d}\u{767e}"),
    (400, "\u{56db}\u{767e}"),
    (300, "\u{53c2}\u{767e}"),
    (200, "\u{5f10}\u{767e}"),
    (100, "\u{58f1}\u{767e}"),
    (90, "\u{4e5d}\u{62fe}"),
    (80, "\u{516b}\u{62fe}"),
    (70, "\u{4e03}\u{62fe}"),
    (60, "\u{516d}\u{62fe}"),
    (50, "\u{4f0d}\u{62fe}"),
    (40, "\u{56db}\u{62fe}"),
    (30, "\u{53c2}\u{62fe}"),
    (20, "\u{5f10}\u{62fe}"),
    (10, "\u{58f1}\u{62fe}"),
    (9, "\u{4e5d}"),
    (8, "\u{516b}"),
    (7, "\u{4e03}"),
    (6, "\u{516d}"),
    (5, "\u{4f0d}"),
    (4, "\u{56db}"),
    (3, "\u{53c2}"),
    (2, "\u{5f10}"),
    (1, "\u{58f1}"),
    (0, "\u{96f6}"),
];

/// One of section 10.3's four Chinese longhand character sets: the ten digits from zero, the
/// tens, hundreds and thousands markers, and the negative sign, from the section's table.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Longhand {
    digits: [char; 10],
    markers: [char; 3],
    negative: char,
    /// Whether this is an informal style, whose 10 to 19 drop the tens digit.
    informal: bool,
}

/// `simp-chinese-informal`.
const SIMP_CHINESE_INFORMAL: Longhand = Longhand {
    digits: [
        '\u{96f6}', '\u{4e00}', '\u{4e8c}', '\u{4e09}', '\u{56db}', '\u{4e94}', '\u{516d}',
        '\u{4e03}', '\u{516b}', '\u{4e5d}',
    ],
    markers: ['\u{5341}', '\u{767e}', '\u{5343}'],
    negative: '\u{8d1f}',
    informal: true,
};
/// `simp-chinese-formal`.
const SIMP_CHINESE_FORMAL: Longhand = Longhand {
    digits: [
        '\u{96f6}', '\u{58f9}', '\u{8d30}', '\u{53c1}', '\u{8086}', '\u{4f0d}', '\u{9646}',
        '\u{67d2}', '\u{634c}', '\u{7396}',
    ],
    markers: ['\u{62fe}', '\u{4f70}', '\u{4edf}'],
    // The table prints 負 beside U+8D1F in this column; the code point is taken, the one the
    // simplified informal column states too.
    negative: '\u{8d1f}',
    informal: false,
};
/// `trad-chinese-informal`.
const TRAD_CHINESE_INFORMAL: Longhand = Longhand {
    digits: [
        '\u{96f6}', '\u{4e00}', '\u{4e8c}', '\u{4e09}', '\u{56db}', '\u{4e94}', '\u{516d}',
        '\u{4e03}', '\u{516b}', '\u{4e5d}',
    ],
    markers: ['\u{5341}', '\u{767e}', '\u{5343}'],
    negative: '\u{8ca0}',
    informal: true,
};
/// `trad-chinese-formal`.
const TRAD_CHINESE_FORMAL: Longhand = Longhand {
    digits: [
        '\u{96f6}', '\u{58f9}', '\u{8cb3}', '\u{53c3}', '\u{8086}', '\u{4f0d}', '\u{9678}',
        '\u{67d2}', '\u{634c}', '\u{7396}',
    ],
    markers: ['\u{62fe}', '\u{4f70}', '\u{4edf}'],
    negative: '\u{8ca0}',
    informal: false,
};

/// The item tags *List Support* requires (pages 1212 and 1213).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListStyle {
    /// A filled circle.
    Disc,
    /// An open circle.
    Circle,
    /// A filled square.
    Square,
    /// No tag at all.
    None,
    /// Western digits.
    Decimal,
    /// Western digits, padded with a zero below ten.
    DecimalLeadingZero,
    /// Digits from another script's own ten, from zero up.
    Digits(char),
    /// Latin letters, lower or upper case.
    Latin(bool),
    /// Greek letters, lower or upper case.
    Greek(bool),
    /// Roman numerals, lower or upper case.
    Roman(bool),
    /// Letters of another script, in the order given: the Kana and Hangul types.
    Letters(&'static [char]),
    /// An additive numbering, its weights largest first: Hebrew and the two Japanese styles.
    Additive {
        /// Each weight and what it is written as.
        weights: &'static [(i64, &'static str)],
        /// The least number the style is defined for; the greatest is
        /// [`ADDITIVE_JAPANESE_MOST`] for a style whose least is zero, and unbounded otherwise.
        least: i64,
    },
    /// One of the four Chinese longhand styles.
    Longhand(&'static Longhand),
}

/// The 2011 draft's upper bound on both Japanese additive styles, whose range is 0 to 9999.
const ADDITIVE_JAPANESE_MOST: i64 = 9999;

impl ListStyle {
    /// A `list-style-type` value, every one *List Support*'s table names; `None` for a value it
    /// does not, which leaves the list's default in its place, as chapter 27's opening has a
    /// processor ignore markup it does not understand (page 1187).
    fn named(value: &str) -> Option<Self> {
        Some(match value {
            "disc" => Self::Disc,
            "circle" => Self::Circle,
            "square" => Self::Square,
            "none" => Self::None,
            "decimal" => Self::Decimal,
            "decimal-leading-zero" => Self::DecimalLeadingZero,
            "lower-alpha" | "lower-latin" => Self::Latin(false),
            "upper-alpha" | "upper-latin" => Self::Latin(true),
            "lower-greek" => Self::Greek(false),
            "upper-greek" => Self::Greek(true),
            "lower-roman" => Self::Roman(false),
            "upper-roman" => Self::Roman(true),
            "arabic-indic" => Self::Digits('\u{660}'),
            "persian" | "urdu" => Self::Digits('\u{6f0}'),
            "thai" => Self::Digits('\u{e50}'),
            "lao" => Self::Digits('\u{ed0}'),
            "cambodian" | "khmer" => Self::Digits('\u{17e0}'),
            "hiragana" => Self::Letters(HIRAGANA),
            "hiragana-iroha" => Self::Letters(HIRAGANA_IROHA),
            "katakana" => Self::Letters(KATAKANA),
            "katakana-iroha" => Self::Letters(KATAKANA_IROHA),
            "hangul" => Self::Letters(HANGUL),
            "hangul-consonant" => Self::Letters(HANGUL_CONSONANT),
            "hebrew" => Self::Additive {
                weights: HEBREW,
                least: 1,
            },
            "japanese-informal" => Self::Additive {
                weights: JAPANESE_INFORMAL,
                least: 0,
            },
            "japanese-formal" => Self::Additive {
                weights: JAPANESE_FORMAL,
                least: 0,
            },
            "simp-chinese-informal" => Self::Longhand(&SIMP_CHINESE_INFORMAL),
            "simp-chinese-formal" => Self::Longhand(&SIMP_CHINESE_FORMAL),
            "trad-chinese-informal" => Self::Longhand(&TRAD_CHINESE_INFORMAL),
            "trad-chinese-formal" => Self::Longhand(&TRAD_CHINESE_FORMAL),
            _ => return None,
        })
    }

    /// HTML's deprecated `type` attribute, which *HTML List Attributes* keeps (page 1211).
    fn html_type(value: &str) -> Option<Self> {
        Some(match value.trim() {
            "1" => Self::Decimal,
            "a" => Self::Latin(false),
            "A" => Self::Latin(true),
            "i" => Self::Roman(false),
            "I" => Self::Roman(true),
            "disc" => Self::Disc,
            "circle" => Self::Circle,
            "square" => Self::Square,
            _ => return None,
        })
    }

    /// Whether the tag is a number of some kind rather than a glyph.
    fn numbered(self) -> bool {
        !matches!(self, Self::Disc | Self::Circle | Self::Square | Self::None)
    }

    /// The tag without its suffix, for a compound tag; `None` for a glyph.
    fn bare(self, number: i64) -> Option<String> {
        match self {
            Self::Disc | Self::Circle | Self::Square | Self::None => None,
            Self::Decimal => Some(number.to_string()),
            Self::DecimalLeadingZero => Some(if (0..10).contains(&number) {
                format!("0{number}")
            } else {
                number.to_string()
            }),
            Self::Digits(zero) => Some(
                number
                    .to_string()
                    .chars()
                    .map(|digit| match digit.to_digit(10) {
                        Some(value) => {
                            char::from_u32(u32::from(zero).saturating_add(value)).unwrap_or(digit)
                        }
                        None => digit,
                    })
                    .collect(),
            ),
            Self::Latin(upper) => alphabetic(number, if upper { 'A' } else { 'a' }, 26, &[]),
            // The Greek alphabet's twenty-four letters, which skip the final sigma's code point.
            Self::Greek(upper) => alphabetic(
                number,
                if upper { '\u{391}' } else { '\u{3b1}' },
                25,
                &[if upper { '\u{3a2}' } else { '\u{3c2}' }],
            ),
            Self::Roman(upper) => roman(number).map(|numeral| {
                if upper {
                    numeral.to_ascii_uppercase()
                } else {
                    numeral
                }
            }),
            Self::Letters(letters) => {
                alphabetic_in(number, letters).or_else(|| Some(number.to_string()))
            }
            Self::Additive { weights, least } => {
                let most = if least == 0 {
                    ADDITIVE_JAPANESE_MOST
                } else {
                    i64::MAX
                };
                // Section 8.5 sends a number outside the range to the fallback style: a Japanese
                // style's is `cjk-decimal`, as section 9.6 states, and Hebrew's the initial
                // `decimal`.
                let within = (least..=most)
                    .contains(&number)
                    .then(|| additive(number, weights))
                    .flatten();
                within.or_else(|| {
                    if least == 0 {
                        Some(cjk_decimal(number))
                    } else {
                        Some(number.to_string())
                    }
                })
            }
            Self::Longhand(set) => longhand(number, set).or_else(|| Some(cjk_decimal(number))),
        }
    }

    /// The tag as an item shows it: a glyph, or a number followed by its full stop, which *List
    /// Layout* says every numeric tag carries (page 1219).
    fn tag(self, number: i64) -> Option<String> {
        match self {
            Self::Disc => Some("\u{2022}".to_owned()),
            Self::Circle => Some("\u{25e6}".to_owned()),
            Self::Square => Some("\u{25aa}".to_owned()),
            Self::None => Some(String::new()),
            numbered => numbered.bare(number).map(|bare| format!("{bare}.")),
        }
    }
}

/// An alphabetic tag: `a` … `z`, `aa` … `zz` and on, from the letters starting at `first`.
fn alphabetic(number: i64, first: char, span: u32, skip: &[char]) -> Option<String> {
    let letters: Vec<char> = (0..span)
        .filter_map(|offset| char::from_u32(u32::from(first).saturating_add(offset)))
        .filter(|letter| !skip.contains(letter))
        .collect();
    alphabetic_in(number, &letters)
}

/// The 2011 draft's alphabetic algorithm over `letters` (section 8.1.3): the number written in
/// bijective base *n*, so that one letter's run is followed by every pair; `None` below one, where
/// the system is not defined.
fn alphabetic_in(number: i64, letters: &[char]) -> Option<String> {
    let base = i64::try_from(letters.len()).ok().filter(|base| *base > 0)?;
    if number < 1 {
        return None;
    }
    let mut out = Vec::new();
    let mut rest = number;
    while rest > 0 {
        rest = rest.saturating_sub(1);
        let index = usize::try_from(rest.rem_euclid(base)).ok()?;
        out.push(*letters.get(index)?);
        rest = rest.checked_div(base)?;
    }
    out.reverse();
    Some(out.into_iter().collect())
}

/// The longest additive representation written, in characters: section 8.1.6 requires a user
/// agent to support representations of at least twenty characters and lets it take the fallback
/// style for longer ones, which Hebrew's unbounded range needs, since its length grows with the
/// number.
const ADDITIVE_LONGEST: usize = 20;

/// The 2011 draft's additive algorithm (section 8.1.6): each weight, largest first, written as
/// many times as it divides what is left; zero is the zero weight's own symbol, and a number the
/// weights cannot sum to, or whose representation passes [`ADDITIVE_LONGEST`], has none here.
fn additive(number: i64, weights: &[(i64, &str)]) -> Option<String> {
    if number == 0 {
        return weights
            .iter()
            .find(|(weight, _)| *weight == 0)
            .map(|(_, symbol)| (*symbol).to_owned());
    }
    let mut rest = number;
    let mut out = String::new();
    let mut length = 0_usize;
    for (weight, symbol) in weights.iter().filter(|(weight, _)| *weight > 0) {
        let times = usize::try_from(rest.checked_div(*weight)?).ok()?;
        length = length.saturating_add(times.saturating_mul(symbol.chars().count()));
        if length > ADDITIVE_LONGEST {
            return None;
        }
        out.push_str(&symbol.repeat(times));
        rest = rest.checked_rem(*weight)?;
    }
    (rest == 0 && !out.is_empty()).then_some(out)
}

/// The 2011 draft's `cjk-decimal`: the number's decimal digits written with [`CJK_DECIMAL`], a
/// negative one behind a hyphen-minus as the draft's numeric styles take it.
fn cjk_decimal(number: i64) -> String {
    let digits: String = number
        .unsigned_abs()
        .to_string()
        .chars()
        .filter_map(|digit| {
            let index = usize::try_from(digit.to_digit(10)?).ok()?;
            CJK_DECIMAL.get(index).copied()
        })
        .collect();
    if number < 0 {
        format!("-{digits}")
    } else {
        digits
    }
}

/// Section 10.3's Chinese longhand algorithm, over -9999 to 9999; `None` outside, where the
/// section sends the number to `cjk-decimal`.
///
/// Its five steps: zero is the zero digit; every digit but zero takes its place's marker, the
/// ones none; an informal style's ten to nineteen drop the tens digit and keep its marker; the
/// trailing zeros go and any run of zeros left becomes one zero digit; the digits are written in
/// the style's characters. A negative number takes the style's negative sign in front.
fn longhand(number: i64, set: &Longhand) -> Option<String> {
    if !(-9999..=9999).contains(&number) {
        return None;
    }
    let magnitude = number.unsigned_abs();
    let digit = |value: u64| {
        usize::try_from(value)
            .ok()
            .and_then(|index| set.digits.get(index))
            .copied()
    };
    let mut out = String::new();
    if number < 0 {
        out.push(set.negative);
    }
    if magnitude == 0 {
        out.push(digit(0)?);
        return Some(out);
    }
    // Thousands, hundreds, tens and ones, with the marker each place takes.
    let places = [
        (magnitude / 1000 % 10, set.markers.get(2).copied()),
        (magnitude / 100 % 10, set.markers.get(1).copied()),
        (magnitude / 10 % 10, set.markers.first().copied()),
        (magnitude % 10, None),
    ];
    let first = places.iter().position(|(value, _)| *value != 0)?;
    let last = places.iter().rposition(|(value, _)| *value != 0)?;
    let mut zero_pending = false;
    for (place, (value, marker)) in places
        .iter()
        .enumerate()
        .take(last.saturating_add(1))
        .skip(first)
    {
        if *value == 0 {
            zero_pending = true;
            continue;
        }
        if zero_pending {
            out.push(digit(0)?);
            zero_pending = false;
        }
        let tens_of_a_teen = set.informal && place == 2 && magnitude / 10 == 1;
        if !tens_of_a_teen {
            out.push(digit(*value)?);
        }
        if let Some(marker) = marker {
            out.push(*marker);
        }
    }
    Some(out)
}

/// A lower-case Roman numeral, from 1 to 3999; `None` outside that range, where the numeral has
/// no standard form.
fn roman(number: i64) -> Option<String> {
    if !(1..=3999).contains(&number) {
        return None;
    }
    let mut rest = number;
    let mut out = String::new();
    for (value, numeral) in [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ] {
        while rest >= value {
            out.push_str(numeral);
            rest = rest.saturating_sub(value);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{ListIndent, Piece, RichText, parse};
    use crate::rich_text::style::{Character, Underline};

    fn read(markup: &str) -> RichText {
        parse(markup, None, Character::root()).expect("the fixture is rich text")
    }

    fn texts(rich: &RichText) -> Vec<Vec<String>> {
        rich.paragraphs
            .iter()
            .map(|paragraph| {
                paragraph
                    .pieces
                    .iter()
                    .map(|piece| match piece {
                        Piece::Text(text, _) => text.clone(),
                        Piece::Break => "<br>".to_owned(),
                        Piece::Tab(count) => format!("<tab {count}>"),
                    })
                    .collect()
            })
            .collect()
    }

    /// Chapter 27's containers do not appear in the output (page 1189), and text outside a `p`
    /// is still text.
    #[test]
    fn the_containers_are_containers_and_text_outside_a_paragraph_is_a_paragraph() {
        let rich = read(
            "<html><body xmlns=\"http://www.w3.org/1999/xhtml\"><p>One</p>Two<p>Three</p></body></html>",
        );
        assert_eq!(texts(&rich), [["One"], ["Two"], ["Three"]]);
    }

    /// White space is compressed to one space and none is kept at a line's ends (page 1194).
    #[test]
    fn white_space_is_compressed_as_a_paragraph_compresses_it() {
        let rich = read("<p>\n   A   lot\tof \r\n space  </p>");
        assert_eq!(texts(&rich), [["A lot of space"]]);
    }

    /// A `br` ends a line, and one at a paragraph's end ends nothing more: chapter 27's Example
    /// 27.2 closes each of three lines with one.
    #[test]
    fn a_line_break_ends_a_line_and_a_last_one_ends_nothing_more() {
        let rich = read("<p>First line.<br/> Second line.<br /></p>");
        assert_eq!(texts(&rich), [["First line.", "<br>", "Second line."]]);
        // An empty paragraph holding only a break is one empty line.
        let rich = read("<p>Before</p><p><br/></p><p>After</p>");
        assert_eq!(texts(&rich), [vec!["Before"], vec!["<br>"], vec!["After"]]);
    }

    /// `b` and `i` and their two style-attribute equivalents are one thing (pages 1199, 1203),
    /// which is Examples 27.18 and 27.22's whole point.
    #[test]
    fn bold_and_italic_come_from_the_element_or_the_attribute_alike() {
        let rich = read(
            "<p>a<b>b</b><span style=\"font-weight:bold\">c</span><i>d</i>\
             <span style=\"font-style:italic\">e</span></p>",
        );
        let styles: Vec<(bool, bool)> = rich.paragraphs[0]
            .pieces
            .iter()
            .filter_map(|piece| match piece {
                Piece::Text(_, style) => Some((style.bold(), style.italic)),
                Piece::Break | Piece::Tab(_) => None,
            })
            .collect();
        // b and the bold span merge into one run, as do i and the italic span.
        assert_eq!(styles, [(false, false), (true, false), (false, true)]);
    }

    /// `sub` and `sup` move the baseline by 15% and 31% of the font height and set 66% of it
    /// (pages 1204, 1205).
    #[test]
    fn subscript_and_superscript_move_and_shrink_by_the_chapters_figures() {
        let rich = parse(
            "<p>x<sub>2</sub><sup>3</sup></p>",
            Some("font-size:10pt"),
            Character::root(),
        )
        .expect("rich text");
        let runs: Vec<(f32, f32)> = rich.paragraphs[0]
            .pieces
            .iter()
            .filter_map(|piece| match piece {
                Piece::Text(_, style) => Some((style.size.at(0.0), style.rise.at(0.0))),
                Piece::Break | Piece::Tab(_) => None,
            })
            .collect();
        let near = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(near(runs[0].0, 10.0) && near(runs[0].1, 0.0));
        assert!(near(runs[1].0, 6.6) && near(runs[1].1, -1.5), "{runs:?}");
        assert!(near(runs[2].0, 6.6) && near(runs[2].1, 3.1), "{runs:?}");
    }

    /// An element chapter 27 does not name is ignored with its whole content: its own example
    /// is `head` (page 1187).
    #[test]
    fn an_element_the_chapter_does_not_name_is_ignored_with_its_content() {
        let rich = read(
            "<html><head><title>Not shown</title></head><body><p>Shown<blink>not</blink></p></body></html>",
        );
        assert_eq!(texts(&rich), [["Shown"]]);
    }

    /// A span styled `xfa-spacerun:yes` keeps its spaces and reads a no-break space as one
    /// (page 1220), in either spelling Examples 27.35 and 27.36 give.
    #[test]
    fn a_space_run_keeps_its_spaces() {
        let rich = read(
            "<p>a <span style=\"xfa-spacerun:yes\">&#160; </span>b <span style=\"xfa-spacerun:yes\">\
             &nbsp;&nbsp; </span>c</p>",
        );
        assert_eq!(rich.text(), "a   b    c");
    }

    /// The hyperlink takes the recommended blue and underline (page 1189), and following it is
    /// not formatting, so nothing is owed (ADR 1660).
    #[test]
    fn a_hyperlink_is_drawn_in_the_recommended_style() {
        let rich = read("<p>see <a href=\"http://example.invalid/\">here</a></p>");
        let Piece::Text(text, style) = &rich.paragraphs[0].pieces[1] else {
            panic!("the link is a run");
        };
        assert_eq!(text, "here");
        assert_eq!(style.colour, Some([0.0, 0.0, 1.0]));
        assert_eq!(style.underline, Underline::Single);
        assert!(rich.unapplied.is_empty(), "{:?}", rich.unapplied);
    }

    /// The default style string sits under the markup: the markup's own style wins where both
    /// state one.
    #[test]
    fn the_default_style_string_is_beneath_the_markup() {
        let rich = parse(
            "<p>a<span style=\"font-size:20pt\">b</span></p>",
            Some("font: 9pt Times; color:#ff0000"),
            Character::root(),
        )
        .expect("rich text");
        let sizes: Vec<(f32, Option<[f32; 3]>, Vec<String>)> = rich.paragraphs[0]
            .pieces
            .iter()
            .filter_map(|piece| match piece {
                Piece::Text(_, style) => {
                    Some((style.size.at(0.0), style.colour, style.families.clone()))
                }
                Piece::Break | Piece::Tab(_) => None,
            })
            .collect();
        assert_eq!(
            sizes[0],
            (9.0, Some([1.0, 0.0, 0.0]), vec!["Times".to_owned()])
        );
        assert_eq!(
            sizes[1],
            (20.0, Some([1.0, 0.0, 0.0]), vec!["Times".to_owned()])
        );
    }

    /// Markup that is not well formed is not rich text, and neither is a string with no element.
    #[test]
    fn what_does_not_parse_or_opens_nothing_is_not_rich_text() {
        assert!(parse("a < b", None, Character::root()).is_none());
        assert!(parse("plain characters", None, Character::root()).is_none());
    }

    /// Ordered and unordered lists, nested, with the default tags *List Support* gives each
    /// level (pages 1209 to 1212) — Example 27.33's shape, in words of this test's own.
    #[test]
    fn lists_take_their_default_tags_level_by_level() {
        let rich = read(
            "<ol><li>red</li><li>green</li><ul><li>pale</li><ul><li>paler</li></ul></ul>\
             <li>blue</li></ol>",
        );
        let tags: Vec<(String, ListIndent)> = rich
            .paragraphs
            .iter()
            .map(|paragraph| {
                (
                    paragraph
                        .tag
                        .as_ref()
                        .map(|tag| tag.text.clone())
                        .unwrap_or_default(),
                    paragraph.list,
                )
            })
            .collect();
        assert_eq!(
            tags,
            [
                ("1.".to_owned(), ListIndent::Levels(1)),
                ("2.".to_owned(), ListIndent::Levels(1)),
                ("\u{2022}".to_owned(), ListIndent::Levels(2)),
                ("\u{25e6}".to_owned(), ListIndent::Levels(3)),
                ("3.".to_owned(), ListIndent::Levels(1)),
            ]
        );
    }

    /// Compound tags concatenate the enclosing items' tags with full stops, each in its own
    /// list's style (pages 1215, 1216).
    #[test]
    fn compound_tags_concatenate_the_enclosing_items() {
        let rich = read(
            "<ol style=\"xfa-list-item-tags:compound\"><li>one<ol style=\"list-style-type:lower-latin\">\
             <li>one a</li><li>one b</li></ol></li><li>two<ol style=\"list-style-type:lower-roman\">\
             <li>two i</li></ol></li></ol>",
        );
        let tags: Vec<String> = rich
            .paragraphs
            .iter()
            .filter_map(|paragraph| paragraph.tag.as_ref().map(|tag| tag.text.clone()))
            .collect();
        assert_eq!(tags, ["1.", "1.a.", "1.b.", "2.", "2.i."]);
    }

    /// A grammar older than 3.3 discards list content (page 1217).
    #[test]
    fn a_grammar_before_three_point_three_discards_its_lists() {
        let rich = read(
            "<body xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\" xfa:spec=\"2.0.2\">\
             <p>kept</p><ul><li>dropped</li></ul></body>",
        );
        assert_eq!(texts(&rich), [["kept"]]);
    }

    /// The tag styles this tree generates.
    #[test]
    fn list_styles_count_as_their_scripts_count() {
        use super::ListStyle;
        assert_eq!(ListStyle::Latin(false).tag(28), Some("ab.".to_owned()));
        assert_eq!(
            ListStyle::Roman(true).tag(1994),
            Some("MCMXCIV.".to_owned())
        );
        assert_eq!(
            ListStyle::Greek(false).tag(18),
            Some("\u{3c3}.".to_owned()),
            "sigma, past the final form"
        );
        assert_eq!(ListStyle::DecimalLeadingZero.tag(7), Some("07.".to_owned()));
        assert_eq!(
            ListStyle::Digits('\u{660}').tag(12),
            Some("\u{661}\u{662}.".to_owned())
        );
    }

    /// The algorithmic and alphabetic types *List Support* requires (pages 1212 and 1213), each
    /// against what its [CSS3-Lists] definition produces: Hebrew's 15 and 16 as the draft's own
    /// pairs, the additive Japanese styles and their `cjk-decimal` fallback past 9999, the Chinese
    /// longhand algorithm's dropped teen digit, collapsed zeros and dropped trailing zeros, and an
    /// alphabet's bijective second letter (ADR 1660).
    #[test]
    fn the_algorithmic_and_alphabetic_types_count_as_the_drafts_state() {
        use super::ListStyle;
        let bare = |name: &str, number: i64| {
            ListStyle::named(name)
                .and_then(|style| style.bare(number))
                .unwrap_or_default()
        };
        assert_eq!(bare("hebrew", 15), "\u{5d8}\u{5d5}");
        assert_eq!(bare("hebrew", 16), "\u{5d8}\u{5d6}");
        assert_eq!(bare("hebrew", 115), "\u{5e7}\u{5d8}\u{5d5}");
        assert_eq!(bare("hebrew", 1000), "\u{5ea}\u{5ea}\u{5e8}");
        assert_eq!(
            bare("hebrew", 100_000),
            "100000",
            "past twenty characters, decimal"
        );
        assert_eq!(
            bare("japanese-informal", 1234),
            "\u{5343}\u{4e8c}\u{767e}\u{4e09}\u{5341}\u{56db}"
        );
        assert_eq!(bare("japanese-formal", 2005), "\u{5f10}\u{9621}\u{4f0d}");
        assert_eq!(bare("japanese-informal", 0), "\u{3007}");
        assert_eq!(
            bare("japanese-informal", 10_000),
            "\u{4e00}\u{3007}\u{3007}\u{3007}\u{3007}"
        );
        assert_eq!(bare("simp-chinese-informal", 15), "\u{5341}\u{4e94}");
        assert_eq!(bare("simp-chinese-formal", 15), "\u{58f9}\u{62fe}\u{4f0d}");
        assert_eq!(
            bare("simp-chinese-informal", 105),
            "\u{4e00}\u{767e}\u{96f6}\u{4e94}"
        );
        assert_eq!(
            bare("trad-chinese-informal", 1010),
            "\u{4e00}\u{5343}\u{96f6}\u{4e00}\u{5341}"
        );
        assert_eq!(bare("trad-chinese-formal", 20), "\u{8cb3}\u{62fe}");
        assert_eq!(bare("trad-chinese-informal", 0), "\u{96f6}");
        assert_eq!(bare("trad-chinese-informal", -3), "\u{8ca0}\u{4e09}");
        assert_eq!(bare("hiragana", 1), "\u{3042}");
        assert_eq!(bare("hiragana", 47), "\u{3042}\u{3042}");
        assert_eq!(bare("katakana-iroha", 25), "\u{30f0}");
        assert_eq!(bare("hangul", 15), "\u{ac00}\u{ac00}");
        assert_eq!(bare("hangul-consonant", 14), "\u{314e}");
        assert_eq!(bare("hangul", 0), "0", "below one, decimal");
        assert_eq!(
            ListStyle::named("hebrew").and_then(|style| style.tag(2)),
            Some("\u{5d1}.".to_owned()),
            "every numeric tag takes the full stop (page 1219)"
        );
    }
}
