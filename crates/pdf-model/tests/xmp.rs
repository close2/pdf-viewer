//! ISO 32000-2 §14.3.2's metadata streams, over every one the corpus carries.
//!
//! The same shape of gate `dates.rs` is, and for the same reason: **a clause that states a
//! grammar is a clause that can audit a corpus.** Each of the 319 packets here is an independent
//! producer's attempt at ISO 16684-1, so running the reader over all of them checks it against a
//! hundred writers at once — and a *count* of properties, ratcheted, is what notices a reader
//! that quietly stops understanding one of the three spellings a property has.
//!
//! What it is not is a check that a value is *right*. Nothing here can know whether a file's
//! `dc:title` is its title. What it checks is that the packets this corpus contains are read, and
//! that the number read only rises.

// no sandbox worker: this gate reads §14.3.2's metadata packets out of the object graph and
// interprets no content stream, so no image reaches `pdf-sandbox` and no count here can
// move with the worker's presence (`tools/conformance/tests/sandbox_gates.rs`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_model::metadata::Information;
use pdf_model::xmp::{DC, PDF, XMP, Xmp};
use pdf_syntax::Document;

/// The fewest metadata streams that may parse.
///
/// 318 of the 319 the corpus carries. The one that does not is `PDFBOX-3148-2-fuzzed.pdf`,
/// whose stream does not decode at all — a refusal from `pdf-syntax`, one layer below this
/// reader, and the right answer for a file whose name says what was done to it.
const MIN_PACKETS: usize = 318;

/// The most streams that may fail to read.
const MAX_REFUSED: usize = 1;

/// The fewest properties the corpus's packets may yield between them.
///
/// Measured at 3191, over 318 packets, the largest of which states 25. A ratchet on the total
/// rather than on any file: a reader that stopped understanding ISO 16684-1 section 7.5's attribute
/// spelling would still parse every packet and would lose most of these without failing anything
/// else.
const MIN_PROPERTIES: usize = 3191;

/// The pdf.js corpus, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    files.sort();
    Some(files)
}

/// Every document-level packet in the corpus, read.
#[test]
#[ignore = "opens the whole corpus"]
fn every_metadata_stream_in_the_corpus_is_read() {
    let Some(files) = corpus() else {
        println!("the pdf.js submodule is not checked out; skipping");
        return;
    };

    let (mut carried, mut parsed, mut properties, mut titles) = (0usize, 0usize, 0usize, 0usize);
    let mut both_titles = 0usize;
    let mut refused: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut disagreements: Vec<String> = Vec::new();
    let mut largest = 0usize;

    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let Ok(catalog) = document.catalog() else {
            continue;
        };
        let Some(result) = Xmp::read(&document, &catalog) else {
            continue;
        };
        carried = carried.saturating_add(1);
        let name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        match result {
            Ok(xmp) => {
                parsed = parsed.saturating_add(1);
                properties = properties.saturating_add(xmp.properties().len());
                largest = largest.max(xmp.properties().len());
                if let Some(title) = xmp.title() {
                    titles = titles.saturating_add(1);
                    // §12.2's `/DisplayDocTitle` names `dc:title` and Table 349's NOTE calls
                    // `/Title` the same fact in the other place. Where the two disagree the
                    // standard says which wins for exactly nothing, so this counts rather than
                    // asserts — and the count being small is the reason the substitution this
                    // program made for 163 sessions was defensible.
                    if let Some(other) = Information::read(&document).title.as_deref() {
                        both_titles = both_titles.saturating_add(1);
                        if other.trim() != title.trim() {
                            disagreements.push(format!("{name}: XMP {title:?} vs /Info {other:?}"));
                        }
                    }
                }
            }
            Err(error) => refused.entry(error.to_string()).or_default().push(name),
        }
    }

    for (error, names) in &refused {
        println!("  refused: {error} — {names:?}");
    }
    let refusals: usize = refused.values().map(Vec::len).sum();
    println!(
        "{carried} documents carry §14.3.2's stream: {parsed} read, {refusals} refused, \
         {properties} properties between them (most in one packet: {largest}), \
         {titles} state dc:title"
    );
    println!(
        "  {both_titles} state both dc:title and §14.3.3's /Title: {} disagree",
        disagreements.len()
    );
    for line in &disagreements {
        println!("    {line}");
    }

    gate_ratchet::floor("metadata packets that parse", parsed, MIN_PACKETS);
    gate_ratchet::ceiling("metadata streams refused", refusals, MAX_REFUSED);
    gate_ratchet::floor(
        "properties read from those packets",
        properties,
        MIN_PROPERTIES,
    );
}

/// The three schemas clause 12 and clause 14 name are the ones the corpus states most.
///
/// Not a ratchet but a census, and it is here because it is the check a *reader* needs: the
/// property this tree will act on is `dc:title`, and a corpus where nothing states one would
/// make [`Xmp::title`] untested by anything but a fixture.
#[test]
#[ignore = "opens the whole corpus"]
fn the_properties_the_pdf_clauses_name_are_the_ones_the_corpus_states() {
    let Some(files) = corpus() else {
        println!("the pdf.js submodule is not checked out; skipping");
        return;
    };

    let mut counts: BTreeMap<(&str, String), usize> = BTreeMap::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let Some(Ok(xmp)) = Xmp::document(&document) else {
            continue;
        };
        for (name, _) in xmp.properties() {
            let schema = match name.namespace.as_str() {
                DC => "dc",
                PDF => "pdf",
                XMP => "xmp",
                _ => continue,
            };
            *counts.entry((schema, name.local.clone())).or_default() += 1;
        }
    }

    for ((schema, local), count) in &counts {
        println!("  {count:4}  {schema}:{local}");
    }

    // Every property Table 349's NOTEs pair with a dictionary entry, and §12.2's own.
    for (schema, local, least) in [
        ("dc", "title", 100),
        ("dc", "creator", 80),
        ("dc", "description", 30),
        ("pdf", "Producer", 290),
        ("pdf", "Keywords", 30),
        ("xmp", "CreatorTool", 280),
        ("xmp", "CreateDate", 300),
        ("xmp", "ModifyDate", 300),
    ] {
        let found = counts
            .get(&(schema, local.to_owned()))
            .copied()
            .unwrap_or_default();
        assert!(
            found >= least,
            "{schema}:{local} was read from {found} documents, down from {least}"
        );
    }
}

/// §14.3.2's *object-level* metadata: a `/Metadata` stream on a form `XObject` reads back.
///
/// Table 348 attaches metadata to a component "through the Metadata entry in a stream or
/// dictionary representing the object", and NOTE 1 names a form among the carriers. The corpus
/// states one on a catalog (319), an image (3) and other streams (55) but **not one on a form
/// `XObject`** — `pdf-model --example object_metadata_census` reads zero across 66 918 documents —
/// so the carrier the clause's own sentence about property-list metadata points at is unexercised
/// by the world and has to be planted to be checked (trap 13).
///
/// What this asserts is that `Xmp::read` is general over Table 348's carriers, not special to the
/// catalog: handed a form `XObject`'s dictionary, it resolves the `/Metadata` reference and reads
/// the packet. It is the reader §14.6.2's property-list metadata and §14.3.2's object-level
/// metadata both reach; there is no in-scope *consumer* of an object-level packet, which is why
/// this is a calibration rather than a rendering path.
#[test]
fn a_metadata_stream_on_a_form_xobject_reads_back() {
    use std::fmt::Write as _;

    use pdf_syntax::ObjectId;

    const PACKET: &str = "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
        <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
        <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
        <rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
        <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">A drawing</rdf:li></rdf:Alt></dc:title>\n\
        </rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>";
    const FORM: &str = "/Fm0 Do";

    // 1 catalog, 2 pages, 3 page, 4 contents, 5 the form XObject bearing /Metadata, 6 the packet.
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << /Fm0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{FORM}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Metadata 6 0 R \
         /Length 0 >>\nstream\n\nendstream\nendobj\n\
         6 0 obj\n<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n{PACKET}\nendstream\n\
         endobj\n",
        FORM.len(),
        PACKET.len(),
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );

    let document = Document::open(out.into_bytes()).expect("the fixture is a valid PDF");
    let form = document.get(ObjectId {
        number: 5,
        generation: 0,
    });
    let form = form.as_stream().expect("the form XObject is a stream");
    let read = Xmp::read(&document, &form.dict)
        .expect("the form XObject states a /Metadata stream")
        .expect("the packet on the form XObject reads back");
    assert_eq!(
        read.title(),
        Some("A drawing"),
        "the form XObject's own metadata is what the general reader returns"
    );
}
