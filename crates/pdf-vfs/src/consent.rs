//! The one seam every question this crate puts to a worker crosses, and what it counts and asks.
//!
//! [`Consenting`] wraps a generation's worker so that `CLAUDE.md` principle 3's consultation
//! is spent in one place and every question is counted in one place; [`Questions`] is what the
//! counts say about a mount. The split of the counting into [`Counted`] (a generation's) and
//! [`Totals`] (the mount's) is argued on those types.

use std::sync::{Arc, Mutex};

use pdf_transform::Operation;

use crate::worker::{Answer, Query, Worker, WorkerError};

/// A worker with `CLAUDE.md` principle 3's *ask* level held beside it.
///
/// **Where the two round trips of ADR 0874 meet.** [`crate::Vfs::consult`] puts the question to the
/// worker and records that it was asked; a face puts it to a person; [`crate::Vfs::answer`] records the
/// yes; and the very next query that performs the operation spends it, once, by going through
/// [`Worker::ask_consented`] instead of [`Worker::ask`].
///
/// **Here rather than on [`crate::Vfs`], and that is the design rather than an economy.** A consent is
/// about one document as it stood when the question was put, so it belongs to the generation:
/// a `Current` thrown away because somebody else edited the file underneath the mount takes the
/// answer with it, and nothing has to remember to. And every question this crate asks goes
/// through `current.worker`, so putting the spend here is what makes it impossible for a call
/// site to forget — there are fifteen of them and no list of which ones matter.
#[derive(Debug)]
pub(crate) struct Consenting {
    /// What actually answers.
    inner: Box<dyn Worker>,
    /// The question outstanding and the answer standing.
    asked: Mutex<Asked>,
    /// What this generation's worker has been asked about already.
    ///
    /// **Here for the same reason the consent is** — every question this crate asks goes through
    /// one of the two methods below, so a counter here cannot be forgotten by a call site — and
    /// it is [`Counted`]'s doc comment that says why a counter of *questions* is the one a cost
    /// property needs.
    counted: Mutex<Counted>,
    /// Where the counts go, which outlives this generation.
    totals: Arc<Totals>,
}

/// What one generation's worker has been asked.
///
/// **The instrument [`crate::Vfs::generated`] is not**, and the difference is
/// `doc/traps/instruments-and-reports.md`'s trap 33. That counter says how many virtual files
/// this tree has produced the bytes of, which is the right question for ADR 0865 section 3's size notes
/// and is blind to work done to *answer* a question: it read **1** while a mount re-ran a page's
/// image extraction on every one of twenty thousand `stat`s, because the bytes had been produced
/// once and the extraction was being run to validate a name (ADR 0886). A property about how
/// often a generator runs has to count the generator running, and [`Worker::ask`] is where it
/// runs.
///
/// Bounded by the layout rather than by the document's bytes: the set below holds one short
/// string per [`Query::subject`], which is a page ordinal, a resolution or a name.
#[derive(Debug, Default)]
struct Counted {
    /// What the last answer about each subject was. See [`Query::subject`] for what is not in
    /// here and why.
    ///
    /// Per generation, because [`Consenting`] is: a question about a document that has been
    /// replaced underneath the mount is a new question rather than a repeat (RFC 0003 section 5.4).
    subjects: std::collections::HashMap<String, Standing>,
}

/// What a whole mount has asked, across every generation it has served.
///
/// **The totals are the mount's and the repeat detection is the generation's**, and the split is
/// the design: a face wants to know what its tree has cost since it was mounted, while "asked
/// twice" is only meaningful about one document. So [`Counted`] lives in the worker that is made
/// per generation and adds into this, which lives in the [`crate::Vfs`].
#[derive(Debug, Default)]
pub(crate) struct Totals {
    /// Questions put to the workers, of every shape.
    asked: std::sync::atomic::AtomicU64,
    /// Questions put about a subject the generation's worker had already answered.
    repeated: std::sync::atomic::AtomicU64,
    /// Questions put about a subject whose last answer was a refusal.
    reasked_after_a_refusal: std::sync::atomic::AtomicU64,
    /// The subjects [`Self::repeated`] counted, each named once, so that a gate can say which
    /// generator ran twice rather than only how often. Bounded by the layout.
    pub(crate) twice: Mutex<Vec<String>>,
}

impl Totals {
    /// One more question, of every shape.
    fn asked(&self) {
        self.asked
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// One more question about a subject already answered, named.
    fn repeated(&self, subject: &str) {
        self.repeated
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut twice = self
            .twice
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !twice.iter().any(|had| had == subject) {
            twice.push(subject.to_owned());
        }
    }

    /// One more question about a subject whose last answer was a refusal.
    fn reasked(&self) {
        self.reasked_after_a_refusal
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// What they add up to.
    pub(crate) fn questions(&self) -> Questions {
        Questions {
            asked: self.asked.load(std::sync::atomic::Ordering::Relaxed),
            repeated: self.repeated.load(std::sync::atomic::Ordering::Relaxed),
            reasked_after_a_refusal: self
                .reasked_after_a_refusal
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// What the last answer about one subject was.
///
/// The distinction is the difference between a cache that failed and a cache that had nothing to
/// hold: **a refusal produces no bytes**, so the tree cannot remember it and the next question
/// about that subject necessarily runs the generator again. Counting that as a repeat would make
/// the floor fire on every document with an unsupported codec on one page, which is a refusal
/// working exactly as trap 5 asks it to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// The worker answered, so there was something to remember.
    Answered,
    /// The worker refused, so there was not.
    Refused,
}

/// What a mount has asked the worker serving the generation it is on.
///
/// Per generation and not per mount, because a worker is made per generation and a question
/// about a document that has been replaced is a new question rather than a repeat
/// (`Counted`, RFC 0003 section 5.4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Questions {
    /// Questions put to the worker, of every shape.
    pub asked: u64,
    /// Questions put to it about a subject it had already **answered**.
    ///
    /// **The number a cost floor is made of.** A repeat is honest exactly where the answer was
    /// forgotten in between — evicted from the cache, or refused for being larger than the whole
    /// budget — so [`crate::Vfs::forgotten`] is the ceiling this may be held to, and a repeat above that
    /// ceiling is a generator run to answer a question rather than to produce bytes.
    pub repeated: u64,
    /// Questions put to it about a subject whose last answer was a **refusal**.
    ///
    /// Not part of the floor above, and it is a shortfall rather than a defect: a refusal
    /// produces no bytes, so there is nothing for the cache to hold and the next question about
    /// that subject runs the generator again — a page whose codec this reader does not have costs
    /// its refusal on every `stat` and every `open`. `doc/todo/58` §5 records it; measured over
    /// the corpus in ADR 0895 section 2.
    pub reasked_after_a_refusal: u64,
}

/// One question put to a person, and one answer held until it is spent.
///
/// One question outstanding at a time and a second replaces it — `viewer_core`'s own rule for
/// `Event::Asking`, so that the two boundaries in this tree behave the same way.
#[derive(Debug, Default)]
struct Asked {
    /// The operation [`crate::Vfs::consult`] last answered `Consulted::Ask` about, until it is
    /// answered.
    outstanding: Option<Operation>,
    /// The operation a person said yes to, until the query that performs it is asked.
    consented: Option<Operation>,
}

impl Consenting {
    /// A worker with nothing asked and nothing answered.
    pub(crate) fn new(inner: Box<dyn Worker>, totals: Arc<Totals>) -> Self {
        Self {
            inner,
            asked: Mutex::new(Asked::default()),
            counted: Mutex::new(Counted::default()),
            totals,
        }
    }

    /// Counts one question, before it is put.
    ///
    /// Counted whatever the answer turns out to be: what this measures is the generator being
    /// *run*, and a run that ends in a refusal cost what a run costs.
    ///
    /// **It can under-count and cannot over-count, and the direction is the point.** The standing
    /// is read here and written when the answer comes back, so two threads asking about one
    /// subject at the same moment both see nothing and neither counts a repeat — a real double
    /// run this misses. The floor built on the number is therefore conservative: it can fail to
    /// fire, and it cannot fire on work that did not happen. Holding the lock across the question
    /// would fix the count by serialising every reader of the mount, which is a worse thing to be
    /// than approximate.
    fn counting(&self, query: &Query) {
        let Some(subject) = query.subject() else {
            return;
        };
        let standing = self
            .counted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .subjects
            .get(&subject)
            .copied();
        self.totals.asked();
        match standing {
            None => {}
            Some(Standing::Answered) => self.totals.repeated(&subject),
            Some(Standing::Refused) => self.totals.reasked(),
        }
    }

    /// Records what the answer turned out to be — [`Standing`] is what the difference decides.
    fn counted(&self, query: &Query, answered: bool) {
        let Some(subject) = query.subject() else {
            return;
        };
        let standing = if answered {
            Standing::Answered
        } else {
            Standing::Refused
        };
        self.counted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .subjects
            .insert(subject, standing);
    }

    /// The lock, poison and all: a panic in a caller must not make this tree unusable, and the
    /// worst a poisoned answer can be is one consent spent or not spent.
    fn held(&self) -> std::sync::MutexGuard<'_, Asked> {
        self.asked
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Records that a person is being asked about `operation`.
    pub(crate) fn putting(&self, operation: Operation) {
        let mut held = self.held();
        held.outstanding = Some(operation);
    }

    /// Records that nothing is outstanding — a consultation that came back anything but *ask*.
    pub(crate) fn nothing_outstanding(&self) {
        let mut held = self.held();
        held.outstanding = None;
    }

    /// The person's answer, and whether there was a question for it to be an answer to.
    ///
    /// A `no` forgets the question and says nothing further, which is what a declined dialogue
    /// means everywhere else in this tree: the operation is simply not done.
    pub(crate) fn answered(&self, proceed: bool) -> bool {
        let mut held = self.held();
        let Some(operation) = held.outstanding.take() else {
            return false;
        };
        held.consented = proceed.then_some(operation);
        true
    }
}

impl Worker for Consenting {
    fn ask(&self, query: &Query) -> Result<Answer, WorkerError> {
        self.counting(query);
        // Spent, or not, before the question is asked — and spent *once*: a yes to deleting one
        // page is not a yes to deleting every page after it.
        let spend = {
            let mut held = self.held();
            match (held.consented, query.operation()) {
                (Some(given), Some(wanted)) if given == wanted => {
                    held.consented = None;
                    true
                }
                _ => false,
            }
        };
        let answer = if spend {
            self.inner.ask_consented(query)
        } else {
            self.inner.ask(query)
        };
        self.counted(query, answer.is_ok());
        answer
    }

    fn ask_consented(&self, query: &Query) -> Result<Answer, WorkerError> {
        self.counting(query);
        let answer = self.inner.ask_consented(query);
        self.counted(query, answer.is_ok());
        answer
    }

    fn is_alive(&self) -> bool {
        self.inner.is_alive()
    }
}
