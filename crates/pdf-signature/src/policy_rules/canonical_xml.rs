//! W3C Canonical XML 1.0 (2001), the inclusive form, over one element of a policy document and
//! everything inside it — the digest input ETSI TS 119 172-2 clause 4.3.2 names for a policy in
//! its XML syntax, which is the `PolicyComponents` element of clause 4.4 under the
//! canonicalisation method the document's own `Digest` states (ADR 2644).
//!
//! The Recommendation is cited by section and paraphrased. Its input is the `XPath` data model a
//! conforming XML processor builds (section 2.1), so the four normalisations it lists are made
//! here as the tokens are read: line breaks become #xA, an attribute value is normalised as a
//! processor with no document type declaration normalises it — every value is CDATA — a CDATA
//! section becomes its characters, and references are replaced. The subset is the one an `XPath`
//! expression selecting the element, its descendants, their attributes and their namespace nodes
//! would make, and section 2.3's rendering is applied to it in document order:
//!
//! - **The namespace axis.** The apex has no ancestor in the subset, so every namespace in scope
//!   on it is rendered, wherever it was declared; below the apex a declaration is rendered only
//!   where its parent has no namespace of that prefix and value. `xmlns=""` is rendered where an
//!   element has no default namespace and its parent has one, and the `xml` prefix never is.
//! - **The attribute axis.** Section 2.4 copies onto the apex the nearest attribute in the `xml`
//!   namespace of each name that an ancestor outside the subset carries and the apex does not.
//! - **Order.** Namespace declarations by prefix, the default first; attributes by namespace URI
//!   and then local name, an unqualified one first (section 2.2); code-point order is the byte
//!   order of UTF-8.
//! - **Escapes.** Character content escapes `&`, `<`, `>` and #xD, an attribute value `&`, `<`,
//!   `"`, #x9, #xA and #xD, the value always between quotation marks (section 2.3).
//! - **Comments** are kept by the method whose URI ends `#WithComments` and dropped by the other.
//!
//! # What is refused
//!
//! Section 2.1's note requires a canonicaliser to fail on a relative namespace URI, so one is
//! refused wherever it is declared. A reference to an entity XML 1.0 section 4.6 does not
//! predefine, a character reference to a character outside XML 1.0's `Char` production, an
//! undeclared prefix, an attribute stated twice under one expanded name, a processing instruction
//! whose target is `xml` or runs into its data, and an encoding declaration naming anything but
//! UTF-8 are not well-formed or not read as written, so each is
//! refused by name rather than canonicalised into octets nobody else would compute. A document
//! type declaration is refused, as the policy reader refuses it: without one no attribute has a
//! default and no entity a declaration, which is what makes this subset of the Recommendation the
//! whole of it for these documents.
//!
//! # Budgets, before the work
//!
//! The policy reader's three bounds hold here in the same places: [`MAX_POLICY_OCTETS`] before a
//! byte is tokenized, [`der::MAX_DEPTH`] on element nesting and [`MAX_ENTRIES`] charged for every
//! element and every namespace declaration, so the canonicaliser accepts what the reader accepts.
//! Every other cost is linear in the octets: each start tag's attributes are sorted once, a
//! prefix is looked up in a table rather than along the stack, and the apex's inherited
//! namespaces and `xml` attributes are gathered once. The output is not held: it is handed to
//! `out` a piece at a time, which is the digest's input.

use std::collections::{HashMap, HashSet};

use super::{Budget, MAX_ENTRIES, MAX_POLICY_OCTETS, PolicyDocumentError};
use crate::der;

/// Canonical XML 1.0's identifier without comments, as XML Signature 1.1 section 6.5.1 names it.
pub const C14N: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";

/// Canonical XML 1.0's identifier with comments.
pub const C14N_WITH_COMMENTS: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315#WithComments";

/// The namespace the `xml` prefix is bound to by definition (XML Namespaces 1.0 section 3).
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// Whether comments are part of the canonical form: the one parameter section 2.1 gives the
/// algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comments {
    /// [`C14N`].
    Omitted,
    /// [`C14N_WITH_COMMENTS`].
    Kept,
}

impl Comments {
    /// The form a `CanonicalizationMethod`'s algorithm URI names, where it names inclusive
    /// Canonical XML 1.0; `None` for any other method, Exclusive Canonical XML and Canonical XML
    /// 1.1 among them.
    #[must_use]
    pub fn of(method: &str) -> Option<Self> {
        match method {
            C14N => Some(Self::Omitted),
            C14N_WITH_COMMENTS => Some(Self::Kept),
            _ => None,
        }
    }
}

/// The element the subset is drawn around: the first at `depth` below the root (the root is at
/// zero) whose expanded name is one of `namespaces` with `local`.
#[derive(Debug, Clone, Copy)]
pub struct Apex<'a> {
    /// How many elements enclose it.
    pub depth: usize,
    /// The namespace URIs its name may be in.
    pub namespaces: &'a [&'a str],
    /// Its local name.
    pub local: &'a str,
}

/// One open element.
struct Open {
    qname: String,
    /// The prefixes its start tag declared, to unbind as it closes.
    declared: Vec<String>,
    /// Its attributes in the `xml` namespace, by local name, kept only outside the subset.
    xml_attributes: Vec<(String, String)>,
}

/// An attribute of the start tag being read, its value normalised.
struct Attribute {
    prefix: String,
    local: String,
    value: String,
}

/// The canonical form of `apex` and its descendants in `octets`, handed to `out` in pieces.
///
/// # Errors
///
/// [`PolicyDocumentError::TooLarge`], [`PolicyDocumentError::NestedTooDeep`] and
/// [`PolicyDocumentError::TooManyEntries`] at the budgets; [`PolicyDocumentError::Xml`] for what
/// the module comment says is refused; and [`PolicyDocumentError::ElementMalformed`] naming the
/// apex where the document holds none.
pub fn canonicalize(
    octets: &[u8],
    apex: &Apex<'_>,
    comments: Comments,
    out: &mut dyn FnMut(&[u8]),
) -> Result<(), PolicyDocumentError> {
    if octets.len() > MAX_POLICY_OCTETS {
        return Err(PolicyDocumentError::TooLarge {
            octets: octets.len(),
        });
    }
    let body = octets.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(octets);
    let text = std::str::from_utf8(body).map_err(|_| refused("the document is not UTF-8"))?;
    let mut writer = Writer {
        budget: Budget(MAX_ENTRIES),
        open: Vec::new(),
        bindings: HashMap::new(),
        pending: Vec::new(),
        declarations: Vec::new(),
        subset: None,
        found: false,
        comments,
        out,
    };
    for token in xmlparser::Tokenizer::from(text) {
        let token = token.map_err(|error| PolicyDocumentError::Xml(error.to_string()))?;
        writer.token(token, apex)?;
    }
    if !writer.open.is_empty() {
        return Err(refused("the document's root element is not closed"));
    }
    if writer.found {
        Ok(())
    } else {
        Err(PolicyDocumentError::ElementMalformed("PolicyComponents"))
    }
}

/// [`PolicyDocumentError::Xml`] with a fixed reason.
fn refused(reason: &str) -> PolicyDocumentError {
    PolicyDocumentError::Xml(reason.to_owned())
}

/// The canonicaliser's state between tokens.
struct Writer<'o> {
    budget: Budget,
    open: Vec<Open>,
    /// Per prefix (empty for the default namespace), the URIs bound to it, innermost last; an
    /// empty URI is `xmlns=""`'s undeclaration.
    bindings: HashMap<String, Vec<String>>,
    /// The attributes of the start tag being read.
    pending: Vec<Attribute>,
    /// The namespace declarations of the start tag being read, prefix and URI.
    declarations: Vec<(String, String)>,
    /// How many elements were open when the apex's start tag was, while it is open.
    subset: Option<usize>,
    found: bool,
    comments: Comments,
    out: &'o mut dyn FnMut(&[u8]),
}

impl Writer<'_> {
    fn emit(&mut self, text: &str) {
        (self.out)(text.as_bytes());
    }

    fn inside(&self) -> bool {
        self.subset.is_some()
    }

    fn token(
        &mut self,
        token: xmlparser::Token<'_>,
        apex: &Apex<'_>,
    ) -> Result<(), PolicyDocumentError> {
        match token {
            xmlparser::Token::Declaration { encoding, .. } => {
                if encoding.is_some_and(|name| !name.as_str().eq_ignore_ascii_case("UTF-8")) {
                    return Err(refused(
                        "the document declares an encoding other than UTF-8, which this \
                         canonicaliser does not read",
                    ));
                }
            }
            xmlparser::Token::DtdStart { .. } | xmlparser::Token::EmptyDtd { .. } => {
                return Err(refused(
                    "the document declares a document type, which this reader does not process",
                ));
            }
            xmlparser::Token::ElementStart { prefix, local, .. } => {
                self.budget.take()?;
                if self.open.len() >= usize::from(der::MAX_DEPTH) {
                    return Err(PolicyDocumentError::NestedTooDeep);
                }
                let qname = if prefix.is_empty() {
                    local.as_str().to_owned()
                } else {
                    format!("{}:{}", prefix.as_str(), local.as_str())
                };
                self.open.push(Open {
                    qname,
                    declared: Vec::new(),
                    xml_attributes: Vec::new(),
                });
                self.pending.clear();
                self.declarations.clear();
            }
            xmlparser::Token::Attribute {
                prefix,
                local,
                value,
                ..
            } => self.attribute(prefix.as_str(), local.as_str(), value.as_str())?,
            xmlparser::Token::ElementEnd { end, .. } => match end {
                xmlparser::ElementEnd::Open => self.start_tag(apex)?,
                xmlparser::ElementEnd::Empty => {
                    self.start_tag(apex)?;
                    self.close()?;
                }
                xmlparser::ElementEnd::Close(prefix, local) => {
                    let matches = self.open.last().is_some_and(|element| {
                        let (written_prefix, written_local) = element
                            .qname
                            .split_once(':')
                            .unwrap_or(("", element.qname.as_str()));
                        written_prefix == prefix.as_str() && written_local == local.as_str()
                    });
                    if !matches {
                        return Err(refused("an element is closed by another element's end tag"));
                    }
                    self.close()?;
                }
            },
            xmlparser::Token::Text { text } => {
                if self.inside() {
                    let text = normalised(text.as_str(), false)?;
                    self.emit(&escaped_text(&text));
                }
            }
            xmlparser::Token::Cdata { text, .. } => {
                if self.inside() {
                    let text = line_breaks(text.as_str());
                    self.emit(&escaped_text(&text));
                }
            }
            xmlparser::Token::Comment { text, .. } => {
                if self.inside() && self.comments == Comments::Kept {
                    let text = line_breaks(text.as_str());
                    self.emit(&format!("<!--{text}-->"));
                }
            }
            xmlparser::Token::ProcessingInstruction {
                target,
                content,
                span,
            } => self.instruction(span.as_str(), target.as_str(), content.map(|c| c.as_str()))?,
            xmlparser::Token::EntityDeclaration { .. } | xmlparser::Token::DtdEnd { .. } => {}
        }
        Ok(())
    }

    /// A processing instruction, written where it is in the subset with one space before its data.
    ///
    /// XML 1.0 section 2.6, productions \[16\] and \[17\]: the target is a name other than `xml` in
    /// any case, and white space separates it from what follows. The tokenizer passes both, and
    /// the form written for either would not be read back as itself, so both are refused.
    fn instruction(
        &mut self,
        span: &str,
        target: &str,
        content: Option<&str>,
    ) -> Result<(), PolicyDocumentError> {
        if target.eq_ignore_ascii_case("xml") {
            return Err(refused(
                "a processing instruction's target is the reserved `xml`",
            ));
        }
        let after_target = span
            .strip_prefix("<?")
            .and_then(|rest| rest.strip_prefix(target))
            .and_then(|rest| rest.chars().next());
        if content.is_some() && !after_target.is_some_and(is_xml_space) {
            return Err(refused(
                "a processing instruction's target is not followed by white space",
            ));
        }
        if self.inside() {
            match content.map(line_breaks) {
                Some(content) => self.emit(&format!("<?{target} {content}?>")),
                None => self.emit(&format!("<?{target}?>")),
            }
        }
        Ok(())
    }

    /// One attribute of the start tag being read: a namespace declaration, charged to the entry
    /// budget and held to the constraints on it, or an attribute kept for the end of the tag.
    fn attribute(
        &mut self,
        prefix: &str,
        local: &str,
        raw: &str,
    ) -> Result<(), PolicyDocumentError> {
        let value = normalised(raw, true)?;
        if prefix == "xmlns" || (prefix.is_empty() && local == "xmlns") {
            self.budget.take()?;
            let declared = if prefix.is_empty() { "" } else { local };
            declaration_is_admitted(declared, &value)?;
            if self.declarations.iter().any(|(seen, _)| seen == declared) {
                return Err(refused("a start tag declares one prefix twice"));
            }
            self.declarations.push((declared.to_owned(), value));
        } else {
            self.pending.push(Attribute {
                prefix: prefix.to_owned(),
                local: local.to_owned(),
                value,
            });
        }
        Ok(())
    }

    /// The URI a prefix is bound to now; `None` where it is unbound or undeclared.
    fn bound(&self, prefix: &str) -> Option<&str> {
        self.bindings
            .get(prefix)
            .and_then(|uris| uris.last())
            .map(String::as_str)
            .filter(|uri| !uri.is_empty())
    }

    /// The end of a start tag: its declarations bound, and the tag rendered where it is in the
    /// subset. An empty-element tag is rendered as a start-tag and end-tag pair (section 2.3),
    /// the end tag written by [`Self::close`].
    fn start_tag(&mut self, apex: &Apex<'_>) -> Result<(), PolicyDocumentError> {
        let declarations = std::mem::take(&mut self.declarations);
        let attributes = std::mem::take(&mut self.pending);
        let depth = self.open.len().saturating_sub(1);
        // What the parent has in scope, read before this tag's declarations are bound.
        let parent: Vec<Option<String>> = declarations
            .iter()
            .map(|(prefix, _)| self.bound(prefix).map(str::to_owned))
            .collect();
        for (prefix, uri) in &declarations {
            self.bindings
                .entry(prefix.clone())
                .or_default()
                .push(uri.clone());
        }
        let element = self
            .open
            .last_mut()
            .ok_or_else(|| refused("an element is closed without being opened"))?;
        element.declared = declarations
            .iter()
            .map(|(prefix, _)| prefix.clone())
            .collect();
        let qname = element.qname.clone();
        let (prefix, local) = qname.split_once(':').unwrap_or(("", qname.as_str()));
        let namespace = match (self.bound(prefix), prefix) {
            (Some(uri), _) => uri.to_owned(),
            (None, "") => String::new(),
            (None, "xml") => XML_NAMESPACE.to_owned(),
            (None, _) => return Err(refused("an element's prefix is not declared")),
        };
        let resolved = self.resolved(attributes)?;
        let entering = self.subset.is_none()
            && !self.found
            && depth == apex.depth
            && local == apex.local
            && apex.namespaces.contains(&namespace.as_str());
        if !self.inside() && !entering {
            if let Some(element) = self.open.last_mut() {
                element.xml_attributes = resolved
                    .into_iter()
                    .filter(|(uri, _)| uri == XML_NAMESPACE)
                    .map(|(_, attribute)| (attribute.local, attribute.value))
                    .collect();
            }
            return Ok(());
        }
        let namespaces = if entering {
            self.subset = Some(depth);
            self.found = true;
            self.in_scope()
        } else {
            declarations
                .into_iter()
                .zip(parent)
                .filter_map(|((prefix, uri), parent)| {
                    let differs = if uri.is_empty() {
                        parent.is_some()
                    } else {
                        parent.as_deref() != Some(uri.as_str())
                    };
                    (differs && prefix != "xml").then_some((prefix, uri))
                })
                .collect()
        };
        self.render(&qname, namespaces, resolved, entering);
        Ok(())
    }

    /// A start tag's attributes with their namespace URIs, in section 2.2's order; one expanded
    /// name stated twice is refused (XML Namespaces 1.0 section 6.3).
    fn resolved(
        &self,
        attributes: Vec<Attribute>,
    ) -> Result<Vec<(String, Attribute)>, PolicyDocumentError> {
        let mut resolved = Vec::with_capacity(attributes.len());
        for attribute in attributes {
            let uri = match attribute.prefix.as_str() {
                "" => String::new(),
                "xml" => XML_NAMESPACE.to_owned(),
                other => self
                    .bound(other)
                    .ok_or_else(|| refused("an attribute's prefix is not declared"))?
                    .to_owned(),
            };
            resolved.push((uri, attribute));
        }
        resolved.sort_by(|(a_uri, a), (b_uri, b)| (a_uri, &a.local).cmp(&(b_uri, &b.local)));
        if resolved
            .windows(2)
            .any(|pair| pair[0].0 == pair[1].0 && pair[0].1.local == pair[1].1.local)
        {
            return Err(refused("a start tag states one attribute twice"));
        }
        Ok(resolved)
    }

    /// One start tag of the subset written: its namespace declarations by prefix, then its
    /// attributes, the apex's with section 2.4's inherited `xml` attributes merged in.
    fn render(
        &mut self,
        qname: &str,
        mut namespaces: Vec<(String, String)>,
        resolved: Vec<(String, Attribute)>,
        apex: bool,
    ) {
        namespaces.sort();
        let mut attributes: Vec<(String, String, String)> = resolved
            .into_iter()
            .map(|(uri, attribute)| {
                let qname = if attribute.prefix.is_empty() {
                    attribute.local.clone()
                } else {
                    format!("{}:{}", attribute.prefix, attribute.local)
                };
                let value = escaped_attribute(&attribute.value);
                (uri, attribute.local, format!("{qname}=\"{value}\""))
            })
            .collect();
        if apex {
            let own: HashSet<String> = attributes
                .iter()
                .filter(|(uri, _, _)| uri == XML_NAMESPACE)
                .map(|(_, local, _)| local.clone())
                .collect();
            for (local, value) in self.inherited_xml_attributes() {
                if !own.contains(&local) {
                    let rendered = format!("xml:{local}=\"{}\"", escaped_attribute(&value));
                    attributes.push((XML_NAMESPACE.to_owned(), local, rendered));
                }
            }
            attributes.sort_by(|(a_uri, a, _), (b_uri, b, _)| (a_uri, a).cmp(&(b_uri, b)));
        }
        let mut tag = format!("<{qname}");
        for (prefix, uri) in &namespaces {
            let colon = if prefix.is_empty() { "" } else { ":" };
            let uri = escaped_attribute(uri);
            tag.push_str(" xmlns");
            tag.push_str(colon);
            tag.push_str(prefix);
            tag.push_str("=\"");
            tag.push_str(&uri);
            tag.push('"');
        }
        for (_, _, rendered) in &attributes {
            tag.push(' ');
            tag.push_str(rendered);
        }
        tag.push('>');
        self.emit(&tag);
    }

    /// Every namespace in scope that the apex renders: each prefix's innermost binding, where it
    /// is a namespace rather than `xmlns=""`, `xml` aside.
    fn in_scope(&self) -> Vec<(String, String)> {
        self.bindings
            .iter()
            .filter(|(prefix, _)| prefix.as_str() != "xml")
            .filter_map(|(prefix, uris)| {
                uris.last()
                    .filter(|uri| !uri.is_empty())
                    .map(|uri| (prefix.clone(), uri.clone()))
            })
            .collect()
    }

    /// Section 2.4's attributes for the apex: the nearest of each `xml` attribute an ancestor
    /// carries, the innermost first.
    fn inherited_xml_attributes(&self) -> Vec<(String, String)> {
        let ancestors = self.open.len().saturating_sub(1);
        let mut seen: HashSet<&str> = HashSet::new();
        let mut found: Vec<(String, String)> = Vec::new();
        for element in self.open.iter().take(ancestors).rev() {
            for (local, value) in &element.xml_attributes {
                if seen.insert(local.as_str()) {
                    found.push((local.clone(), value.clone()));
                }
            }
        }
        found
    }

    /// An element's close: its end tag rendered where it is in the subset, its prefixes unbound.
    fn close(&mut self) -> Result<(), PolicyDocumentError> {
        let element = self
            .open
            .pop()
            .ok_or_else(|| refused("an element is closed without being opened"))?;
        if self.inside() {
            self.emit(&format!("</{}>", element.qname));
        }
        if self.subset == Some(self.open.len()) {
            self.subset = None;
        }
        for prefix in &element.declared {
            if let Some(uris) = self.bindings.get_mut(prefix) {
                uris.pop();
            }
        }
        Ok(())
    }
}

/// XML Namespaces 1.0 section 3's constraints on a declaration, and Canonical XML 1.0 section
/// 2.1's note on a relative namespace URI.
fn declaration_is_admitted(prefix: &str, uri: &str) -> Result<(), PolicyDocumentError> {
    match prefix {
        "xmlns" => return Err(refused("the prefix xmlns is declared")),
        "xml" if uri != XML_NAMESPACE => {
            return Err(refused("the prefix xml is bound to another namespace"));
        }
        "" if uri.is_empty() => return Ok(()),
        _ if uri.is_empty() => return Err(refused("a prefix is declared with an empty namespace")),
        _ => {}
    }
    if prefix != "xml" && (uri == XML_NAMESPACE || uri == "http://www.w3.org/2000/xmlns/") {
        return Err(refused("a reserved namespace is bound to another prefix"));
    }
    if is_absolute(uri) {
        Ok(())
    } else {
        Err(refused(
            "a namespace URI is relative, which Canonical XML 1.0 section 2.1 refuses",
        ))
    }
}

/// Whether a URI reference opens with RFC 3986 section 3.1's scheme and its colon.
fn is_absolute(uri: &str) -> bool {
    let Some((scheme, _)) = uri.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// XML 1.0 section 2.3's production \[3\], `S`: space, tab, carriage return and line feed.
fn is_xml_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// XML 1.0 section 2.11's line-end handling: each #xD #xA pair and each #xD alone become #xA.
fn line_breaks(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Raw character data or an attribute's raw value as the data model holds it: line breaks
/// handled; in an attribute each white space character written literally made a space (XML 1.0
/// section 3.3.3, every value CDATA without a declaration); and references replaced.
fn normalised(raw: &str, attribute: bool) -> Result<String, PolicyDocumentError> {
    let raw = line_breaks(raw);
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw.as_str();
    while let Some(at) = rest.find(['&', '\n', '\t']) {
        let (before, after) = rest.split_at(at);
        out.push_str(before);
        let Some(first) = after.chars().next() else {
            break;
        };
        if first != '&' {
            out.push(if attribute { ' ' } else { first });
            rest = &after[first.len_utf8()..];
            continue;
        }
        let end = after
            .find(';')
            .ok_or_else(|| refused("a reference is not closed by ;"))?;
        out.push(referenced(&after[1..end])?);
        rest = &after[end.saturating_add(1)..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The character one reference names: one of XML 1.0 section 4.6's five entities, or a character
/// reference to a character section 2.2's `Char` admits.
fn referenced(name: &str) -> Result<char, PolicyDocumentError> {
    let named = match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    };
    if let Some(character) = named {
        return Ok(character);
    }
    let code = if let Some(hex) = name.strip_prefix("#x") {
        (!hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| u32::from_str_radix(hex, 16).ok())
            .flatten()
    } else if let Some(decimal) = name.strip_prefix('#') {
        (!decimal.is_empty() && decimal.bytes().all(|b| b.is_ascii_digit()))
            .then(|| decimal.parse::<u32>().ok())
            .flatten()
    } else {
        return Err(refused(
            "a reference names an entity no declaration defines",
        ));
    };
    code.and_then(char::from_u32)
        .filter(|&c| {
            matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && !matches!(c, '\u{FFFE}' | '\u{FFFF}'))
        })
        .ok_or_else(|| refused("a character reference names no character XML admits"))
}

/// Section 2.3's escapes for a text node.
fn escaped_text(text: &str) -> String {
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

/// Section 2.3's escapes for an attribute value.
fn escaped_attribute(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests;
