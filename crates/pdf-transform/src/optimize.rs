//! `optimize` — one document rewritten smaller, RFC 0002 section 6.5.
//!
//! # Two schools, and why only one of them is here
//!
//! RFC 0002 section 6.5 surveys both. **Ghostscript re-distils**: `-sDEVICE=pdfwrite` interprets
//! the document down to marks and writes a new one, so what comes out is appearance-preserving
//! and structurally somebody else's — tagging, form structure and every object the distiller did
//! not understand are gone. **qpdf preserves structure**: object streams, cross-reference
//! streams, recompression, dead-object removal, each a decision about the *file* and none about
//! the page.
//!
//! This verb is the second school, and the choice is not a preference: `CLAUDE.md`'s amended
//! exclusion draws the line at "does the operation invent marks?", and its sentence about this
//! suite is that "every content stream in their output is a producer's, carried byte for byte or
//! **recompressed without reinterpretation**". A re-distiller's content streams are its own. So
//! nothing here decodes a page, and the one thing that decodes a *stream* — the recompressor in
//! [`pdf_syntax::serialize`] — puts the identical bytes back under a different §7.4 filter.
//!
//! # The four lossless passes, and what each is derived from
//!
//! 1. **Reachability.** §7.5.5's Table 15 makes `/Root` "[t]he catalog dictionary for the PDF
//!    file" and §7.7.2's Table 29 makes the catalog "the root of a document's object hierarchy".
//!    An object no path from that root reaches is an object no reader of the output can ask for,
//!    so it is not written. This is the pass ADR 0818 named and declined to invent inside
//!    `split`: a piece over-copies because a widget's field `/Parent` reaches the whole `AcroForm`
//!    tree, and the answer to that was always "run `optimize` on the piece" rather than a pruning
//!    policy in one verb.
//! 2. **Object streams**, §7.5.7, which is the producer half this tree owed from the day
//!    `pdf_syntax::serialize` landed (ADR 0817). Every object the clause permits goes into a
//!    `FlateDecode`-encoded carrier; the clause's own list decides which, and the serializer's
//!    `packable` states it sentence by sentence.
//! 3. **Cross-reference streams**, §7.5.8, which object streams force: Table 18's type 2 entry
//!    is the only way to say where a compressed object is, and only a cross-reference stream has
//!    entry types.
//! 4. **Recompression**, §7.4.4.1's `FlateDecode` on the way out, over the decoded bytes of
//!    whatever chain the producer used. A stream that fails to shrink keeps what its producer
//!    wrote — qpdf's rule for `--optimize-images`, and the right rule for every stream, because
//!    a verb called `optimize` that made a file larger would be a defect.
//!
//! Two smaller ones fall out of the first and are worth naming because they are §7.3.10's
//! answer rather than a policy:
//!
//! - **A reference to an object the document does not hold is not followed**, and the object is
//!   not written. "[A]n indirect reference to an undefined object shall not be considered an
//!   error by a PDF processor; it shall be treated as a reference to the null object", so the
//!   output states `null` where the source stated a reference, and the two say the same thing to
//!   every reader. A source object whose *value* is `null` is the same case and gets the same
//!   answer, because §7.3.10 gives a reader no way to tell the two apart.
//! - **A stream's `/Length` is not followed either.** §7.3.8.2 makes it "[t]he number of bytes
//!   from the beginning of the line following the keyword stream", which is a statement about
//!   the file being written; the serializer re-derives it as a direct integer, so a source that
//!   stated it indirectly leaves an object the output refers to from nowhere. Not following it
//!   is what keeps that object out.
//!
//! # What is not here, and it is named rather than omitted
//!
//! - **Lossy image optimisation.** RFC 0002 section 6.5 proposes `--images downsample=…,quality=…`
//!   and section 13's second question makes it conditional on a **DCT encoder, a dependency this
//!   tree does not have** — `zune-jpeg` decodes only, and `doc/stack.md` is where such a decision
//!   is argued. Without one, "recompress as DCT where smaller" cannot be done at all, and
//!   downsampling to `FlateDecode`-compressed raw samples makes a photograph *larger*, so qpdf's
//!   keep-the-original rule would keep every image and the flag would be a switch that does
//!   nothing while claiming to. It is not implemented, no flag states it, and `doc/todo/57`
//!   carries it with the dependency it waits on.
//! - **Linearisation.** `--linearize` is refused by name, pointing at `CLAUDE.md`'s sentence:
//!   "Annex F stays excluded until linearisation is separately ratified."
//! - **Encryption on the way out.** The serializer emits no `/Encrypt`, so optimising an
//!   encrypted document produces an unencrypted one, and that is a warning rather than a silence.
//!
//! # Idempotence
//!
//! RFC 0002 section 9's second property gate: "`optimize` is idempotent — its own output,
//! optimized again, is byte-identical". It holds because every decision above is a function of
//! the input's object graph and nothing else — the walk's order is `Dictionary`'s own key order,
//! which is sorted; the numbering is the walk's; the deflate level is fixed; §14.4's identifier
//! is a digest of the bytes; and the two entries the serializer rewrites (`/Length`) or may
//! rewrite (`/Filter` under recompression) are exactly the two the walk refuses to follow or the
//! recompressor refuses to touch. `tests/optimize.rs` and `tests/optimize_corpus.rs` are what
//! check it rather than this paragraph.

use std::collections::VecDeque;

use pdf_model::Pages;
use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};
use pdf_syntax::serialize::{Assembly, Form, ObjectStreams, Options, Streams, serialize};
use std::io::Write as _;

use crate::pattern::{Fill, Pattern};
use crate::{Origin, Output, Refusal, Report, Sinks, Warning};

/// The deepest an object's value tree is walked for references.
///
/// One more than `pdf_syntax::Limits::DEFAULT`'s `max_depth`, so that every object the parser
/// admitted has its references reached; the serializer's own rewrite depth is the same number
/// for the same reason.
const MAX_WALK_DEPTH: usize = 257;

/// One document rewritten smaller.
#[derive(Debug, Clone, PartialEq)]
pub struct OptimizePlan {
    /// Which source.
    pub source: usize,
    /// How the one output is named.
    pub names: Pattern,
    /// Whether objects the catalog cannot reach are dropped.
    ///
    /// On by default and switchable off, because pruning is the one pass here that can change
    /// what a document *holds* rather than how it states it: it rests on this program's reading
    /// of what makes an object reachable, and a caller who does not want to rest on that has
    /// somewhere to stand.
    pub prune: bool,
    /// §7.5.7's object streams.
    pub object_streams: ObjectStreams,
    /// What happens to a stream's bytes.
    pub streams: Streams,
}

/// What one run of the verb changed, for the report and for the walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Savings {
    /// How many bytes the source was.
    pub before: u64,
    /// How many the output is.
    pub after: u64,
    /// How many objects the source's cross-reference structure declares, less the free head.
    pub objects_before: u32,
    /// How many the output holds, object streams and the cross-reference stream included.
    pub objects_after: u32,
    /// How many of the output's objects are inside a §7.5.7 object stream.
    pub compressed: u32,
    /// How many object streams carry them.
    pub object_streams: u32,
    /// How many streams were decoded and re-encoded because the result was smaller.
    pub recompressed: u64,
    /// How many bytes those re-encodings saved.
    pub recompression_saved: u64,
}

impl Savings {
    /// The per-category account RFC 0002 section 4.5 asks `optimize` for.
    pub(crate) fn to_json(self) -> Vec<(String, crate::json::Value)> {
        use crate::json::Value;
        vec![
            ("bytes_before".to_owned(), Value::bytes(self.before)),
            ("bytes_after".to_owned(), Value::bytes(self.after)),
            (
                "objects_before".to_owned(),
                Value::Integer(i64::from(self.objects_before)),
            ),
            (
                "objects_after".to_owned(),
                Value::Integer(i64::from(self.objects_after)),
            ),
            (
                "compressed_objects".to_owned(),
                Value::Integer(i64::from(self.compressed)),
            ),
            (
                "object_streams".to_owned(),
                Value::Integer(i64::from(self.object_streams)),
            ),
            (
                "recompressed_streams".to_owned(),
                Value::bytes(self.recompressed),
            ),
            (
                "recompression_saved".to_owned(),
                Value::bytes(self.recompression_saved),
            ),
        ]
    }
}

/// Rewrites one document smaller and writes it.
///
/// `at` is the document's position among the opened ones — one, for this verb.
///
/// # Errors
///
/// [`Refusal::NoSuchSource`], [`Refusal::Reconstructed`] where the source states its structure
/// only through §C.4's recovery, [`Refusal::Assembly`] where the document cannot be rewritten at
/// all, and [`Refusal::Sink`] where the output cannot be written.
pub(crate) fn run(
    plan: &OptimizePlan,
    at: usize,
    documents: &[Document],
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<(), Refusal> {
    let document = documents.get(at).ok_or(Refusal::NoSuchSource {
        at: plan.source,
        count: documents.len(),
    })?;

    // §7.5.5's Table 15: `/Root` is "( Required; shall be an indirect reference ) The catalog
    // dictionary for the PDF file". A trailer stating it any other way is a file this verb
    // cannot rewrite without inventing the catalog's identity, so it says so.
    let Some(root) = document
        .trailer()
        .get("Root")
        .and_then(Object::as_reference)
    else {
        return Err(Refusal::Reconstructed(
            "§7.5.5's Table 15 makes /Root \"( Required; shall be an indirect reference ) The \
             catalog dictionary for the PDF file\", and this document's trailer does not state \
             one"
            .to_owned(),
        ));
    };
    refuse_a_document_only_recovery_reads(document, root)?;

    let mut assembly = Assembly::new(vec![document]);
    let mapped = copy_closure(&mut assembly, document, root, plan.prune)
        .map_err(|error| Refusal::Assembly(error.to_string()))?;
    assembly.set_root(mapped);
    // §14.3.3's document information dictionary is the trailer's second root: nothing in the
    // catalog reaches it, so a walk from `/Root` alone would drop a document's title and author.
    if let Some(info) = document
        .trailer()
        .get("Info")
        .and_then(Object::as_reference)
    {
        let carried = copy_closure(&mut assembly, document, info, plan.prune)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
        assembly.set_info(Some(carried));
    }

    if assembly.has_encrypted_source() {
        report.warnings.push(Warning {
            source: plan.source,
            page: None,
            detail: "§7.6: this document is encrypted and the rewritten one is not, because this \
                     writer emits no /Encrypt"
                .to_owned(),
        });
    }

    let options = Options {
        form: Form::of(document),
        object_streams: plan.object_streams,
        streams: plan.streams,
    };
    let version = document
        .version()
        .unwrap_or(pdf_syntax::Version { major: 1, minor: 7 });
    let expanded = plan.names.expand(&Fill {
        ordinal: 1,
        count: 1,
        page: None,
        label: None,
        title: None,
    });
    let mut writer = sinks.open(&expanded.name).map_err(|error| Refusal::Sink {
        name: expanded.name.clone(),
        error,
    })?;
    let written = serialize(&assembly, version, options, &mut writer)
        .map_err(|error| Refusal::Assembly(format!("{}: {error}", expanded.name)))?;
    writer.flush().map_err(|error| Refusal::Sink {
        name: expanded.name.clone(),
        error,
    })?;
    drop(writer);

    if written.dangling > 0 {
        report.warnings.push(Warning {
            source: plan.source,
            page: None,
            detail: format!(
                "§7.3.10: {} reference(s) named an object this document does not hold and were \
                 written as null",
                written.dangling
            ),
        });
    }

    report.outputs.push(Output {
        name: expanded.name,
        bytes: written.bytes,
        sanitised: expanded.sanitised,
        origin: Origin::Optimized {
            source: plan.source,
            pages: Pages::new(document).len(),
            savings: Savings {
                before: u64::try_from(document.bytes().len()).unwrap_or(u64::MAX),
                after: written.bytes,
                objects_before: declared_objects(document),
                objects_after: written.objects,
                compressed: written.compressed,
                object_streams: written.object_streams,
                recompressed: written.recompressed,
                recompression_saved: written.saved,
            },
        },
    });
    Ok(())
}

/// Refuses a document whose structure this program reaches only by §C.4's recovery.
///
/// **The one refusal this verb has, and the corpus walk is why it exists.** Four documents
/// rewrote into files with no page at all, and every one of them was a file this tree opens by
/// *recovering* what its own trailer misstates: `poppler-742-0-fuzzed.pdf`, whose `/Root` names
/// an object that is not there; `issue9418.pdf`, whose every object is misfiled so that `/Root`
/// names a §14.3.3 information dictionary; and `issue19484_1.pdf` and `issue19484_2.pdf`, whose
/// catalog's `/Pages` names nothing. `pdf_model::Pages` finds their pages anyway, by looking for
/// what §7.7.3.3's Table 31 describes, and that is right for a *reader*.
///
/// It is not right for a writer, and the two clauses say why. §7.5.5's Table 15: `/Root` is
/// "( Required; shall be an indirect reference ) The catalog dictionary for the PDF file".
/// §7.7.2's Table 29: `/Pages` is "( Required; shall be an indirect reference ) The page tree
/// node that shall be the root of the document's page tree". Where either is false, the document
/// this program displays is one it *reconstructed*, and a rewrite of it would be a file stating
/// a structure no producer wrote — which is the far side of `CLAUDE.md`'s line, not the near one.
///
/// So it is refused by name (trap 5: unsupported input stays loud) rather than rewritten into a
/// document with nothing in it. A caller who wants such a file repaired is asking for a
/// different verb than this one.
///
/// # Errors
///
/// [`Refusal::Reconstructed`], naming which of the two clauses the document does not satisfy —
/// RFC 0002 section 4.4's exit 4 rather than its 2, because this tree reads and draws such a
/// document and it is the *writer* that declines.
fn refuse_a_document_only_recovery_reads(
    document: &Document,
    root: ObjectId,
) -> Result<(), Refusal> {
    let catalog = document.get(root);
    let Some(catalog) = catalog.as_dict() else {
        return Err(Refusal::Reconstructed(format!(
            "§7.5.5's Table 15 makes /Root \"( Required; shall be an indirect reference ) The \
             catalog dictionary for the PDF file\", and object {} is not a dictionary; this \
             document is one only §C.4's recovery reads, and rewriting it would state a \
             structure no producer wrote",
            root.number
        )));
    };
    let pages = catalog
        .get("Pages")
        .map_or(Object::Null, |value| document.resolve(value));
    if pages.as_dict().is_none() {
        return Err(Refusal::Reconstructed(
            "§7.7.2's Table 29 makes /Pages \"( Required; shall be an indirect reference ) The \
             page tree node that shall be the root of the document's page tree\", and this \
             catalog's does not resolve to one; this document is one only §C.4's recovery pages, \
             and rewriting it would state a structure no producer wrote"
                .to_owned(),
        ));
    }
    Ok(())
}

/// Copies `start` and everything it reaches into the assembly, answering its output number.
///
/// Breadth-first in `Dictionary`'s own key order, which is sorted, so the output's numbering is
/// a function of the source's object graph — RFC 0002 section 9's first layer, and the whole of
/// why optimising an optimised file changes nothing.
///
/// With `prune` false the closure is still walked, and every object number the trailer's
/// `/Size` declares is copied besides. Table 15 makes `/Size` "[t]he total number of entries in
/// the PDF file's cross-reference table", and §7.5.4 numbers those entries 0 to `/Size` − 1
/// with 0 the free head, so 1 to `/Size` − 1 is every number the file states anything about.
fn copy_closure(
    assembly: &mut Assembly<'_>,
    document: &Document,
    start: ObjectId,
    prune: bool,
) -> Result<ObjectId, pdf_syntax::AssemblyError> {
    let mapped = assembly.copy(0, start)?;
    let mut queue: VecDeque<ObjectId> = VecDeque::new();
    queue.push_back(start);

    if !prune {
        let size = document
            .trailer()
            .get("Size")
            .and_then(Object::as_integer)
            .unwrap_or(0);
        for number in 1..u32::try_from(size).unwrap_or(0) {
            let id = ObjectId::new(number, 0);
            if document.get(id) == Object::Null {
                continue;
            }
            if assembly.copied(0, id).is_none() {
                assembly.copy(0, id)?;
                queue.push_back(id);
            }
        }
    }

    while let Some(id) = queue.pop_front() {
        let value = document.get(id);
        reach(assembly, document, &value, 0, &mut queue)?;
    }
    Ok(mapped)
}

/// Every reference in one value, copied and queued.
fn reach(
    assembly: &mut Assembly<'_>,
    document: &Document,
    value: &Object,
    depth: usize,
    queue: &mut VecDeque<ObjectId>,
) -> Result<(), pdf_syntax::AssemblyError> {
    if depth >= MAX_WALK_DEPTH {
        return Ok(());
    }
    match value {
        Object::Reference(id) => {
            if assembly.copied(0, *id).is_some() {
                return Ok(());
            }
            // §7.3.10: a reference to an object the file does not hold "shall be treated as a
            // reference to the null object", and a source object whose value *is* null says the
            // same thing. Neither is written, and the serializer states §7.3.10's null in place
            // of the reference and counts it.
            if document.get(*id) == Object::Null {
                return Ok(());
            }
            assembly.copy(0, *id)?;
            queue.push_back(*id);
        }
        Object::Array(items) => {
            for item in items {
                reach(assembly, document, item, depth.saturating_add(1), queue)?;
            }
        }
        Object::Dictionary(dict) => {
            for (_, item) in dict.iter() {
                reach(assembly, document, item, depth.saturating_add(1), queue)?;
            }
        }
        Object::Stream(stream) => {
            for (key, item) in stream.dict.iter() {
                // §7.3.8.2's `/Length` is re-derived by the writer as a direct integer, so an
                // object the source stated it in is referred to by nothing in the output. Not
                // following it is what keeps that object out — and what keeps a second run of
                // this verb from removing something the first one left.
                if key.as_bytes() == b"Length" {
                    continue;
                }
                reach(assembly, document, item, depth.saturating_add(1), queue)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// How many objects the source's trailer declares, less §7.5.4's free head.
///
/// Table 15's `/Size` is "[t]he total number of entries in the PDF file's cross-reference
/// table", and entry 0 "shall always be free", so the count of objects a reader could ask for
/// is one less. A document whose `/Size` is absent or nonsense counts as none rather than as a
/// guess.
fn declared_objects(document: &Document) -> u32 {
    document
        .trailer()
        .get("Size")
        .and_then(Object::as_integer)
        .and_then(|size| u32::try_from(size).ok())
        .map_or(0, |size| size.saturating_sub(1))
}
