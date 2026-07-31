//! The PDF object model: ISO 32000-2 §7.3.1's nine basic types, plus indirect references.
//!
//! > PDF syntax includes nine basic types of objects: boolean values, integers, real
//! > numbers, strings, names, arrays, dictionaries, streams, and the null object.
//!
//! Nine, and this comment said eight for the project's whole life — integer and real are
//! two of them, which is §7.3.3's own division and the reason [`Object::as_number`] exists
//! beside [`Object::as_integer`].
//!
//! This is deliberately a *syntactic* model. A dictionary here is a dictionary, not a
//! page or a font — giving objects meaning is `pdf-model`'s job. Keeping the split sharp
//! means the fuzzing surface is a pure bytes-to-object-graph function with no
//! higher-level state to configure.

use std::collections::BTreeMap;
use std::sync::Arc;

/// An object number and generation, identifying an indirect object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId {
    /// The object number. Zero is reserved for the head of the free list.
    pub number: u32,
    /// The generation number, incremented when an object number is reused.
    pub generation: u16,
}

impl ObjectId {
    /// Creates an identifier.
    #[must_use]
    pub const fn new(number: u32, generation: u16) -> Self {
        Self { number, generation }
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} R", self.number, self.generation)
    }
}

/// A PDF name, such as `/Type`.
///
/// Stored decoded: `#`-escapes are resolved at parse time, so a comparison against
/// `"Type"` is a plain byte comparison. Names are compared as bytes rather than as text
/// because the specification defines them that way and a name is not required to be
/// valid UTF-8.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(pub Arc<[u8]>);

impl Name {
    /// Creates a name from already-decoded bytes.
    #[must_use]
    pub fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self(bytes.into())
    }

    /// Returns the decoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Returns the name as text, if it happens to be valid UTF-8.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.0).ok()
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == other.as_bytes()
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "/{}", self.as_str().unwrap_or("<non-utf8>"))
    }
}

/// A PDF dictionary.
///
/// Ordered by key so that iteration is deterministic — which matters because a
/// non-deterministic order would make golden-image and fuzz-crash reproduction
/// unreliable. A `BTreeMap` also keeps worst-case lookup logarithmic on hostile input,
/// where a hash map could be driven into collisions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dictionary(BTreeMap<Name, Object>);

impl Dictionary {
    /// Creates an empty dictionary.
    #[must_use]
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Inserts a key, returning the previous value if the key was already present.
    ///
    /// A duplicate key is malformed but occurs in real files; the specification does not
    /// say which wins, and the returned value lets a caller notice and decide.
    pub fn insert(&mut self, key: Name, value: Object) -> Option<Object> {
        self.0.insert(key, value)
    }

    /// Looks up a key by name.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Object> {
        self.0.get(&Name::new(key.as_bytes().to_vec()))
    }

    /// Removes a key, returning its value if it was present.
    ///
    /// Nothing in this crate removes an entry — a parsed dictionary is what the file says. The
    /// caller is §12.3.4, where the standard itself takes entries away: a thumbnail image's
    /// dictionary keeps five of Table 87's entries and "all of the other entries … shall be
    /// ignored if present".
    pub fn remove(&mut self, key: &str) -> Option<Object> {
        self.0.remove(&Name::new(key.as_bytes().to_vec()))
    }

    /// Looks up a key by an already-constructed name.
    ///
    /// Avoids allocating a `Name` when the caller already holds one, which the parser does
    /// on every entry.
    #[must_use]
    pub fn get_by_name(&self, key: &Name) -> Option<&Object> {
        self.0.get(key)
    }

    /// Returns the number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the dictionary has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterates over the entries in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&Name, &Object)> {
        self.0.iter()
    }
}

/// A stream: a dictionary plus raw, still-encoded data.
///
/// The data is kept encoded. Decoding needs the filter chain, which needs the document
/// to resolve indirect `/Length` and `/DecodeParms` values, so it happens a layer up.
/// Storing raw bytes also means a stream can be re-encoded byte-for-byte, which matters
/// for incremental update.
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    /// The stream dictionary.
    pub dict: Dictionary,
    /// The raw bytes between `stream` and `endstream`.
    pub data: Arc<[u8]>,
    /// True when the document is encrypted and this stream's data could not be decrypted,
    /// in which case [`Self::data`] is empty rather than ciphertext.
    ///
    /// Encryption is syntax — ISO 32000-2 puts it in clause 7 — but the reason this flag is
    /// on the object rather than handled where it arises is principle 3's: a stream that
    /// silently became empty is a page that draws blank and reports nothing, which is
    /// indistinguishable from a page that *is* blank. Carrying the fact means
    /// [`crate::Document::decoded_stream_data`] can refuse, and every caller already treats
    /// a refusal as something to report.
    pub decryption_failed: bool,
}

/// Any PDF object.
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    /// The null object, and the value of a dangling reference.
    Null,
    /// A boolean.
    Boolean(bool),
    /// An integer.
    ///
    /// `i64` rather than `i32`: the specification's limit is 2^31-1, but real files
    /// exceed it and a viewer that refused them would be less useful than one that
    /// carries the value and lets a validator complain.
    Integer(i64),
    /// A real number.
    Real(f64),
    /// A string, in its decoded byte form.
    String(Arc<[u8]>),
    /// A name.
    Name(Name),
    /// An array.
    Array(Vec<Object>),
    /// A dictionary.
    Dictionary(Dictionary),
    /// A stream.
    Stream(Arc<Stream>),
    /// A reference to an indirect object.
    Reference(ObjectId),
}

impl Object {
    /// Returns the integer value, if this is an integer.
    #[must_use]
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the numeric value, accepting either an integer or a real.
    ///
    /// PDF's `number` type covers both, and a file may write `0` where `0.0` is meant,
    /// so most callers want this rather than [`Self::as_integer`].
    #[must_use]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            #[expect(
                clippy::cast_precision_loss,
                reason = "an integer beyond 2^53 cannot be represented exactly, but such \
                          a value is already far outside any coordinate or count a \
                          document can meaningfully use"
            )]
            Self::Integer(value) => Some(*value as f64),
            Self::Real(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the name, if this is a name.
    #[must_use]
    pub fn as_name(&self) -> Option<&Name> {
        match self {
            Self::Name(name) => Some(name),
            _ => None,
        }
    }

    /// Returns the array, if this is an array.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Object]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// Returns the dictionary, if this is a dictionary *or* a stream.
    ///
    /// Streams are included because a stream is a dictionary with data attached, and
    /// nearly every caller wanting a dictionary will accept either.
    #[must_use]
    pub fn as_dict(&self) -> Option<&Dictionary> {
        match self {
            Self::Dictionary(dict) => Some(dict),
            Self::Stream(stream) => Some(&stream.dict),
            _ => None,
        }
    }

    /// Returns the stream, if this is a stream.
    #[must_use]
    pub fn as_stream(&self) -> Option<&Arc<Stream>> {
        match self {
            Self::Stream(stream) => Some(stream),
            _ => None,
        }
    }

    /// Returns the referenced identifier, if this is a reference.
    #[must_use]
    pub fn as_reference(&self) -> Option<ObjectId> {
        match self {
            Self::Reference(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the string bytes, if this is a string.
    #[must_use]
    pub fn as_string(&self) -> Option<&[u8]> {
        match self {
            Self::String(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// Returns `true` for the null object.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns a short name for this object's type, for diagnostics.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Real(_) => "real",
            Self::String(_) => "string",
            Self::Name(_) => "name",
            Self::Array(_) => "array",
            Self::Dictionary(_) => "dictionary",
            Self::Stream(_) => "stream",
            Self::Reference(_) => "reference",
        }
    }
}
