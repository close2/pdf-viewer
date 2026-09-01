//! Remembering what a reference renderer produced, so it is asked once rather than daily.
//!
//! # Why this exists
//!
//! The oracle gate spends about 1020 seconds of processor time in `pdftoppm`, `mutool` and
//! `gs` against 46 in our own pipeline — a ratio above twenty to one, which means the gate is
//! very largely a measurement of three other programs. Nothing about their answers changes
//! between runs: the same version of the same renderer, given the same file and the same
//! command line, produces the same picture. Asking again is work whose result is already
//! known, and it is the single largest cost of the loop between writing a feature and finding
//! out which pages it moved.
//!
//! # The risk, and how the key answers it
//!
//! `doc/HANDOVER.md` names both halves: a content-addressed cache of reference renders is the
//! obvious lever, "with the equally obvious risk that a cache key omitting one variable (the
//! crop-box flag, the renderer version) would compare against stale renders in silence". A
//! cache that hands back the wrong picture does not fail — it moves the gate's verdict, which
//! is the one thing in this repository that is supposed to be independent of us.
//!
//! So the key is not a list of variables somebody remembered to include. It is built from
//! **the command line the harness is about to run**, taken from [`Reference::build_command`]
//! itself, with the two paths that vary by page replaced by placeholders — plus the
//! renderer's version string and the document's SHA-256. Adding `-cropbox` to an invocation
//! changes the key because the flag is *in* the key; there is no separate list to forget to
//! update. The one thing this cannot see is a renderer whose output changes without its
//! version string changing, which is a distribution's problem and is why [`Cache::clear`]
//! exists.
//!
//! # The one outcome that is not a function of its inputs
//!
//! A timeout. Everything else about a reference invocation is decided by the file and the
//! command line, but how long it took is decided by the machine, so
//! [`HarnessError::RendererTimedOut`] is its own variant and is remembered differently from
//! the failures that *are* deterministic — a renderer refusing a damaged file refuses it
//! every time.
//!
//! Not remembering timeouts at all was the first design, and measurement rejected it: with
//! everything else cached, **two pages out of 1794 were 46 of the run's 57 seconds**.
//! `bomb_giant.pdf` and `bug1978317.pdf` are decompression bombs, two renderers apiece are
//! given thirty seconds on each and none of them returns, and a gate whose wall clock is
//! otherwise its slowest core spent four fifths of it waiting to kill processes it had
//! already killed yesterday.
//!
//! So a timeout *is* remembered, and for a week — [`TIMEOUT_MEMORY`]. That is a deliberate
//! trade with three parts written down:
//!
//! - **The gate is already non-deterministic about this.** A renderer that needs 29 seconds
//!   on an idle machine and 31 under load changes the verdict of an uncached run too. The
//!   cache does not add that; it makes one observation sticky rather than flapping, and a
//!   sticky one is at least *printed*.
//! - **A wrongly-remembered timeout cannot hide a page the gate is watching.** A page whose
//!   reference times out leaves the comparison, and the ratchets are checked for equality in
//!   both directions — so any listed page that silently stopped being compared fails the
//!   build with "no longer contradicted". What it can hide is a page nobody has listed, which
//!   is what the expiry bounds.
//! - **A week is the bound, not forever.** [`Cache::clear`] and `PDFREF_CACHE=off` are the
//!   immediate remedies; the expiry is what happens when nobody thinks to use them.
//!
//! # What a hit owes: the renderer's words as well as its picture
//!
//! [`Cache::render`] promises that "a cached page's evidence directory is indistinguishable
//! from an uncached one's". It was not, and the one file missing is the one that says *why* a
//! renderer produced what it produced: [`Reference::render_within`] sends both of the
//! renderer's output streams to `<name>.log` beside its image, and only a **miss** ran it. So
//! on a run with a 99.8% hit rate every verdict was reached from rasters while every
//! diagnosis came from log files some earlier run happened to leave behind — and a page whose
//! whole evidence is what three programs *said* had nothing to read.
//!
//! That is not only a diagnostic loss. [`crate::Testimony`] makes a renderer's own words part
//! of the rule: a flat sheet from a program that said it could not decode the page is not that
//! program's reading of it, which no predicate over pixels can establish (ADR 0769). A rule
//! that reads a file only present on a miss would reach different verdicts on the first run
//! and the second, which is the one thing a cache may never do.
//!
//! So the log is stored beside the picture and restored with it, empty included — an empty log
//! is a renderer that said nothing, which is a fact about the page and not an absence of one.
//! It is stored **only** beside a picture: a stored failure already carries the renderer's
//! sentence in its own text, and a reference that produced no raster does not vote.
//!
//! # Proving it changes nothing
//!
//! The claim a cache has to earn is that the gate reaches the same verdict with it as
//! without. Two things establish that here. `a_hit_reproduces_what_the_renderer_produced`
//! renders a page uncached, then twice through a cache, and demands all three rasters be
//! byte-identical — and, since the log joined the entry, that the log a hit leaves in the work
//! directory is byte-identical to the one the renderer itself wrote; and the oracle takes
//! `PDFREF_CACHE=off`, so the whole 1794-page run can be made to ask the renderers again and
//! its numbers compared against a cached run's.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use pdf_render::Raster;

use crate::digest::Sha256;
use crate::{HarnessError, Reference, digest, png_io};

/// Changed whenever a stored entry's *meaning* changes, so old entries cannot be read as new
/// ones. Part of every key, so a bump invalidates the whole cache without deleting anything.
///
/// **`-2` since the eight-hundred-and-forty-second session**, when an entry stopped being a
/// picture and became a picture *and what the renderer said while producing it*. The bump is
/// what a bump is for and the alternative was rejected deliberately: treating an entry with no
/// stored log as a miss would leave old entries readable as new ones by a second, ad-hoc route,
/// and would make "no log stored" and "the renderer said nothing" the same thing on disk — which
/// is exactly the distinction [`crate::Testimony`] rests on. It costs one re-render of every
/// entry, about a thousand seconds of `pdftoppm`, `mutool` and `gs` over this corpus, once.
const FORMAT: &str = "pdfref-reference-cache-2";

/// How long a remembered timeout is believed before the renderer is given another chance.
///
/// A week, and the number is a judgement rather than a measurement: long enough that the two
/// documents which actually do this are not re-killed daily, short enough that a machine that
/// was briefly overloaded recovers without anybody diagnosing it. See the module comment for
/// why a timeout is remembered at all, and for what bounds the damage if one is wrong.
const TIMEOUT_MEMORY: std::time::Duration = std::time::Duration::from_hours(24 * 7);

/// Where a reference render is remembered, and how many times that saved a run.
#[derive(Debug)]
pub struct Cache {
    /// `None` disables the cache entirely, which is what verifies it changes no verdict.
    root: Option<PathBuf>,
    /// One digest per document, because a 352-page document is otherwise hashed 352 times.
    ///
    /// Keyed by path: a file is not expected to change while a run is comparing against it,
    /// and one that did would be a corpus being edited underneath a gate.
    documents: Mutex<HashMap<PathBuf, [u8; 32]>>,
    /// One identity per renderer, because establishing it spawns a process.
    identities: Mutex<HashMap<Reference, String>>,
    hits: AtomicU64,
    misses: AtomicU64,
    remembered_timeouts: AtomicU64,
}

/// What the cache did over a run, for the report that says where the time went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Statistics {
    /// Renders answered from disk.
    pub hits: u64,
    /// Renders that had to be produced.
    pub misses: u64,
    /// How many of the hits were a remembered timeout rather than a picture.
    ///
    /// Reported separately because it is the only kind of entry whose truth can decay, and
    /// because a rising count means pages are quietly leaving the comparison.
    pub remembered_timeouts: u64,
}

impl Statistics {
    /// Share of requests answered without running a renderer, in `0.0..=1.0`.
    #[must_use]
    pub fn hit_rate(self) -> f64 {
        let total = self.hits.saturating_add(self.misses);
        if total == 0 {
            return 0.0;
        }
        // Both counts are page tallies, far below the 2^53 an `f64` represents exactly.
        #[expect(
            clippy::cast_precision_loss,
            reason = "counts of pages, orders of magnitude below f64's exact integer range"
        )]
        {
            self.hits as f64 / total as f64
        }
    }
}

impl Cache {
    /// A cache stored under `root`.
    ///
    /// The caller chooses the directory, and whether to have one at all: this crate cannot
    /// find a build directory from inside a test, which is the same reason
    /// [`crate::reference::default_work_dir`] leaves that to whoever knows better.
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Some(root.into()),
            documents: Mutex::new(HashMap::new()),
            identities: Mutex::new(HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            remembered_timeouts: AtomicU64::new(0),
        }
    }

    /// A cache that stores nothing and answers nothing, for a run that must not be cached.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            root: None,
            documents: Mutex::new(HashMap::new()),
            identities: Mutex::new(HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            remembered_timeouts: AtomicU64::new(0),
        }
    }

    /// Whether anything is being stored.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.root.is_some()
    }

    /// Where entries are kept, if anywhere.
    #[must_use]
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// What the cache did so far.
    #[must_use]
    pub fn statistics(&self) -> Statistics {
        Statistics {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            remembered_timeouts: self.remembered_timeouts.load(Ordering::Relaxed),
        }
    }

    /// Empties the cache.
    ///
    /// The escape hatch for the one variable the key cannot see: a renderer whose output
    /// changed while its version string did not.
    ///
    /// # Errors
    ///
    /// Whatever removing the directory produced.
    pub fn clear(&self) -> std::io::Result<()> {
        match &self.root {
            Some(root) if root.exists() => std::fs::remove_dir_all(root),
            _ => Ok(()),
        }
    }

    /// Renders `page` of `pdf` with `reference`, from the cache where possible.
    ///
    /// On a hit the stored PNG is copied into `work_dir` under the name the renderer would
    /// have written, so that a cached page's evidence directory is indistinguishable from an
    /// uncached one's — the artefacts are what anybody diagnosing a disagreement actually
    /// opens, and a cache that quietly emptied them would have paid for speed with the thing
    /// the gate is for.
    ///
    /// # Errors
    ///
    /// As [`Reference::render`]. A cached failure is returned as
    /// [`HarnessError::RendererFailed`] carrying the message the renderer produced when it
    /// was last run.
    pub fn render(
        &self,
        reference: Reference,
        pdf: &Path,
        page: u32,
        dpi: u32,
        work_dir: &Path,
    ) -> Result<Raster, HarnessError> {
        let Some(entry) = self.entry_for(reference, pdf, page, dpi, work_dir) else {
            return reference.render(pdf, page, dpi, work_dir);
        };

        if let Some(stored) = read_entry(&entry, reference, work_dir) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            if matches!(stored, Err(HarnessError::RendererTimedOut { .. })) {
                self.remembered_timeouts.fetch_add(1, Ordering::Relaxed);
            }
            return stored;
        }
        self.misses.fetch_add(1, Ordering::Relaxed);

        let produced = reference.render(pdf, page, dpi, work_dir);
        write_entry(&entry, reference, work_dir, &produced);
        produced
    }

    /// The path a render would be stored at, or `None` when nothing can be stored.
    ///
    /// `None` means the cache is off, the document could not be hashed, or the renderer
    /// could not be identified — every one of which is a reason to fall through to the
    /// uncached path rather than to fail, because none of them is what the caller asked
    /// about.
    fn entry_for(
        &self,
        reference: Reference,
        pdf: &Path,
        page: u32,
        dpi: u32,
        work_dir: &Path,
    ) -> Option<PathBuf> {
        let root = self.root.as_ref()?;
        let document = self.document_digest(pdf)?;
        let identity = self.identity(reference)?;

        let mut key = Sha256::new();
        key.update_field(FORMAT.as_bytes());
        key.update_field(reference.name().as_bytes());
        key.update_field(identity.as_bytes());
        key.update_field(&document);
        key.update_field(&page.to_be_bytes());
        key.update_field(&dpi.to_be_bytes());
        // The invocation itself, so that a changed flag is a changed key. This is the whole
        // defence against the stale-render failure described at the top of this file.
        for word in reference.command_signature(pdf, page, dpi, work_dir) {
            key.update_field(word.as_bytes());
        }
        let key = key.hex();

        // Two levels, because a flat directory of six thousand entries is slow to list and
        // unpleasant to look at. `get` cannot fail: a hex digest is 64 characters.
        let prefix = key.get(..2).unwrap_or("00");
        Some(root.join(prefix).join(key))
    }

    /// The document's digest, computed once per file per run.
    fn document_digest(&self, pdf: &Path) -> Option<[u8; 32]> {
        if let Ok(known) = self.documents.lock()
            && let Some(digest) = known.get(pdf)
        {
            return Some(*digest);
        }
        let digest = digest::of_file(pdf).ok()?;
        if let Ok(mut known) = self.documents.lock() {
            known.insert(pdf.to_path_buf(), digest);
        }
        Some(digest)
    }

    /// What distinguishes this renderer's *build* from another, computed once per run.
    ///
    /// For the three external renderers that is their version string, which is what changes
    /// when a distribution updates them. For `hayro` it is the digest of the executable
    /// itself: it is built by this workspace, it has no version flag, and trap 10 in the
    /// handover is about exactly the mistake of comparing against a stale build of it.
    fn identity(&self, reference: Reference) -> Option<String> {
        if let Ok(known) = self.identities.lock()
            && let Some(identity) = known.get(&reference)
        {
            return Some(identity.clone());
        }
        let identity = if reference == Reference::Hayro {
            let program = reference.program_path();
            digest::of_file(&program).ok().map(|d| {
                let mut hex = Sha256::new();
                hex.update(&d);
                hex.hex()
            })?
        } else {
            reference.version()?
        };
        if let Ok(mut known) = self.identities.lock() {
            known.insert(reference, identity.clone());
        }
        Some(identity)
    }
}

/// Distinguishes one writer's temporary file from another's.
///
/// Several threads may miss on the same entry at once, and a shared temporary name would let
/// one rename another's half-written file into place.
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// Extension of a stored successful render.
const IMAGE: &str = "png";
/// Extension of the renderer's own diagnostics, stored beside [`IMAGE`] and never alone.
///
/// Beside a picture only. A stored [`FAILURE`] already holds the renderer's own sentence as its
/// whole text, and a reference that produced no raster takes no part in a consensus, so there
/// is nothing a second copy of its words would decide.
const LOG: &str = "log";
/// Extension of a stored deterministic failure, holding the message it produced.
const FAILURE: &str = "err";
/// Extension of a remembered timeout, holding the budget it outlived in milliseconds.
///
/// Its own kind rather than a `FAILURE` whose text says "exceeded", because this is the entry
/// whose truth decays: [`read_timeout`] believes it only while it is younger than
/// [`TIMEOUT_MEMORY`], and the file's own modification time is what says how old it is.
const TIMEOUT: &str = "slow";

/// Reads a stored render, or `None` if there is not one.
///
/// A stored entry that cannot be decoded is treated as absent rather than as an error: a
/// truncated file is a killed run's leftovers, and re-rendering is both correct and cheap.
fn read_entry(
    entry: &Path,
    reference: Reference,
    work_dir: &Path,
) -> Option<Result<Raster, HarnessError>> {
    let image = entry.with_extension(IMAGE);
    if image.is_file() {
        let raster = png_io::read(&image).ok()?;
        // The renderer's own output files, where the artefact writer, anybody reading the
        // evidence directory, and `Reference::testimony` all expect them.
        if std::fs::create_dir_all(work_dir).is_ok() {
            let _ = std::fs::copy(&image, work_dir.join(format!("{}.png", reference.name())));
            // Removed first, then copied: a log left by another page's render of the same
            // reference would otherwise be read as this page's testimony, which is the stale
            // -artefact failure this whole entry exists to close.
            let log = work_dir.join(format!("{}.log", reference.name()));
            let _ = std::fs::remove_file(&log);
            let _ = std::fs::copy(entry.with_extension(LOG), log);
        }
        return Some(Ok(raster));
    }

    let failure = entry.with_extension(FAILURE);
    if let Ok(detail) = std::fs::read_to_string(failure) {
        return Some(Err(HarnessError::RendererFailed { reference, detail }));
    }

    read_timeout(&entry.with_extension(TIMEOUT), reference)
}

/// A remembered timeout, if there is one and it is still young enough to believe.
///
/// The entry's own modification time is its age: writing a timestamp into the file would be a
/// second copy of a fact the filesystem already holds, and two copies of a fact is one of them
/// being wrong later. A clock that has gone backwards produces an age that cannot be computed,
/// which is treated as expired — the safe direction, since expiry costs a re-render and
/// believing a stale entry costs a page.
fn read_timeout(entry: &Path, reference: Reference) -> Option<Result<Raster, HarnessError>> {
    let milliseconds: u64 = std::fs::read_to_string(entry).ok()?.trim().parse().ok()?;
    let age = std::fs::metadata(entry)
        .and_then(|metadata| metadata.modified())
        .ok()?
        .elapsed()
        .ok()?;
    if age > TIMEOUT_MEMORY {
        return None;
    }
    Some(Err(HarnessError::RendererTimedOut {
        reference,
        budget: std::time::Duration::from_millis(milliseconds),
    }))
}

/// Stores what a render produced.
///
/// Failures are stored as well as pictures — a renderer that refuses a damaged file refuses it
/// every time — and a timeout is stored in its own kind, whose age decides whether a later run
/// believes it. A missing renderer and an undecodable PNG are stored as nothing at all: the
/// first says something about the machine's installation rather than the document, and the
/// second is how a truncated file from a killed run presents itself.
///
/// A picture is stored with the renderer's own log beside it, and the **log goes in first**:
/// [`read_entry`] tests for the image, so an image renamed into place ahead of its log would be
/// a hit whose testimony is missing — the very thing this entry was added to prevent.
///
/// Every write goes to a temporary name and is renamed into place, so that a run killed
/// mid-write cannot leave a truncated PNG for the next run to trust. Errors are ignored
/// throughout: a cache that cannot write is a cache that is slow, not a gate that is wrong.
fn write_entry(
    entry: &Path,
    reference: Reference,
    work_dir: &Path,
    produced: &Result<Raster, HarnessError>,
) {
    let Some(parent) = entry.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }

    if produced.is_ok() {
        // An absent log is stored as an empty one rather than as nothing, so that a hit
        // restores what a miss produced in every case: a renderer that said nothing said
        // nothing, and that is a reading of the entry rather than a gap in it.
        let said = work_dir.join(format!("{}.log", reference.name()));
        store(entry, LOG, |to| {
            std::fs::write(to, std::fs::read(&said).unwrap_or_default()).map(|()| 0)
        });
    }

    let (extension, source) = match produced {
        Ok(_) => (IMAGE, None),
        Err(HarnessError::RendererFailed { detail, .. }) => (FAILURE, Some(detail.clone())),
        Err(HarnessError::RendererTimedOut { budget, .. }) => (
            TIMEOUT,
            Some(
                u64::try_from(budget.as_millis())
                    .unwrap_or(u64::MAX)
                    .to_string(),
            ),
        ),
        // A renderer that is not installed, or a PNG this harness cannot read: neither is a
        // property of the document, and remembering either would outlive its cause.
        Err(_) => return,
    };

    if let Some(detail) = source {
        store(entry, extension, |to| {
            std::fs::write(to, &detail).map(|()| 0)
        });
    } else {
        // The renderer's own PNG rather than a re-encoding of the decoded raster: it is
        // already on disk, copying is cheaper than encoding, and a byte-for-byte copy cannot
        // introduce a difference of its own.
        let output = work_dir.join(format!("{}.png", reference.name()));
        store(entry, extension, |to| std::fs::copy(&output, to));
    }
}

/// Writes one part of an entry, atomically.
///
/// A temporary name and a rename, so that a run killed mid-write cannot leave a truncated file
/// for the next run to trust, and so that several threads missing on the same entry at once
/// cannot rename one another's half-written files into place.
fn store(entry: &Path, extension: &str, produce: impl FnOnce(&Path) -> std::io::Result<u64>) {
    let temporary = entry.with_extension(format!(
        "{extension}.tmp{}-{}",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    if produce(&temporary).is_ok() {
        let _ = std::fs::rename(&temporary, entry.with_extension(extension));
    } else {
        let _ = std::fs::remove_file(&temporary);
    }
}

#[cfg(test)]
mod tests {
    use super::Cache;

    /// A disabled cache must be transparent: no entries, no hits, and nothing on disk.
    #[test]
    fn a_disabled_cache_stores_nothing() {
        let cache = Cache::disabled();
        assert!(!cache.is_enabled());
        assert_eq!(cache.root(), None);
        assert_eq!(cache.statistics().hits, 0);
        assert!((cache.statistics().hit_rate() - 0.0).abs() < f64::EPSILON);
    }
}
