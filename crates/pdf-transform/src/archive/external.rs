//! The stream data a source keeps outside its own file, brought inside it.
//!
//! ISO 19005-2 section 6.1.7.1 and ISO 19005-4 section 6.1.6.1 forbid a stream dictionary the
//! keys that put its data somewhere else, and the requirement's own NOTE says why: the listed
//! keys are the ones that point at data external to the file. A format whose whole promise is
//! that the bytes are here cannot admit a stream whose bytes are on a server.
//!
//! # The two shapes a failure here has, and only one of them needs anything from outside
//!
//! §7.3.8.2's Table 5 makes `/F` "[t]he file containing the stream data" and says that where it
//! is present "the bytes between stream and endstream shall be ignored"; `/FFilter` and
//! `/FDecodeParms` are the filter and the parameters applied to *that* file's data. So:
//!
//! - **A stream stating `/F`** has its data outside the file, and nothing in the document can
//!   supply it. Embedding means somebody fetching those bytes and handing them over, which is
//!   [`super::ArchivePlan::external_data`] and `doc/adr/1199`'s decision about where the fetch
//!   lives.
//! - **A stream stating one of the filter keys and no `/F`** states filters for an external file
//!   that is not there. Table 5 gives those keys meaning only through `/F`, so a conforming
//!   reader decodes such a stream by `/Filter` and never consults them: removing them changes
//!   nothing any reader computes, and the requirement is answered out of the file's own bytes.
//!
//! # Why the resolution is the caller's and never this function's
//!
//! RFC 0002 section 9's determinism claim rests on [`crate::apply`] being a pure function of its
//! inputs, and `doc/questions/A54` is the shape this project chose for everything needing the
//! outside world: `apply` returns a request or is handed the result, and the caller performs the
//! act. So nothing here opens a path, and what a caller may resolve is bounded by the rule
//! `doc/adr/1155` states for a name a document wrote — a single path component, against the
//! directory the document itself is in. [`ExternalData`] is what a caller reads to decide, in
//! [`pdf_model::file_spec::FileSpec`]'s reading of §7.11 rather than a second one written here; §7.11.5's
//! URL is named as itself rather than passed off as a file name, because it is not one.

use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_archive::Outcome;
use pdf_model::file_spec::FileSpec;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};

use super::decision::Because;

/// The requirement this module answers, by the identifier the validator reports it under.
const SITE: &str = "file-structure/no-external-stream-data";

/// One stream whose dictionary states a key ISO 19005 forbids, and what that key says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalStream {
    /// The stream object, in the source's own numbering.
    pub at: ObjectId,
    /// Where the data is, as the file's own file specification says.
    pub data: ExternalData,
}

/// What a stream's `/F` names, read as §7.11 defines a file specification.
///
/// The reading is [`pdf_model::file_spec::FileSpec`]'s rather than this module's: §7.11.1's two forms,
/// §7.11.2.1's components and their escapes, and §7.11.5's URL are one clause family with one
/// reader in this tree, and a converter that split a path itself would be a second one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalData {
    /// §7.11.2's simple file specification, in the standard's own platform-independent form.
    ///
    /// Whether it resolves to anything, and whether a caller is willing to read it, is the
    /// caller's to decide — and what it decides over is `components`, because §7.11.2.1's
    /// SOLIDUS is "a generic component separator" whatever the platform underneath spells its
    /// own with.
    Named {
        /// The specification as a person reads it, for the report.
        shown: String,
        /// §7.11.2.1's components, with the escapes removed.
        components: Vec<Vec<u8>>,
        /// Whether §7.11.2.2 makes it absolute, which is a leading SOLIDUS.
        absolute: bool,
    },
    /// §7.11.5's uniform resource locator, which the dictionary declares by `/FS` `/URL`.
    ///
    /// Named as itself because it is not a file name on any filesystem: a caller applying a rule
    /// about path components to an RFC 3986 URI would be applying the wrong rule, and a caller
    /// that fetched it would be performing a network operation this program does not have.
    AtUrl(String),
    /// The stream states no `/F`, so the forbidden keys it does state describe filters for an
    /// external file that is not there.
    NoneNamed,
    /// A `/F` that is neither of §7.11.1's two forms, so it names no file this reader can find.
    Unreadable,
}

/// Every stream the external-data requirement was failed at, with what its `/F` names.
///
/// **The population is the validator's findings rather than a walk of this crate's**, which is
/// the rule every remedy here follows: which stream fails which rule is `pdf_archive`'s reading,
/// and a second walk would be a second reading of ISO 19005. A report of another target, or one
/// whose verdict never reached this requirement, yields nothing.
#[must_use]
pub fn external_stream_data(
    document: &Document,
    report: &pdf_archive::Report,
) -> Vec<ExternalStream> {
    let Some(judgement) = report.failures().find(|judgement| judgement.id == SITE) else {
        return Vec::new();
    };
    let Outcome::Failed { places, .. } = &judgement.outcome else {
        return Vec::new();
    };
    let mut out: Vec<ExternalStream> = Vec::new();
    for finding in places {
        let Some(at) = finding.place.object else {
            continue;
        };
        if out.iter().any(|held| held.at == at) {
            continue;
        }
        let object = document.get(at);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        out.push(ExternalStream {
            at,
            data: names(document, &stream.dict),
        });
    }
    out
}

/// What one stream dictionary's `/F` says, read as §7.11's two forms.
fn names(document: &Document, dict: &Dictionary) -> ExternalData {
    let specification = document.get_key(dict, "F");
    if specification.is_null() {
        return ExternalData::NoneNamed;
    }
    let Some(specification) = FileSpec::parse(document, &specification) else {
        return ExternalData::Unreadable;
    };
    if let Some(url) = specification.url() {
        return ExternalData::AtUrl(url);
    }
    let Some(shown) = specification.display_name() else {
        return ExternalData::Unreadable;
    };
    ExternalData::Named {
        shown,
        components: specification.components(),
        absolute: specification.is_absolute(),
    }
}

/// The streams this conversion rewrites so that their data is inside the file.
pub(super) struct Embedded {
    /// The replacement stream for each object, keyed in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, Object>,
}

/// Works out the replacement for every stream the requirement was failed at.
///
/// `supplied` is what the caller resolved — [`super::ArchivePlan::external_data`]. A stream whose
/// data is outside the file and whose bytes are not there cannot be rewritten, and the whole
/// preparation fails rather than half of it: a file that embedded one external stream and left
/// another still fails the requirement, so a partial answer would change bytes for nothing.
pub(super) fn embed(
    document: &Document,
    report: &pdf_archive::Report,
    supplied: &BTreeMap<ObjectId, Arc<[u8]>>,
) -> Result<Embedded, Because> {
    let wanted = external_stream_data(document, report);
    if wanted.is_empty() {
        return Err(Because::NotThisTarget(NOTHING_NAMED));
    }
    let mut at = BTreeMap::new();
    for stream in &wanted {
        let object = document.get(stream.at);
        let Some(source) = object.as_stream() else {
            return Err(Because::NotThisTarget(NOT_A_STREAM));
        };
        let bytes = match stream.data {
            // The orphan filter keys: the data is already in the file and the keys describe
            // nothing, so the stream keeps every byte it has.
            ExternalData::NoneNamed => None,
            _ => match supplied.get(&stream.at) {
                Some(bytes) => Some(Arc::clone(bytes)),
                None => return Err(Because::NotThisTarget(NOT_RESOLVED)),
            },
        };
        at.insert(
            stream.at,
            embedded_stream(&source.dict, &source.data, bytes),
        );
    }
    Ok(Embedded { at })
}

/// One stream with its data inside the file and the keys that put it outside removed.
///
/// `bytes` is what the caller resolved, or `None` for a stream whose data was never outside the
/// file at all. Table 5 is the whole of the construction:
///
/// - the **data** is the external file's, written where the stream's own bytes were — which the
///   table says a reader was ignoring while `/F` stood;
/// - **`/Filter`** and **`/DecodeParms`** become the `F`-prefixed pair, because those are the
///   filter and the parameters the table applies to the data now being written; where the pair
///   is absent the entries go, since the external file's data was unfiltered;
/// - **`/Length`** is restated, the table making it the number of bytes between the keywords;
/// - the keys ISO 19005 forbids are removed, all four names of them
///   ([`pdf_archive::table::EXTERNAL_DATA_KEYS`]).
fn embedded_stream(from: &Dictionary, held: &Arc<[u8]>, bytes: Option<Arc<[u8]>>) -> Object {
    let mut dict = from.clone();
    let carried = bytes.is_some();
    if carried {
        let filter = dict.get("FFilter").cloned();
        let parameters = dict.get("FDecodeParms").cloned();
        set_or_remove(&mut dict, "Filter", filter);
        set_or_remove(&mut dict, "DecodeParms", parameters);
    }
    for key in pdf_archive::table::EXTERNAL_DATA_KEYS {
        dict.remove(key);
    }
    let data = bytes.unwrap_or_else(|| Arc::clone(held));
    if carried {
        dict.insert(
            Name::new(&b"Length"[..]),
            Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
        );
    }
    Object::Stream(Arc::new(Stream {
        dict,
        data,
        decryption_failed: false,
    }))
}

/// Writes an entry, or removes it where the value it would take is absent.
fn set_or_remove(dict: &mut Dictionary, key: &str, value: Option<Object>) {
    match value {
        Some(value) => {
            dict.insert(Name::new(key.as_bytes()), value);
        }
        None => {
            dict.remove(key);
        }
    }
}

/// Why the preparation found nothing, though a requirement asked for it.
const NOTHING_NAMED: &str = "this requirement was failed at no stream this reader can find, so \
     there is nothing to bring into the file";

/// Why a finding naming something that is not a stream stops the rewrite.
const NOT_A_STREAM: &str = "this requirement was failed at an object that is not a stream in \
     this reader's object graph, so what the keys describe cannot be worked out";

/// Why a stream whose data is outside the file is not rewritten.
///
/// `doc/adr/1199`: the fetch is the caller's, so the sentence names the switch that performs it
/// and the one shape it will not perform.
pub(super) const NOT_RESOLVED: &str = "a stream in this file states F, which puts its data on \
     somebody else's disk or server, and nobody has handed those bytes to this conversion. Both \
     parts forbid the key, so no other target helps. `--resolve-external-data` reads the file a \
     stream names when that name is a plain file name beside the document itself, which is the \
     only resolution this program performs; a name that is a URL, or that reaches out of the \
     document's own directory, is refused by that rule rather than fetched";
