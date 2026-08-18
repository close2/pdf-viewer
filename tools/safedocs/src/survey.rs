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
    /// Codes shown on page one that reached a glyph the font program describes as empty,
    /// while the page reported nothing.
    ///
    /// Separated from the count above in the four-hundred-and-thirty-fourth session, because
    /// only one of the two is a mark the reader loses. ADR 0270.
    pub codes_reaching_a_blank_glyph: usize,
    /// Whether *this process's* press budget decided the verdict rather than the document.
    ///
    /// `pdf_model::colour::MAX_PRESSES` is spent by whichever documents the process interpreted
    /// first, so a document naming the ninth distinct press is reported for a reason that
    /// belongs to eight other files. This survey walks its documents under `rayon`, so which
    /// ones those are is decided by the scheduler — which is why three runs of one unchanged
    /// tree over one unchanged directory printed 30, 36 and 33 such reports. A verdict carrying
    /// this is not a fact about the document and is counted apart from one that is. ADR 0416.
    pub press_beyond_this_process: bool,
    /// Whether every report this document made was one of those, so that it would have been
    /// [`Outcome::Complete`] in a process with a press slot to spare.
    ///
    /// The exact half of the flag above, and the one the summary subtracts. `3990833.pdf` is
    /// why the two are separate: it is incomplete for §11.4.4's non-isolated group whatever
    /// the press table holds, *and* met the budget on some runs, so counting it as the budget's
    /// moved the file-decided figure the subtraction exists to hold still.
    pub incomplete_only_beyond_this_process: bool,
}

impl Verdict {
    /// Whether the document drew with nothing reported.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self.outcome, Outcome::Complete)
    }

    /// Whether the verdict is the document's own rather than this process's press budget's.
    ///
    /// A document can be incomplete for both reasons at once — three of the crawl's 287
    /// press-naming documents are — and this says the report is not *only* the budget's.
    #[must_use]
    pub const fn is_the_documents_own(&self) -> bool {
        !self.press_beyond_this_process
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
    let mut noted = Noted::default();
    let outcome = read_and_draw(path, &mut noted);
    Verdict {
        name,
        outcome,
        taken: started.elapsed(),
        codes_without_a_glyph: noted.without_a_glyph,
        codes_reaching_a_blank_glyph: noted.blank,
        press_beyond_this_process: noted.press_beyond_this_process,
        incomplete_only_beyond_this_process: noted.reports > 0
            && noted.reports == noted.reports_beyond_this_process,
    }
}

/// What one document's page one said that its [`Outcome`] does not carry.
#[derive(Debug, Default)]
struct Noted {
    /// Codes that reached no glyph at all.
    without_a_glyph: usize,
    /// Codes that reached a glyph the program describes as empty.
    blank: usize,
    /// Whether this process's press budget decided the verdict. See [`Verdict`]'s field.
    press_beyond_this_process: bool,
    /// How many reports the page made at all.
    reports: usize,
    /// How many of them this process's press budget made.
    reports_beyond_this_process: usize,
}

/// The body of [`examine`], separated so that the timing wraps every path out of it.
fn read_and_draw(path: &Path, noted: &mut Noted) -> Outcome {
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
    // Unconditionally, unlike the two counts below it: what it says is *why* a document is
    // incomplete, so recording it only for the complete ones would record it nowhere.
    noted.press_beyond_this_process = interpretation.press_beyond_this_process;
    noted.reports = interpretation.unsupported.len();
    noted.reports_beyond_this_process = interpretation.reports_beyond_this_process;
    if interpretation.is_complete() {
        noted.without_a_glyph = interpretation.codes_without_a_glyph;
        noted.blank = interpretation.codes_reaching_a_blank_glyph;
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
