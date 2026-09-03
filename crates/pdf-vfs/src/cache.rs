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
        if shared.len() > self.budget {
            return shared;
        }
        held.tick = held.tick.saturating_add(1);
        let tick = held.tick;
        let key = Key {
            generation: generation.into(),
            path: path.to_owned(),
        };
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
    }

    /// How many bytes are held.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.lock().bytes
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
