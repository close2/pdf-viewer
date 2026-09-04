//! The generation cache: content addressed by (generation key, path), bounded in bytes.
//!
//! RFC 0003 section 5.5 asks for exactly this — "[c]ache: content-addressed by (generation key,
//! path), bounded (memory plus optional disk bound, both explicit budgets in principle-3 style),
//! shared between the two faces through the core" — and section 5.5's *other* rule is what makes
//! the cache load-bearing rather than an optimisation:
//!
//! **No virtual file is stat'd before it is generated.** A FUSE `stat` must state a size before
//! the bytes exist and the kernel clamps reads at the stated size, so an estimate silently
//! truncates the file for every reader (the ffmpegfs lesson, RFC 0003 section 2). So `stat`
//! generates, and without a cache every `stat`-then-`open`-then-`read` — which is what every
//! `cp` is — would generate the same page twice.
//!
//! # The bound, and what it is not
//!
//! One explicit ceiling in bytes, evicting least-recently-used entries until the new one fits;
//! an entry larger than the whole budget is answered and **not** stored, because refusing to
//! answer would make the budget a limit on what the mount can serve rather than on what it
//! remembers. That is the distinction `doc/todo/10` draws between a bound that caps size and a
//! bound that guards against a bomb: this is neither, it is a memory budget, and the guard
//! against a decompression bomb is `pdf_syntax::Limits` inside the worker where the bytes are.
//!
//! There is no disk half yet, and RFC 0003 section 5.5 offers it as optional. Not building it is
//! stated in `crate::Vfs::shortfalls` rather than left to be discovered.
//!
//! # Two things outlive the bytes, and neither is an estimate
//!
//! The budget above is on *content*, and content is not the only thing a generation costs. Two
//! notes are therefore kept past eviction, each derived from a run that actually happened:
//!
//! - **a length** ([`Held::sizes`]), so that a second `stat` is free — round 911's finding; and
//! - **a directory's own names** ([`Held::inventories`]), so that a path under `images/NNNN/` can
//!   be validated, listed and `stat`ed without re-running the extraction that named it — round
//!   923's.
//!
//! Both are bounded by the document rather than by a number, and [`Cache::retain`] drops them with
//! everything else when the generation changes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::generation::Generation;

/// What the cache is keyed by.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    /// Which generation of the document produced these bytes.
    generation: GenerationKey,
    /// The canonical path.
    path: String,
}

/// [`Generation`] as a hashable key.
///
/// `Generation` is `Copy` and `Eq` but not `Hash`, because it is a *value* a face compares and
/// nothing else; this is the one place it is used as a map key, so the conversion is here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GenerationKey {
    /// Modification time, in nanoseconds.
    modified_nanos: Option<i128>,
    /// Length in bytes.
    size: u64,
    /// §7.5.5's last `startxref` offset.
    startxref: Option<u64>,
}

impl From<Generation> for GenerationKey {
    fn from(generation: Generation) -> Self {
        Self {
            modified_nanos: generation.modified_nanos,
            size: generation.size,
            startxref: generation.startxref,
        }
    }
}

/// One cached output.
#[derive(Debug)]
struct Entry {
    /// The bytes, shared with every reader that holds them open.
    bytes: Arc<[u8]>,
    /// When it was last touched, in this cache's own counter.
    used: u64,
}

/// The cache.
#[derive(Debug)]
pub struct Cache {
    /// Everything in it.
    entries: Mutex<Held>,
    /// The ceiling in bytes.
    budget: usize,
}

/// The mutable half, under one lock.
#[derive(Debug, Default)]
struct Held {
    /// The entries.
    entries: HashMap<Key, Entry>,
    /// How many bytes they add up to.
    bytes: usize,
    /// A monotonic counter standing in for a clock: the cache has no clock, by the same rule
    /// `pdf-transform` has none (RFC 0002 section 5's second rule).
    tick: u64,
    /// How long each generated output turned out to be, kept after its bytes are evicted.
    ///
    /// **This is what makes a second `ls -l` free, and a mount by hand is what asked for it.**
    /// RFC 0003 section 5.5 makes a `stat` generate, because "an under-estimate silently
    /// truncates a page" — and a size that came off real bytes is not an estimate. A directory
    /// larger than the budget therefore used to cost its whole generation *on every listing*: on
    /// ISO 32000-2's own 1023 pages, `ls -l pages/` took 2 min 45 s the first time and **4 min
    /// 03 s the second**, because 1023 pieces of about 1.8 MB do not fit in the budget and every
    /// entry had been evicted by the time the listing came round again (round 911).
    ///
    /// Bounded by the document rather than by a number: one note per path per generation, and
    /// [`Cache::retain`] drops every other generation's along with its bytes. A note is a path
    /// and eight bytes, so a scanned book of ten thousand pages costs a few megabytes.
    sizes: HashMap<Key, u64>,
    /// The names one *directory's* generator produced, where producing them is the expensive part.
    ///
    /// **The second kind of entry `doc/todo/58` §5 said this cache did not have**, and round 923
    /// measured what its absence cost. `images/NNNN/` is the one directory in RFC 0003 section 4
    /// whose listing *is* an extraction's own output names — deliberately, so that a listing and a
    /// read cannot disagree — and every path under it is therefore validated by running that
    /// extraction: `Vfs::stat`, `Vfs::open` and `Vfs::list` alike. On
    /// `tika-issue-tracker/batch1/PDFBOX/PDFBOX-186-0.pdf`, which states 10 084 images on one page,
    /// that is 176 ms a question against outputs of 352 bytes each — so twenty thousand questions
    /// took over an hour while the *bytes* sat in the cache untouched, `Vfs::generated` stuck at one.
    ///
    /// Names rather than bytes, so the byte budget is untouched and this is bounded by the document:
    /// one entry per directory per generation, dropped by [`Cache::retain`] with everything else.
    inventories: HashMap<Key, Arc<[String]>>,
    /// How many entries this cache has stopped holding *within* a generation.
    ///
    /// **The only honest explanation for a generator being run twice**, and that is what it is
    /// for. [`crate::Vfs::questions`] counts a question the worker was asked about a subject it
    /// had already answered; every such repeat is preceded by the entry for it leaving this
    /// cache, either evicted to make room or refused for being larger than the whole budget. A
    /// repeat with no forgetting behind it is work done to answer a question rather than to
    /// produce bytes, which is `doc/traps/instruments-and-reports.md`'s trap 33 and is what ADR
    /// 0886 found after it had cost a hundredfold for four sessions.
    ///
    /// [`Cache::retain`]'s drops are deliberately **not** counted: those belong to a generation
    /// the document no longer has, and a question about a generation that is gone is a new
    /// question rather than a repeat.
    forgotten: u64,
}

impl Cache {
    /// A cache holding at most `budget` bytes of generated content.
    #[must_use]
    pub fn new(budget: usize) -> Self {
        Self {
            entries: Mutex::new(Held::default()),
            budget,
        }
    }

    /// The bytes at `path` in `generation`, where they have been generated and not evicted.
    #[must_use]
    pub fn get(&self, generation: Generation, path: &str) -> Option<Arc<[u8]>> {
        let mut held = self.lock();
        held.tick = held.tick.saturating_add(1);
        let tick = held.tick;
        let entry = held.entries.get_mut(&Key {
            generation: generation.into(),
            path: path.to_owned(),
        })?;
        entry.used = tick;
        Some(Arc::clone(&entry.bytes))
    }

    /// Remembers `bytes` as the content of `path` in `generation`, and hands them back.
    ///
    /// Evicts least-recently-used entries until the new one fits. An entry larger than the whole
    /// budget is handed back without being stored — see the module comment.
    pub fn put(&self, generation: Generation, path: &str, bytes: Vec<u8>) -> Arc<[u8]> {
        let shared: Arc<[u8]> = Arc::from(bytes.into_boxed_slice());
        let mut held = self.lock();
        let key = Key {
            generation: generation.into(),
            path: path.to_owned(),
        };
        // The length is remembered whatever happens to the bytes, **including for an entry too
        // large to store at all** — which is the case the first version of this got wrong and a
        // gate caught (round 911, trap 13): a page out of a real document is often bigger than a
        // small budget, and those are exactly the files whose `stat` is expensive.
        held.sizes
            .insert(key.clone(), u64::try_from(shared.len()).unwrap_or(u64::MAX));
        if shared.len() > self.budget {
            // Answered and not stored, so every later question about this path runs the
            // generator again: one forgetting per refusal, which is what makes the count an
            // upper bound on the repeats it can explain.
            held.forgotten = held.forgotten.saturating_add(1);
            return shared;
        }
        held.tick = held.tick.saturating_add(1);
        let tick = held.tick;
        if let Some(previous) = held.entries.remove(&key) {
            held.bytes = held.bytes.saturating_sub(previous.bytes.len());
        }
        while held.bytes.saturating_add(shared.len()) > self.budget {
            let Some(oldest) = held
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            if let Some(removed) = held.entries.remove(&oldest) {
                held.bytes = held.bytes.saturating_sub(removed.bytes.len());
                held.forgotten = held.forgotten.saturating_add(1);
            }
        }
        held.bytes = held.bytes.saturating_add(shared.len());
        held.entries.insert(
            key,
            Entry {
                bytes: Arc::clone(&shared),
                used: tick,
            },
        );
        shared
    }

    /// How long the output at this path turned out to be, where it has ever been generated.
    ///
    /// [`Held::sizes`] says why this outlives the bytes. A `None` means the path has not been
    /// generated in this generation at all, which is the only case that has to do the work.
    #[must_use]
    pub fn size_of(&self, generation: Generation, path: &str) -> Option<u64> {
        self.lock()
            .sizes
            .get(&Key {
                generation: generation.into(),
                path: path.to_owned(),
            })
            .copied()
    }

    /// The names this directory's generator produced, where it has been run in this generation.
    ///
    /// [`Held::inventories`] says why a name outlives the bytes it names, and why only a
    /// directory whose listing costs a generation has one.
    #[must_use]
    pub fn inventory(&self, generation: Generation, path: &str) -> Option<Arc<[String]>> {
        self.lock()
            .inventories
            .get(&Key {
                generation: generation.into(),
                path: path.to_owned(),
            })
            .map(Arc::clone)
    }

    /// Remembers that this directory's generator produced exactly these names.
    ///
    /// The caller has just run the generator, so these are the run's own outputs and not a guess
    /// — the same rule [`Cache::put`]'s size note obeys, and for the same reason.
    pub fn note_inventory(&self, generation: Generation, path: &str, names: Arc<[String]>) {
        self.lock().inventories.insert(
            Key {
                generation: generation.into(),
                path: path.to_owned(),
            },
            names,
        );
    }

    /// Forgets everything that is not `generation`'s.
    ///
    /// RFC 0003 section 5.4's rule in one line: "a changed key rebuilds the virtual tree". An
    /// entry from a generation the document no longer has is not stale data to be revalidated,
    /// it is an answer to a question about a document that is gone.
    pub fn retain(&self, generation: Generation) {
        let keep = GenerationKey::from(generation);
        let mut held = self.lock();
        let mut dropped = 0_usize;
        held.entries.retain(|key, entry| {
            let kept = key.generation == keep;
            if !kept {
                dropped = dropped.saturating_add(entry.bytes.len());
            }
            kept
        });
        held.bytes = held.bytes.saturating_sub(dropped);
        held.sizes.retain(|key, _| key.generation == keep);
        held.inventories.retain(|key, _| key.generation == keep);
    }

    /// How many bytes are held.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.lock().bytes
    }

    /// How many entries this cache has stopped holding within a generation.
    ///
    /// [`Held::forgotten`] says what the number is for: it is the ceiling on how many times a
    /// generator can honestly be run twice for the same subject.
    #[must_use]
    pub fn forgotten(&self) -> u64 {
        self.lock().forgotten
    }

    /// The lock, with a poisoned one taken anyway: a panic in another thread's `put` cannot make
    /// the mount stop answering, and the invariant a poisoned lock protects here is a byte count
    /// that [`Self::retain`] recomputes from the entries themselves.
    fn lock(&self) -> std::sync::MutexGuard<'_, Held> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::Cache;
    use crate::generation::Generation;

    fn generation(size: u64) -> Generation {
        Generation {
            modified_nanos: Some(0),
            size,
            startxref: Some(9),
        }
    }

    #[test]
    fn a_generation_is_part_of_the_key() {
        let cache = Cache::new(1024);
        cache.put(generation(10), "/text/0001.txt", b"one".to_vec());
        assert!(cache.get(generation(10), "/text/0001.txt").is_some());
        assert!(cache.get(generation(11), "/text/0001.txt").is_none());
    }

    #[test]
    fn the_budget_evicts_the_least_recently_used() {
        let cache = Cache::new(8);
        cache.put(generation(1), "/a", vec![0; 4]);
        cache.put(generation(1), "/b", vec![0; 4]);
        assert!(cache.get(generation(1), "/a").is_some());
        cache.put(generation(1), "/c", vec![0; 4]);
        assert!(cache.get(generation(1), "/b").is_none());
        assert!(cache.get(generation(1), "/a").is_some());
        assert!(cache.get(generation(1), "/c").is_some());
        assert!(cache.bytes() <= 8);
    }

    #[test]
    fn an_entry_larger_than_the_budget_is_answered_and_not_kept() {
        let cache = Cache::new(4);
        let bytes = cache.put(generation(1), "/big", vec![7; 16]);
        assert_eq!(bytes.len(), 16);
        assert!(cache.get(generation(1), "/big").is_none());
        assert_eq!(cache.bytes(), 0);
    }

    #[test]
    fn a_directorys_names_outlive_the_bytes_and_the_generation_takes_them() {
        let cache = Cache::new(4);
        let names: std::sync::Arc<[String]> =
            vec![String::from("01.jpg"), String::from("02.jpg")].into();
        cache.note_inventory(generation(1), "/images/0001", std::sync::Arc::clone(&names));
        // The bytes of one output are far past this budget and are therefore not kept; the
        // directory's names are a different note and are.
        cache.put(generation(1), "/images/0001/01.jpg", vec![7; 16]);
        assert!(cache.get(generation(1), "/images/0001/01.jpg").is_none());
        assert_eq!(
            cache.inventory(generation(1), "/images/0001").as_deref(),
            Some(&names[..])
        );
        assert!(cache.inventory(generation(2), "/images/0001").is_none());
        cache.retain(generation(2));
        assert!(cache.inventory(generation(1), "/images/0001").is_none());
    }

    #[test]
    fn what_the_cache_stops_holding_is_counted_because_it_is_what_explains_a_repeat() {
        let cache = Cache::new(8);
        assert_eq!(cache.forgotten(), 0);
        cache.put(generation(1), "/a", vec![0; 4]);
        cache.put(generation(1), "/b", vec![0; 4]);
        assert_eq!(cache.forgotten(), 0, "nothing has been dropped yet");
        cache.put(generation(1), "/c", vec![0; 4]);
        assert_eq!(cache.forgotten(), 1, "one entry was evicted to make room");
        cache.put(generation(1), "/big", vec![0; 16]);
        assert_eq!(
            cache.forgotten(),
            2,
            "an entry past the budget is stored nowhere"
        );
        // A generation the document no longer has is not a forgetting: a question about it is a
        // new question rather than a repeat, which is why `retain` does not count.
        cache.retain(generation(2));
        assert_eq!(cache.forgotten(), 2);
    }

    #[test]
    fn retaining_one_generation_forgets_the_others() {
        let cache = Cache::new(1024);
        cache.put(generation(1), "/a", vec![0; 4]);
        cache.put(generation(2), "/a", vec![0; 4]);
        cache.retain(generation(2));
        assert!(cache.get(generation(1), "/a").is_none());
        assert!(cache.get(generation(2), "/a").is_some());
        assert_eq!(cache.bytes(), 4);
    }
}
