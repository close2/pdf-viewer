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
//! An XMP packet is RDF/XML narrowed to one shape. Properties live on or under `rdf:Description`
//! elements, and ISO 16684-1 section 7.5 gives a simple property two spellings that mean the same
//! thing — an attribute on the description, or a child element:
//!
//! ```xml
//! <rdf:Description rdf:about="" pdf:Producer="An exporter"/>
//! <rdf:Description rdf:about=""><pdf:Producer>An exporter</pdf:Producer></rdf:Description>
//! ```
//!
//! and three container forms, `rdf:Alt`, `rdf:Seq` and `rdf:Bag`, whose items are `rdf:li`
//! elements. A language alternative is an `rdf:Alt` whose items carry `xml:lang`; `x-default` is
//! ISO 16684-1 section 8.2.2.4's name for the one to show when nothing better is known, and it is
//! what §12.2's `/DisplayDocTitle` ends up asking for.
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
//! *structure* — ISO 16684-1 section 7.6, an `rdf:parseType="Resource"` or a nested
//! `rdf:Description` — is recorded as [`Value::Structure`]: the property is reported as present and
//! its value is reported as uninterpreted, which is the difference between a gap and a silence.
//! Nothing in clause 12 or 14 asks for one; `xmpMM:DerivedFrom` and `xmpTPg:MaxPageSize` are the
//! common ones and neither reaches a pixel.
//!
//! **A caller that needs the fields asks for them.** [`Xmp::parse_detail`] walks the same grammar
//! and keeps everything, as [`Detail`] rather than [`Value`]: ISO 19005-2's extension schema
//! container is a bag of structures whose fields are sequences of structures, so a validator
//! cannot work from a value that says only *structure*. It is a second entry point rather than a
//! richer [`Value`] because the two readings have different callers — nothing that draws a page
//! wants the larger one, and the packet is untrusted bytes whose parsed size this module bounds.
//!
//! Qualifiers other than `xml:lang` (ISO 16684-1 section 7.7) are dropped, which is the same
//! statement: the property keeps its value and loses an annotation on it.

use pdf_syntax::{Dictionary, Document};

/// The Dublin Core namespace, which carries `dc:title`, `dc:creator` and `dc:description`.
pub const DC: &str = "http://purl.org/dc/elements/1.1/";
/// Adobe's PDF schema: `pdf:Producer`, `pdf:Keywords`, `pdf:Trapped`.
pub const PDF: &str = "http://ns.adobe.com/pdf/1.3/";
/// The XMP basic schema: `xmp:CreatorTool`, `xmp:CreateDate`, `xmp:ModifyDate`.
pub const XMP: &str = "http://ns.adobe.com/xap/1.0/";
/// The RDF syntax namespace, whose `Description`, `Alt`, `Seq`, `Bag` and `li` are the grammar.
///
/// Public because ISO 16684-1 section 6.2 makes it a namespace a *property* may not be in, save
/// for `rdf:type`, and a validator asking that has to spell the URI the same way this reader
/// resolved it to.
pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
/// The XML namespace, bound to the `xml` prefix by the XML specification itself and never
/// declared. Its `lang` attribute is what makes an `rdf:Alt` a language alternative.
///
/// Public for [`RDF`]'s reason: section 6.2 restricts it in the same sentence.
pub const XML: &str = "http://www.w3.org/XML/1998/namespace";

/// The XMP Media Management namespace, whose `History` property records what was done to a file.
///
/// Public because [`record`] writes into that property and a caller that reads the result back
/// — `pdf_archive` is the one here — has to spell the same URI rather than the same prefix.
pub const XMP_MM: &str = "http://ns.adobe.com/xap/1.0/mm/";

/// The field namespace of ISO 16684-1's `ResourceEvent` value type.
///
/// `xmpMM:History` is an ordered array of `ResourceEvent` structures, and the fields an
/// [`Event`] states — `action`, `parameters`, `when` — are that value type's rather than any
/// one schema's, which is why they are in a namespace of their own.
pub const RESOURCE_EVENT: &str = "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#";

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
    /// The bytes are not text in any of the three encodings ISO 16684-1 section 7.1 names.
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
    /// ISO 16684-1 section 7.6's structured value, present and not interpreted. See the module
    /// comment.
    Structure,
}

/// A property's value with everything the packet states about it, [`Value::Structure`] included.
///
/// [`Value`] is what a *viewer* needs: one string to show, or a list of them. This is what a
/// *validator* needs, and it exists because ISO 19005-2 section 6.6.2.3.3's extension schema
/// container schema is a bag of structures whose fields are themselves sequences of structures — a
/// reader that reported only that a structure was *present* could not check one field of it. Read
/// with [`Xmp::parse_detail`], which a caller asks for deliberately: building this costs a second
/// representation of the packet, and nothing that draws a page wants one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detail {
    /// A simple property: one string, as [`Value::Text`].
    Text(String),
    /// `rdf:Alt`, each item with its `xml:lang` where it states one.
    Alt(Vec<(Option<String>, Detail)>),
    /// `rdf:Seq`, an ordered array.
    Seq(Vec<Detail>),
    /// `rdf:Bag`, an unordered array.
    Bag(Vec<Detail>),
    /// ISO 16684-1 section 7.6's structured value, with its fields in the order stated.
    Structure(Vec<Property>),
}

impl Detail {
    /// The text of a simple value, and nothing for any other shape.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text.as_str()),
            _ => None,
        }
    }

    /// A structure's fields, in the order the packet states them.
    #[must_use]
    pub fn fields(&self) -> Option<&[Property]> {
        match self {
            Self::Structure(fields) => Some(fields.as_slice()),
            _ => None,
        }
    }

    /// One field of a structure, by namespace URI and local name.
    #[must_use]
    pub fn field(&self, namespace: &str, local: &str) -> Option<&Property> {
        self.fields()?
            .iter()
            .find(|field| field.name.namespace == namespace && field.name.local == local)
    }

    /// The items of an `rdf:Seq` or an `rdf:Bag`, and nothing for any other shape.
    ///
    /// An `rdf:Alt`'s items are [`Self::alternatives`] instead, because dropping their languages
    /// here would make the two arrays look interchangeable when they are not.
    #[must_use]
    pub fn array(&self) -> Option<&[Detail]> {
        match self {
            Self::Seq(items) | Self::Bag(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    /// An `rdf:Alt`'s items, each with the `xml:lang` it states.
    #[must_use]
    pub fn alternatives(&self) -> Option<&[(Option<String>, Detail)]> {
        match self {
            Self::Alt(items) => Some(items.as_slice()),
            _ => None,
        }
    }
}

/// One property or field as a packet states it, for [`Xmp::parse_detail`].
///
/// The prefix is here because a prefix is *usually* not a name — see the module comment — and twice
/// in ISO 19005 it is: ISO 19005-2 section 6.6.2.3.3 requires the fields of its four value types to
/// be spelled with the prefixes its tables name, and section 6.6.2.2 says a prefix means nothing
/// *except* where one is identified as required. A reader that resolved the prefix away could
/// not answer that requirement, so it is kept beside the resolved name rather than instead of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// The resolved name: a namespace URI and a local name.
    pub name: Name,
    /// The prefix the packet spelled it with, empty where it used none.
    pub prefix: String,
    /// The value.
    pub value: Detail,
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
        let text = text_of(bytes)?;
        Ok(Self {
            properties: Reader::new(false).run(&text)?.properties,
        })
    }

    /// Parses a packet's bytes, keeping the structure fields [`Value`] drops.
    ///
    /// Every property in the order the packet states them, as [`Detail`] rather than [`Value`].
    /// The two readings are the same walk over the same grammar and differ only in what they
    /// keep, so a caller that wants both parses twice deliberately rather than paying for the
    /// larger one everywhere.
    ///
    /// # Errors
    ///
    /// As [`Xmp::parse`].
    pub fn parse_detail(bytes: &[u8]) -> Result<Vec<Property>, XmpError> {
        let text = text_of(bytes)?;
        Ok(Reader::new(true).run(&text)?.details)
    }

    /// How many top-level `rdf:RDF` elements a packet's bytes state.
    ///
    /// ISO 16684-1 section 7.1 requires a single XMP packet to be serialised using a single
    /// `rdf:RDF` element, and neither [`Self::parse`] nor [`Self::parse_detail`] can be asked
    /// how many there were: both hand back the properties, which two roots would merge into one
    /// list. A validator has to be able to tell those apart, so the count is its own reading of
    /// the same walk rather than a field on [`Xmp`] — an [`Xmp`] built by
    /// [`Self::from_properties`] never saw a packet and could only lie about it.
    ///
    /// # Errors
    ///
    /// As [`Self::parse`].
    pub fn rdf_elements(bytes: &[u8]) -> Result<usize, XmpError> {
        let text = text_of(bytes)?;
        Ok(Reader::new(false).run(&text)?.rdf_elements)
    }

    /// Every element of a packet that carries character data where none is permitted.
    ///
    /// ISO 16684-1 section 7.2 closes by confining non-white character data to the element
    /// content of the leaf elements that stand for simple XMP values; everything else in a
    /// packet — the `rdf:RDF`, a description, one of the three containers, a property whose
    /// value some child element carries — may hold white space, markup and nothing besides.
    /// Neither [`Self::parse`] nor [`Self::parse_detail`] can be asked about it, for
    /// [`Self::rdf_elements`]'s reason: both hand back the values, and the text a packet put
    /// where no value could hold it is gone from those by construction.
    ///
    /// Each element is named as the packet spelled its tag, in the order the elements closed, and
    /// an element is named once however many runs of character data it held. The list is
    /// **not** a rule about which elements are leaves in some other sense: an element this reader
    /// does not interpret, and anything above the packet's `rdf:RDF`, are passed over rather than
    /// reported, so what comes back under-reports and never over-reports.
    ///
    /// # Errors
    ///
    /// As [`Self::parse`].
    pub fn stray_character_data(bytes: &[u8]) -> Result<Vec<String>, XmpError> {
        let text = text_of(bytes)?;
        Ok(Reader::new(false).run(&text)?.strays)
    }

    /// Every property, in the order the packet states them.
    #[must_use]
    pub fn properties(&self) -> &[(Name, Value)] {
        &self.properties
    }

    /// The packet a caller already holds the properties of.
    ///
    /// [`Self::properties`]'s inverse, and it exists because a packet read in one process is
    /// needed in another: `viewer-confined` carries a confined viewer's `/Metadata` to its host,
    /// and without this the host would have the properties and no [`Xmp`] to ask
    /// [`Self::title`] of. A reader that can do something no caller can ask for is the failure
    /// `doc/todo/01`'s fifth sweep looks for.
    ///
    /// **The parser's bounds are not this constructor's.** [`Self::parse`] refuses a packet that
    /// states more properties, deeper nesting or longer values than its budgets allow, because
    /// those bound what *hostile bytes* can make this reader build; a caller assembling a list it
    /// already holds is not that, and the transport that does it bounds its own message.
    #[must_use]
    pub fn from_properties(properties: Vec<(Name, Value)>) -> Self {
        Self { properties }
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
    /// Section 8.2.2.4 puts them in, its first item being the default. An array gives its first
    /// item; a structure gives nothing, because it has no single string to give.
    #[must_use]
    pub fn text(&self, namespace: &str, local: &str) -> Option<&str> {
        match self.value(namespace, local)? {
            Value::Text(text) => Some(text.as_str()),
            Value::Alt(items) => items
                .iter()
                .find(|(lang, _)| {
                    lang.as_deref()
                        .is_some_and(|lang| same_language(lang, "x-default"))
                })
                .or_else(|| items.first())
                .map(|(_, text)| text.as_str()),
            Value::Seq(items) | Value::Bag(items) => items.first().map(String::as_str),
            Value::Structure => None,
        }
    }

    /// One property in a particular language, where it is a language alternative that has it.
    ///
    /// The match is on the `xml:lang` the packet wrote, not on RFC 4647's lookup: a fallback
    /// from `en-GB` to `en` is a policy a *host* has, and inventing one here would put it out of
    /// that host's reach. Case is [`same_language`]'s business rather than the tag's.
    #[must_use]
    pub fn text_in(&self, namespace: &str, local: &str, language: &str) -> Option<&str> {
        match self.value(namespace, local)? {
            Value::Alt(items) => items
                .iter()
                .find(|(lang, _)| {
                    lang.as_deref()
                        .is_some_and(|lang| same_language(lang, language))
                })
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

/// A schema's properties, to be stated in a packet as ISO 16684-1 section 7.5's simple values.
///
/// One namespace and one prefix rather than a property list of arbitrary names, because that is
/// what a *schema* is and what the one caller writes: an identification schema is a fixed set of
/// scalar properties in one namespace whose prefix its own subclause makes required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema<'a> {
    /// The namespace URI the properties belong to.
    pub namespace: &'a str,
    /// The prefix to spell them with.
    pub prefix: &'a str,
    /// The properties, as local name and value, in the order they are to be written.
    pub properties: &'a [(&'a str, String)],
}

/// Why a packet could not be written.
///
/// Separate from [`XmpError`] because the two say different things: that one is a packet this
/// tree cannot *read*, and these are all reasons a packet cannot be *edited* while leaving the
/// rest of it exactly as its producer wrote it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum WriteError {
    /// The packet is not UTF-8, and this writer edits a packet's own bytes.
    #[error("the packet is not UTF-8 text, and this writer edits a packet's own bytes")]
    NotUtf8,
    /// The packet's XML does not parse, so no span in it can be trusted.
    #[error("the packet's XML does not parse: {detail}")]
    Malformed {
        /// What the tokenizer said.
        detail: String,
    },
    /// The packet holds no `rdf:RDF` element a description can go inside.
    ///
    /// Exactly one, with a close tag of its own, is what this writer needs: a packet stating two
    /// leaves no answer to which of them the schema belongs in, and a self-closing one has no
    /// inside at all.
    #[error(
        "the packet states {found} rdf:RDF elements and none this writer can put a description \
         inside"
    )]
    NoPlaceForADescription {
        /// How many were found.
        found: usize,
    },
    /// The packet states an `xmpMM:History` this writer cannot append an event to.
    ///
    /// Never *replaced*: a history is the record of what was done to a file before this
    /// conversion, and overwriting it would lose exactly the provenance the subclause that asks
    /// for the new entry exists to keep. So a packet whose history is not one `rdf:Seq` this
    /// writer can find the end of leaves the packet untouched and the caller saying so.
    #[error(
        "the packet states {found} xmpMM:History properties and none this writer can append an \
         event to"
    )]
    NoPlaceForAnEvent {
        /// How many were found.
        found: usize,
    },
    /// The packet nests deeper than [`MAX_DEPTH`].
    #[error("the packet nests deeper than this writer follows")]
    TooDeep,
    /// The packet is larger than [`MAX_BYTES`].
    #[error("the packet is {bytes} bytes, past this writer's bound")]
    TooLarge {
        /// What the packet holds.
        bytes: usize,
    },
}

/// A fresh packet stating exactly `schema`'s properties and nothing else.
///
/// The wrapper is the one ISO 32000-2 §14.3.2's own EXAMPLE prints — the `<?xpacket>` header with
/// its identifier, `x:xmpmeta`, and one `rdf:RDF` — so nothing here is a convention read off
/// another producer's file. What goes inside is one `rdf:Description` whose subject is the empty
/// string, which is that example's spelling of "this document".
///
/// For a document that *has* a packet, [`restate`] is the function: this one would throw the
/// producer's metadata away.
#[must_use]
pub fn packet(schema: &Schema<'_>) -> Vec<u8> {
    let mut out = String::new();
    out.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    out.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n");
    out.push_str("<rdf:RDF xmlns:rdf=\"");
    escaped(RDF, &mut out);
    out.push_str("\">\n");
    description(schema, &mut out);
    out.push_str("</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>");
    out.into_bytes()
}

/// `bytes` with every property in `namespaces` removed and `schema`'s stated in their place.
///
/// **Every other byte of the packet crosses unchanged**, which is the whole point of doing this
/// by span surgery rather than by parsing to a value and printing it again. This module's reader
/// keeps neither an `rdf:about` subject nor a qualifier other than `xml:lang`
/// (ISO 16684-1 section 7.7), so a writer built on the reading would silently drop what the
/// reading drops. A producer's packet is metadata somebody wrote deliberately; the only thing a
/// conversion has any business changing in it is the schema it was asked to change.
///
/// `namespaces` is a list rather than one URI because a schema can be printed with more than one
/// spelling of its own namespace, and a property left behind under the other spelling would be a
/// second claim beside the one just written.
///
/// # Errors
///
/// [`WriteError`], every variant of which leaves the packet untouched: a caller that cannot
/// write is expected to say so rather than to write something else.
pub fn restate(
    bytes: &[u8],
    namespaces: &[&str],
    schema: &Schema<'_>,
) -> Result<Vec<u8>, WriteError> {
    if bytes.len() > MAX_BYTES {
        return Err(WriteError::TooLarge { bytes: bytes.len() });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| WriteError::NotUtf8)?;
    let edit = Editor::run(text, Cut::Namespaces(namespaces))?;
    let Some(at) = edit.inside_rdf else {
        return Err(WriteError::NoPlaceForADescription {
            found: edit.rdf_elements,
        });
    };

    // The cuts, then the one insertion, taken in the order they occur in the packet — so that a
    // property stated *after* the `rdf:RDF` this writes into is still removed.
    let mut out = String::with_capacity(text.len());
    let mut cut = 0usize;
    let mut written = false;
    let copy = |out: &mut String, from: usize, to: usize| {
        out.push_str(text.get(from..to).unwrap_or_default());
    };
    for (from, to) in edit.deletions {
        if from < cut {
            continue;
        }
        if !written && at <= from {
            copy(&mut out, cut, at);
            description(schema, &mut out);
            cut = at;
            written = true;
        }
        copy(&mut out, cut, from);
        cut = to;
    }
    if !written {
        copy(&mut out, cut, at);
        description(schema, &mut out);
        cut = at;
    }
    copy(&mut out, cut, text.len());
    Ok(out.into_bytes())
}

/// `bytes` with each of `properties` gone, and every other byte of the packet unchanged.
///
/// The population is *properties of the packet* rather than every element that happens to carry
/// one of these names: ISO 16684-1 section 7.4 puts a property directly inside an
/// `rdf:Description` inside the packet's `rdf:RDF`, and a field of some structured value deeper
/// down belongs to that value rather than to the document. Both of section 7.5's spellings go —
/// the property element, and the attribute of the description standing for a simple value —
/// because both state the same property.
///
/// **Removing a property is a loss, and this function does not decide that it is allowed.** It is
/// the mechanism `doc/pdf-a-conversion-limits.md` section 3.9 chooses between three routes, two of
/// which are closed; the caller is what asks whether it may be used.
///
/// # Errors
///
/// [`WriteError`], every variant of which leaves the packet untouched. `Ok` with nothing removed
/// is not an error here: a caller that has to know whether the properties are gone reads the
/// result back, which is what the packet says rather than what this writer intended.
pub fn remove(bytes: &[u8], properties: &[Name]) -> Result<Vec<u8>, WriteError> {
    if bytes.len() > MAX_BYTES {
        return Err(WriteError::TooLarge { bytes: bytes.len() });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| WriteError::NotUtf8)?;
    let edit = Editor::run(text, Cut::Properties(properties))?;
    let mut out = String::with_capacity(text.len());
    let mut cut = 0usize;
    for (from, to) in edit.deletions {
        // A deletion inside one already taken is covered by it; `Opened::removing` keeps nested
        // elements from recording one, and this is the same rule for two properties that overlap
        // in no other way than by the packet being malformed.
        if from < cut {
            continue;
        }
        out.push_str(text.get(cut..from).unwrap_or_default());
        cut = to;
    }
    out.push_str(text.get(cut..).unwrap_or_default());
    Ok(out.into_bytes())
}

/// One action recorded in `xmpMM:History`.
///
/// The three fields ISO 19005-2 section 6.6.6 names of a recorded action. ISO 19005-4 section
/// 6.7.5 requires two of them and demotes `parameters` to a recommendation, so an event stating
/// all three satisfies both parts and neither part's reader has to know which wrote it.
///
/// The strings are the caller's: this module writes what it is given, escaped, and decides
/// nothing about what an action is called or when it happened. [`instant`] is what turns a
/// [`std::time::SystemTime`] into the `when` field's form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event<'a> {
    /// What was done, in one word — the `stEvt:action` field.
    pub action: &'a str,
    /// What it was done with or to — the `stEvt:parameters` field.
    pub parameters: &'a str,
    /// When, as [`instant`] writes it — the `stEvt:when` field.
    pub when: &'a str,
}

/// `bytes` with one more event appended to the packet's `xmpMM:History`.
///
/// **Appended, never replaced.** A history the producer wrote is a record of what happened to
/// the file before this program saw it, and an entry added to the end is the only edit that
/// leaves that record intact. Where the packet states no history at all, one is created holding
/// this single event; where it states one this writer cannot find the end of, nothing is written
/// and the caller is told — [`WriteError::NoPlaceForAnEvent`].
///
/// Every other byte of the packet crosses unchanged, for [`restate`]'s reason and by the same
/// means: the spans are found by one pass of the tokenizer and the text is copied around them.
///
/// # Errors
///
/// [`WriteError`], every variant of which leaves the packet untouched.
pub fn record(bytes: &[u8], event: &Event<'_>) -> Result<Vec<u8>, WriteError> {
    if bytes.len() > MAX_BYTES {
        return Err(WriteError::TooLarge { bytes: bytes.len() });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| WriteError::NotUtf8)?;
    let edit = Editor::run(text, Cut::Namespaces(&[]))?;
    // A history whose sequence this writer found: the event goes at the end of that sequence,
    // after every entry already in it.
    let (at, whole) = match (edit.history_seq_end, edit.history_elements) {
        (Some(at), 1) => (at, false),
        (_, 0) => (
            edit.inside_rdf.ok_or(WriteError::NoPlaceForADescription {
                found: edit.rdf_elements,
            })?,
            true,
        ),
        (_, found) => return Err(WriteError::NoPlaceForAnEvent { found }),
    };
    let mut out = String::with_capacity(text.len().saturating_add(512));
    out.push_str(text.get(..at).unwrap_or_default());
    if whole {
        history(event, &mut out);
    } else {
        item(event, &mut out);
    }
    out.push_str(text.get(at..).unwrap_or_default());
    Ok(out.into_bytes())
}

/// One whole `xmpMM:History`, in its own description, for a packet that states none.
fn history(event: &Event<'_>, out: &mut String) {
    out.push_str("<rdf:Description rdf:about=\"\" xmlns:rdf=\"");
    escaped(RDF, out);
    out.push_str("\" xmlns:xmpMM=\"");
    escaped(XMP_MM, out);
    out.push_str("\">\n<xmpMM:History><rdf:Seq>\n");
    item(event, out);
    out.push_str("</rdf:Seq></xmpMM:History>\n</rdf:Description>\n");
}

/// One `rdf:li` holding one `ResourceEvent`, with the two prefixes it uses declared on it.
///
/// Declared rather than inherited, for [`description`]'s reason: what `rdf` and `stEvt` are bound
/// to where this is inserted is the producer's business, and an element carrying its own bindings
/// means the same thing wherever it is put.
fn item(event: &Event<'_>, out: &mut String) {
    out.push_str("<rdf:li rdf:parseType=\"Resource\" xmlns:rdf=\"");
    escaped(RDF, out);
    out.push_str("\" xmlns:stEvt=\"");
    escaped(RESOURCE_EVENT, out);
    out.push_str("\">\n");
    for (local, value) in [
        ("action", event.action),
        ("parameters", event.parameters),
        ("when", event.when),
    ] {
        out.push_str("<stEvt:");
        out.push_str(local);
        out.push('>');
        escaped(value, out);
        out.push_str("</stEvt:");
        out.push_str(local);
        out.push_str(">\n");
    }
    out.push_str("</rdf:li>\n");
}

/// How many seconds a day holds on the clock `SystemTime` reports, which counts no leap second.
const SECONDS_A_DAY: u64 = 86_400;

/// The days from 1970-01-01 to 2000-03-01, the epoch the civil-date arithmetic below counts from.
///
/// Shifting the epoch to the day after a leap day makes February the last month of the year, so
/// the leap day is the last day of it and no month length depends on the leap rule.
const DAYS_TO_2000_03_01: i64 = 11_017;

/// One instant as the `when` field of an [`Event`] states it.
///
/// ISO 16684-1 gives a date the form of a subset of ISO 8601, and `Z` is that form's spelling of
/// UTC — which is what this writes, because a converter has no business claiming a time zone the
/// file says nothing about. Seconds are whole: the field records when an action happened, and a
/// fraction of a second is precision nobody asked for.
///
/// `None` for an instant before 1970, which a system clock reports only when it is wrong; a
/// caller that gets one writes no event rather than a date it made up.
#[must_use]
pub fn instant(at: std::time::SystemTime) -> Option<String> {
    let seconds = at.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    let (days, rest) = (seconds / SECONDS_A_DAY, seconds % SECONDS_A_DAY);
    let (hour, minute, second) = (rest / 3_600, (rest / 60) % 60, rest % 60);
    let (year, month, day) = civil(i64::try_from(days).ok()?);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

/// The Gregorian year, month and day `days` after 1970-01-01.
///
/// Howard Hinnant's `civil_from_days`, whose whole trick is the epoch shift [`DAYS_TO_2000_03_01`]
/// names: with March first, the four hundred year cycle divides evenly and the arithmetic is
/// exact integer division with no table of month lengths and no leap-year branch.
///
/// `days` is a whole number of days since 1970-01-01 and its one caller derives it from a
/// `u64` count of seconds, so it is non-negative and at most `u64::MAX / 86 400`, which is under
/// 2.2 × 10¹⁴. Every product below is bounded by that: `era` is at most that over 146 097,
/// `day_of_era` is under 146 097 by construction, `year_of_era` is under 400 and `day_of_year`
/// under 366 — so the largest value any expression here takes is `era * 400`, eleven decimal
/// digits inside `i64`.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every term is bounded by the caller's range, stated in the paragraph above"
)]
fn civil(days: i64) -> (i64, i64, i64) {
    let shifted = days - DAYS_TO_2000_03_01;
    // The 400-year cycle holds 146 097 days, and `div_euclid` keeps that true before 2000 too.
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    // The year within the era, by removing the leap days a 4-, 100- and 400-year rule adds.
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    // March is month 0 in this frame, and the 153-day five-month pattern gives the rest exactly.
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2) + 2_000;
    (year, month, day)
}

/// One `rdf:Description` stating a schema's properties, with both prefixes it uses declared on it.
///
/// Declared rather than inherited: what a prefix is bound to at the point this is inserted is the
/// producer's business, and an element that carries its own bindings means the same thing wherever
/// it is put.
fn description(schema: &Schema<'_>, out: &mut String) {
    out.push_str("<rdf:Description rdf:about=\"\" xmlns:rdf=\"");
    escaped(RDF, out);
    out.push_str("\" xmlns:");
    out.push_str(schema.prefix);
    out.push_str("=\"");
    escaped(schema.namespace, out);
    out.push_str("\">\n");
    for (local, value) in schema.properties {
        out.push('<');
        out.push_str(schema.prefix);
        out.push(':');
        out.push_str(local);
        out.push('>');
        escaped(value, out);
        out.push_str("</");
        out.push_str(schema.prefix);
        out.push(':');
        out.push_str(local);
        out.push_str(">\n");
    }
    out.push_str("</rdf:Description>\n");
}

/// Text with XML's three markup characters replaced by their entities.
fn escaped(text: &str, out: &mut String) {
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            other => out.push(other),
        }
    }
}

/// One open element, while the spans are being collected.
#[derive(Debug)]
struct Opened {
    /// The prefix and local name as the packet spelled them, kept so that a close tag can be
    /// checked against the tag it closes — a mismatch means every span after it is untrustworthy.
    tag: (String, String),
    /// How many namespace bindings this element declared, popped when it closes.
    bindings: usize,
    /// The byte offset of this element's `<`, where this element is the one being removed.
    ///
    /// `None` for an element that stays and for one inside an element already being removed:
    /// a nested deletion inside a deletion would record a range the outer one already covers.
    removing: Option<usize>,
    /// Whether this element or an ancestor is being removed.
    doomed: bool,
    /// Which of the three elements a writer has to find this one is, where it is one of them.
    landmark: Landmark,
}

/// An element one of the writers has to be able to find again, by its resolved name.
///
/// One field rather than three flags because they are mutually exclusive: no element is both the
/// packet's `rdf:RDF` and a property of the media management schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Landmark {
    /// Any other element, which is almost all of them.
    Ordinary,
    /// The packet's `rdf:RDF`, inside which a fresh `rdf:Description` goes.
    Rdf,
    /// An `rdf:Description` directly inside one, which is where ISO 16684-1 section 7.4 puts the
    /// packet's own properties — so an element inside it is a property and one deeper is not.
    Description,
    /// An `xmpMM:History` property.
    History,
    /// The `rdf:Seq` that is one's value, at the end of which an event goes.
    HistorySeq,
}

/// Whether one of the properties named for removal is this namespace and local name.
///
/// ISO 16684-1 section 6.2's own equivalence: two XMP names are the same when their namespace URI
/// and their local name are identical, with no case folding and no normalisation.
fn listed(properties: &[Name], namespace: &str, local: &str) -> bool {
    properties
        .iter()
        .any(|wanted| wanted.namespace == namespace && wanted.local == local)
}

/// Which of a packet's properties an edit takes out.
///
/// Two populations rather than one predicate, because the two writers want genuinely different
/// things: [`restate`] replaces a *schema*, so everything in its namespace goes wherever it sits,
/// and [`remove`] takes named properties out of a schema that stays, so a field sharing a name
/// with one deeper inside somebody else's value is not it.
#[derive(Debug, Clone, Copy)]
enum Cut<'a> {
    /// Every element and attribute whose namespace is one of these, at any depth.
    Namespaces(&'a [&'a str]),
    /// Exactly these properties of the packet, and nothing nested inside another value.
    Properties(&'a [Name]),
}

/// The spans one pass over a packet found.
struct Editor {
    /// Prefix-to-URI bindings, innermost last, as [`Reader`] keeps them and for its reason.
    bindings: Vec<(String, String)>,
    stack: Vec<Opened>,
    /// The ranges to cut, ordered by where they start.
    deletions: Vec<(usize, usize)>,
    /// The offset just before the packet's `</rdf:RDF>`, where exactly one was found.
    inside_rdf: Option<usize>,
    /// How many `rdf:RDF` elements the packet stated.
    rdf_elements: usize,
    /// The offset just before the `</rdf:Seq>` of the packet's one `xmpMM:History`.
    ///
    /// `None` where the packet states none, states more than one, or states one whose value is
    /// not a single `rdf:Seq` with a close tag of its own — [`record`] tells those three apart by
    /// reading [`Self::history_elements`] beside this.
    history_seq_end: Option<usize>,
    /// How many `xmpMM:History` properties the packet stated.
    history_elements: usize,
}

impl Editor {
    /// Walks the packet, recording what to cut and where to insert.
    fn run(text: &str, cut: Cut<'_>) -> Result<Self, WriteError> {
        let mut editor = Self {
            bindings: Vec::new(),
            stack: Vec::new(),
            deletions: Vec::new(),
            inside_rdf: None,
            rdf_elements: 0,
            history_seq_end: None,
            history_elements: 0,
        };
        // An element's own prefix may be bound by an attribute of that same element, so nothing
        // is resolved until the element's attributes have all arrived — [`Reader::run`] takes
        // the same shape for the same reason.
        let mut pending: Option<(String, String, usize)> = None;
        let mut attributes: Vec<(String, String, String, usize, usize)> = Vec::new();
        for token in xmlparser::Tokenizer::from(text) {
            let token = token.map_err(|error| WriteError::Malformed {
                detail: error.to_string(),
            })?;
            match token {
                xmlparser::Token::ElementStart {
                    prefix,
                    local,
                    span,
                } => {
                    pending = Some((prefix.to_string(), local.to_string(), span.start()));
                    attributes.clear();
                }
                xmlparser::Token::Attribute {
                    prefix,
                    local,
                    value,
                    span,
                } => attributes.push((
                    prefix.to_string(),
                    local.to_string(),
                    value.to_string(),
                    span.start(),
                    span.end(),
                )),
                xmlparser::Token::ElementEnd { end, span } => match end {
                    xmlparser::ElementEnd::Open => {
                        let Some(open) = pending.take() else { continue };
                        editor.open(&open, &attributes, cut)?;
                    }
                    xmlparser::ElementEnd::Empty => {
                        let Some(open) = pending.take() else { continue };
                        let tag = (open.0.clone(), open.1.clone());
                        editor.open(&open, &attributes, cut)?;
                        // A self-closing element has no inside, so an `rdf:RDF` written this way
                        // is not a place a description can go: `closed` is told so.
                        editor.close(&tag, span.end(), None)?;
                    }
                    xmlparser::ElementEnd::Close(prefix, local) => {
                        let tag = (prefix.to_string(), local.to_string());
                        editor.close(&tag, span.end(), Some(span.start()))?;
                    }
                },
                _ => {}
            }
        }
        if let Some(open) = editor.stack.last() {
            return Err(WriteError::Malformed {
                detail: format!("<{}> is never closed", spelled(&open.tag)),
            });
        }
        editor.deletions.sort_unstable();
        Ok(editor)
    }

    /// Resolves a prefix against the bindings in scope, innermost first.
    fn namespace(&self, prefix: &str) -> &str {
        if prefix == "xml" {
            return XML;
        }
        self.bindings
            .iter()
            .rev()
            .find(|(bound, _)| bound == prefix)
            .map_or("", |(_, uri)| uri.as_str())
    }

    /// Opens an element: installs its bindings, and decides whether it or any of its attributes
    /// state a property of one of the namespaces being restated.
    fn open(
        &mut self,
        (prefix, local, start): &(String, String, usize),
        attributes: &[(String, String, String, usize, usize)],
        cut: Cut<'_>,
    ) -> Result<(), WriteError> {
        if self.stack.len() >= MAX_DEPTH {
            return Err(WriteError::TooDeep);
        }
        let mut bindings = 0usize;
        for (prefix, local, value, ..) in attributes {
            match (prefix.as_str(), local.as_str()) {
                ("", "xmlns") => self.bindings.push((String::new(), value.clone())),
                ("xmlns", _) => self.bindings.push((local.clone(), value.clone())),
                _ => continue,
            }
            bindings = bindings.saturating_add(1);
        }

        let inherited = self.stack.last().is_some_and(|open| open.doomed);
        let namespace = self.namespace(prefix).to_owned();
        let under = self.stack.last().map(|open| open.landmark);
        // A property is what sits directly inside a description; an attribute form sits *on* one.
        let describes = namespace == RDF && local == "Description" && under == Some(Landmark::Rdf);
        let own = match cut {
            Cut::Namespaces(namespaces) => namespaces.contains(&namespace.as_str()),
            Cut::Properties(properties) => {
                under == Some(Landmark::Description) && listed(properties, &namespace, local)
            }
        };
        let rdf = namespace == RDF && local == "RDF";
        if rdf {
            self.rdf_elements = self.rdf_elements.saturating_add(1);
        }
        // ISO 16684-1 gives an ordered array the `rdf:Seq` spelling, so the sequence directly
        // inside the property element is the one an event is appended to. A `Seq` deeper than
        // that belongs to a field of some entry rather than to the history.
        let in_history = under == Some(Landmark::History);
        let landmark = if rdf {
            Landmark::Rdf
        } else if describes {
            Landmark::Description
        } else if namespace == XMP_MM && local == "History" {
            self.history_elements = self.history_elements.saturating_add(1);
            Landmark::History
        } else if in_history && namespace == RDF && local == "Seq" {
            Landmark::HistorySeq
        } else {
            Landmark::Ordinary
        };
        if !inherited && !own {
            // ISO 16684-1 section 7.5's other spelling: a property stated as an attribute of the
            // description it belongs to. Removing the attribute removes the property, and the
            // element it sat on is somebody else's.
            for (prefix, local, _, from, to) in attributes {
                if prefix.is_empty() {
                    continue;
                }
                let attribute = self.namespace(prefix);
                let matched = match cut {
                    Cut::Namespaces(namespaces) => namespaces.contains(&attribute),
                    Cut::Properties(properties) => {
                        describes && listed(properties, attribute, local)
                    }
                };
                if matched {
                    self.deletions.push((*from, *to));
                }
            }
        }
        self.stack.push(Opened {
            tag: (prefix.clone(), local.clone()),
            bindings,
            removing: (own && !inherited).then_some(*start),
            doomed: own || inherited,
            landmark,
        });
        Ok(())
    }

    /// Closes an element, ending a deletion where this element began one.
    ///
    /// `content_ends` is where the close tag starts, and `None` for a self-closing element.
    fn close(
        &mut self,
        tag: &(String, String),
        end: usize,
        content_ends: Option<usize>,
    ) -> Result<(), WriteError> {
        let Some(open) = self.stack.pop() else {
            return Err(WriteError::Malformed {
                detail: format!("</{}> closes nothing", spelled(tag)),
            });
        };
        if content_ends.is_some() && open.tag != *tag {
            return Err(WriteError::Malformed {
                detail: format!("</{}> closes <{}>", spelled(tag), spelled(&open.tag)),
            });
        }
        for _ in 0..open.bindings {
            self.bindings.pop();
        }
        if let Some(from) = open.removing {
            self.deletions.push((from, end));
        }
        if open.landmark == Landmark::Rdf {
            self.inside_rdf = match (self.rdf_elements, content_ends) {
                (1, Some(at)) => Some(at),
                _ => None,
            };
        }
        if open.landmark == Landmark::HistorySeq {
            // The one history, with the one sequence in it, and neither written self-closing:
            // anything else leaves this `None` and [`record`] refuses rather than guessing which
            // of two sequences an event belongs at the end of.
            self.history_seq_end = match (self.history_elements, self.history_seq_end, content_ends)
            {
                (1, None, Some(at)) => Some(at),
                _ => None,
            };
        }
        Ok(())
    }
}

/// Whether two `xml:lang` values name the same language.
///
/// ISO 16684-1 section 6.4 requires every comparison of `xml:lang` values to be
/// case-insensitive, and it says so by way of IETF RFC 3066, whose tags are ASCII throughout —
/// so an ASCII fold is the whole of the rule rather than an approximation of it. This module
/// compared the tags exactly until the nine-hundred-and-forty-sixth session, which made a packet
/// writing `X-Default` or `EN-GB` a packet whose title this reader could not find: the sentence
/// became readable when a copy of that standard reaching its section 7.2 arrived.
fn same_language(one: &str, other: &str) -> bool {
    one.eq_ignore_ascii_case(other)
}

/// The packet's bytes as text, in whichever of the three encodings ISO 16684-1 section 7.1 names
/// — UTF-8, UTF-16 or UTF-32. Paraphrased rather than quoted, because a verbatim sentence in this
/// tree is one the conformance checker verifies against `doc/md/`, and that directory holds
/// ISO 32000-2 and not this standard.
///
/// **Section 7.1 puts the choice between the three beyond its own scope** and leaves it to
/// whichever standard embeds the packet, so a reader that took one of them for the rule would be
/// inventing one. §14.3.2 embeds packets in PDF and states no encoding either. This citation
/// said section 7.3.2 until the nine-hundred-and-forty-sixth session, when a copy of the standard
/// reaching section 7.2 arrived and the sentence turned out to be in section 7.1 saying rather
/// less than the citation implied.
///
/// UTF-8 is what every one of the 319 corpus streams uses and what the `<?xpacket>` header's
/// `begin` attribute signals by carrying U+FEFF in the packet's own encoding. The other two are
/// decoded here rather than refused, because refusing a spelling the standard names is a gap
/// dressed as a limit — and both are twenty lines.
/// A packet's bytes as text, refusing one past [`MAX_BYTES`] before decoding it.
fn text_of(bytes: &[u8]) -> Result<String, XmpError> {
    if bytes.len() > MAX_BYTES {
        return Err(XmpError::TooLarge { bytes: bytes.len() });
    }
    decode(bytes)
}

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
    /// `structured` is ISO 16684-1 section 7.6's `rdf:parseType="Resource"`, which says the value
    /// is a structure *before* any child arrives — and saying so up front is the only way a
    /// self-closing structured property is distinguishable from an empty simple one.
    Property { name: Name, structured: bool },
    /// A field of a structured value: the same element shape as [`Kind::Property`], belonging to
    /// the structure that contains it rather than to the packet.
    ///
    /// Kept apart from a property for exactly that reason — a field recorded at the top level
    /// would make `xmpMM:History`'s `stEvt:action` look like a property of the document.
    Field { name: Name, structured: bool },
    /// `rdf:Alt`, `rdf:Seq` or `rdf:Bag` under a property, a field or an item.
    Container { ordered: Container },
    /// `rdf:li` under a container.
    Item {
        language: Option<String>,
        structured: bool,
    },
    /// An `rdf:Description` nested inside a property, a field or an item.
    ///
    /// ISO 16684-1 section 7.6's other spelling of a structured value: the fields are the
    /// description's children and its attributes rather than the property element's.
    Nested,
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
    /// The same, in the shape [`Detail`] keeps. `None` throughout unless detail is wanted.
    detail: Option<Detail>,
    /// A container's items so far.
    items: Vec<(Option<String>, String)>,
    /// The same items as [`Detail`], empty unless detail is wanted.
    item_details: Vec<(Option<String>, Detail)>,
    /// The fields of a structured value, empty unless detail is wanted.
    fields: Vec<Property>,
    /// Whether a child this reader does not interpret has closed inside this element, which
    /// makes the value a structure whatever else it holds.
    opaque: bool,
    /// How many child elements have opened inside this one.
    ///
    /// An element with a child is not a leaf, and ISO 16684-1 section 7.2's restriction on where
    /// character data may appear turns on exactly that — see [`Xmp::stray_character_data`]. A
    /// count rather than a flag because it costs the same and reads as what it is.
    children: usize,
}

/// The walk over one packet.
struct Reader {
    stack: Vec<Frame>,
    /// Prefix-to-URI bindings, innermost last. A prefix rebound in a child shadows its parent's,
    /// which is why this is a stack searched backwards rather than a map.
    bindings: Vec<(String, String)>,
    properties: Vec<(Name, Value)>,
    /// Whether to build [`Reader::details`] as well, which is [`Xmp::parse_detail`]'s reading.
    detailed: bool,
    details: Vec<Property>,
    /// How many fields of structured values have been kept, against [`MAX_PROPERTIES`].
    ///
    /// A separate count from the properties': a packet's fields are unbounded in a way its
    /// properties are not, and bounding them together would make [`Xmp::parse`] refuse packets
    /// it accepts today.
    fields: usize,
    /// How many top-level `rdf:RDF` elements the packet stated, for [`Xmp::rdf_elements`].
    rdf_elements: usize,
    /// The elements that carried character data where ISO 16684-1 section 7.2 allows none.
    ///
    /// Built by every walk rather than by a third entry point, because it costs a conforming
    /// packet nothing: the vector stays empty, and what fills it is one comparison per element
    /// over character content this reader has already accumulated.
    strays: Vec<String>,
}

impl Reader {
    fn new(detailed: bool) -> Self {
        Self {
            stack: Vec::new(),
            bindings: Vec::new(),
            properties: Vec::new(),
            detailed,
            details: Vec::new(),
            fields: 0,
            rdf_elements: 0,
            strays: Vec::new(),
        }
    }

    fn run(mut self, text: &str) -> Result<Self, XmpError> {
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
        Ok(self)
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
        if kind == Kind::Rdf {
            self.rdf_elements = self.rdf_elements.saturating_add(1);
        }

        // §7.5's attribute form: every attribute of a description that is neither a namespace
        // declaration nor RDF's own is a simple property of it — of the packet where the
        // description is a top-level one, and of the structure where it is nested inside a value.
        let mut fields = Vec::new();
        if matches!(kind, Kind::Description | Kind::Nested) {
            for (prefix, local, value) in attributes {
                if prefix == "xmlns" || (prefix.is_empty() && local == "xmlns") {
                    continue;
                }
                let namespace = self.namespace(prefix);
                if namespace == RDF || namespace == XML || prefix.is_empty() {
                    continue;
                }
                let stated = Name {
                    namespace,
                    local: local.clone(),
                };
                // An attribute value is no more unescaped than character content is.
                let text = unescaped(value);
                if kind == Kind::Description {
                    let detail = self.detailed.then(|| Detail::Text(text.clone()));
                    self.record(stated, prefix.clone(), Value::Text(text), detail)?;
                } else if self.detailed {
                    self.keep_field(
                        &mut fields,
                        Property {
                            name: stated,
                            prefix: prefix.clone(),
                            value: Detail::Text(text),
                        },
                    )?;
                }
            }
        }

        if let Some(parent) = self.stack.last_mut() {
            parent.children = parent.children.saturating_add(1);
        }
        self.stack.push(Frame {
            tag: name,
            kind,
            bindings,
            text: String::new(),
            value: None,
            detail: None,
            items: Vec::new(),
            item_details: Vec::new(),
            fields,
            opaque: false,
            children: 0,
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
                name: named(namespace, local),
                structured: self.resource(attributes),
            },
            // A property, a field and an item hold a value, so the same three things may open
            // inside each: an array, a nested description, or — where the element has already
            // said its value is a structure — a field of it.
            Some(
                Kind::Property { structured, .. }
                | Kind::Field { structured, .. }
                | Kind::Item { structured, .. },
            ) => match (rdf, local) {
                (true, "Alt") => Kind::Container {
                    ordered: Container::Alt,
                },
                (true, "Seq") => Kind::Container {
                    ordered: Container::Seq,
                },
                (true, "Bag") => Kind::Container {
                    ordered: Container::Bag,
                },
                (true, "Description") => Kind::Nested,
                _ if *structured => Kind::Field {
                    name: named(namespace, local),
                    structured: self.resource(attributes),
                },
                _ => Kind::Uninterpreted,
            },
            Some(Kind::Nested) => Kind::Field {
                name: named(namespace, local),
                structured: self.resource(attributes),
            },
            Some(Kind::Container { .. }) => {
                if rdf && local == "li" {
                    let language = attributes
                        .iter()
                        .find(|(prefix, local, _)| self.namespace(prefix) == XML && local == "lang")
                        .map(|(_, _, value)| unescaped(value));
                    Kind::Item {
                        language,
                        structured: self.resource(attributes),
                    }
                } else {
                    Kind::Uninterpreted
                }
            }
            Some(Kind::Uninterpreted) => Kind::Uninterpreted,
        }
    }

    /// Whether an element states ISO 16684-1 section 7.6's `rdf:parseType="Resource"`.
    fn resource(&self, attributes: &[(String, String, String)]) -> bool {
        attributes.iter().any(|(prefix, local, value)| {
            self.namespace(prefix) == RDF && local == "parseType" && value == "Resource"
        })
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
        if self.strays.len() < MAX_PROPERTIES && carries_stray_character_data(&frame) {
            self.strays.push(spelled(&frame.tag));
        }

        let detailed = self.detailed;
        let text = trimmed(&frame.text);
        match frame.kind {
            Kind::Item {
                language,
                structured,
            } => {
                let detail = detailed
                    .then(|| settle(frame.detail, frame.fields, frame.opaque, structured, &text));
                self.push_item(language, text, detail)?;
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
                let items = frame.item_details;
                let detail = detailed.then(|| match ordered {
                    Container::Alt => Detail::Alt(items),
                    Container::Seq => {
                        Detail::Seq(items.into_iter().map(|(_, detail)| detail).collect())
                    }
                    Container::Bag => {
                        Detail::Bag(items.into_iter().map(|(_, detail)| detail).collect())
                    }
                });
                self.decide(value, detail);
            }
            Kind::Property { name, structured } => {
                let value = frame.value.unwrap_or_else(|| {
                    if structured {
                        Value::Structure
                    } else {
                        Value::Text(text.clone())
                    }
                });
                let prefix = frame.tag.0;
                let detail = detailed
                    .then(|| settle(frame.detail, frame.fields, frame.opaque, structured, &text));
                self.record(name, prefix, value, detail)?;
            }
            Kind::Field { name, structured } => {
                if detailed {
                    if self.fields >= MAX_PROPERTIES {
                        return Err(XmpError::TooMuch { what: "properties" });
                    }
                    let field = Property {
                        name,
                        prefix: frame.tag.0,
                        value: settle(frame.detail, frame.fields, frame.opaque, structured, &text),
                    };
                    if let Some(structure) = self.stack.last_mut() {
                        structure.fields.push(field);
                        self.fields = self.fields.saturating_add(1);
                    }
                }
            }
            // A nested description is the other spelling of a structured value; an uninterpreted
            // element under a value-holder is one this reader does not read into. Both make the
            // value a structure, which is the difference between a gap and a silence.
            Kind::Nested => {
                let detail = detailed.then_some(Detail::Structure(frame.fields));
                self.decide(Value::Structure, detail);
            }
            Kind::Uninterpreted => {
                if let Some(property) = self.stack.last_mut()
                    && matches!(
                        property.kind,
                        Kind::Property { .. } | Kind::Field { .. } | Kind::Item { .. }
                    )
                {
                    property.value = Some(Value::Structure);
                    property.opaque = true;
                }
            }
            Kind::Outside | Kind::Rdf | Kind::Description => {}
        }
        Ok(())
    }

    fn record(
        &mut self,
        name: Name,
        prefix: String,
        value: Value,
        detail: Option<Detail>,
    ) -> Result<(), XmpError> {
        if self.properties.len() >= MAX_PROPERTIES {
            return Err(XmpError::TooMuch { what: "properties" });
        }
        if let Some(detail) = detail {
            self.details.push(Property {
                name: name.clone(),
                prefix,
                value: detail,
            });
        }
        self.properties.push((name, value));
        Ok(())
    }

    /// Hands a closing child's value to the property, field or item that contains it.
    fn decide(&mut self, value: Value, detail: Option<Detail>) {
        if let Some(property) = self.stack.last_mut() {
            property.value = Some(value);
            if detail.is_some() {
                property.detail = detail;
            }
        }
    }

    /// Hands a closing `rdf:li`'s value to the container that holds it.
    fn push_item(
        &mut self,
        language: Option<String>,
        text: String,
        detail: Option<Detail>,
    ) -> Result<(), XmpError> {
        if let Some(container) = self.stack.last_mut() {
            if container.items.len() >= MAX_ITEMS {
                return Err(XmpError::TooMuch {
                    what: "array items",
                });
            }
            container.items.push((language.clone(), text));
            if let Some(detail) = detail {
                container.item_details.push((language, detail));
            }
        }
        Ok(())
    }

    /// Keeps one field of a structured value, against [`MAX_PROPERTIES`].
    fn keep_field(&mut self, fields: &mut Vec<Property>, field: Property) -> Result<(), XmpError> {
        if self.fields >= MAX_PROPERTIES {
            return Err(XmpError::TooMuch { what: "properties" });
        }
        self.fields = self.fields.saturating_add(1);
        fields.push(field);
        Ok(())
    }
}

/// What a property, a field or an item's value is once its element has closed.
///
/// A child element that decided it wins; otherwise the element is a structure where it said so,
/// where it holds fields, or where something this reader does not interpret closed inside it, and
/// is its character content in every other case.
fn settle(
    decided: Option<Detail>,
    fields: Vec<Property>,
    opaque: bool,
    structured: bool,
    text: &str,
) -> Detail {
    decided.unwrap_or_else(|| {
        if structured || opaque || !fields.is_empty() {
            Detail::Structure(fields)
        } else {
            Detail::Text(text.to_owned())
        }
    })
}

/// A resolved name from its two halves.
fn named(namespace: &str, local: &str) -> Name {
    Name {
        namespace: namespace.to_owned(),
        local: local.to_owned(),
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

/// Whether one element held character data ISO 16684-1 section 7.2 gives it no room for.
///
/// That subclause's last sentence confines non-white character data to the element content of
/// leaf elements standing for simple XMP values. Two of the three tests it needs are structural
/// and need no further clause: an element with a child element is not a leaf, and RDF's own
/// grammar elements stand for a description or one of clause 6.3.4's arrays rather than for a
/// simple value. The third is the packet's own declaration — section 7.6's
/// `rdf:parseType="Resource"` says the value is a structure before any child arrives.
///
/// **Two kinds are passed over deliberately**, and each would need a clause this project does not
/// hold to report: an element above the packet's `rdf:RDF`, whose subject is section 7.3, and one
/// whose content this reader does not interpret, which it therefore cannot call a leaf simple
/// value either. Under-reporting is the standing direction of error.
fn carries_stray_character_data(frame: &Frame) -> bool {
    if frame.text.chars().all(is_xml_white_space) {
        return false;
    }
    match &frame.kind {
        Kind::Outside | Kind::Uninterpreted => false,
        Kind::Rdf | Kind::Description | Kind::Nested | Kind::Container { .. } => true,
        Kind::Property { structured, .. }
        | Kind::Field { structured, .. }
        | Kind::Item { structured, .. } => *structured || frame.children > 0,
    }
}

/// White space as ISO 16684-1 clause 3 defines it: a space, a carriage return, a line feed or a
/// tab, and nothing else.
///
/// Narrower than [`str::trim`]'s Unicode notion on purpose — a no-break space is character data
/// that standard's own definition does not excuse, and trimming it away would lose the finding.
fn is_xml_white_space(character: char) -> bool {
    matches!(character, ' ' | '\r' | '\n' | '\t')
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
    use super::{
        DC, Event, Name, PDF, RESOURCE_EVENT, Schema, Value, WriteError, XMP, XMP_MM, Xmp,
        XmpError, instant, packet, record, remove, restate,
    };

    /// The identification namespace, spelled as ISO 19005-4 section 6.7.3 prints it. Used here
    /// only as *a* namespace to restate: nothing in this module knows what PDF/A is.
    const IDENTIFICATION: &str = "https://www.aiim.org/pdfa/ns/id/";
    /// The same namespace with the scheme ISO 19005-2 section 6.6.4 prints, which is why
    /// [`restate`] takes a list.
    const OLD_IDENTIFICATION: &str = "http://www.aiim.org/pdfa/ns/id/";

    /// The schema the tests below write.
    fn identification(part: &str) -> Vec<(&'static str, String)> {
        vec![("part", part.to_owned()), ("rev", "2020".to_owned())]
    }

    /// The event the tests below record.
    const CONVERTED: Event<'static> = Event {
        action: "converted",
        parameters: "a DeviceN DefaultCMYK stating ISO 32000-2 10.4.2.5's transform",
        when: "2026-09-10T11:22:33Z",
    };

    /// The three fields of the one entry a packet's history holds, in that entry's order.
    fn recorded(bytes: &[u8]) -> Vec<Vec<(String, String)>> {
        let properties = Xmp::parse_detail(bytes).expect("the packet parses");
        let mut out = Vec::new();
        for property in &properties {
            if property.name.namespace != XMP_MM || property.name.local != "History" {
                continue;
            }
            for entry in property.value.array().unwrap_or_default() {
                out.push(
                    entry
                        .fields()
                        .unwrap_or_default()
                        .iter()
                        .map(|field| {
                            (
                                field.name.local.clone(),
                                field.value.text().unwrap_or_default().to_owned(),
                            )
                        })
                        .collect(),
                );
            }
        }
        out
    }

    #[test]
    fn a_packet_with_no_history_gains_one_holding_the_event() {
        let before = packet(&Schema {
            namespace: IDENTIFICATION,
            prefix: "pdfaid",
            properties: &identification("2"),
        });
        let after = record(&before, &CONVERTED).expect("a packet this module wrote takes an event");
        let entries = recorded(&after);
        assert_eq!(entries.len(), 1, "one entry: {entries:?}");
        assert_eq!(
            entries[0],
            vec![
                ("action".to_owned(), CONVERTED.action.to_owned()),
                ("parameters".to_owned(), CONVERTED.parameters.to_owned()),
                ("when".to_owned(), CONVERTED.when.to_owned()),
            ]
        );
        // The identification schema the packet already stated is untouched by the recording.
        let read = Xmp::parse(&after).expect("the packet parses");
        assert_eq!(read.text(IDENTIFICATION, "part"), Some("2"));
        assert_eq!(Xmp::rdf_elements(&after), Ok(1));
        let fields = Xmp::parse_detail(&after).expect("the packet parses");
        assert!(
            fields.iter().any(|property| property
                .value
                .array()
                .and_then(|items| items.first())
                .and_then(|entry| entry.field(RESOURCE_EVENT, "when"))
                .is_some()),
            "the fields are in the ResourceEvent namespace"
        );
    }

    /// ISO 19005-2 section 6.6.5's whole point: a history is the record of what happened before,
    /// so an event is appended to it and the producer's entries stay where they were.
    #[test]
    fn an_event_is_appended_after_the_entries_a_producer_wrote() {
        let before = br#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about="" xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/"
 xmlns:stEvt="http://ns.adobe.com/xap/1.0/sType/ResourceEvent#">
<xmpMM:History><rdf:Seq>
<rdf:li rdf:parseType="Resource">
<stEvt:action>created</stEvt:action>
<stEvt:parameters>by hand</stEvt:parameters>
<stEvt:when>2016-04-05T13:19:21+01:00</stEvt:when>
</rdf:li>
</rdf:Seq></xmpMM:History>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;
        let after = record(before, &CONVERTED).expect("a history takes another entry");
        let entries = recorded(&after);
        assert_eq!(entries.len(), 2, "both entries: {entries:?}");
        assert_eq!(entries[0][0].1, "created", "the producer's entry is first");
        assert_eq!(entries[1][0].1, "converted", "and ours is after it");
        assert!(
            String::from_utf8_lossy(&after).contains("2016-04-05T13:19:21+01:00"),
            "the producer's own bytes cross unchanged"
        );
    }

    /// A packet stating two histories is one this writer cannot append to without choosing which,
    /// and choosing would be inventing where the event belongs.
    #[test]
    fn a_packet_stating_two_histories_is_refused_rather_than_edited() {
        let before = br#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about="" xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/">
<xmpMM:History><rdf:Seq></rdf:Seq></xmpMM:History>
</rdf:Description>
<rdf:Description rdf:about="" xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/">
<xmpMM:History><rdf:Seq></rdf:Seq></xmpMM:History>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;
        assert_eq!(
            record(before, &CONVERTED),
            Err(WriteError::NoPlaceForAnEvent { found: 2 })
        );
    }

    /// The `when` field's form, at four instants the calendar's own rules pick out.
    #[test]
    fn an_instant_is_the_utc_calendar_date_of_the_seconds_since_the_epoch() {
        let at = |seconds| {
            instant(std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds))
                .expect("an instant after the epoch")
        };
        assert_eq!(at(0), "1970-01-01T00:00:00Z");
        // 2000-02-29, the leap day the four-hundred-year rule keeps.
        assert_eq!(at(951_782_400), "2000-02-29T00:00:00Z");
        // 2100 is a century that is not a leap year, so February ends on the 28th: the
        // hundred-year rule takes that leap day away where the four-year rule would give it.
        assert_eq!(at(4_107_542_399), "2100-02-28T23:59:59Z");
        assert_eq!(at(1_767_225_599), "2025-12-31T23:59:59Z");
        assert_eq!(
            instant(std::time::UNIX_EPOCH - std::time::Duration::from_secs(1)),
            None,
            "a clock reporting a time before the epoch is one nothing here writes a date from"
        );
    }

    #[test]
    fn a_fresh_packet_states_the_schema_and_reads_back() {
        let properties = identification("4");
        let written = packet(&Schema {
            namespace: IDENTIFICATION,
            prefix: "pdfaid",
            properties: &properties,
        });
        let read = Xmp::parse(&written).expect("the packet this module wrote parses");
        assert_eq!(read.text(IDENTIFICATION, "part"), Some("4"));
        assert_eq!(read.text(IDENTIFICATION, "rev"), Some("2020"));
        assert_eq!(
            Xmp::rdf_elements(&written),
            Ok(1),
            "one rdf:RDF element, which is what ISO 19005 requires of a packet"
        );
    }

    #[test]
    fn restating_replaces_the_schema_and_leaves_every_other_property_alone() {
        let before = br#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
 xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/" pdfaid:conformance="B">
<pdf:Producer>Somebody's exporter</pdf:Producer>
<pdfaid:part>2</pdfaid:part>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;
        let properties = identification("4");
        let after = restate(
            before,
            &[IDENTIFICATION, OLD_IDENTIFICATION],
            &Schema {
                namespace: IDENTIFICATION,
                prefix: "pdfaid",
                properties: &properties,
            },
        )
        .expect("a packet this writer can edit");
        let read = Xmp::parse(&after).expect("the edited packet parses");
        assert_eq!(
            read.text(PDF, "Producer"),
            Some("Somebody's exporter"),
            "the producer's own property crosses untouched"
        );
        assert_eq!(
            read.text(IDENTIFICATION, "part"),
            Some("4"),
            "the new claim"
        );
        assert_eq!(
            read.text(OLD_IDENTIFICATION, "conformance"),
            None,
            "the attribute spelling of the old claim is gone with the element spelling"
        );
        assert_eq!(read.text(IDENTIFICATION, "rev"), Some("2020"));
        assert!(
            String::from_utf8_lossy(&after).contains("<?xpacket end=\"w\"?>"),
            "the wrapper is the producer's, byte for byte"
        );
    }

    /// A packet stating a property twice — once as an element and once as an attribute — and a
    /// field of a structured value that shares one of their local names.
    const TWO_SPELLINGS: &[u8] = br#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
 xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/" pdf:Keywords="one two">
<pdf:Producer>Somebody's exporter</pdf:Producer>
<xmpMM:History><rdf:Seq><rdf:li rdf:parseType="Resource">
<pdf:Producer>a field that is not a property</pdf:Producer>
</rdf:li></rdf:Seq></xmpMM:History>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

    #[test]
    fn a_named_property_goes_in_both_its_spellings_and_a_field_of_that_name_stays() {
        let after = remove(
            TWO_SPELLINGS,
            &[
                Name {
                    namespace: PDF.to_owned(),
                    local: "Producer".to_owned(),
                },
                Name {
                    namespace: PDF.to_owned(),
                    local: "Keywords".to_owned(),
                },
            ],
        )
        .expect("a packet this writer can edit");
        let read = Xmp::parse(&after).expect("the edited packet parses");
        assert_eq!(
            read.text(PDF, "Producer"),
            None,
            "the element spelling is cut"
        );
        assert_eq!(
            read.text(PDF, "Keywords"),
            None,
            "and so is the attribute spelling of a simple value"
        );
        let history = recorded(&after);
        assert_eq!(
            history.len(),
            1,
            "the history entry stays: a field of a structured value is not a property of the \
             packet, whatever it is called"
        );
        assert!(
            String::from_utf8_lossy(&after).contains("a field that is not a property"),
            "and neither is its value"
        );
    }

    #[test]
    fn a_property_no_packet_states_leaves_every_byte_alone() {
        let after = remove(
            TWO_SPELLINGS,
            &[Name {
                namespace: DC.to_owned(),
                local: "title".to_owned(),
            }],
        )
        .expect("a packet this writer can edit");
        assert_eq!(
            after, TWO_SPELLINGS,
            "nothing named, nothing cut, and no byte rewritten on the way through"
        );
    }

    #[test]
    fn a_packet_whose_tags_do_not_nest_is_refused_rather_than_cut() {
        assert!(matches!(
            remove(
                b"<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
                  <a></b></rdf:RDF>",
                &[]
            ),
            Err(WriteError::Malformed { .. })
        ));
    }

    #[test]
    fn a_packet_with_no_place_for_a_description_is_refused_rather_than_replaced() {
        assert_eq!(
            restate(
                b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"/>",
                &[IDENTIFICATION],
                &Schema {
                    namespace: IDENTIFICATION,
                    prefix: "pdfaid",
                    properties: &[],
                },
            ),
            Err(WriteError::NoPlaceForADescription { found: 0 })
        );
    }

    #[test]
    fn a_packet_whose_tags_do_not_match_is_refused_rather_than_edited() {
        let broken =
            b"<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><a></b></rdf:RDF>";
        assert!(
            matches!(
                restate(
                    broken,
                    &[IDENTIFICATION],
                    &Schema {
                        namespace: IDENTIFICATION,
                        prefix: "pdfaid",
                        properties: &[],
                    },
                ),
                Err(WriteError::Malformed { .. })
            ),
            "a span in a packet whose tags do not nest is not a span this writer trusts"
        );
    }

    #[test]
    fn a_value_carrying_markup_is_escaped() {
        let properties = vec![("part", "4 < 5 & true".to_owned())];
        let written = packet(&Schema {
            namespace: IDENTIFICATION,
            prefix: "pdfaid",
            properties: &properties,
        });
        let read = Xmp::parse(&written).expect("the packet parses");
        assert_eq!(read.text(IDENTIFICATION, "part"), Some("4 < 5 & true"));
    }

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

    /// ISO 16684-1 section 7.6's structured value is reported as present and uninterpreted, which
    /// is the whole difference between a gap and a silence.
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

    /// The nested reading keeps what the flat one drops, and the flat one is unchanged by it.
    ///
    /// The packet is ISO 19005-2 section 6.6.2.3.3's extension schema container in miniature: a bag
    /// whose items are structures, one of whose fields is a sequence of structures. Nothing but
    /// [`Xmp::parse_detail`] can see past the outermost of those.
    #[test]
    fn a_structure_is_read_to_its_fields_when_a_caller_asks_for_them() {
        const EXTENSION: &str = "http://www.aiim.org/pdfa/ns/extension/";
        const SCHEMA: &str = "http://www.aiim.org/pdfa/ns/schema#";
        const PROPERTY: &str = "http://www.aiim.org/pdfa/ns/property#";
        let packet = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
          <rdf:Description rdf:about=""
              xmlns:pdfaExtension="http://www.aiim.org/pdfa/ns/extension/"
              xmlns:pdfaSchema="http://www.aiim.org/pdfa/ns/schema#"
              xmlns:pdfaProperty="http://www.aiim.org/pdfa/ns/property#">
            <pdfaExtension:schemas>
              <rdf:Bag>
                <rdf:li rdf:parseType="Resource">
                  <pdfaSchema:namespaceURI>http://example.test/ns/</pdfaSchema:namespaceURI>
                  <pdfaSchema:prefix>ex</pdfaSchema:prefix>
                  <pdfaSchema:property>
                    <rdf:Seq>
                      <rdf:li rdf:parseType="Resource">
                        <pdfaProperty:name>Serial</pdfaProperty:name>
                      </rdf:li>
                    </rdf:Seq>
                  </pdfaSchema:property>
                </rdf:li>
              </rdf:Bag>
            </pdfaExtension:schemas>
          </rdf:Description>
        </rdf:RDF>"#;

        // The flat reading has the bag and one empty item where the schema description is: an
        // array item is a string there, and a structure has no string to be.
        let flat = Xmp::parse(packet.as_bytes()).expect("well-formed");
        assert_eq!(
            flat.value(EXTENSION, "schemas"),
            Some(&Value::Bag(vec![String::new()]))
        );

        let detailed = Xmp::parse_detail(packet.as_bytes()).expect("well-formed");
        let [schemas] = detailed.as_slice() else {
            panic!("the packet states one property, and a field is not a property");
        };
        assert_eq!(schemas.prefix, "pdfaExtension");
        let [schema] = schemas.value.array().expect("an rdf:Bag") else {
            panic!("the bag holds one schema description");
        };
        assert_eq!(
            schema
                .field(SCHEMA, "namespaceURI")
                .and_then(|field| field.value.text()),
            Some("http://example.test/ns/")
        );
        let properties = schema
            .field(SCHEMA, "property")
            .expect("the description states its properties");
        let [property] = properties.value.array().expect("an rdf:Seq") else {
            panic!("one property is described");
        };
        assert_eq!(
            property
                .field(PROPERTY, "name")
                .and_then(|field| field.value.text()),
            Some("Serial")
        );
        assert_eq!(
            property.field(PROPERTY, "valueType"),
            None,
            "a field the packet does not state is absent rather than empty"
        );
    }

    /// The other spelling of a structure, and the qualifiers an `rdf:Alt` keeps.
    #[test]
    fn a_nested_description_is_a_structure_and_an_alt_keeps_its_languages() {
        let packet = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
          <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/"
                           xmlns:xmpTPg="http://ns.adobe.com/xap/1.0/t/pg/"
                           xmlns:stDim="http://ns.adobe.com/xap/1.0/sType/Dimensions#">
            <xmpTPg:MaxPageSize>
              <rdf:Description stDim:w="595" stDim:h="842"/>
            </xmpTPg:MaxPageSize>
            <dc:title><rdf:Alt>
              <rdf:li xml:lang="x-default">Report</rdf:li>
              <rdf:li>Bericht</rdf:li>
            </rdf:Alt></dc:title>
          </rdf:Description>
        </rdf:RDF>"#;
        let detailed = Xmp::parse_detail(packet.as_bytes()).expect("well-formed");
        let size = &detailed
            .iter()
            .find(|property| property.name.local == "MaxPageSize")
            .expect("stated")
            .value;
        assert_eq!(
            size.field("http://ns.adobe.com/xap/1.0/sType/Dimensions#", "w")
                .and_then(|field| field.value.text()),
            Some("595"),
            "a nested description's attributes are the structure's fields"
        );
        let title = &detailed
            .iter()
            .find(|property| property.name.local == "title")
            .expect("stated")
            .value;
        let items = title.alternatives().expect("an rdf:Alt");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0.as_deref(), Some("x-default"));
        assert_eq!(items[0].1.text(), Some("Report"));
        assert_eq!(
            items[1].0, None,
            "an item that states no language is reported without one rather than dropped"
        );
    }

    /// ISO 16684-1 section 6.4 makes every comparison of `xml:lang` values case-insensitive.
    #[test]
    fn a_language_tag_is_matched_without_regard_to_case() {
        let packet = PACKET
            .replace("x-default", "X-Default")
            .replace("de-DE", "DE-de");
        let xmp = Xmp::parse(packet.as_bytes()).expect("well-formed");
        assert_eq!(
            xmp.title(),
            Some("Annual & final report"),
            "the default alternative is found whatever case the packet wrote it in"
        );
        assert_eq!(xmp.text_in(DC, "title", "de-de"), Some("Jahresbericht"));
        assert_eq!(
            xmp.text_in(DC, "title", "fr"),
            None,
            "and nothing is invented"
        );
    }

    /// ISO 16684-1 section 7.1 serialises one packet as one `rdf:RDF` element, and a validator
    /// has to be able to see how many a stream states.
    #[test]
    fn the_rdf_roots_of_a_packet_are_counted() {
        assert_eq!(
            Xmp::rdf_elements(PACKET.as_bytes()).expect("well-formed"),
            1
        );

        let two = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"/>\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"/>\
             </x:xmpmeta>";
        assert_eq!(Xmp::rdf_elements(two.as_bytes()).expect("well-formed"), 2);

        let none = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"/>";
        assert_eq!(Xmp::rdf_elements(none.as_bytes()).expect("well-formed"), 0);
    }

    /// ISO 16684-1 section 7.2 keeps non-white character data out of everything but a leaf
    /// element standing for a simple value, and a validator has to be able to see where it sat.
    #[test]
    fn character_data_outside_a_simple_value_is_named() {
        assert!(
            Xmp::stray_character_data(PACKET.as_bytes())
                .expect("well-formed")
                .is_empty(),
            "a conforming packet indents freely and reports nothing"
        );

        // One run of text in each of the four places a packet has no room for it: the RDF root,
        // a description, a container, and a property whose value a child element carries.
        let stray = PACKET
            .replace("<dc:title>", "<dc:title>lost")
            .replace("<rdf:Alt>", "<rdf:Alt>lost")
            .replace("</rdf:Description>", "lost</rdf:Description>")
            .replace("</rdf:RDF>", "lost</rdf:RDF>");
        let found = Xmp::stray_character_data(stray.as_bytes()).expect("well-formed");
        assert_eq!(
            found,
            vec![
                "rdf:Alt".to_owned(),
                "dc:title".to_owned(),
                "rdf:Description".to_owned(),
                "rdf:RDF".to_owned(),
            ],
            "each element is named once, in the order the elements closed"
        );
    }

    /// The two structural halves of the same sentence, apart from each other.
    #[test]
    fn a_leaf_simple_value_may_hold_text_and_a_structure_may_not() {
        let leaves = "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
             <rdf:Description rdf:about=\"\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">\
             <pdf:Keywords>annual, report</pdf:Keywords>\
             </rdf:Description></rdf:RDF>";
        assert!(
            Xmp::stray_character_data(leaves.as_bytes())
                .expect("well-formed")
                .is_empty()
        );

        // Section 7.6's structured spelling: the element says its value is a structure before any
        // field arrives, so its own character content stands for nothing.
        let structured = "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
             <rdf:Description rdf:about=\"\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">\
             <pdf:Keywords rdf:parseType=\"Resource\">annual</pdf:Keywords>\
             </rdf:Description></rdf:RDF>";
        assert_eq!(
            Xmp::stray_character_data(structured.as_bytes()).expect("well-formed"),
            vec!["pdf:Keywords".to_owned()]
        );
    }

    /// White space is ISO 16684-1 clause 3's four characters, and a no-break space is not one.
    #[test]
    fn a_no_break_space_is_character_data() {
        let packet = "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
             \u{a0}<rdf:Description rdf:about=\"\"/></rdf:RDF>";
        assert_eq!(
            Xmp::stray_character_data(packet.as_bytes()).expect("well-formed"),
            vec!["rdf:RDF".to_owned()]
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
