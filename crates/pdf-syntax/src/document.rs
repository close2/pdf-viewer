//! A whole file: cross-references, object access, and stream decoding.
//!
//! # Lazy by design
//!
//! Opening a document reads the cross-reference information and nothing else. Objects are
//! parsed when asked for, and decoded streams are cached. That is what makes a 500-page
//! file open as fast as a 5-page one, which `CLAUDE.md` principle 2 requires — eagerly
//! walking a page tree of thousands of nodes is the most common reason viewers feel slow
//! to start.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::{SyntaxError, SyntaxResult};
use crate::object::{Dictionary, Object, ObjectId, Stream};
use crate::parser::{Limits, Parser};
use crate::xref::{Location, XrefTable};

/// The most indirect references that will be followed in a chain.
///
/// `1 0 obj 2 0 R endobj` pointing back at itself is a cycle, and a chain of a thousand
/// references is hostile rather than merely unusual.
const MAX_REFERENCE_DEPTH: usize = 64;

/// An open PDF file.
///
/// Holds the bytes and resolves objects on demand. Cheap to open and cheap to clone the
/// underlying bytes, since they are shared.
pub struct Document {
    bytes: Arc<[u8]>,
    xref: XrefTable,
    limits: Limits,
    /// Objects already parsed, so a repeated lookup does not re-parse.
    cache: RefCell<BTreeMap<u32, Object>>,
    /// Object streams already expanded, keyed by the stream's object number.
    expanded_streams: RefCell<BTreeMap<u32, Arc<BTreeMap<u32, Object>>>>,
}

impl std::fmt::Debug for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Document")
            .field("bytes", &self.bytes.len())
            .field("objects", &self.xref.len())
            .field("recovered_by_scan", &self.xref.recovered_by_scan())
            // The caches are deliberately omitted: they are an implementation detail whose
            // contents depend on access history, and printing them would make debug output
            // both enormous and non-reproducible.
            .finish_non_exhaustive()
    }
}

impl Document {
    /// Opens a document from its bytes.
    ///
    /// # Errors
    ///
    /// [`SyntaxError::NoHeader`] if this is not a PDF, and
    /// [`SyntaxError::NoCrossReferences`] if no objects can be located even by scanning.
    pub fn open(bytes: impl Into<Arc<[u8]>>) -> SyntaxResult<Self> {
        Self::open_with_limits(bytes, Limits::DEFAULT)
    }

    /// Opens a document with explicit resource bounds.
    ///
    /// # Errors
    ///
    /// As [`Self::open`].
    pub fn open_with_limits(bytes: impl Into<Arc<[u8]>>, limits: Limits) -> SyntaxResult<Self> {
        let bytes = bytes.into();
        let xref = crate::xref::read(&bytes, limits)?;
        Ok(Self {
            bytes,
            xref,
            limits,
            cache: RefCell::new(BTreeMap::new()),
            expanded_streams: RefCell::new(BTreeMap::new()),
        })
    }

    /// Returns the trailer dictionary.
    #[must_use]
    pub fn trailer(&self) -> &Dictionary {
        self.xref.trailer()
    }

    /// Returns the cross-reference table.
    #[must_use]
    pub fn xref(&self) -> &XrefTable {
        &self.xref
    }

    /// Returns `true` if the cross-reference table had to be rebuilt by scanning.
    #[must_use]
    pub fn was_recovered(&self) -> bool {
        self.xref.recovered_by_scan()
    }

    /// Returns the document catalogue.
    ///
    /// # Errors
    ///
    /// [`SyntaxError::TrailerMissing`] when the trailer has no `/Root`, or it does not
    /// resolve to a dictionary. Without a catalogue there is no page tree, so this is fatal
    /// rather than recoverable.
    pub fn catalog(&self) -> SyntaxResult<Dictionary> {
        let root = self
            .trailer()
            .get("Root")
            .cloned()
            .ok_or(SyntaxError::TrailerMissing { key: "/Root" })?;

        self.resolve(&root)
            .as_dict()
            .cloned()
            .ok_or(SyntaxError::TrailerMissing {
                key: "/Root (not a dictionary)",
            })
    }

    /// Fetches an indirect object, returning [`Object::Null`] if it cannot be read.
    ///
    /// Null rather than an error because the specification says a reference to a
    /// non-existent object *is* null. A missing object is therefore ordinary, not
    /// exceptional, and forcing every caller to handle a `Result` for it would bury the
    /// cases that matter.
    #[must_use]
    pub fn get(&self, id: ObjectId) -> Object {
        if let Some(cached) = self.cache.borrow().get(&id.number) {
            return cached.clone();
        }

        let object = self.load(id).unwrap_or(Object::Null);
        self.cache.borrow_mut().insert(id.number, object.clone());
        object
    }

    /// Resolves an object if it is a reference, following chains.
    ///
    /// A reference to a reference is unusual but legal. The chain is bounded by
    /// [`MAX_REFERENCE_DEPTH`], and a cycle resolves to null rather than looping.
    #[must_use]
    pub fn resolve(&self, object: &Object) -> Object {
        let mut current = object.clone();
        for _ in 0..MAX_REFERENCE_DEPTH {
            match current {
                Object::Reference(id) => current = self.get(id),
                other => return other,
            }
        }
        Object::Null
    }

    /// Looks up a key in a dictionary and resolves the result.
    ///
    /// The common operation by a wide margin: nearly every value in a PDF may be indirect,
    /// so a bare `dict.get` is almost always a bug waiting to happen.
    #[must_use]
    pub fn get_key(&self, dict: &Dictionary, key: &str) -> Object {
        dict.get(key)
            .map_or(Object::Null, |object| self.resolve(object))
    }

    /// Loads an object from wherever the cross-reference table says it is.
    fn load(&self, id: ObjectId) -> Option<Object> {
        match self.xref.location(id.number)? {
            Location::Offset(offset) => {
                let mut parser = Parser::at(&self.bytes, offset, self.limits);
                let (found, object) = parser.parse_indirect_object().ok()?;
                // A table pointing at the wrong object is a real corruption. Rejecting the
                // mismatch is safer than returning another object's contents under this
                // number, which would corrupt the document graph silently.
                if found.number == id.number {
                    Some(object)
                } else {
                    None
                }
            }
            Location::InStream { stream, index } => {
                let expanded = self.expand_object_stream(stream)?;
                // Objects in a stream are addressed by their own number, and `index` is
                // only a hint about ordering, so the number is authoritative.
                expanded
                    .get(&id.number)
                    .cloned()
                    .or_else(|| expanded.values().nth(index as usize).cloned())
            }
        }
    }

    /// Expands an object stream into its contained objects.
    fn expand_object_stream(&self, number: u32) -> Option<Arc<BTreeMap<u32, Object>>> {
        if let Some(cached) = self.expanded_streams.borrow().get(&number) {
            return Some(Arc::clone(cached));
        }

        let Location::Offset(offset) = self.xref.location(number)? else {
            // An object stream inside another object stream is not permitted, and
            // following it would be a route to unbounded recursion.
            return None;
        };

        let mut parser = Parser::at(&self.bytes, offset, self.limits);
        let (_, object) = parser.parse_indirect_object().ok()?;
        let stream = object.as_stream()?;
        let data = self.decoded_stream_data(stream)?;

        let count = self.get_key(&stream.dict, "N").as_integer().unwrap_or(0);
        let first = self
            .get_key(&stream.dict, "First")
            .as_integer()
            .and_then(|value| usize::try_from(value).ok())?;

        // The header is a list of (object number, relative offset) pairs.
        let mut header = crate::lexer::Lexer::new(data.get(..first).unwrap_or_default());
        let mut pairs = Vec::new();
        for _ in 0..count.max(0) {
            let (Some(crate::Token::Integer(object_number)), Some(crate::Token::Integer(at))) =
                (header.next_token(), header.next_token())
            else {
                break;
            };
            if let (Ok(object_number), Ok(at)) = (u32::try_from(object_number), usize::try_from(at))
            {
                pairs.push((object_number, at));
            }
        }

        let mut objects = BTreeMap::new();
        for (object_number, relative) in pairs {
            let start = first.saturating_add(relative);
            if start >= data.len() {
                continue;
            }
            let mut parser = Parser::at(&data, start, self.limits);
            if let Ok(parsed) = parser.parse_object() {
                objects.insert(object_number, parsed);
            }
        }

        let expanded = Arc::new(objects);
        self.expanded_streams
            .borrow_mut()
            .insert(number, Arc::clone(&expanded));
        Some(expanded)
    }

    /// Returns a stream's decoded data.
    ///
    /// # Errors
    ///
    /// Returns `None` when a filter in the chain is not supported, rather than returning
    /// the encoded bytes. Handing back compressed data as if it were decoded would produce
    /// garbage that looks like a rendering bug.
    #[must_use]
    pub fn decoded_stream_data(&self, stream: &Stream) -> Option<Arc<[u8]>> {
        let filters = self.filter_chain(&stream.dict);
        if filters.is_empty() {
            return Some(Arc::clone(&stream.data));
        }

        let mut data: Arc<[u8]> = Arc::clone(&stream.data);
        for (index, filter) in filters.iter().enumerate() {
            let parms = self.decode_parms(&stream.dict, index);
            data = crate::filter::decode_with_parms(filter, &data, parms.as_ref(), self.limits)?;
        }
        Some(data)
    }

    /// Returns a stream's data with every filter applied up to a trailing image codec.
    ///
    /// An image stream's chain may mix ordinary filters with a codec:
    /// `[/ASCIIHexDecode /JBIG2Decode]` is the arrangement ISO 32000-2 §7.4.7's own worked
    /// example uses, and `[/FlateDecode /DCTDecode]` occurs in the corpus. Only the last
    /// entry can be a codec — everything before it is a byte-to-byte transformation that
    /// has to run first, and a codec handed still-compressed bytes fails in a way that
    /// reads as a broken image rather than as a missing step.
    ///
    /// # Errors
    ///
    /// Returns `None` when a filter before the codec is unsupported, for the same reason
    /// [`Self::decoded_stream_data`] does.
    #[must_use]
    pub fn image_stream(&self, stream: &Stream) -> Option<ImageStream> {
        let filters = self.filter_chain(&stream.dict);
        let codec_at = filters.len().checked_sub(1).filter(|last| {
            filters
                .get(*last)
                .is_some_and(|name| crate::filter::is_image_codec(name))
        });

        let mut data: Arc<[u8]> = Arc::clone(&stream.data);
        for (index, filter) in filters.iter().enumerate() {
            if Some(index) == codec_at {
                break;
            }
            let parms = self.decode_parms(&stream.dict, index);
            data = crate::filter::decode_with_parms(filter, &data, parms.as_ref(), self.limits)?;
        }

        Some(ImageStream {
            codec: codec_at.and_then(|index| filters.get(index).cloned()),
            parms: codec_at.and_then(|index| self.decode_parms(&stream.dict, index)),
            data,
        })
    }

    /// Returns the `/DecodeParms` entry for the filter at `index`.
    ///
    /// The key may hold a single dictionary or an array with one entry per filter, and
    /// either may be indirect.
    fn decode_parms(&self, dict: &Dictionary, index: usize) -> Option<Dictionary> {
        match self.get_key(dict, "DecodeParms") {
            Object::Dictionary(parms) => Some(parms),
            Object::Array(items) => items
                .get(index)
                .map(|item| self.resolve(item))
                .and_then(|item| item.as_dict().cloned()),
            _ => None,
        }
    }

    /// Returns the filter names for a stream, in application order.
    fn filter_chain(&self, dict: &Dictionary) -> Vec<Vec<u8>> {
        let filter = self.get_key(dict, "Filter");
        match filter {
            Object::Name(name) => vec![name.as_bytes().to_vec()],
            Object::Array(items) => items
                .iter()
                .map(|item| self.resolve(item))
                .filter_map(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Returns the bytes the document was opened from.
    #[must_use]
    pub fn bytes(&self) -> &Arc<[u8]> {
        &self.bytes
    }

    /// Returns the resource bounds in force.
    #[must_use]
    pub fn limits(&self) -> Limits {
        self.limits
    }
}

/// A stream's data with its image codec, if any, still to be applied.
///
/// Returned by [`Document::image_stream`]. The split exists because a codec's output is a
/// raster rather than bytes, so it cannot be the return value of a filter chain.
#[derive(Debug, Clone)]
pub struct ImageStream {
    /// The image codec left on the data, by name, or `None` if the chain was all ordinary
    /// filters and the data is already samples.
    pub codec: Option<Vec<u8>>,
    /// The codec's own `/DecodeParms`, which is where `/JBIG2Globals` lives (Table 12).
    pub parms: Option<Dictionary>,
    /// The data with every filter before the codec applied.
    pub data: Arc<[u8]>,
}
