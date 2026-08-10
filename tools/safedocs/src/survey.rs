//! This tree's corpus gate, run over documents that are not the corpus.
//!
//! # Why this is a copy of `crates/pdf-model/tests/corpus.rs` rather than a call into it
//!
//! That file is a *gate*: its counts are a ratchet with a constant per category, and every
//! one of those constants is an argument about the 974 pdf.js documents. Pointing it at a new
//! population would move every constant at once and the numbers would then be about nothing.
//! `doc/todo/03` says this outright — a new corpus is a separate command until it has earned
//! a place — so what is shared is the *shape* of the report and not the ratchet.
//!
//! What the shape is, and it is the gate's own five questions: does the document open, does
//! it reach a first page, does it draw with nothing reported, what does it report, and does
//! it take longer than the per-document budget.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::{Document, SyntaxError};
use rayon::prelude::{
    IndexedParallelIterator as _, IntoParallelRefIterator as _, ParallelIterator as _,
};
use render_cpu::CpuRasterizer;

/// Pixel budget per page, the same one `tests/corpus.rs` uses.
const PIXEL_BUDGET: u64 = 64 << 20;

/// What `tests/corpus.rs` allows one document, and what a report here means by "slow".
pub const PER_DOCUMENT_BUDGET: Duration = Duration::from_secs(30);

/// What happened to one document.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// Opened, reached a first page, and drew with nothing reported.
    Complete,
    /// Drew, and said what it could not draw.
    Incomplete(String),
    /// §7.6.4.1's default user password was refused; a person would be asked for one.
    Locked,
    /// Encrypted in a way §7.6 states and this reader does not implement.
    UnreadableEncryption(String),
    /// Could not be opened at all.
    Unopenable(String),
    /// Opened, and has no reachable first page.
    Pageless,
    /// Could not be read off the disk.
    Unreadable(String),
}

/// One document's line in the report.
#[derive(Debug, Clone)]
pub struct Verdict {
    /// The document's file name.
    pub name: String,
    /// What happened.
    pub outcome: Outcome,
    /// How long it took to open, interpret and rasterise.
    pub taken: Duration,
    /// Codes shown on page one that reached no glyph while the page reported nothing.
    pub codes_without_a_glyph: usize,
}

impl Verdict {
    /// Whether the document drew with nothing reported.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self.outcome, Outcome::Complete)
    }

    /// Whether it took longer than [`PER_DOCUMENT_BUDGET`].
    #[must_use]
    pub fn is_slow(&self) -> bool {
        self.taken > PER_DOCUMENT_BUDGET
    }
}

/// Runs every document in `files` and returns one verdict apiece, in the input's order.
///
/// Rasterisation is included for the reason `tests/corpus.rs` gives: it is where a display
/// list with impossible geometry would surface, and the interpreter is capable of making one.
#[must_use]
pub fn survey(files: &[PathBuf]) -> Vec<Verdict> {
    let verdicts: Mutex<Vec<(usize, Verdict)>> = Mutex::new(Vec::with_capacity(files.len()));
    files.par_iter().enumerate().for_each(|(index, path)| {
        let verdict = examine(path);
        if let Ok(mut collected) = verdicts.lock() {
            collected.push((index, verdict));
        }
    });
    let mut collected = verdicts.into_inner().unwrap_or_else(|poisoned| {
        // A poisoned lock means one examination panicked, which is itself the finding; what
        // survives is every other document's verdict, and losing them as well would hide it.
        poisoned.into_inner()
    });
    collected.sort_by_key(|(index, _)| *index);
    collected.into_iter().map(|(_, verdict)| verdict).collect()
}

/// Opens, interprets and rasterises one document's first page.
fn examine(path: &Path) -> Verdict {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let started = Instant::now();
    let mut codes_without_a_glyph = 0;
    let outcome = read_and_draw(path, &mut codes_without_a_glyph);
    Verdict {
        name,
        outcome,
        taken: started.elapsed(),
        codes_without_a_glyph,
    }
}

/// The body of [`examine`], separated so that the timing wraps every path out of it.
fn read_and_draw(path: &Path, codes_without_a_glyph: &mut usize) -> Outcome {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => return Outcome::Unreadable(error.to_string()),
    };
    let document = match Document::open(bytes) {
        Ok(document) => document,
        Err(SyntaxError::PasswordRequired) => return Outcome::Locked,
        Err(error @ SyntaxError::UnsupportedEncryption { .. }) => {
            return Outcome::UnreadableEncryption(error.to_string());
        }
        Err(error) => return Outcome::Unopenable(error.to_string()),
    };
    let Some(page) = pdf_model::Pages::new(&document).get(0) else {
        return Outcome::Pageless;
    };

    let interpretation = pdf_model::interpret(&document, &page);
    if interpretation.is_complete() {
        *codes_without_a_glyph = interpretation.codes_without_a_glyph;
    }
    if let Ok(target) = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET) {
        // Discarded deliberately: an unsupported command is already counted below. What this
        // call proves is that the rasteriser returns rather than panicking or looping.
        drop(CpuRasterizer::new().rasterize(&interpretation.display_list, target));
    }
    if interpretation.is_complete() {
        Outcome::Complete
    } else {
        Outcome::Incomplete(format!("{:?}", interpretation.unsupported))
    }
}

/// Whether the sandboxed image decoder is available, and why not when it is not.
///
/// `tests/corpus.rs` *fails* on this, because a missing worker would silently turn 152
/// documents' images into reports and move its ratchets. Nothing here is ratcheted, so this
/// reports instead — but it still has to be said, for the same reason: a survey run without
/// the worker is a different measurement wearing the same words.
///
/// # Errors
///
/// The confinement's own sentence, when the worker cannot be started or confined.
pub fn sandbox_available() -> Result<(), String> {
    pdf_sandbox::Sandbox::shared()
        .confinement()
        .map(drop)
        .map_err(|error| error.to_string())
}
