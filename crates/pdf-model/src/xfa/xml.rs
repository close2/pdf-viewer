//! One XFA packet as a tree of elements, each carrying the byte ranges it occupies.
//!
//! The ranges are the point. RFC 0011 section 5's second step writes a `datasets` packet in which
//! the values a person changed are new and every other byte is the producer's, so the writer
//! splices into the packet's own text rather than serialising a tree of its own: a value it never
//! touched cannot come back reformatted, re-namespaced or re-escaped. Reading is
//! `xmlparser`'s, the tokenizer XMP and XFDF already use, under the depth and element budgets
//! below, because a packet is a stream's decoded bytes and as hostile as any (principle 3).

use std::ops::Range;

/// The deepest element nesting this module follows.
///
/// XFA 3.3 states no limit. The deepest packet of the census RFC 0011 section 4 took — a
/// government form's template of 1.6 MB — nests a few dozen elements, so this is a bound on a
/// hostile file and not a reading.
pub(crate) const MAX_DEPTH: usize = 256;

/// The most elements this module keeps of one packet; the same kind of bound as [`MAX_DEPTH`].
pub(crate) const MAX_ELEMENTS: usize = 1 << 20;

/// Why a packet could not be read as XML.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum XmlError {
    /// XFA 3.3 writes its packets in UTF-8, and this one is not.
    #[error("the packet is not UTF-8")]
    NotUtf8,
    /// The tokenizer refused the text, or an element was left open or closed unopened.
    #[error("the packet's XML is malformed: {0}")]
    Malformed(String),
    /// The packet nests elements deeper than this reader's depth bound.
    #[error("the packet nests elements deeper than {MAX_DEPTH}")]
    TooDeep,
    /// The packet holds more elements than this reader's element bound.
    #[error("the packet holds more than {MAX_ELEMENTS} elements")]
    TooMany,
}

/// One element of a packet.
#[derive(Debug, Clone)]
pub(crate) struct Element {
    /// The local name, the prefix dropped: XFA 3.3 names a template's containers and a data
    /// node by it, and a packet may bind its namespace to any prefix.
    pub(crate) local: String,
    /// The name as the start tag spells it, for an end tag this module writes.
    pub(crate) qualified: String,
    /// Every attribute, by local name, its value with XML's references expanded.
    pub(crate) attributes: Vec<(String, String)>,
    /// The element's own child elements, in document order.
    pub(crate) children: Vec<usize>,
    /// The element's direct character data, references expanded and CDATA taken as written.
    pub(crate) text: String,
    /// The `>` or `/>` that ends the start tag.
    pub(crate) start_end: Range<usize>,
    /// What lies between the start tag and the end tag; `None` for an empty-element tag.
    pub(crate) content: Option<Range<usize>>,
}

impl Element {
    /// The value of the attribute of this local name.
    pub(crate) fn attribute(&self, local: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(name, _)| name == local)
            .map(|(_, value)| value.as_str())
    }
}

/// A packet's elements, in document order.
#[derive(Debug, Clone)]
pub(crate) struct Tree {
    /// Every element; an element's index is its position in document order.
    pub(crate) elements: Vec<Element>,
}

impl Tree {
    /// The element at an index this tree handed out.
    pub(crate) fn get(&self, index: usize) -> Option<&Element> {
        self.elements.get(index)
    }

    /// The first child of `parent` with this local name.
    pub(crate) fn child(&self, parent: usize, local: &str) -> Option<usize> {
        self.get(parent)?.children.iter().copied().find(|&child| {
            self.get(child)
                .is_some_and(|element| element.local == local)
        })
    }

    /// The first element of this local name, at any depth — how a whole-resource stream's
    /// `datasets` packet is found inside its `xdp:xdp` (§K.2's first form), and its `template`
    /// only where no child of the root is one.
    pub(crate) fn first(&self, local: &str) -> Option<usize> {
        self.elements
            .iter()
            .position(|element| element.local == local)
    }
}

/// Reads a packet's bytes into a [`Tree`].
///
/// # Errors
///
/// [`XmlError`], whose variants are the reasons.
pub(crate) fn parse(bytes: &[u8]) -> Result<Tree, XmlError> {
    let text = std::str::from_utf8(bytes).map_err(|_| XmlError::NotUtf8)?;
    let mut elements: Vec<Element> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    // Where the content of each open element starts, beside `open`.
    let mut content_from: Vec<usize> = Vec::new();
    let unbalanced = || XmlError::Malformed("an element is closed without being opened".to_owned());
    for token in xmlparser::Tokenizer::from(text) {
        let token = token.map_err(|error| XmlError::Malformed(error.to_string()))?;
        match token {
            xmlparser::Token::ElementStart { local, span, .. } => {
                if elements.len() >= MAX_ELEMENTS {
                    return Err(XmlError::TooMany);
                }
                if open.len() >= MAX_DEPTH {
                    return Err(XmlError::TooDeep);
                }
                let index = elements.len();
                if let Some(parent) = open.last().and_then(|&parent| elements.get_mut(parent)) {
                    parent.children.push(index);
                }
                let qualified = span.as_str().trim_start_matches('<').to_owned();
                elements.push(Element {
                    local: local.as_str().to_owned(),
                    qualified,
                    attributes: Vec::new(),
                    children: Vec::new(),
                    text: String::new(),
                    start_end: span.end()..span.end(),
                    content: None,
                });
                open.push(index);
                content_from.push(span.end());
            }
            xmlparser::Token::Attribute { local, value, .. } => {
                let element = open
                    .last()
                    .and_then(|&index| elements.get_mut(index))
                    .ok_or_else(unbalanced)?;
                let mut expanded = String::with_capacity(value.len());
                crate::xmp::unescape(value.as_str(), &mut expanded);
                element
                    .attributes
                    .push((local.as_str().to_owned(), expanded));
            }
            xmlparser::Token::ElementEnd { end, span } => match end {
                xmlparser::ElementEnd::Open => {
                    let element = open
                        .last()
                        .and_then(|&index| elements.get_mut(index))
                        .ok_or_else(unbalanced)?;
                    element.start_end = span.range();
                    if let Some(from) = content_from.last_mut() {
                        *from = span.end();
                    }
                }
                xmlparser::ElementEnd::Empty => {
                    let index = open.pop().ok_or_else(unbalanced)?;
                    content_from.pop();
                    let element = elements.get_mut(index).ok_or_else(unbalanced)?;
                    element.start_end = span.range();
                }
                xmlparser::ElementEnd::Close(prefix, local) => {
                    let index = open.pop().ok_or_else(unbalanced)?;
                    let from = content_from.pop().ok_or_else(unbalanced)?;
                    let element = elements.get_mut(index).ok_or_else(unbalanced)?;
                    // The tokenizer does not pair end tags with start tags; a mismatch is a packet
                    // whose structure is not the one its text claims, and a splice into it would
                    // land somewhere nobody meant.
                    let closing = if prefix.is_empty() {
                        local.as_str().to_owned()
                    } else {
                        format!("{}:{}", prefix.as_str(), local.as_str())
                    };
                    if closing != element.qualified {
                        return Err(XmlError::Malformed(format!(
                            "<{}> is closed by </{closing}>",
                            element.qualified
                        )));
                    }
                    element.content = Some(from..span.start());
                }
            },
            xmlparser::Token::Text { text } => {
                if let Some(element) = open.last().and_then(|&index| elements.get_mut(index)) {
                    crate::xmp::unescape(text.as_str(), &mut element.text);
                }
            }
            xmlparser::Token::Cdata { text, .. } => {
                if let Some(element) = open.last().and_then(|&index| elements.get_mut(index)) {
                    element.text.push_str(text.as_str());
                }
            }
            _ => {}
        }
    }
    // `xmlparser` hands back the tokens of a truncated packet without complaint (the same
    // reason `xfdf::read` checks its balance).
    if !open.is_empty() {
        return Err(XmlError::Malformed("an element is left open".to_owned()));
    }
    Ok(Tree { elements })
}

/// `text` as XML character data: the three characters markup reserves, and the carriage return,
/// which XML's end-of-line handling would otherwise turn into a line feed on the next read
/// (XML 1.0 section 2.11) — §12.7.4.3 makes it a line break in a `/V`, so it is written as a
/// reference and survives.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            other => out.push(other),
        }
    }
    out
}

/// Whether `name` can be written as an element's tag without escaping, which a data node's name
/// taken from a template must be before this module writes one (XML 1.0 section 2.3's `Name`, its
/// ASCII part, and any character past it, which that production admits nearly whole).
pub(crate) fn is_name(name: &str) -> bool {
    let mut characters = name.chars();
    let starts = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || !first.is_ascii());
    starts
        && characters.all(|next| {
            next.is_ascii_alphanumeric() || matches!(next, '_' | '-' | '.') || !next.is_ascii()
        })
}

#[cfg(test)]
mod tests {
    use super::{XmlError, escape, parse};

    /// A start tag's end and an element's content are the ranges a splice needs, and an
    /// empty-element tag has no content range.
    #[test]
    fn ranges_are_the_text_they_name() {
        let text = "<a x=\"1&amp;2\"><b>one &lt;two&gt;</b><c/></a>";
        let tree = parse(text.as_bytes()).expect("well formed");
        let a = tree.get(0).expect("a");
        assert_eq!(a.attribute("x"), Some("1&2"));
        let b = tree.get(1).expect("b");
        assert_eq!(b.text, "one <two>");
        let content = b.content.clone().expect("content");
        assert_eq!(&text[content], "one &lt;two&gt;");
        let c = tree.get(2).expect("c");
        assert!(c.content.is_none());
        assert_eq!(&text[c.start_end.clone()], "/>");
    }

    /// A mismatched end tag and a truncated packet are refused rather than read as a shorter one.
    #[test]
    fn unbalanced_packets_are_refused() {
        assert!(matches!(
            parse(b"<a><b></a></b>"),
            Err(XmlError::Malformed(_))
        ));
        assert!(matches!(parse(b"<a><b>"), Err(XmlError::Malformed(_))));
    }

    /// A carriage return survives a write and a read: XML would normalise a literal one away.
    #[test]
    fn a_carriage_return_is_written_as_a_reference() {
        let written = format!("<v>{}</v>", escape("one\rtwo & <three>"));
        let tree = parse(written.as_bytes()).expect("well formed");
        assert_eq!(tree.get(0).expect("v").text, "one\rtwo & <three>");
    }
}
