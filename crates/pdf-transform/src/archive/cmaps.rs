//! A `CMap` a composite font names and the file does not carry, embedded from the published set.
//!
//! ISO 19005-2 section 6.2.11.3.3 and ISO 19005-4 section 6.2.10.3.3 require every `CMap` a file
//! uses to be embedded unless it is on the base standard's list of predefined ones, and limit what
//! any `CMap` may build on to that same list. A font naming one of Adobe's other published `CMaps`
//! fails the first half, and the refusal's old reasoning was that supplying one means writing the
//! encoding its producer did not. That is true of an invented `CMap` and false of a published one:
//! the name *is* the mapping, and `data/cmaps` carries every program Adobe publishes under the
//! licence `doc/third-party-data.md` records. So embedding it writes the same mapping a reader
//! would have used, and the file becomes self-contained, which is what the clause is for
//! (`doc/pdf-a-mitigations.md` section 5, `doc/adr/1286`).
//!
//! ISO 32000-2 §9.7.5.3 is the construction. The stream's data is the program itself, byte for
//! byte — Adobe's copyright notice travels in its header comments, which is what the licence asks —
//! and Table 118's dictionary states what the program states:
//!
//! > The name of the CMap. It shall be the same as the value of CMapName in the CMap file.
//!
//! > The value of this entry shall be the same as the value of CIDSystemInfo in the CMap file.
//!
//! **What refuses, each by name.** A program that builds on a `CMap` the list does not hold, since
//! the embedded one would then reference it and the clause's second sentence forbids that; a
//! program using an operator §9.7.5.4 forbids in an embedded `CMap` (`beginbfchar`, `usefont`,
//! `beginrearrangedfont`, `beginusematrix`); a program that is not a `CMapType` 1 code-to-CID map;
//! and a descendant `CIDFont` whose character collection the program's does not agree with, because
//! ISO 19005-2 section 6.2.11.3.1 holds an embedded `CMap` and its font to one collection and the
//! embedding would make the file fail there instead.

use std::collections::BTreeMap;

use pdf_archive::Target;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;

use crate::json::Value;

use super::COMPRESSION_LEVEL;
use super::decision::Because;
use super::prepare::Spare;

/// The requirement a `preserve` with `source = "shipped-cmaps"` answers.
pub(super) const SITE: &str = "fonts/cmap-embedded-or-predefined";

/// What an operator answering the site with `preserve` is agreeing to.
pub(super) const EMBEDDED: &str = "each CMap a composite font names that the base standard does \
     not predefine is embedded from Adobe's published programs, byte for byte, so the archive \
     carries the mapping the name already meant; the report names every CMap embedded";

/// Why a font naming a `CMap` no published program is called is not answered.
const NOT_SHIPPED: &str = "a composite font names a CMap that neither the base standard \
     predefines nor Adobe publishes among the programs this converter carries, so there is no \
     published mapping to embed and writing one would be deciding what the font's codes select";

/// Why a published program that builds on an unlisted `CMap` is not embedded.
const BUILDS_ON_AN_UNLISTED_CMAP: &str = "the published program this composite font's CMap name \
     denotes builds on another CMap through usecmap, and that one is not on the base standard's \
     list — ISO 19005-2 section 6.2.11.3.3 lets a CMap reference only CMaps on that list, so the \
     embedded program would fail the clause it was embedded to answer. Writing the two programs as \
     one is not built";

/// Why a published program using an operator §9.7.5.4 forbids is not embedded.
const FORBIDDEN_OPERATOR: &str = "the published program this composite font's CMap name denotes \
     uses an operator ISO 32000-2 \u{a7}9.7.5.4 forbids in an embedded CMap used as a Type 0 \
     font's encoding (bfchar or bfrange mappings, usefont, a rearranged font or a use matrix)";

/// Why a published program that does not map codes to CIDs is not embedded.
const NOT_A_CID_MAP: &str = "the published program this composite font's CMap name denotes does \
     not state CMapType 1, so it is not a map from character codes to CIDs and cannot be the \
     encoding of a Type 0 font";

/// Why a published program whose header this converter cannot read is not embedded.
const HEADER_UNREAD: &str = "the published program this composite font's CMap name denotes does \
     not state, in the form Adobe's files use, the CMapName, Registry, Ordering and Supplement \
     ISO 32000-2 Table 118 requires the stream dictionary to repeat";

/// Why a font whose descendant disagrees with the program's character collection is refused.
const COLLECTIONS_DISAGREE: &str = "the published program this composite font's CMap name denotes \
     describes a character collection its descendant CIDFont does not — another Registry or \
     Ordering, or a lower Supplement — and ISO 19005-2 section 6.2.11.3.1 holds an embedded CMap \
     and its CIDFont to one collection, so the embedding would make the file fail there instead";

/// Why a font this converter cannot reach as an object is not answered.
const FONT_NOT_AN_OBJECT: &str = "a composite font naming an undefined CMap is not a dictionary \
     of its own that this rewrite can give an Encoding entry";

/// Why the program could not be compressed for the stream.
const NOT_ENCODED: &str = "the published CMap program could not be compressed into a stream";

/// Why no object number was free for the stream.
const NO_NUMBER: &str = "no object number was free for an embedded CMap stream";

/// One `CMap` embedded from the published set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedCMap {
    /// The composite font whose `/Encoding` now names the stream.
    pub font: ObjectId,
    /// The `CMap`'s name, as the font stated it and the program states it.
    pub name: String,
}

impl EmbeddedCMap {
    /// One embedded `CMap` as JSON.
    pub(super) fn to_json(&self) -> Value {
        Value::Object(vec![
            (
                "font".to_owned(),
                Value::text(format!("{} {}", self.font.number, self.font.generation)),
            ),
            ("cmap".to_owned(), Value::text(self.name.clone())),
        ])
    }
}

/// The streams to add and the fonts that name them.
#[derive(Debug)]
pub(super) struct ShippedCMaps {
    /// For each composite font, the stream its `/Encoding` names instead of the name.
    pub(super) encodings: BTreeMap<ObjectId, ObjectId>,
    /// The streams, one per distinct name.
    pub(super) written: BTreeMap<ObjectId, Object>,
    /// One row per font, for the report.
    pub(super) embedded: Vec<EmbeddedCMap>,
}

/// What a published program's header states, read the way Adobe's files write it.
struct Header {
    /// `/CMapName`.
    name: String,
    /// The `/Registry` of its `/CIDSystemInfo`.
    registry: Vec<u8>,
    /// The `/Ordering` of its `/CIDSystemInfo`.
    ordering: Vec<u8>,
    /// The `/Supplement` of its `/CIDSystemInfo`.
    supplement: i64,
    /// `/WMode`, 0 where the program states none (Table 118's default).
    wmode: i64,
    /// `/CMapType`.
    cmap_type: Option<i64>,
    /// Whether an operator §9.7.5.4 forbids appears.
    forbidden: bool,
}

/// Reads a program's header as its tokens: `/Key value def` pairs, comments skipped.
///
/// Adobe's files are regular PostScript resource files — every one this tree carries writes these
/// entries on lines of their own in this form — so the reading is of that form, and a program that
/// states them some other way refuses by name rather than being guessed at.
fn header(program: &[u8]) -> Option<Header> {
    let text = String::from_utf8_lossy(program);
    let tokens: Vec<&str> = text
        .lines()
        .map(|line| line.split('%').next().unwrap_or_default())
        .flat_map(str::split_whitespace)
        .collect();
    let after = |key: &str| {
        tokens
            .windows(2)
            .find(|pair| pair[0] == key)
            .map(|pair| pair[1])
    };
    let string = |token: &str| {
        token
            .strip_prefix('(')
            .and_then(|rest| rest.strip_suffix(')'))
            .map(|inner| inner.as_bytes().to_vec())
    };
    let forbidden = tokens.iter().any(|token| {
        matches!(
            *token,
            "beginbfchar" | "beginbfrange" | "usefont" | "beginrearrangedfont" | "beginusematrix"
        )
    });
    Some(Header {
        name: after("/CMapName")?.strip_prefix('/')?.to_owned(),
        registry: string(after("/Registry")?)?,
        ordering: string(after("/Ordering")?)?,
        supplement: after("/Supplement")?.parse().ok()?,
        wmode: after("/WMode")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        cmap_type: after("/CMapType").and_then(|value| value.parse().ok()),
        forbidden,
    })
}

/// Whether a composite font's descendant describes the collection the program does.
fn collections_agree(document: &Document, font: &Dictionary, header: &Header) -> bool {
    let descendant = document
        .get_key(font, "DescendantFonts")
        .as_array()
        .and_then(|fonts| fonts.first().cloned())
        .map(|first| document.resolve(&first));
    let Some(Object::Dictionary(descendant)) = descendant else {
        return false;
    };
    let Object::Dictionary(info) = document.get_key(&descendant, "CIDSystemInfo") else {
        return false;
    };
    let text = |key: &str| document.get_key(&info, key).as_string().map(<[u8]>::to_vec);
    text("Registry").as_deref() == Some(header.registry.as_slice())
        && text("Ordering").as_deref() == Some(header.ordering.as_slice())
        && document
            .get_key(&info, "Supplement")
            .as_integer()
            .is_some_and(|supplement| supplement >= header.supplement)
}

/// The stream §9.7.5.3 asks for, holding the published program unchanged.
fn stream_for(program: &[u8], header: &Header, parent: Option<&str>) -> Option<Object> {
    let name = |text: &str| Object::Name(Name::new(text.as_bytes()));
    let mut info = Dictionary::new();
    info.insert(
        Name::new(&b"Registry"[..]),
        Object::String(header.registry.clone().into()),
    );
    info.insert(
        Name::new(&b"Ordering"[..]),
        Object::String(header.ordering.clone().into()),
    );
    info.insert(
        Name::new(&b"Supplement"[..]),
        Object::Integer(header.supplement),
    );
    let mut dict = Dictionary::new();
    dict.insert(Name::new(&b"Type"[..]), name("CMap"));
    dict.insert(Name::new(&b"CMapName"[..]), name(&header.name));
    dict.insert(Name::new(&b"CIDSystemInfo"[..]), Object::Dictionary(info));
    dict.insert(Name::new(&b"WMode"[..]), Object::Integer(header.wmode));
    // §9.7.5.4 a): a program's `usecmap` reference is named by the dictionary as well.
    if let Some(parent) = parent {
        dict.insert(Name::new(&b"UseCMap"[..]), name(parent));
    }
    let encoded = flate_encode(program, COMPRESSION_LEVEL)?;
    dict.insert(Name::new(&b"Filter"[..]), name("FlateDecode"));
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: encoded.into(),
        decryption_failed: false,
    })))
}

/// Works out the stream for every font the requirement reported, or the reason one cannot be.
///
/// The whole preparation refuses rather than half of it: a file that embedded one `CMap` and left
/// another still fails the requirement, so a partial answer would change bytes for nothing.
pub(super) fn prepare(
    document: &Document,
    target: Target,
    spare: &mut Spare,
) -> Result<ShippedCMaps, Because> {
    let mut by_name: BTreeMap<String, ObjectId> = BTreeMap::new();
    let mut out = ShippedCMaps {
        encodings: BTreeMap::new(),
        written: BTreeMap::new(),
        embedded: Vec::new(),
    };
    for (font, cmap) in pdf_archive::fonts_naming_an_undefined_cmap(document, target) {
        let Object::Dictionary(dict) = document.get(font) else {
            return Err(Because::NotBuiltYet(FONT_NOT_AN_OBJECT));
        };
        let program =
            pdf_font::predefined::program(&cmap).ok_or(Because::NotBuiltYet(NOT_SHIPPED))?;
        let header = header(&program)
            .filter(|header| header.name == cmap)
            .ok_or(Because::NotBuiltYet(HEADER_UNREAD))?;
        if header.cmap_type != Some(1) {
            return Err(Because::NotBuiltYet(NOT_A_CID_MAP));
        }
        if header.forbidden {
            return Err(Because::NotBuiltYet(FORBIDDEN_OPERATOR));
        }
        let parent = pdf_font::predefined::builds_on(&program);
        if parent
            .as_deref()
            .is_some_and(|parent| !pdf_archive::cmap_is_predefined(parent))
        {
            return Err(Because::NotBuiltYet(BUILDS_ON_AN_UNLISTED_CMAP));
        }
        if !collections_agree(document, &dict, &header) {
            return Err(Because::NotBuiltYet(COLLECTIONS_DISAGREE));
        }
        let at = if let Some(at) = by_name.get(&cmap) {
            *at
        } else {
            let at = spare
                .take(document)
                .ok_or(Because::NotBuiltYet(NO_NUMBER))?;
            let stream = stream_for(&program, &header, parent.as_deref())
                .ok_or(Because::NotBuiltYet(NOT_ENCODED))?;
            out.written.insert(at, stream);
            by_name.insert(cmap.clone(), at);
            at
        };
        out.encodings.insert(font, at);
        out.embedded.push(EmbeddedCMap { font, name: cmap });
    }
    Ok(out)
}
