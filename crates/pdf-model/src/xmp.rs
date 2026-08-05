//! ISO 32000-2 §14.3.2's metadata streams: XMP, read.
//!
//! > The contents of a metadata stream shall be the metadata represented in Extensible Markup
//! > Language (XML) and the grammar of the XML representing the metadata shall be defined
//! > according to the extensible metadata platform specification (ISO 16684-1).
//!
//! That sentence is why this module exists and why it took until the two-hundred-and-ninety-fourth
//! session to write: reading it is an XML parser over untrusted bytes, which is a dependency
//! decision rather than a reading. ADR 0186 takes it. `xmlparser` is a pull tokenizer with no
//! dependencies at all, `#![forbid(unsafe_code)]`, that resolves no entity and opens no file — so
//! the two attacks XML is famous for, the billion-laughs expansion and the external entity, have
//! nothing to work with. What is left to bound is this module's own stack and allocation, which
//! the four constants below do.
//!
//! # What a metadata stream is, structurally
//!
//! An XMP packet is RDF/XML narrowed to one shape. Properties live on or under
//! `rdf:Description` elements, and ISO 16684-1 section 7.5 gives a simple property two spellings that
//! mean the same thing — an attribute on the description, or a child element:
//!
//! ```xml
//! <rdf:Description rdf:about="" pdf:Producer="An exporter"/>
//! <rdf:Description rdf:about=""><pdf:Producer>An exporter</pdf:Producer></rdf:Description>
//! ```
//!
//! and three container forms, `rdf:Alt`, `rdf:Seq` and `rdf:Bag`, whose items are `rdf:li`
//! elements. A language alternative is an `rdf:Alt` whose items carry `xml:lang`; `x-default` is
//! ISO 16684-1 section 8.2.2.4's name for the one to show when nothing better is known, and it is what
//! §12.2's `/DisplayDocTitle` ends up asking for.
//!
//! # A prefix is not a name
//!
//! `dc:title` is not a name; `{http://purl.org/dc/elements/1.1/}title` is. XML lets a document
//! bind any prefix to any namespace, and XMP packets in the wild do — `<pdfaid:part>` and
//! `<pdfaId:part>` are the same property. So every element and attribute name here is resolved
//! through the `xmlns` bindings in scope before it is compared with anything, and [`Xmp::text`]
//! takes a namespace **URI**. The prefixes are constants ([`DC`], [`PDF`], [`XMP`]) so that a
//! caller never spells one.
//!
//! # What is read and what is deliberately not
//!
//! Simple properties in both spellings, and all three containers. A property whose value is a
//! *structure* — ISO 16684-1 section 7.6, an `rdf:parseType="Resource"` or a nested `rdf:Description` —
//! is recorded as [`Value::Structure`]: the property is reported as present and its value is
//! reported as uninterpreted, which is the difference between a gap and a silence. Nothing in
//! clause 12 or 14 asks for one; `xmpMM:DerivedFrom` and `xmpTPg:MaxPageSize` are the common
//! ones and neither reaches a pixel.
//!
//! Qualifiers other than `xml:lang` (ISO 16684-1 section 7.7) are dropped, which is the same statement:
//! the property keeps its value and loses an annotation on it.

use pdf_syntax::{Dictionary, Document};

/// The Dublin Core namespace, which carries `dc:title`, `dc:creator` and `dc:description`.
pub const DC: &str = "http://purl.org/dc/elements/1.1/";
/// Adobe's PDF schema: `pdf:Producer`, `pdf:Keywords`, `pdf:Trapped`.
pub const PDF: &str = "http://ns.adobe.com/pdf/1.3/";
/// The XMP basic schema: `xmp:CreatorTool`, `xmp:CreateDate`, `xmp:ModifyDate`.
pub const XMP: &str = "http://ns.adobe.com/xap/1.0/";
/// The RDF syntax namespace, whose `Description`, `Alt`, `Seq`, `Bag` and `li` are the grammar.
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
/// The XML namespace, bound to the `xml` prefix by the XML specification itself and never
/// declared. Its `lang` attribute is what makes an `rdf:Alt` a language alternative.
const XML: &str = "http://www.w3.org/XML/1998/namespace";

/// The largest metadata stream this module will look at, decoded.
///
/// The clause states no limit and a stream is arbitrary compressed data, so this is a
/// decompression-bomb bound rather than a reading (principle 3). Measured: the largest of the
/// 319 corpus streams decodes to 78 121 bytes, so 8 MiB is a hundred times the largest packet
/// anyone here writes.
const MAX_BYTES: usize = 8 << 20;

/// The deepest element nesting this module will follow.
///
/// The tokenizer is iterative, so this bounds *this* module's stack vector rather than the
/// parser's. XMP's own grammar is five deep at its worst — `rdf:RDF`, `rdf:Description`, a
/// property, a container, an `rdf:li` — and a structured value adds two.
const MAX_DEPTH: usize = 64;

/// Most properties one packet may state.
const MAX_PROPERTIES: usize = 4096;

/// Most items one container may hold.
const MAX_ITEMS: usize = 4096;

/// Most bytes one property value may carry.
///
/// `dc:description` is prose and `pdf:Keywords` can be a paragraph, so this is generous; what it
/// refuses is a packet that spends a megabyte on one string.
const MAX_VALUE_BYTES: usize = 1 << 20;

/// Why a metadata stream could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum XmpError {
    /// The stream would not decode — a filter this tree refuses, or a missing decryption key.
    #[error("the metadata stream would not decode")]
    Undecodable,
    /// The packet is larger than [`MAX_BYTES`].
    #[error("the metadata stream is {bytes} bytes, past this reader's {MAX_BYTES}-byte bound")]
    TooLarge {
        /// What the stream decoded to.
        bytes: usize,
    },
    /// The bytes are not text in any encoding ISO 16684-1 section 7.3.2 permits.
    #[error("the metadata stream is not UTF-8, UTF-16 or UTF-32 text")]
    NotText,
    /// The XML is malformed, at a line and column of the decoded packet.
    #[error("malformed XML at {line}:{column}: {detail}")]
    Malformed {
        /// The line the tokenizer stopped on, counting from one.
        line: u32,
        /// The column it stopped at, counting from one.
        column: u32,
        /// What it said.
        detail: String,
    },
    /// A close tag names an element other than the one it closes, or an element is never closed.
    ///
    /// A separate variant because it is a separate reader's finding: `xmlparser` is a
    /// *tokenizer*, so it checks a tag's syntax and not the document's tree, and nothing but
    /// this module notices that `<a></b>` is not XML.
    #[error("unbalanced XML: {detail}")]
    Unbalanced {
        /// Which element, and what closed it.
        detail: String,
    },
    /// One of this module's four budgets was reached.
    #[error("the packet exceeds this reader's bound on {what}")]
    TooMuch {
        /// Which bound: `"nesting depth"`, `"properties"`, `"array items"`, `"value length"`.
        what: &'static str,
    },
}

/// A property's value, in the three shapes ISO 16684-1 gives one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A simple property: one string, whatever the spelling it arrived in.
    Text(String),
    /// `rdf:Alt`, whose items are alternatives and whose first is the default.
    ///
    /// Each is its `xml:lang` and its text. A language alternative that states no `xml:lang` is
    /// legal RDF and gets `None`, and [`Xmp::text`] treats it as the default.
    Alt(Vec<(Option<String>, String)>),
    /// `rdf:Seq`, an ordered array.
    Seq(Vec<String>),
    /// `rdf:Bag`, an unordered array. Kept distinct from [`Value::Seq`] because the file said so.
    Bag(Vec<String>),
    /// ISO 16684-1 section 7.6's structured value, present and not interpreted. See the module comment.
    Structure,
}

/// A resolved property name: a namespace URI and a local name, never a prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The namespace URI the element's prefix was bound to, empty for an unqualified name.
    pub namespace: String,
    /// The local part.
    pub local: String,
}

/// One document's or object's XMP, as a list of properties in the order the packet states them.
///
/// A list rather than a map: a packet may state one property twice — two `rdf:Description`
/// elements about the same subject is the ordinary way to split schemas — and a map would drop
/// the second silently. Lookup is linear over a few dozen entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Xmp {
    properties: Vec<(Name, Value)>,
}

impl Xmp {
    /// Reads the `/Metadata` stream of a catalog, page, or any of Table 348's other components.
    ///
    /// `None` where the dictionary states no `/Metadata` — the entry is optional everywhere it
    /// appears. Table 347's `/Type /Metadata` and `/Subtype /XML` are *required* entries, and a
    /// stream that omits them is still read: the requirement binds a writer, and refusing to read
    /// a packet whose bytes are plainly XMP because its dictionary is short of a name would be
    /// this reader inventing a rule. Measured, and the measurement is why the sentence is short:
    /// all 319 corpus streams state both entries correctly, so nothing here depends on it.
    ///
    /// # Errors
    ///
    /// [`XmpError`] where the stream is present and could not be read, which is the case worth
    /// reporting: a document that carries metadata this program cannot parse should say so.
    pub fn read(document: &Document, dictionary: &Dictionary) -> Option<Result<Self, XmpError>> {
        let object = document.get_key(dictionary, "Metadata");
        let stream = object.as_stream()?;
        Some(match document.decoded_stream_data(stream) {
            Some(bytes) => Self::parse(&bytes),
            None => Err(XmpError::Undecodable),
        })
    }

    /// The document-level packet, from the catalog's `/Metadata` (§7.7.2, Table 29).
    ///
    /// # Errors
    ///
    /// As [`Xmp::read`].
    pub fn document(document: &Document) -> Option<Result<Self, XmpError>> {
        let catalog = document.catalog().ok()?;
        Self::read(document, &catalog)
    }

    /// Parses a packet's bytes.
    ///
    /// # Errors
    ///
    /// [`XmpError`] for a packet that is too large, is not text, is not well-formed XML, or
    /// exceeds one of this module's four budgets.
    pub fn parse(bytes: &[u8]) -> Result<Self, XmpError> {
        if bytes.len() > MAX_BYTES {
            return Err(XmpError::TooLarge { bytes: bytes.len() });
        }
        let text = decode(bytes)?;
        Reader::new().run(&text)
    }

    /// Every property, in the order the packet states them.
    #[must_use]
    pub fn properties(&self) -> &[(Name, Value)] {
        &self.properties
    }

    /// The value of one property, by namespace URI and local name.
    #[must_use]
    pub fn value(&self, namespace: &str, local: &str) -> Option<&Value> {
        self.properties
            .iter()
            .find(|(name, _)| name.namespace == namespace && name.local == local)
            .map(|(_, value)| value)
    }

    /// One property as the single string a caller wants to show.
    ///
    /// A simple property gives its text. A language alternative gives `x-default` where the
    /// packet states one, and otherwise its first item — which is the order ISO 16684-1
    /// section 8.2.2.4 puts them in, its first item being the default. An array gives its first
    /// item; a structure gives nothing, because it has no single string to give.
    #[must_use]
    pub fn text(&self, namespace: &str, local: &str) -> Option<&str> {
        match self.value(namespace, local)? {
            Value::Text(text) => Some(text.as_str()),
            Value::Alt(items) => items
                .iter()
                .find(|(lang, _)| lang.as_deref() == Some("x-default"))
                .or_else(|| items.first())
                .map(|(_, text)| text.as_str()),
            Value::Seq(items) | Value::Bag(items) => items.first().map(String::as_str),
            Value::Structure => None,
        }
    }

    /// One property in a particular language, where it is a language alternative that has it.
    ///
    /// The match is on the exact `xml:lang` the packet wrote, not on RFC 4647's lookup: a
    /// fallback from `en-GB` to `en` is a policy a *host* has, and inventing one here would put
    /// it out of that host's reach.
    #[must_use]
    pub fn text_in(&self, namespace: &str, local: &str, language: &str) -> Option<&str> {
        match self.value(namespace, local)? {
            Value::Alt(items) => items
                .iter()
                .find(|(lang, _)| lang.as_deref() == Some(language))
                .map(|(_, text)| text.as_str()),
            _ => None,
        }
    }

    /// One property's items, where it is an array.
    #[must_use]
    pub fn items(&self, namespace: &str, local: &str) -> Option<&[String]> {
        match self.value(namespace, local)? {
            Value::Seq(items) | Value::Bag(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    /// `dc:title`, which is what §12.2's `/DisplayDocTitle` names.
    ///
    /// > A flag specifying whether the window's title bar should display the document title taken
    /// > from the `dc:title` element of the XMP metadata stream (see 14.3.2, "Metadata streams").
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.text(DC, "title")
    }

    /// `dc:creator`, the XMP counterpart Table 349's NOTE gives `/Author`.
    ///
    /// A `Seq` in XMP against one string in the dictionary, which is §14.3.1's whole argument for
    /// the stream: "a document's authors can be represented as a list".
    #[must_use]
    pub fn authors(&self) -> Option<&[String]> {
        self.items(DC, "creator")
    }

    /// `dc:description`, the counterpart of `/Subject`.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.text(DC, "description")
    }

    /// `pdf:Producer`, the counterpart of `/Producer`.
    #[must_use]
    pub fn producer(&self) -> Option<&str> {
        self.text(PDF, "Producer")
    }

    /// `pdf:Keywords`, the counterpart of `/Keywords`.
    #[must_use]
    pub fn keywords(&self) -> Option<&str> {
        self.text(PDF, "Keywords")
    }

    /// `xmp:CreatorTool`, the counterpart of `/Creator`.
    #[must_use]
    pub fn creator_tool(&self) -> Option<&str> {
        self.text(XMP, "CreatorTool")
    }

    /// `xmp:CreateDate`, the counterpart of `/CreationDate`.
    ///
    /// The string as written. XMP dates are ISO 8601 and §7.9.4's are not, so this is *not* a
    /// [`pdf_syntax::Date`] and is deliberately not converted into one: they are two grammars,
    /// and a reader that silently reshaped one into the other would be answering a question
    /// about the file with a guess.
    #[must_use]
    pub fn created(&self) -> Option<&str> {
        self.text(XMP, "CreateDate")
    }

    /// `xmp:ModifyDate`, the counterpart of `/ModDate`. As [`Xmp::created`].
    #[must_use]
    pub fn modified(&self) -> Option<&str> {
        self.text(XMP, "ModifyDate")
    }

    /// Whether the packet stated nothing this reader understood.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }
}

/// The packet's bytes as text, in whichever of the three encodings ISO 16684-1 section 7.3.2
/// permits — UTF-8, UTF-16 or UTF-32. Paraphrased rather than quoted, because a verbatim
/// sentence in this tree is one the conformance checker verifies against `doc/md/`, and that
/// directory holds ISO 32000-2 and not this standard.
///
/// UTF-8 is what every one of the 319 corpus streams uses and what the `<?xpacket>` header's
/// `begin` attribute signals by carrying U+FEFF in the packet's own encoding. The other two are
/// decoded here rather than refused, because refusing a spelling the clause permits is a gap
/// dressed as a limit — and both are twenty lines.
fn decode(bytes: &[u8]) -> Result<String, XmpError> {
    // A byte-order mark decides between them; the clause's own signalling is exactly this.
    match bytes {
        [0xFF, 0xFE, 0x00, 0x00, rest @ ..] => decode_32(rest, u32::from_le_bytes),
        [0x00, 0x00, 0xFE, 0xFF, rest @ ..] => decode_32(rest, u32::from_be_bytes),
        [0xFF, 0xFE, rest @ ..] => decode_16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => decode_16(rest, u16::from_be_bytes),
        [0xEF, 0xBB, 0xBF, rest @ ..] => {
            String::from_utf8(rest.to_vec()).map_err(|_| XmpError::NotText)
        }
        _ => String::from_utf8(bytes.to_vec()).map_err(|_| XmpError::NotText),
    }
}

/// UTF-16, either byte order, with the surrogate pairs joined.
fn decode_16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> Result<String, XmpError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(XmpError::NotText);
    }
    let units = bytes.chunks_exact(2).map(|pair| match pair {
        &[a, b] => unit([a, b]),
        // `chunks_exact(2)` yields nothing else; the arm exists because the slice pattern is
        // not exhaustive to the compiler.
        _ => 0,
    });
    char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .map_err(|_| XmpError::NotText)
}

/// UTF-32, either byte order.
fn decode_32(bytes: &[u8], unit: fn([u8; 4]) -> u32) -> Result<String, XmpError> {
    if !bytes.len().is_multiple_of(4) {
        return Err(XmpError::NotText);
    }
    bytes
        .chunks_exact(4)
        .map(|quad| match quad {
            &[a, b, c, d] => char::from_u32(unit([a, b, c, d])).ok_or(XmpError::NotText),
            _ => Err(XmpError::NotText),
        })
        .collect()
}

/// What an element is, decided when it opens from what its parent is.
///
/// The grammar is shallow enough that this is the whole of it: RDF/XML's generality is not
/// reachable from a packet ISO 16684-1 describes, so an element that is not one of these is one
/// whose content this reader does not interpret.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    /// Anything above `rdf:RDF` — `x:xmpmeta`, or the packet's own root.
    Outside,
    /// `rdf:RDF`.
    Rdf,
    /// `rdf:Description`.
    Description,
    /// A property element, directly under a description.
    ///
    /// `structured` is ISO 16684-1 section 7.6's `rdf:parseType="Resource"`, which says the value is a
    /// structure *before* any child arrives — and saying so up front is the only way a
    /// self-closing structured property is distinguishable from an empty simple one.
    Property { name: Name, structured: bool },
    /// `rdf:Alt`, `rdf:Seq` or `rdf:Bag` under a property.
    Container { ordered: Container },
    /// `rdf:li` under a container.
    Item { language: Option<String> },
    /// An element whose content is not interpreted, and what to record for it.
    Uninterpreted,
}

/// Which of ISO 16684-1's three array forms a container is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Container {
    Alt,
    Seq,
    Bag,
}

/// One open element.
#[derive(Debug)]
struct Frame {
    /// The prefix and local name as the file spelled them, kept only so that a close tag can be
    /// checked against the tag it closes — see [`XmpError::Unbalanced`].
    tag: (String, String),
    kind: Kind,
    /// How many namespace bindings this element declared, popped when it closes.
    bindings: usize,
    /// The character content seen so far, unescaped.
    text: String,
    /// A property's value, where a child element has already decided it.
    value: Option<Value>,
    /// A container's items so far.
    items: Vec<(Option<String>, String)>,
}

/// The walk over one packet.
struct Reader {
    stack: Vec<Frame>,
    /// Prefix-to-URI bindings, innermost last. A prefix rebound in a child shadows its parent's,
    /// which is why this is a stack searched backwards rather than a map.
    bindings: Vec<(String, String)>,
    properties: Vec<(Name, Value)>,
}

impl Reader {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            bindings: Vec::new(),
            properties: Vec::new(),
        }
    }

    fn run(mut self, text: &str) -> Result<Xmp, XmpError> {
        // Attributes arrive as their own tokens between `ElementStart` and `ElementEnd`, so an
        // element's name cannot be resolved until they have all been seen: a namespace an
        // element uses may be declared by that same element.
        let mut pending: Option<(String, String)> = None;
        let mut attributes: Vec<(String, String, String)> = Vec::new();

        for token in xmlparser::Tokenizer::from(text) {
            let token = token.map_err(|error| {
                let at = error.pos();
                XmpError::Malformed {
                    line: at.row,
                    column: at.col,
                    detail: error.to_string(),
                }
            })?;
            match token {
                xmlparser::Token::ElementStart { prefix, local, .. } => {
                    pending = Some((prefix.to_string(), local.to_string()));
                    attributes.clear();
                }
                xmlparser::Token::Attribute {
                    prefix,
                    local,
                    value,
                    ..
                } => {
                    attributes.push((prefix.to_string(), local.to_string(), value.to_string()));
                }
                xmlparser::Token::ElementEnd { end, .. } => match end {
                    xmlparser::ElementEnd::Open => {
                        let Some(name) = pending.take() else { continue };
                        self.open(name, &attributes)?;
                    }
                    xmlparser::ElementEnd::Empty => {
                        let Some(name) = pending.take() else { continue };
                        let tag = name.clone();
                        self.open(name, &attributes)?;
                        self.close(&tag)?;
                    }
                    xmlparser::ElementEnd::Close(prefix, local) => {
                        self.close(&(prefix.to_string(), local.to_string()))?;
                    }
                },
                xmlparser::Token::Text { text } | xmlparser::Token::Cdata { text, .. } => {
                    if let Some(frame) = self.stack.last_mut() {
                        if frame.text.len() > MAX_VALUE_BYTES {
                            return Err(XmpError::TooMuch {
                                what: "value length",
                            });
                        }
                        unescape(text.as_str(), &mut frame.text);
                    }
                }
                // A declaration, a processing instruction (`<?xpacket>` is one), a comment and
                // the DTD carry no property. An `<!ENTITY>` declaration arriving here and being
                // dropped is the billion-laughs defence, stated: nothing substitutes it.
                _ => {}
            }
        }
        if let Some(frame) = self.stack.last() {
            return Err(XmpError::Unbalanced {
                detail: format!("<{}> is never closed", spelled(&frame.tag)),
            });
        }
        Ok(Xmp {
            properties: self.properties,
        })
    }

    /// Resolves a prefix against the bindings in scope, innermost first.
    fn namespace(&self, prefix: &str) -> String {
        if prefix == "xml" {
            return XML.to_owned();
        }
        self.bindings
            .iter()
            .rev()
            .find(|(bound, _)| bound == prefix)
            .map_or_else(String::new, |(_, uri)| uri.clone())
    }

    /// Opens an element: installs its namespace bindings, decides what it is, and — for a
    /// description — reads ISO 16684-1 section 7.5's attribute spelling of a simple property.
    fn open(
        &mut self,
        name: (String, String),
        attributes: &[(String, String, String)],
    ) -> Result<(), XmpError> {
        if self.stack.len() >= MAX_DEPTH {
            return Err(XmpError::TooMuch {
                what: "nesting depth",
            });
        }

        // Bindings first: an element may declare the namespace its own prefix uses.
        let mut bindings: usize = 0;
        for (prefix, local, value) in attributes {
            match (prefix.as_str(), local.as_str()) {
                ("", "xmlns") => {
                    self.bindings.push((String::new(), value.clone()));
                    bindings = bindings.saturating_add(1);
                }
                ("xmlns", _) => {
                    self.bindings.push((local.clone(), value.clone()));
                    bindings = bindings.saturating_add(1);
                }
                _ => {}
            }
        }

        let namespace = self.namespace(&name.0);
        let local = name.1.as_str();
        let parent = self.stack.last().map(|frame| &frame.kind);
        let kind = self.classify(&namespace, local, attributes, parent);

        // §7.5's attribute form: every attribute of a description that is neither a namespace
        // declaration nor RDF's own is a simple property of it.
        if kind == Kind::Description {
            for (prefix, local, value) in attributes {
                if prefix == "xmlns" || (prefix.is_empty() && local == "xmlns") {
                    continue;
                }
                let namespace = self.namespace(prefix);
                if namespace == RDF || namespace == XML || prefix.is_empty() {
                    continue;
                }
                self.record(
                    Name {
                        namespace,
                        local: local.clone(),
                    },
                    // An attribute value is no more unescaped than character content is.
                    Value::Text(unescaped(value)),
                )?;
            }
        }

        self.stack.push(Frame {
            tag: name,
            kind,
            bindings,
            text: String::new(),
            value: None,
            items: Vec::new(),
        });
        Ok(())
    }

    /// What an element is, from its resolved name and what contains it.
    fn classify(
        &self,
        namespace: &str,
        local: &str,
        attributes: &[(String, String, String)],
        parent: Option<&Kind>,
    ) -> Kind {
        let rdf = namespace == RDF;
        match parent {
            None | Some(Kind::Outside) => {
                if rdf && local == "RDF" {
                    Kind::Rdf
                } else {
                    Kind::Outside
                }
            }
            Some(Kind::Rdf) => {
                if rdf && local == "Description" {
                    Kind::Description
                } else {
                    Kind::Uninterpreted
                }
            }
            Some(Kind::Description) => Kind::Property {
                name: Name {
                    namespace: namespace.to_owned(),
                    local: local.to_owned(),
                },
                structured: attributes.iter().any(|(prefix, local, value)| {
                    self.namespace(prefix) == RDF && local == "parseType" && value == "Resource"
                }),
            },
            Some(Kind::Property { .. }) => match (rdf, local) {
                (true, "Alt") => Kind::Container {
                    ordered: Container::Alt,
                },
                (true, "Seq") => Kind::Container {
                    ordered: Container::Seq,
                },
                (true, "Bag") => Kind::Container {
                    ordered: Container::Bag,
                },
                _ => Kind::Uninterpreted,
            },
            Some(Kind::Container { .. }) => {
                if rdf && local == "li" {
                    let language = attributes
                        .iter()
                        .find(|(prefix, local, _)| self.namespace(prefix) == XML && local == "lang")
                        .map(|(_, _, value)| unescaped(value));
                    Kind::Item { language }
                } else {
                    Kind::Uninterpreted
                }
            }
            Some(Kind::Item { .. } | Kind::Uninterpreted) => Kind::Uninterpreted,
        }
    }

    /// Closes the innermost element, handing its value to whatever contains it.
    fn close(&mut self, tag: &(String, String)) -> Result<(), XmpError> {
        let Some(frame) = self.stack.pop() else {
            return Err(XmpError::Unbalanced {
                detail: format!("</{}> closes nothing", spelled(tag)),
            });
        };
        if frame.tag != *tag {
            return Err(XmpError::Unbalanced {
                detail: format!("<{}> is closed by </{}>", spelled(&frame.tag), spelled(tag)),
            });
        }
        self.bindings
            .truncate(self.bindings.len().saturating_sub(frame.bindings));

        match frame.kind {
            Kind::Item { language } => {
                if let Some(container) = self.stack.last_mut() {
                    if container.items.len() >= MAX_ITEMS {
                        return Err(XmpError::TooMuch {
                            what: "array items",
                        });
                    }
                    container.items.push((language, trimmed(&frame.text)));
                }
            }
            Kind::Container { ordered } => {
                let value = match ordered {
                    Container::Alt => Value::Alt(frame.items),
                    Container::Seq => {
                        Value::Seq(frame.items.into_iter().map(|(_, text)| text).collect())
                    }
                    Container::Bag => {
                        Value::Bag(frame.items.into_iter().map(|(_, text)| text).collect())
                    }
                };
                if let Some(property) = self.stack.last_mut() {
                    property.value = Some(value);
                }
            }
            Kind::Property { name, structured } => {
                let value = frame.value.unwrap_or_else(|| {
                    if structured {
                        Value::Structure
                    } else {
                        Value::Text(trimmed(&frame.text))
                    }
                });
                self.record(name, value)?;
            }
            // An uninterpreted element under a property is ISO 16684-1 section 7.6's structured value:
            // the property is present and this reader does not read what it holds. Recording it
            // as such is the difference between a gap and a silence.
            Kind::Uninterpreted => {
                if matches!(
                    self.stack.last().map(|frame| &frame.kind),
                    Some(Kind::Property { .. })
                ) && let Some(property) = self.stack.last_mut()
                {
                    property.value = Some(Value::Structure);
                }
            }
            Kind::Outside | Kind::Rdf | Kind::Description => {}
        }
        Ok(())
    }

    fn record(&mut self, name: Name, value: Value) -> Result<(), XmpError> {
        if self.properties.len() >= MAX_PROPERTIES {
            return Err(XmpError::TooMuch { what: "properties" });
        }
        self.properties.push((name, value));
        Ok(())
    }
}

/// A tag as the file spelled it, for an error message.
fn spelled(tag: &(String, String)) -> String {
    if tag.0.is_empty() {
        tag.1.clone()
    } else {
        format!("{}:{}", tag.0, tag.1)
    }
}

/// A simple property's text, with the white space a pretty-printer added taken off.
///
/// ISO 16684-1 section 7.4 makes an element's character content the value, and every XMP writer
/// indents. Trimming the outside and keeping the inside is what the Adobe toolkit does and what
/// a `<dc:title>\n  Report\n</dc:title>` plainly means; a value whose leading space matters
/// cannot be expressed either way, which is a property of the format rather than a choice here.
fn trimmed(text: &str) -> String {
    text.trim().to_owned()
}

/// Expands the entity references XML defines, appending to `out`.
///
/// Five predefined general entities and numeric character references, which is everything a
/// packet may use: the tokenizer resolves nothing, and a *declared* entity is dropped with its
/// declaration (see the module comment). An undefined reference is kept verbatim, because
/// dropping it would silently delete text a producer wrote and erroring would refuse a whole
/// packet over one ampersand.
pub(crate) fn unescape(text: &str, out: &mut String) {
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return;
        };
        let reference = &rest[1..end];
        match reference {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => match numeric(reference) {
                Some(character) => out.push(character),
                None => out.push_str(&rest[..=end]),
            },
        }
        rest = &rest[end.saturating_add(1)..];
    }
    out.push_str(rest);
}

/// [`unescape`] into a fresh string, for the places a value arrives whole.
fn unescaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    unescape(text, &mut out);
    out
}

/// `#x41` or `#65`, where it names a character.
fn numeric(reference: &str) -> Option<char> {
    let digits = reference.strip_prefix('#')?;
    let value = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse::<u32>().ok()?,
    };
    char::from_u32(value)
}

#[cfg(test)]
mod tests {
    use super::{DC, PDF, Value, XMP, Xmp, XmpError};

    /// A packet in the shape Adobe's own writer produces, with all three property spellings.
    const PACKET: &str = r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
      xmlns:xmp="http://ns.adobe.com/xap/1.0/"
      pdf:Producer="An exporter"
      xmp:CreateDate="2014-03-14T12:42:11+01:00">
   <dc:title>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">Annual &amp; final report</rdf:li>
     <rdf:li xml:lang="de-DE">Jahresbericht</rdf:li>
    </rdf:Alt>
   </dc:title>
   <dc:creator><rdf:Seq><rdf:li>John Doe</rdf:li><rdf:li>Jane Roe</rdf:li></rdf:Seq></dc:creator>
   <pdf:Keywords>annual, report</pdf:Keywords>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

    /// The three spellings ISO 16684-1 gives a property, and the two §12.2 and Table 349 name.
    #[test]
    fn a_packet_states_its_properties_in_three_shapes_and_all_three_are_read() {
        let xmp = Xmp::parse(PACKET.as_bytes()).expect("the fixture is well-formed XMP");

        // The attribute spelling, §7.5.
        assert_eq!(xmp.producer(), Some("An exporter"));
        assert_eq!(xmp.created(), Some("2014-03-14T12:42:11+01:00"));
        // The element spelling.
        assert_eq!(xmp.keywords(), Some("annual, report"));
        // A language alternative, whose default is `x-default` and not its first item (ISO
        // 16684-1 section 8.2.2.4) — and whose `&amp;` is expanded, because the tokenizer
        // expands nothing.
        assert_eq!(xmp.title(), Some("Annual & final report"));
        assert_eq!(xmp.text_in(DC, "title", "de-DE"), Some("Jahresbericht"));
        assert_eq!(xmp.text_in(DC, "title", "fr-FR"), None);
        // An ordered array, which is §14.3.1's whole argument for the stream over the
        // dictionary: "a document's authors can be represented as a list".
        assert_eq!(
            xmp.authors(),
            Some(["John Doe".to_owned(), "Jane Roe".to_owned()].as_slice())
        );
        assert!(!xmp.is_empty());
    }

    /// A prefix is not a name: the same packet with every prefix renamed reads the same.
    #[test]
    fn a_property_is_its_namespace_uri_and_not_the_prefix_a_file_chose() {
        let renamed = PACKET
            .replace("rdf:", "r:")
            .replace("xmlns:rdf=", "xmlns:r=")
            .replace("dc:", "DublinCore:")
            .replace("xmlns:dc=", "xmlns:DublinCore=")
            .replace("pdf:", "p:")
            .replace("xmlns:pdf=", "xmlns:p=");
        let xmp = Xmp::parse(renamed.as_bytes()).expect("renaming prefixes changes no name");
        assert_eq!(xmp.title(), Some("Annual & final report"));
        assert_eq!(xmp.producer(), Some("An exporter"));
        assert_eq!(xmp.keywords(), Some("annual, report"));
    }

    /// ISO 16684-1 section 7.6's structured value is reported as present and uninterpreted, which is
    /// the whole difference between a gap and a silence.
    #[test]
    fn a_structured_value_is_recorded_rather_than_mistaken_for_text() {
        let packet = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
          <rdf:Description rdf:about="" xmlns:xmpTPg="http://ns.adobe.com/xap/1.0/t/pg/"
                                        xmlns:stDim="http://ns.adobe.com/xap/1.0/sType/Dimensions#">
            <xmpTPg:MaxPageSize>
              <rdf:Description stDim:w="595" stDim:h="842" stDim:unit="Points"/>
            </xmpTPg:MaxPageSize>
          </rdf:Description>
        </rdf:RDF>"#;
        let xmp = Xmp::parse(packet.as_bytes()).expect("well-formed");
        assert_eq!(
            xmp.value("http://ns.adobe.com/xap/1.0/t/pg/", "MaxPageSize"),
            Some(&Value::Structure),
            "the property is present; its value is not interpreted"
        );
        assert_eq!(
            xmp.text("http://ns.adobe.com/xap/1.0/t/pg/", "MaxPageSize"),
            None,
            "and a structure has no single string to show"
        );
    }

    /// The two attacks XML is famous for, and what makes each inert here.
    #[test]
    fn neither_an_entity_bomb_nor_an_external_entity_does_anything() {
        // The billion laughs. Nothing substitutes a declared entity, so `&lol9;` stays as the
        // eight bytes a producer wrote and the packet costs its own length.
        let bomb = r#"<?xml version="1.0"?>
        <!DOCTYPE lolz [
          <!ENTITY lol "lol">
          <!ENTITY lol1 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">
          <!ENTITY lol2 "&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;">
        ]>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
          <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/">
            <pdf:Producer>&lol2;</pdf:Producer>
          </rdf:Description>
        </rdf:RDF>"#;
        let xmp = Xmp::parse(bomb.as_bytes()).expect("a DTD is tokenised and dropped");
        assert_eq!(
            xmp.producer(),
            Some("&lol2;"),
            "an undefined reference is kept verbatim rather than expanded or deleted"
        );

        // The external entity. `SYSTEM` names a file; nothing here opens one, and the reference
        // to it is text like any other.
        let external = r#"<?xml version="1.0"?>
        <!DOCTYPE r [ <!ENTITY xxe SYSTEM "file:///etc/passwd"> ]>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
          <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/">
            <pdf:Producer>&xxe;</pdf:Producer>
          </rdf:Description>
        </rdf:RDF>"#;
        let xmp = Xmp::parse(external.as_bytes()).expect("an external identifier is inert");
        assert_eq!(xmp.producer(), Some("&xxe;"));
    }

    /// Every budget refuses rather than allocating, and the refusal names which one.
    #[test]
    fn the_four_budgets_refuse_and_say_which() {
        let deep = format!(
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">{}{}</rdf:RDF>",
            "<a>".repeat(200),
            "</a>".repeat(200)
        );
        assert_eq!(
            Xmp::parse(deep.as_bytes()),
            Err(XmpError::TooMuch {
                what: "nesting depth"
            })
        );

        assert!(matches!(
            Xmp::parse(&vec![b' '; (8 << 20) + 1]),
            Err(XmpError::TooLarge { .. })
        ));

        // Not text in any of the three encodings the clause permits.
        assert_eq!(Xmp::parse(&[0xC3, 0x28]), Err(XmpError::NotText));

        // Malformed XML is a report and not a panic — and the two shapes it takes are two
        // different readers noticing. A tag whose *syntax* is wrong is the tokenizer's finding;
        // a tag that closes the wrong element is this module's, because `xmlparser` checks
        // tokens and never builds a tree.
        assert!(matches!(
            Xmp::parse(b"<rdf:RDF <"),
            Err(XmpError::Malformed { .. })
        ));
        assert!(matches!(
            Xmp::parse(b"<rdf:RDF><unclosed>"),
            Err(XmpError::Unbalanced { .. })
        ));
        assert!(matches!(
            Xmp::parse(b"<a></b>"),
            Err(XmpError::Unbalanced { .. })
        ));
    }

    /// UTF-16 is one of the three encodings ISO 16684-1 section 7.3.2 permits, so it is read.
    #[test]
    fn the_other_encodings_the_clause_permits_are_decoded_rather_than_refused() {
        let packet = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
          <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
                           pdf:Producer="Ünicode"/></rdf:RDF>"#;
        let mut utf16 = vec![0xFF, 0xFE];
        for unit in packet.encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(
            Xmp::parse(&utf16).expect("UTF-16 is text").producer(),
            Some("Ünicode")
        );

        let mut utf32 = vec![0x00, 0x00, 0xFE, 0xFF];
        for character in packet.chars() {
            utf32.extend_from_slice(&(character as u32).to_be_bytes());
        }
        assert_eq!(
            Xmp::parse(&utf32).expect("UTF-32 is text").producer(),
            Some("Ünicode")
        );
    }

    /// A packet stating nothing this reader understands is empty rather than an error, and a
    /// property stated twice keeps both — which is why the store is a list.
    #[test]
    fn a_property_stated_twice_keeps_both_statements() {
        let packet = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                                 xmlns:pdf="http://ns.adobe.com/pdf/1.3/">
          <rdf:Description rdf:about="" pdf:Producer="First"/>
          <rdf:Description rdf:about="" pdf:Producer="Second"/>
        </rdf:RDF>"#;
        let xmp = Xmp::parse(packet.as_bytes()).expect("well-formed");
        assert_eq!(xmp.properties().len(), 2);
        assert_eq!(
            xmp.producer(),
            Some("First"),
            "lookup answers with the first statement, and the second is not lost"
        );

        assert!(
            Xmp::parse(b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"/>")
                .expect("well-formed")
                .is_empty()
        );
    }

    /// The `xmp:` accessors name the properties Table 349's NOTEs pair with the dictionary.
    #[test]
    fn the_accessors_are_the_properties_table_349_names() {
        let xmp = Xmp::parse(PACKET.as_bytes()).expect("well-formed");
        assert_eq!(xmp.text(PDF, "Producer"), xmp.producer());
        assert_eq!(xmp.text(XMP, "CreateDate"), xmp.created());
        assert_eq!(xmp.modified(), None, "the fixture states no xmp:ModifyDate");
        assert_eq!(xmp.creator_tool(), None);
        assert_eq!(xmp.description(), None);
    }
}
