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
                character.colour = Some([0.0, 0.0, 1.0]);
                character.underline = style::Underline::Single;
                if element.attribute("", "href").is_some() {
                    self.unapplied
                        .note("following the hyperlink an <a href> encloses");
                }
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
            // outside the file.
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
            .and_then(|value| ListStyle::named(&value, &mut self.unapplied))
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

/// The item tags *List Support* requires (pages 1212 and 1213), as far as this tree generates
/// them.
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
}

impl ListStyle {
    /// A `list-style-type` value, or `None` with the value noted where this tree does not
    /// generate it — the algorithmic Hebrew, Japanese and Chinese numberings and the Hangul, Kana
    /// and Iroha alphabets — which leaves the list's default in its place.
    fn named(value: &str, unapplied: &mut Unapplied) -> Option<Self> {
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
            "hangul"
            | "hangul-consonant"
            | "hebrew"
            | "hiragana"
            | "hiragana-iroha"
            | "japanese-formal"
            | "japanese-informal"
            | "katakana"
            | "katakana-iroha"
            | "simp-chinese-formal"
            | "simp-chinese-informal"
            | "trad-chinese-formal"
            | "trad-chinese-informal" => {
                unapplied.note(format!("list-style-type:{value}"));
                return None;
            }
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

    /// The hyperlink takes the recommended blue and underline (page 1189), and its following is
    /// reported.
    #[test]
    fn a_hyperlink_is_drawn_in_the_recommended_style_and_its_following_is_named() {
        let rich = read("<p>see <a href=\"http://example.invalid/\">here</a></p>");
        let Piece::Text(text, style) = &rich.paragraphs[0].pieces[1] else {
            panic!("the link is a run");
        };
        assert_eq!(text, "here");
        assert_eq!(style.colour, Some([0.0, 0.0, 1.0]));
        assert_eq!(style.underline, Underline::Single);
        assert!(rich.unapplied.phrase().contains("hyperlink"));
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
}
