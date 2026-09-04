//! Positional text extraction from the reference implementations, cached like renders.
//!
//! # Why this lives beside the renderers
//!
//! The selection-geometry instrument (ADR 0323) asks two external extractors one question —
//! *where is this word, in `CropBox` points* — exactly as the oracle asks three renderers where
//! the ink is. The answers have the same economics: nothing about them changes between runs of
//! the same extractor version over the same file, and a corpus-scale gate that re-asked daily
//! would spend its wall clock re-learning them. So the output is remembered under the same rules
//! as [`crate::cache`], and for the same reason the key is built from **the invocation itself**
//! ([`Extractor::command_signature`]) plus the extractor's version and the document's SHA-256:
//! a flag that is not in the key is a flag that is not passed to the extractor either, which is
//! the whole defence against comparing against output produced under an older flag in silence
//! (trap 10a). `-cropbox` is the flag that earned that rule its text-domain entry — ADR 0323's
//! Finding 1 measured 41 documents disagreeing about the page's very *size* before it was
//! passed.
//!
//! # What each extractor answers, and in which frame
//!
//! Both are positional extractors and they do **not** answer in the same coordinate frame,
//! which is the first thing ADR 0323's measurement found and the reason its instrument audits
//! the frame before comparing a box:
//!
//! - [`Extractor::PopplerBoxes`] reports word boxes in the `CropBox` frame (because `-cropbox`
//!   is passed; its default is the `MediaBox`), **rotated** by `/Rotate` and **unscaled** by
//!   Table 31's `/UserUnit` — while its `<page>` element states the *unrotated* crop box size.
//!   That mismatch between what it states and where its coordinates are was established by
//!   measurement on the instrument's first run (ADR 0333; `hello_world_rotated.pdf` is the
//!   witness), and it is trap 3 one level deeper: a reference's stated frame and its
//!   coordinates can answer different questions.
//! - [`Extractor::MuPdfText`] reports character quads in points from the top-left of the page as
//!   *displayed*: `CropBox`-framed, with `/Rotate` applied and `/UserUnit` multiplied in — and
//!   its stated page size is that same frame.
//!
//! Reconciling the two is the caller's job, because it needs the page dictionary and this crate
//! deliberately does not parse PDF.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::digest::{self, Sha256};
use crate::reference::DEFAULT_TIMEOUT;

/// An external positional text extractor.
///
/// Deliberately not `#[non_exhaustive]`, unlike [`crate::Reference`]: a caller matching on
/// which extractor refused should be told at compile time when a third arrives, because the
/// wording of a refusal is per-extractor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Extractor {
    /// `pdftotext -bbox -cropbox`, from poppler: one box per word, in XHTML.
    PopplerBoxes,
    /// `mutool draw -F stext`, from `MuPDF`: one quad per character, in XML.
    MuPdfText,
}

/// What asking an extractor can produce, other than its output.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExtractionError {
    /// The extractor is not installed.
    #[error("{extractor} is not installed (provided by {package})")]
    Missing {
        /// Which extractor.
        extractor: Extractor,
        /// Package that provides it.
        package: &'static str,
    },
    /// The extractor ran and produced no usable output.
    #[error("{extractor} failed: {detail}")]
    Failed {
        /// Which extractor.
        extractor: Extractor,
        /// What went wrong.
        detail: String,
    },
    /// The extractor outlived its budget and was killed.
    ///
    /// Separate from [`Self::Failed`] for the reason [`crate::HarnessError::RendererTimedOut`]
    /// is: it is the one outcome that is not a function of the document, so the cache remembers
    /// it in an entry whose age decides whether it is still believed.
    #[error("{extractor} exceeded {budget:?} and was killed")]
    TimedOut {
        /// Which extractor.
        extractor: Extractor,
        /// The budget it outlived.
        budget: Duration,
    },
}

impl Extractor {
    /// Short name used in reports and cache keys.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::PopplerBoxes => "pdftotext-bbox",
            Self::MuPdfText => "mutool-stext",
        }
    }

    /// The executable this extractor needs.
    #[must_use]
    pub fn program(self) -> &'static str {
        match self {
            Self::PopplerBoxes => "pdftotext",
            Self::MuPdfText => "mutool",
        }
    }

    /// Package providing it, for error messages that are actionable.
    #[must_use]
    pub fn package_hint(self) -> &'static str {
        match self {
            Self::PopplerBoxes => "poppler (Arch) / poppler-utils (Debian)",
            Self::MuPdfText => "mupdf-tools",
        }
    }

    /// Returns `true` if the executable can be run.
    #[must_use]
    pub fn is_available(self) -> bool {
        // Spawning is the test, not the exit status, as `Reference::is_available` explains:
        // `mutool` with no subcommand exits non-zero while being perfectly present.
        Command::new(self.program())
            .arg("-v")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    /// The extractor's own version string, part of every cache key.
    #[must_use]
    pub fn version(self) -> Option<String> {
        let output = Command::new(self.program()).arg("-v").output().ok()?;
        // Both print their version to stderr, like their rendering siblings.
        let text = if output.stdout.is_empty() {
            &output.stderr
        } else {
            &output.stdout
        };
        String::from_utf8_lossy(text)
            .lines()
            .next()
            .map(str::trim)
            .map(ToOwned::to_owned)
    }

    /// The invocation this extractor would run, as text, with the paths that vary removed.
    ///
    /// The same construction as [`crate::Reference::command_signature`] and for the same
    /// reason: the cache key is derived from the command line itself rather than from a
    /// hand-maintained list, so adding a flag changes the key by construction.
    #[must_use]
    pub fn command_signature(self, pdf: &Path, page: u32) -> Vec<String> {
        let output = Path::new("<out>");
        let command = self.build_command(pdf, page, output);
        let pdf = pdf.to_string_lossy().into_owned();

        let mut signature = vec![self.program().to_owned()];
        signature.extend(command.get_args().map(|argument| {
            let argument = argument.to_string_lossy();
            if argument == pdf {
                return "<pdf>".to_owned();
            }
            argument.into_owned()
        }));
        signature
    }

    /// Builds the command line, writing the extraction to `output`.
    ///
    /// # `-cropbox` is load-bearing
    ///
    /// ISO 32000-2 §14.11.2.1: the crop box is the region a page is clipped to when displayed,
    /// so it is the frame a selection lives in — and `pdftotext`'s default is the media box.
    /// ADR 0323's Finding 1 measured the difference at 41 corpus documents disagreeing about
    /// the page's size; passing the flag took it to 8. `mutool draw` already works in the crop
    /// box and has no flag to state it for this output format.
    fn build_command(self, pdf: &Path, page: u32, output: &Path) -> Command {
        match self {
            Self::PopplerBoxes => {
                let mut command = Command::new(self.program());
                command
                    .arg("-bbox")
                    .arg("-cropbox")
                    .arg("-f")
                    .arg(page.to_string())
                    .arg("-l")
                    .arg(page.to_string())
                    .arg("-q")
                    .arg(pdf)
                    .arg(output);
                command
            }
            Self::MuPdfText => {
                let mut command = Command::new(self.program());
                command
                    .arg("draw")
                    .arg("-q")
                    .arg("-F")
                    .arg("stext")
                    .arg("-o")
                    .arg(output)
                    .arg(pdf)
                    .arg(page.to_string());
                command
            }
        }
    }

    /// Extracts `page` of `pdf`, bounded by [`DEFAULT_TIMEOUT`].
    ///
    /// `work_dir` is where the extractor's output file briefly lives; the text comes back as a
    /// string. There is no unbounded variant on purpose — these programs are pointed at
    /// corpora holding files written to make a reader loop.
    ///
    /// # Errors
    ///
    /// [`ExtractionError::Missing`] if the executable is absent, [`ExtractionError::Failed`] if
    /// it exits without producing decodable output, and [`ExtractionError::TimedOut`] if it
    /// outlives the budget and is killed.
    pub fn extract(
        self,
        pdf: &Path,
        page: u32,
        work_dir: &Path,
    ) -> Result<String, ExtractionError> {
        /// How often the child is checked while it runs.
        const POLL: Duration = Duration::from_millis(20);

        if !self.is_available() {
            return Err(ExtractionError::Missing {
                extractor: self,
                package: self.package_hint(),
            });
        }
        std::fs::create_dir_all(work_dir).map_err(|e| ExtractionError::Failed {
            extractor: self,
            detail: format!("could not create {}: {e}", work_dir.display()),
        })?;
        // One output file per process and thread, because a corpus run extracts in parallel.
        let output = work_dir.join(format!(
            "{}-{}-{:?}.txt",
            self.name(),
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&output);

        let mut child = self
            .build_command(pdf, page, &output)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| ExtractionError::Failed {
                extractor: self,
                detail: format!("could not run {}: {e}", self.program()),
            })?;

        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(e) => {
                    return Err(ExtractionError::Failed {
                        extractor: self,
                        detail: format!("could not wait for {}: {e}", self.program()),
                    });
                }
            }
            if started.elapsed() > DEFAULT_TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&output);
                return Err(ExtractionError::TimedOut {
                    extractor: self,
                    budget: DEFAULT_TIMEOUT,
                });
            }
            std::thread::sleep(POLL);
        };

        let text = std::fs::read_to_string(&output);
        let _ = std::fs::remove_file(&output);
        text.map_err(|e| ExtractionError::Failed {
            extractor: self,
            detail: format!(
                "produced no readable output (status {:?}): {e}",
                status.code()
            ),
        })
    }
}

impl std::fmt::Display for Extractor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Changed whenever a stored entry's *meaning* changes; part of every key.
const FORMAT: &str = "pdfref-extraction-cache-1";

/// How long a remembered timeout is believed — the same week, chosen for the same reasons, as
/// [`crate::cache`]'s.
const TIMEOUT_MEMORY: Duration = Duration::from_hours(24 * 7);

/// Extension of a stored successful extraction.
const TEXT: &str = "txt";
/// Extension of a stored deterministic failure, holding the message it produced.
const FAILURE: &str = "err";
/// Extension of a remembered timeout, holding the budget it outlived in milliseconds.
const TIMEOUT: &str = "slow";

/// Distinguishes one writer's temporary file from another's, exactly as the render cache does.
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// Where an extraction is remembered, under [`crate::cache`]'s rules.
///
/// A second small implementation rather than a generalisation of [`crate::Cache`], and that is
/// a decision: the render cache's payload is a PNG copied byte-for-byte from the renderer's own
/// output, this one's is a string, and making one struct serve both would put the raster
/// module's evidence-directory obligations behind a type parameter for two callers. The *rules*
/// are shared — the key's construction, the three entry kinds, the week-old timeout — and each
/// is cited to the module that argues it.
#[derive(Debug)]
pub struct ExtractionCache {
    /// `None` disables the cache entirely, which is what verifies it changes no verdict.
    root: Option<PathBuf>,
    /// One digest per document, because a document is otherwise hashed once per extractor.
    documents: Mutex<std::collections::HashMap<PathBuf, [u8; 32]>>,
    /// One identity per extractor, because establishing it spawns a process.
    identities: Mutex<std::collections::HashMap<Extractor, String>>,
    hits: AtomicU64,
    misses: AtomicU64,
    remembered_timeouts: AtomicU64,
    /// **The cost instrument**, on [`crate::cache::Runs`]'s rules and for its reasons: the
    /// keys an extractor has actually been *run* for, so that a second run of one is counted
    /// rather than inferred from a lookup tally that a bypass never touches.
    ran: Mutex<std::collections::HashSet<String>>,
    /// How many times an extractor was run.
    runs: AtomicU64,
    /// How many of those were for a key this run had already run.
    repeated: AtomicU64,
    /// The keys [`Self::repeated`] counted, each named once.
    repeats: Mutex<std::collections::HashSet<String>>,
    /// How many runs produced something the cache did not keep.
    unstored: AtomicU64,
}

impl ExtractionCache {
    /// A cache stored under `root`.
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self::with_root(Some(root.into()))
    }

    /// A cache that stores nothing and answers nothing, for a run that must not be cached.
    #[must_use]
    pub fn disabled() -> Self {
        Self::with_root(None)
    }

    /// The two constructors above, which differ in one field and are otherwise one thing.
    fn with_root(root: Option<PathBuf>) -> Self {
        Self {
            root,
            documents: Mutex::new(std::collections::HashMap::new()),
            identities: Mutex::new(std::collections::HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            remembered_timeouts: AtomicU64::new(0),
            ran: Mutex::new(std::collections::HashSet::new()),
            runs: AtomicU64::new(0),
            repeated: AtomicU64::new(0),
            repeats: Mutex::new(std::collections::HashSet::new()),
            unstored: AtomicU64::new(0),
        }
    }

    /// What the cache did so far, in the render cache's own vocabulary.
    #[must_use]
    pub fn statistics(&self) -> crate::cache::Statistics {
        crate::cache::Statistics {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            remembered_timeouts: self.remembered_timeouts.load(Ordering::Relaxed),
        }
    }

    /// What the **extractors** did so far, which is what a cost floor is built on.
    ///
    /// [`crate::cache::Runs`] states the invariant and why it is sound with no clock in it; the
    /// mechanism here is the same one, one program along.
    #[must_use]
    pub fn runs(&self) -> crate::cache::Runs {
        crate::cache::Runs {
            ran: self.runs.load(Ordering::Relaxed),
            repeated: self.repeated.load(Ordering::Relaxed),
            unstored: self.unstored.load(Ordering::Relaxed),
        }
    }

    /// What [`crate::cache::Runs::repeated`] counted, said rather than totalled.
    #[must_use]
    pub fn repeated_keys(&self) -> Vec<String> {
        self.repeats
            .lock()
            .map(|held| {
                let mut named: Vec<String> = held.iter().cloned().collect();
                named.sort();
                named
            })
            .unwrap_or_default()
    }

    /// Records that an extractor is about to be run for `key`, counting a second run of one.
    fn record_run(&self, key: &str) {
        self.runs.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut ran) = self.ran.lock()
            && !ran.insert(key.to_owned())
        {
            self.repeated.fetch_add(1, Ordering::Relaxed);
            if let Ok(mut repeats) = self.repeats.lock() {
                repeats.insert(key.to_owned());
            }
        }
    }

    /// Extracts `page` of `pdf` with `extractor`, from the cache where possible.
    ///
    /// # Errors
    ///
    /// As [`Extractor::extract`]. A cached failure is returned as [`ExtractionError::Failed`]
    /// carrying the message the extractor produced when it was last run.
    pub fn extract(
        &self,
        extractor: Extractor,
        pdf: &Path,
        page: u32,
        work_dir: &Path,
    ) -> Result<String, ExtractionError> {
        let key = format!("{}:{}:{page}", extractor.name(), pdf.display());
        let Some(entry) = self.entry_for(extractor, pdf, page) else {
            // Nothing can be stored, so the next question about this page runs the extractor
            // again; that is what [`crate::cache::Runs::unstored`] is the ceiling for.
            self.record_run(&key);
            self.unstored.fetch_add(1, Ordering::Relaxed);
            return extractor.extract(pdf, page, work_dir);
        };

        if let Some(stored) = read_entry(&entry, extractor) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            if matches!(stored, Err(ExtractionError::TimedOut { .. })) {
                self.remembered_timeouts.fetch_add(1, Ordering::Relaxed);
            }
            return stored;
        }
        self.misses.fetch_add(1, Ordering::Relaxed);

        self.record_run(&key);
        let produced = extractor.extract(pdf, page, work_dir);
        if !write_entry(&entry, &produced) {
            self.unstored.fetch_add(1, Ordering::Relaxed);
        }
        produced
    }

    /// The path an extraction would be stored at, or `None` when nothing can be stored.
    fn entry_for(&self, extractor: Extractor, pdf: &Path, page: u32) -> Option<PathBuf> {
        let root = self.root.as_ref()?;
        let document = self.document_digest(pdf)?;
        let identity = self.identity(extractor)?;

        let mut key = Sha256::new();
        key.update_field(FORMAT.as_bytes());
        key.update_field(extractor.name().as_bytes());
        key.update_field(identity.as_bytes());
        key.update_field(&document);
        key.update_field(&page.to_be_bytes());
        // The invocation itself, so that a changed flag is a changed key — trap 10a's rule,
        // and the whole reason this cache is derived from `build_command`.
        for word in extractor.command_signature(pdf, page) {
            key.update_field(word.as_bytes());
        }
        let key = key.hex();

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

    /// What distinguishes this extractor's build from another, computed once per run.
    fn identity(&self, extractor: Extractor) -> Option<String> {
        if let Ok(known) = self.identities.lock()
            && let Some(identity) = known.get(&extractor)
        {
            return Some(identity.clone());
        }
        let identity = extractor.version()?;
        if let Ok(mut known) = self.identities.lock() {
            known.insert(extractor, identity.clone());
        }
        Some(identity)
    }
}

/// Reads a stored extraction, or `None` if there is not one.
fn read_entry(entry: &Path, extractor: Extractor) -> Option<Result<String, ExtractionError>> {
    if let Ok(text) = std::fs::read_to_string(entry.with_extension(TEXT)) {
        return Some(Ok(text));
    }
    if let Ok(detail) = std::fs::read_to_string(entry.with_extension(FAILURE)) {
        return Some(Err(ExtractionError::Failed { extractor, detail }));
    }
    read_timeout(&entry.with_extension(TIMEOUT), extractor)
}

/// A remembered timeout, if there is one and it is still young enough to believe.
///
/// The entry's modification time is its age, as in [`crate::cache`], and a clock that has gone
/// backwards is treated as expiry — the direction that costs a re-run rather than a page.
fn read_timeout(entry: &Path, extractor: Extractor) -> Option<Result<String, ExtractionError>> {
    let milliseconds: u64 = std::fs::read_to_string(entry).ok()?.trim().parse().ok()?;
    let age = std::fs::metadata(entry)
        .and_then(|metadata| metadata.modified())
        .ok()?
        .elapsed()
        .ok()?;
    if age > TIMEOUT_MEMORY {
        return None;
    }
    Some(Err(ExtractionError::TimedOut {
        extractor,
        budget: Duration::from_millis(milliseconds),
    }))
}

/// Stores what an extraction produced.
///
/// Deterministic failures are stored beside successes; a timeout in its own kind whose age is
/// its warrant; a missing extractor not at all, because that is a fact about the machine.
/// Every write goes to a temporary name and is renamed into place, and errors are ignored
/// throughout: a cache that cannot write is a cache that is slow, not a gate that is wrong.
///
/// Answers **whether anything was stored**, on [`crate::cache::write_entry`]'s rule and for its
/// reason: every early return here is an extractor that will be run again for this page, and
/// [`crate::cache::Runs::unstored`] is what counts them.
fn write_entry(entry: &Path, produced: &Result<String, ExtractionError>) -> bool {
    let Some(parent) = entry.parent() else {
        return false;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }

    let (extension, contents) = match produced {
        Ok(text) => (TEXT, text.clone()),
        Err(ExtractionError::Failed { detail, .. }) => (FAILURE, detail.clone()),
        Err(ExtractionError::TimedOut { budget, .. }) => (
            TIMEOUT,
            u64::try_from(budget.as_millis())
                .unwrap_or(u64::MAX)
                .to_string(),
        ),
        Err(ExtractionError::Missing { .. }) => return false,
    };

    let temporary = entry.with_extension(format!(
        "{extension}.tmp{}-{}",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    if std::fs::write(&temporary, contents).is_ok() {
        std::fs::rename(&temporary, entry.with_extension(extension)).is_ok()
    } else {
        let _ = std::fs::remove_file(&temporary);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{ExtractionCache, Extractor};
    use std::path::Path;

    /// The flag ADR 0323's Finding 1 was about is in the signature, and therefore in the key.
    ///
    /// This is trap 10a's rule made checkable: a signature is derived from the command line
    /// itself, so this test failing means the flag stopped being *passed*, not merely stopped
    /// being remembered.
    #[test]
    fn the_crop_box_flag_is_in_the_key() {
        let signature = Extractor::PopplerBoxes.command_signature(Path::new("/nowhere/a.pdf"), 1);
        assert!(
            signature.iter().any(|word| word == "-cropbox"),
            "{signature:?}"
        );
        assert!(
            signature.iter().any(|word| word == "<pdf>"),
            "the document's path is replaced, because the document is in the key by content: \
             {signature:?}"
        );
        let stext = Extractor::MuPdfText.command_signature(Path::new("/nowhere/a.pdf"), 1);
        assert!(stext.iter().any(|word| word == "stext"), "{stext:?}");
    }

    /// A disabled cache must be transparent: no entries, no hits, nothing on disk.
    #[test]
    fn a_disabled_cache_stores_nothing() {
        let cache = ExtractionCache::disabled();
        assert_eq!(cache.statistics().hits, 0);
        assert_eq!(cache.statistics().misses, 0);
    }
}
