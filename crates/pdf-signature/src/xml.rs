//! The XML this crate reads, folded into a bounded tree of elements before any clause is applied.
//!
//! Two ETSI syntaxes arrive as XML: TS 119 172-2's signature policies and TS 119 612's trusted
//! lists. Both are a stranger's bytes, so both are read by one reader that charges every element
//! and every namespace declaration to a budget the caller sets, refuses a document type
//! declaration (which is where entity expansion lives), and resolves each element's namespace by
//! XML Namespaces 1.0 section 6.2. What an element *means* is the caller's; this module only says
//! what was written.
//!
//! The tokenizer is `xmlparser`, the one XMP, XFDF and XFA are read with (ADR 0186).

use crate::der;
use crate::x509;

/// Why a document's XML was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum XmlError {
    /// Not UTF-8, not well-formed, or declaring a document type; the reason is said.
    Unreadable(String),
    /// Elements nest deeper than [`der::MAX_DEPTH`].
    NestedTooDeep,
    /// More elements and declarations than the caller's budget.
    TooManyEntries,
}

/// One element of the document.
#[derive(Debug)]
pub(crate) struct Element {
    /// The prefix its tag was written with, until the start tag's declarations are read.
    pub(crate) prefix: String,
    /// The namespace name its prefix resolves to, empty for none.
    pub(crate) namespace: String,
    /// `{namespace}local`, the name an extension is reported by.
    pub(crate) expanded: String,
    pub(crate) local: String,
    /// The unprefixed attributes, by local name, references expanded.
    pub(crate) attributes: Vec<(String, String)>,
    pub(crate) children: Vec<usize>,
    /// Direct character data, references expanded.
    pub(crate) text: String,
}

/// The document's elements in document order; the root is the first.
#[derive(Debug)]
pub(crate) struct Tree {
    pub(crate) elements: Vec<Element>,
}

/// Charges one entry to `remaining`.
fn take(remaining: &mut usize) -> Result<(), XmlError> {
    *remaining = remaining.checked_sub(1).ok_or(XmlError::TooManyEntries)?;
    Ok(())
}

/// The tokenizer's events folded into a [`Tree`], every element and declaration charged to
/// `remaining` before the work.
pub(crate) fn read_tree(octets: &[u8], remaining: &mut usize) -> Result<Tree, XmlError> {
    let body = octets.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(octets);
    // Neither ETSI syntax names a character encoding other than UTF-8 — TS 119 612 clause 5.1.1
    // requires it of a trusted list — and XML 1.0 section 4.3.3 makes UTF-8 one every processor
    // reads, so a document in another is refused by name rather than misread.
    let text = std::str::from_utf8(body)
        .map_err(|_| XmlError::Unreadable("the document is not UTF-8".to_owned()))?;
    let mut elements: Vec<Element> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    let mut scope = Declarations::default();
    let unbalanced =
        || XmlError::Unreadable("an element is closed without being opened".to_owned());
    for token in xmlparser::Tokenizer::from(text) {
        let token = token.map_err(|error| XmlError::Unreadable(error.to_string()))?;
        match token {
            xmlparser::Token::DtdStart { .. } | xmlparser::Token::EmptyDtd { .. } => {
                return Err(XmlError::Unreadable(
                    "the document declares a document type, which this reader does not process"
                        .to_owned(),
                ));
            }
            xmlparser::Token::ElementStart { prefix, local, .. } => {
                take(remaining)?;
                if open.len() >= usize::from(der::MAX_DEPTH) {
                    return Err(XmlError::NestedTooDeep);
                }
                let index = elements.len();
                if let Some(parent) = open.last().and_then(|&parent| elements.get_mut(parent)) {
                    parent.children.push(index);
                }
                elements.push(Element {
                    prefix: prefix.as_str().to_owned(),
                    namespace: String::new(),
                    expanded: String::new(),
                    local: local.as_str().to_owned(),
                    attributes: Vec::new(),
                    children: Vec::new(),
                    text: String::new(),
                });
                open.push(index);
                scope.declared.push(0);
            }
            xmlparser::Token::Attribute {
                prefix,
                local,
                value,
                ..
            } => {
                let mut expanded = String::with_capacity(value.len());
                unescape(value.as_str(), &mut expanded);
                let (prefix, local) = (prefix.as_str(), local.as_str());
                if prefix == "xmlns" || (prefix.is_empty() && local == "xmlns") {
                    take(remaining)?;
                    scope
                        .declare(if prefix.is_empty() { "" } else { local }, expanded)
                        .ok_or_else(unbalanced)?;
                } else if prefix.is_empty() {
                    open.last()
                        .and_then(|&index| elements.get_mut(index))
                        .ok_or_else(unbalanced)?
                        .attributes
                        .push((local.to_owned(), expanded));
                }
            }
            xmlparser::Token::ElementEnd { end, .. } => {
                end_tag(end, &mut elements, &mut open, &mut scope)?;
            }
            xmlparser::Token::Text { text } => {
                if let Some(element) = open.last().and_then(|&index| elements.get_mut(index)) {
                    unescape(text.as_str(), &mut element.text);
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
    // The tokenizer hands back the tokens of a truncated document without complaint.
    if !open.is_empty() || elements.is_empty() {
        return Err(XmlError::Unreadable(
            "the document's root element is not closed".to_owned(),
        ));
    }
    Ok(Tree { elements })
}

/// The end of a start tag, where its namespace is resolved, or an element's close.
fn end_tag(
    end: xmlparser::ElementEnd<'_>,
    elements: &mut [Element],
    open: &mut Vec<usize>,
    scope: &mut Declarations,
) -> Result<(), XmlError> {
    let unbalanced =
        || XmlError::Unreadable("an element is closed without being opened".to_owned());
    if matches!(
        end,
        xmlparser::ElementEnd::Open | xmlparser::ElementEnd::Empty
    ) {
        let element = open
            .last()
            .and_then(|&index| elements.get_mut(index))
            .ok_or_else(unbalanced)?;
        resolve(element, &scope.bindings)?;
    }
    if let xmlparser::ElementEnd::Close(prefix, local) = end {
        let element = open
            .last()
            .and_then(|&index| elements.get(index))
            .ok_or_else(unbalanced)?;
        if element.prefix != prefix.as_str() || element.local != local.as_str() {
            return Err(XmlError::Unreadable(format!(
                "<{}> is closed by another element's end tag",
                element.local
            )));
        }
    }
    if !matches!(end, xmlparser::ElementEnd::Open) {
        open.pop().ok_or_else(unbalanced)?;
        scope.close().ok_or_else(unbalanced)?;
    }
    Ok(())
}

/// The namespace declarations in scope, innermost last, and how many each open element added.
#[derive(Default)]
struct Declarations {
    bindings: Vec<(String, String)>,
    declared: Vec<usize>,
}

impl Declarations {
    /// Binds `prefix` (empty for the default namespace) on the innermost open element; `None`
    /// where no element is open.
    fn declare(&mut self, prefix: &str, uri: String) -> Option<()> {
        let count = self.declared.last_mut()?;
        *count = count.saturating_add(1);
        self.bindings.push((prefix.to_owned(), uri));
        Some(())
    }

    /// Drops the innermost element's declarations as it closes.
    fn close(&mut self) -> Option<()> {
        let added = self.declared.pop()?;
        self.bindings
            .truncate(self.bindings.len().saturating_sub(added));
        Some(())
    }
}

/// An element's namespace, from the declarations in scope at the end of its start tag (XML
/// Namespaces 1.0 section 6.2).
fn resolve(element: &mut Element, bindings: &[(String, String)]) -> Result<(), XmlError> {
    let uri = bindings
        .iter()
        .rev()
        .find(|(prefix, _)| *prefix == element.prefix)
        .map(|(_, uri)| uri.as_str());
    let uri = match (uri, element.prefix.as_str()) {
        (Some(uri), _) => uri,
        (None, "") => "",
        (None, prefix) => {
            return Err(XmlError::Unreadable(format!(
                "the prefix {prefix} is not declared"
            )));
        }
    };
    element.namespace = uri.to_owned();
    element.expanded = format!("{{{uri}}}{}", element.local);
    Ok(())
}

/// Expands the five entities XML 1.0 section 4.6 predefines and section 4.1's character
/// references, appending to `out`; any other reference is kept as written.
fn unescape(text: &str, out: &mut String) {
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return;
        };
        let reference = &rest[1..end];
        let character = match reference {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => reference
                .strip_prefix("#x")
                .map(|hex| u32::from_str_radix(hex, 16))
                .or_else(|| reference.strip_prefix('#').map(str::parse::<u32>))
                .and_then(Result::ok)
                .and_then(char::from_u32),
        };
        match character {
            Some(character) => out.push(character),
            None => out.push_str(&rest[..=end]),
        }
        rest = &rest[end.saturating_add(1)..];
    }
    out.push_str(rest);
}

/// An `xs:base64Binary` value (RFC 4648 section 4), white space ignored as XML Schema's lexical
/// space allows; `None` where it is not one.
pub(crate) fn base64(text: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let symbols: Vec<u8> = text
        .bytes()
        .filter(|octet| !octet.is_ascii_whitespace())
        .collect();
    if !symbols.len().is_multiple_of(4) {
        return None;
    }
    let last = (symbols.len() / 4).checked_sub(1);
    let mut out = Vec::with_capacity(symbols.len());
    for (index, group) in symbols.chunks_exact(4).enumerate() {
        // `=` pads only the last group, and at most two of its symbols.
        let padding = group
            .iter()
            .rev()
            .take_while(|octet| **octet == b'=')
            .count();
        if padding > 2 || (padding > 0 && Some(index) != last) {
            return None;
        }
        let mut word = 0_u32;
        for (at, symbol) in group.iter().enumerate() {
            let sextet = if at < 4_usize.saturating_sub(padding) {
                u32::try_from(ALPHABET.iter().position(|letter| letter == symbol)?).ok()?
            } else {
                0
            };
            word = (word << 6) | sextet;
        }
        out.extend_from_slice(word.to_be_bytes().get(1..4_usize.saturating_sub(padding))?);
    }
    Some(out)
}

/// An `xs:dateTime` (XML Schema 1.1 Part 2 section 3.3.7) as an instant: a four-digit year,
/// fractional seconds dropped, and a time zone required — an instant without one is not an
/// instant — `Z` or an offset of at most fourteen hours.
pub(crate) fn date_time(spelled: &str) -> Option<x509::Instant> {
    let bytes = spelled.as_bytes();
    let fixed = bytes.get(..19)?;
    let shape = |at: usize, separator: u8| fixed.get(at) == Some(&separator);
    if !(shape(4, b'-') && shape(7, b'-') && shape(10, b'T') && shape(13, b':') && shape(16, b':'))
    {
        return None;
    }
    let mut rest = bytes.get(19..)?;
    if let Some(fraction) = rest.strip_prefix(b".") {
        let digits = fraction
            .iter()
            .take_while(|octet| octet.is_ascii_digit())
            .count();
        if digits == 0 {
            return None;
        }
        rest = fraction.get(digits..)?;
    }
    let offset_seconds: i64 = match rest {
        b"Z" => 0,
        [sign @ (b'+' | b'-'), h1, h0, b':', m1, m0] => {
            let number = |pair: [u8; 2]| std::str::from_utf8(&pair).ok()?.parse::<i64>().ok();
            let hours = number([*h1, *h0])?;
            let minutes = number([*m1, *m0])?;
            let total = hours.checked_mul(60)?.checked_add(minutes)?;
            if minutes > 59 || total > 14 * 60 {
                return None;
            }
            let seconds = total.checked_mul(60)?;
            if *sign == b'+' {
                seconds
            } else {
                seconds.checked_neg()?
            }
        }
        _ => return None,
    };
    let mut digits = Vec::with_capacity(15);
    for (index, octet) in fixed.iter().enumerate() {
        if ![4, 7, 10, 13, 16].contains(&index) {
            digits.push(*octet);
        }
    }
    digits.push(b'Z');
    let local = x509::read_time_digits(&digits, true)?;
    // The instant is the local time less its offset from UTC.
    Some(x509::Instant::from_unix_seconds(
        local.unix_seconds().checked_sub(offset_seconds)?,
    ))
}
