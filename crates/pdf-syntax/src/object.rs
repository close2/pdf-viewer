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
use std::fmt::Write as _;
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

    /// ISO 32000-2 §7.3.5's written form of this name: what follows the SOLIDUS in a PDF file.
    ///
    /// > When writing a name in a PDF file, a SOLIDUS (2Fh) (/) shall be used to introduce a
    /// > name. The SOLIDUS is not part of the name but is a prefix indicating that what follows
    /// > is a sequence of characters representing the name in the PDF file and shall follow these
    /// > rules:
    ///
    /// The three rules, verbatim:
    ///
    /// > a) A NUMBER SIGN (23h) (#) in a name shall be written by using its 2-digit hexadecimal
    /// > code (23), preceded by the NUMBER SIGN.
    ///
    /// > b) Any character in a name that is a regular character (other than NUMBER SIGN) shall be
    /// > written as itself or by using its 2-digit hexadecimal code, preceded by the NUMBER SIGN.
    ///
    /// > c) Any character that is not a regular character shall be written using its 2-digit
    /// > hexadecimal code, preceded by the NUMBER SIGN only.
    ///
    /// The SOLIDUS is the caller's, because the clause says it is not part of the name — which is
    /// also why a name written into a *dictionary key* and one written into a *content stream*
    /// can share this one function and differ only in what surrounds it.
    ///
    /// Rule b) offers a choice, and the clause narrows it two paragraphs later:
    ///
    /// > Regular characters that are outside the range EXCLAMATION MARK(21h) (!) to TILDE (7Eh)
    /// > (~) should be written using the hexadecimal notation.
    ///
    /// So a byte goes out as itself exactly when it is regular by §7.2.3, lies inside that range,
    /// and is not the number sign; the eight delimiters, the six white-space bytes, `#` and every
    /// byte above 126 go out as `#xx`. Whitespace is not a choice at all — "[w]hitespace used as
    /// part of a name shall always be coded using the 2-digit hexadecimal notation" — and it falls
    /// out of the same test rather than being a second rule.
    ///
    /// **The two braces are escaped as well, and that is a choice rather than the rule.** They are
    /// regular characters outside a type 4 program (§7.2.3, and `lexer::is_delimiter` for the
    /// erratum that says so in Table 2 itself), so rule b) lets this writer put either out as
    /// itself — and rule b) equally lets it write the hexadecimal code, which is what it does. The
    /// cost is two bytes in a name almost no document has; what it buys is that a name this program
    /// writes is read the same way by a reader that takes Table 2's ten delimiters unconditionally,
    /// which is how this tree itself read them until the eight-hundred-and-nineteenth session.
    ///
    /// **What comes back is therefore always ASCII**, which is what lets a content stream this
    /// program builds be a `String` while the name inside it is bytes.
    ///
    /// The inverse is [`crate::Lexer`], which expands `#xx` as it reads a name, so this function
    /// and that one are §7.3.5 written down once in each direction. Nothing else in this tree
    /// spells either of them.
    #[must_use]
    pub fn escaped(&self) -> String {
        let mut out = String::with_capacity(self.0.len());
        for byte in self.0.iter().copied() {
            if byte != b'#'
                && !matches!(byte, b'{' | b'}')
                && (0x21..=0x7e).contains(&byte)
                && crate::lexer::is_regular(byte)
            {
                out.push(char::from(byte));
            } else {
                // Infallible: a `String` sink never fails, and every byte written here is ASCII.
                let _ = write!(out, "#{byte:02X}");
            }
        }
        out
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == other.as_bytes()
    }
}

/// Lets a [`Dictionary`] be probed with a borrowed `&[u8]` instead of an allocated key.
///
/// `Borrow` demands that the borrowed and the owned form agree on `Eq`, `Ord` and `Hash`. The
/// equality half is what ISO 32000-2 §7.3.5 says a name *is*:
///
/// > any two name objects that, after all escaping is expanded (see below), and the resulting
/// > sequences of bytes are not an exact binary match denote different objects
///
/// and the ordering and hashing halves hold by construction rather than by inspection: every one
/// of those five traits on [`Name`] is derived, a derive on a one-field tuple struct delegates to
/// the field, and `Arc<[u8]>` delegates to `[u8]`. So a name's ordering *is* its bytes' ordering,
/// the `BTreeMap`'s invariant is untouched, and `borrows_exactly_as_it_compares` is the standing
/// check that a later hand-written impl of any of the five does not break it silently.
///
/// What it buys, which is why a four-line impl earns a comment this long: [`Dictionary::get`] used
/// to build a heap-allocated `Name` for the key on **every dictionary lookup in the program** —
/// a `Vec`, then the `Arc<[u8]>` it is copied into, then both freed. One cold search of
/// ISO 32000-2's 1023 pages makes **3 278 302** calls to it, and callgrind over two binaries from
/// one tree puts the whole sweep at **37 642 044 068 → 36 920 639 974 instructions, −1.92%** —
/// about 220 instructions a call, with `malloc` down 19.1% and `free` down 15.7%. The readback of
/// every page is byte-identical, which is what says this is a speed-up and not a change. ADR 0335;
/// `doc/performance.md` has the rest of the table.
impl std::borrow::Borrow<[u8]> for Name {
    fn borrow(&self) -> &[u8] {
        &self.0
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
    ///
    /// Probes the map with the caller's own bytes through [`Name`]'s `Borrow<[u8]>`, which is
    /// what keeps the hottest path in the program allocation-free; the impl carries the number.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Object> {
        self.0.get(key.as_bytes())
    }

    /// Removes a key, returning its value if it was present.
    ///
    /// Nothing in this crate removes an entry — a parsed dictionary is what the file says. The
    /// caller is §12.3.4, where the standard itself takes entries away: a thumbnail image's
    /// dictionary keeps five of Table 87's entries and "all of the other entries … shall be
    /// ignored if present".
    pub fn remove(&mut self, key: &str) -> Option<Object> {
        self.0.remove(key.as_bytes())
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

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use super::{Dictionary, Name, Object};

    /// Names chosen for the three ways a byte-wise order can be got wrong: a prefix against what
    /// extends it (`Type` / `TypeX`), a byte above 127 against one below it — names are bytes and
    /// `#`-escapes are decoded, so a `/#80` is a real key — and the empty name, which ISO 32000-2
    /// §7.3.5 admits outright:
    ///
    /// > The token SOLIDUS (a slash followed by no regular characters) introduces a unique valid
    /// > name defined by the empty sequence of characters.
    const SAMPLES: [&[u8]; 8] = [
        b"",
        b"A",
        b"AA",
        b"Type",
        b"TypeX",
        b"\x7f",
        b"\x80",
        b"\xff\x00",
    ];

    fn hash_of(value: &impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    /// `Borrow<[u8]>` is only sound while `Eq`, `Ord` and `Hash` agree between the two forms, and
    /// [`Dictionary`]'s `BTreeMap` is probed through it on every lookup in the program. The
    /// agreement is by construction — all five traits are derived — so what this guards against is
    /// a later hand-written impl of any of them, which would break the map's invariant silently.
    #[test]
    fn borrows_exactly_as_it_compares() {
        for left in SAMPLES {
            let name = Name::new(left);
            assert_eq!(Borrow::<[u8]>::borrow(&name), left);
            assert_eq!(hash_of(&name), hash_of(&left));
            for right in SAMPLES {
                assert_eq!(
                    name.cmp(&Name::new(right)),
                    left.cmp(right),
                    "{left:?} against {right:?}"
                );
                assert_eq!(name == Name::new(right), left == right);
            }
        }
    }

    /// The lookup itself, over the same samples: what was inserted under a `Name` is what a
    /// `&str` key finds, and no neighbouring key answers for it.
    #[test]
    fn a_borrowed_key_finds_what_an_owned_one_stored() {
        let mut dict = Dictionary::new();
        for (index, key) in SAMPLES.iter().enumerate() {
            dict.insert(
                Name::new(*key),
                Object::Integer(i64::try_from(index).expect("eight fits")),
            );
        }
        for (index, key) in SAMPLES.iter().enumerate() {
            let text = std::str::from_utf8(key);
            let Ok(text) = text else {
                // Two samples are not UTF-8, which is exactly why `Name` is bytes; they are
                // reachable through `get_by_name` and are covered by the ordering test above.
                assert!(dict.get_by_name(&Name::new(*key)).is_some());
                continue;
            };
            assert_eq!(
                dict.get(text),
                Some(&Object::Integer(i64::try_from(index).expect("eight fits"))),
                "{text:?}"
            );
        }
        assert_eq!(dict.get("Typ"), None);
        assert_eq!(dict.get("TypeXY"), None);
        assert_eq!(dict.remove("Type"), Some(Object::Integer(3)));
        assert_eq!(dict.get("Type"), None);
        assert_eq!(dict.get("TypeX"), Some(&Object::Integer(4)));
    }
}
