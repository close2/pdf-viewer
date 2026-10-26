//! What this document has, answered once and remembered per generation: the page names, the
//! embedded files, the metadata stream and the images of a page.
//!
//! The free functions the directory, the read and the write modules share. The inventories that
//! need the worker are remembered on [`crate::Current`] or in the cache, so that a name's
//! validation does not ask the same question again (trap 33).

use std::sync::Arc;

use crate::cache::Cache;
use crate::error::VfsError;
use crate::layout::{Generator, Kind, Route};
use crate::path;
use crate::worker::{Answer, Query, Worker};
use crate::{Current, DirEntry, Embedded, mismatch};

/// The row a path names, checked against what this document actually has.
///
/// [`path::resolve`] answers structurally; this is where a page number past the end, an
/// unoffered resolution and an attachment the document does not file become
/// [`VfsError::NoSuchPath`]. Two answers rather than one on purpose: a face passes arbitrary
/// paths, and "not in the layout" and "not in this document" are the same error to it and
/// different errors to a reader of this code.
pub(crate) fn locate_in(
    current: &Current,
    path: &str,
    resolutions: &[u32],
    cache: &Cache,
) -> Result<(&'static Route, path::Captures), VfsError> {
    let missing = || VfsError::NoSuchPath(path.to_owned());
    let (route, captures) = path::resolve(path).ok_or_else(missing)?;
    if let Some(page) = captures.page
        && (page > current.pages || page == 0)
    {
        return Err(missing());
    }
    if let Some(dpi) = captures.dpi
        && !resolutions.contains(&dpi)
    {
        return Err(missing());
    }
    match route.generator {
        // A page ordinal has exactly one spelling, and `path::resolve` already held it to the
        // minimum width; this is where the *document's* width decides, so a 12 000-page
        // document's pages are `00001.pdf` and nothing answers to `0001.pdf`.
        Generator::ExtractedPage | Generator::RenderedPage | Generator::PageText => {
            let page = captures.page.ok_or_else(missing)?;
            if !spells(current, path, page) {
                return Err(missing());
            }
        }
        Generator::ImageInventory | Generator::ImagePageOrdinals => {
            if let Some(page) = captures.page
                && !spells(current, path, page)
            {
                return Err(missing());
            }
        }
        Generator::ExtractedImage => {
            let page = captures.page.ok_or_else(missing)?;
            let name = captures.name.clone().ok_or_else(missing)?;
            // The inventory rather than the extraction. Validating a name by re-running the run
            // that produced it is what made a wide directory quadratic (ADR 0886): on
            // a page of ten thousand images every `stat` and every `open` paid 176 ms here while
            // the bytes it was about sat in the cache.
            if !spells(current, path, page) || !image_names(cache, current, page)?.contains(&name) {
                return Err(missing());
            }
        }
        Generator::ExtractedAttachment => {
            let name = captures.name.clone().ok_or_else(missing)?;
            if !attachments(current)?
                .iter()
                .any(|embedded| embedded.file_name == name)
            {
                return Err(missing());
            }
        }
        _ => {}
    }
    // §14.3.2's stream is the one file in this tree whose *existence* the document states, so it
    // is the one row whose validation cannot derive its answer. It asks [`metadata_stream`]
    // rather than the worker, which is what keeps it one question a generation. Outside the match
    // rather than an arm of it, because a `?` cannot live in a match guard and an arm that only
    // asks a question reads as though it were doing something else.
    if route.generator == Generator::MetadataStream && metadata_stream(cache, current)?.is_none() {
        return Err(missing());
    }
    Ok((route, captures))
}

/// Whether the path spells its page ordinal at this document's own width.
///
/// [`path::resolve`] has already held the ordinal to one spelling at the *minimum* width, so
/// what is left is the document's own: a twelve-thousand-page document's pages are `00001.pdf`
/// and nothing answers to `0001.pdf`. It is asked of every component rather than of the one the
/// route puts the ordinal in, which is loose in principle and exact for this table — the only
/// component that can equal a page's stem is the one the ordinal came from, and every row whose
/// path holds a second variable component (an image's name, an attachment's) checks that
/// component against the document's own inventory in the same breath.
pub(crate) fn spells(current: &Current, path: &str, page: usize) -> bool {
    let width = width(current);
    let expected = path::page_name_stem(page, width);
    path::components(path).is_some_and(|parts| {
        parts.iter().any(|part| {
            part == &expected.as_str()
                || part
                    .split_once('.')
                    .is_some_and(|(stem, _)| stem == expected.as_str())
        })
    })
}

/// How many digits a page ordinal takes in this document.
///
/// RFC 0003 section 4's "zero-padded ordinal; width from page count", with four as the floor
/// so that a five-page document reads like the RFC's own example.
pub(crate) fn width(current: &Current) -> usize {
    current.pages.to_string().len().max(path::MIN_ORDINAL_WIDTH)
}

/// One name per page, at the document's width, with this extension.
pub(crate) fn page_names(current: &Current, extension: &str) -> Vec<DirEntry> {
    let width = width(current);
    (1..=current.pages)
        .map(|page| DirEntry {
            name: path::page_name(page, width, extension),
            kind: Kind::File,
        })
        .collect()
}

/// §7.11.4's embedded files, read once per generation, under the names the tree lists.
///
/// ISO 32000-2 §7.11.4.1:
///
/// > This makes the PDF file a self-contained unit that can be stored or transmitted as a
/// > single entity.
///
/// Which is why the mount can show them at all, and why the names are the document's rather
/// than this program's: the file specification says what the file is called. What this does
/// to them is the minimum a directory requires — `pdf_transform::pattern::sanitise`'s
/// replacement of the bytes a file name may not hold, then `.` and `..` and the empty name,
/// then a deterministic suffix where two of the document's names collide after that. Every
/// step is recorded in the map, so a read is looked up by the *document's* name and never by
/// the sanitised one.
pub(crate) fn attachments(current: &Current) -> Result<Arc<Vec<Embedded>>, VfsError> {
    let mut held = current
        .attachments
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(known) = held.as_ref() {
        return Ok(Arc::clone(known));
    }
    let Answer::Attachments(entries) = current.worker.ask(&Query::AttachmentInventory)? else {
        return Err(VfsError::Worker(mismatch(&Answer::Absent, "attachments")));
    };
    let mut taken: Vec<String> = Vec::new();
    let mut embedded = Vec::with_capacity(entries.len());
    for entry in entries {
        let safe = safe_name(&entry.name);
        let mut file_name = safe.clone();
        let mut suffix = 1_u32;
        while taken.contains(&file_name) {
            suffix = suffix.saturating_add(1);
            file_name = format!("{safe}~{suffix}");
        }
        taken.push(file_name.clone());
        embedded.push(Embedded {
            file_name,
            document_name: entry.name,
        });
    }
    let shared = Arc::new(embedded);
    *held = Some(Arc::clone(&shared));
    Ok(shared)
}

/// §14.3.2's document-level metadata stream, asked of the worker once a generation.
///
/// **`meta/xmp.xml` is the one file in this tree whose existence the *document* states**, so it is
/// the one row whose path validation cannot derive its answer and has to ask a question. That
/// question is not the whole stream, put again on every listing of `/meta` and on every `stat`
/// and `open` under it — trap 33's shape exactly, and invisible to `Vfs::generated` because the
/// bytes are produced once. [`crate::Vfs::questions`] is what counts it (ADR 0894).
///
/// So: the bytes go into the cache under the path they belong to, which means a validation warms
/// the read the way a listing of `images/NNNN/` does; a document that states no stream is
/// remembered as stating none, which costs one bool a generation; and the only way this asks the
/// worker twice is the cache forgetting the bytes in between, which [`crate::Vfs::forgotten`] counts.
pub(crate) fn metadata_stream(
    cache: &Cache,
    current: &Current,
) -> Result<Option<Arc<[u8]>>, VfsError> {
    // The layout's own row, which has no variable component: `crate::layout`'s table spells it
    // once and this is the only other place it is written. `tests/a_face.rs` holds the pair
    // together by asking for the file and requiring that nothing was asked twice — a constant
    // that did not match the row would cache under a path no reader looks at, and the repeat
    // would say so.
    const XMP: &str = "/meta/xmp.xml";
    if let Some(bytes) = cache.get(current.generation, XMP) {
        return Ok(Some(bytes));
    }
    let mut known = current
        .metadata
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if *known == Some(false) {
        return Ok(None);
    }
    match current.worker.ask(&Query::MetadataStream)? {
        Answer::Bytes(bytes) => {
            *known = Some(true);
            Ok(Some(cache.put(current.generation, XMP, bytes)))
        }
        Answer::Absent => {
            *known = Some(false);
            Ok(None)
        }
        other => Err(VfsError::Worker(mismatch(&other, "bytes"))),
    }
}

/// The directory one page's images are listed in.
fn images_directory(current: &Current, page: usize) -> String {
    format!("/images/{}", path::page_name_stem(page, width(current)))
}

/// What `images/NNNN/` holds, without producing it a second time.
///
/// **The listing of that directory *is* the extraction's own output names** — the whole argument
/// for `crate::layout`'s departure from RFC 0003 section 4, because a listing and a read that are
/// one call cannot disagree — so every question about a path under it used to run the extraction:
/// [`locate_in`]'s validation, [`crate::Vfs::entries_of`]'s listing, and the [`crate::Vfs::stat`] and
/// [`crate::Vfs::open`] that go through the first of those. On a page of ten thousand images that is one
/// extraction per question rather than per run (ADR 0886).
///
/// So the *names* are kept where the content is kept, past the eviction of the content, exactly as
/// a length is: `Cache::inventory`. They are a run's own output names rather than a guess, which is
/// the same rule RFC 0003 section 5.5 states for a size.
pub(crate) fn image_names(
    cache: &Cache,
    current: &Current,
    page: usize,
) -> Result<Arc<[String]>, VfsError> {
    if let Some(names) = cache.inventory(current.generation, &images_directory(current, page)) {
        return Ok(names);
    }
    let mut names: Vec<String> = Vec::new();
    for name in extract_images(cache, current, page)?.keys() {
        names.push(name.clone());
    }
    Ok(names.into())
}

/// One extraction, every output kept, and the names noted.
///
/// A page's images come out of one `pdf_transform::images` run, so reading them one at a time
/// would re-run it once per file — and a `cp -r` of an `images/NNNN/` directory reads them all.
/// The whole run goes into the cache under each output's own path; the cache's byte budget decides
/// how much of it survives, which is the one place a decision about memory belongs. The
/// inventory is noted beside it and is not under that budget, because a name is not content.
pub(crate) fn extract_images(
    cache: &Cache,
    current: &Current,
    page: usize,
) -> Result<std::collections::BTreeMap<String, Arc<[u8]>>, VfsError> {
    let directory = images_directory(current, page);
    let mut kept = std::collections::BTreeMap::new();
    for (output, bytes) in images(current, page)? {
        let at = format!("{directory}/{output}");
        let shared = cache.put(current.generation, &at, bytes);
        kept.insert(output, shared);
    }
    let names: Arc<[String]> = kept.keys().cloned().collect::<Vec<String>>().into();
    cache.note_inventory(current.generation, &directory, names);
    Ok(kept)
}

/// One page's images, by the name each output took: the run itself, asked of the worker.
fn images(
    current: &Current,
    page: usize,
) -> Result<std::collections::BTreeMap<String, Vec<u8>>, VfsError> {
    match current.worker.ask(&Query::ExtractImages { page })? {
        Answer::Files(files) => Ok(files),
        other => Err(VfsError::Worker(mismatch(&other, "files"))),
    }
}

/// A name a directory can hold, from a name a document wrote.
///
/// Three steps, and the order is the point: `pdf_transform::pattern::sanitise` first, because it
/// is the tree's one statement of which bytes a file name may not hold; then the two names that
/// are not names, `.` and `..`, and the empty one, each replaced rather than rejected — a
/// document may file an attachment under any string, and refusing to show it would be a silent
/// loss.
pub(crate) fn safe_name(name: &str) -> String {
    let clean = pdf_transform::pattern::sanitise(name);
    match clean.as_str() {
        "" | "." => String::from("_"),
        ".." => String::from("__"),
        _ => clean,
    }
}

/// The last component of a layout pattern, which is the name it takes in its parent's listing.
pub(crate) fn last_component(pattern: &str) -> String {
    pattern.rsplit('/').next().unwrap_or(pattern).to_owned()
}
