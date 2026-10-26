//! The mount's lifetime: which generation is being served, and what the mount has done since it
//! opened.
//!
//! RFC 0003 section 5.4's rule lives in [`Vfs::current`] and every public operation of the other
//! modules begins with it. The counters beside it ([`Vfs::generated`], [`Vfs::questions`],
//! [`Vfs::asked_twice`], [`Vfs::forgotten`]) are the instruments a gate reads to say that
//! something was not done again.

use std::sync::{Arc, Mutex};

use crate::commit::Provenance;
use crate::consent::{Consenting, Questions};
use crate::error::VfsError;
use crate::generation::Generation;
use crate::worker::{Answer, Query, Worker};
use crate::{Current, Serving, Vfs, mismatch};

impl Vfs {
    /// Which generation is being served, building it where nothing has been read yet.
    ///
    /// **This is RFC 0003 section 5.4's rule and every public operation begins with it**: the
    /// backing is asked for its key, and a key that differs from the one being served throws the
    /// generation away — the worker, the inventories and every cached output. A stale answer here
    /// is not a slow answer, it is a wrong one.
    ///
    /// # Errors
    ///
    /// [`VfsError::Backing`] where the file cannot be stat'd or opened, and [`VfsError::Worker`]
    /// where a worker cannot be started or the page count cannot be read.
    pub(crate) fn current(&self) -> Result<Arc<Current>, VfsError> {
        let mut held = self.held();
        self.current_in(&mut held)
    }

    /// The lock over everything a commit changes.
    pub(crate) fn held(&self) -> std::sync::MutexGuard<'_, Serving> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// [`Vfs::current`], with the lock already held — which is what a commit needs, because the
    /// old generation, the write and the new generation are one transaction.
    pub(crate) fn current_in(&self, held: &mut Serving) -> Result<Arc<Current>, VfsError> {
        let generation = self
            .backing
            .generation()
            .map_err(|error| VfsError::Backing {
                document: self.backing.describe(),
                error,
            })?;
        // **Two questions, and both have to be `yes`.** The key is RFC 0003 section 5.4's rule; the
        // second is RFC 0003 section 6's — a confined worker is killable by design, and one the
        // kernel has ended answers `false` for ever after. Asking it again would produce a second,
        // stranger error about a closed descriptor, so the generation is thrown away and the next
        // operation starts a fresh worker over the same file. `InProcess` is always alive, so this
        // costs the unconfined path an atomic load.
        if let Some(current) = held.current.as_ref()
            && current.generation == generation
            && current.worker.is_alive()
        {
            return Ok(Arc::clone(current));
        }
        let bytes = self.backing.bytes().map_err(|error| VfsError::Backing {
            document: self.backing.describe(),
            error,
        })?;
        let worker = Consenting::new(
            self.workers.spawn(
                bytes,
                self.password.as_ref(),
                self.config.policy,
                self.config.budget,
            )?,
            Arc::clone(&self.questions),
        );
        let pages = match worker.ask(&Query::PageCount)? {
            Answer::Count(pages) => pages,
            other => return Err(VfsError::Worker(mismatch(&other, "count"))),
        };
        // Before anything of the new generation is answered, everything of the old one is
        // forgotten. In this order, so that no window exists in which a caller could be handed
        // an entry keyed by a generation the document no longer has.
        self.cache.retain(generation);
        // Whose edit this is. `ours` is set by [`Vfs::flush`] under this same lock, so the only
        // way a generation is `Ours` is that this tree wrote the key that produced it; a key
        // that arrived any other way is somebody else's, whether or not a write of ours ever
        // happened.
        let provenance = if held.current.is_none() && held.ours.is_none() {
            Provenance::Opened
        } else if held.ours.take() == Some(generation) {
            Provenance::Ours
        } else {
            held.ours = None;
            held.foreign_edits = held.foreign_edits.saturating_add(1);
            Provenance::Foreign
        };
        let current = Arc::new(Current {
            generation,
            worker,
            pages,
            attachments: Mutex::new(None),
            metadata: Mutex::new(None),
            provenance,
        });
        held.current = Some(Arc::clone(&current));
        Ok(current)
    }

    /// How many virtual files this tree has produced the bytes of, since it opened.
    ///
    /// Monotonic, and the counterpart to what the cache holds: see [`Vfs::generated`]'s field.
    #[must_use]
    pub fn generated(&self) -> u64 {
        self.generated.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// What this mount has asked its workers, over every generation it has served.
    ///
    /// **The cost instrument**, and [`Questions::repeated`] is the number a floor is made of.
    /// It asks nothing of any worker and builds no generation: a mount that has answered nothing
    /// yet has asked nothing, and says so.
    #[must_use]
    pub fn questions(&self) -> Questions {
        self.questions.questions()
    }

    /// Which subjects the worker was asked about twice after answering, each named once.
    ///
    /// What [`Questions::repeated`] counts, said rather than totalled — so a gate that fails on
    /// the count can name the generator that ran twice instead of leaving the next round to find
    /// it. Bounded by the layout, like the map behind it.
    #[must_use]
    pub fn asked_twice(&self) -> Vec<String> {
        self.questions
            .twice
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// How many generated outputs the cache has stopped holding within a generation.
    ///
    /// The ceiling [`Questions::repeated`] is honestly allowed: `Cache`'s own field says why, and
    /// `doc/todo/58` §5 is where the floor built on the pair is recorded.
    #[must_use]
    pub fn forgotten(&self) -> u64 {
        self.cache.forgotten()
    }

    /// Whose edit produced the generation being served.
    ///
    /// # Errors
    ///
    /// As [`Vfs::list`].
    pub fn provenance(&self) -> Result<Provenance, VfsError> {
        Ok(self.current()?.provenance)
    }

    /// The generation being served right now, for a face that wants to report it.
    ///
    /// # Errors
    ///
    /// As [`Vfs::list`].
    pub fn generation(&self) -> Result<Generation, VfsError> {
        Ok(self.current()?.generation)
    }

    /// How many pages the document has.
    ///
    /// # Errors
    ///
    /// As [`Vfs::list`].
    pub fn pages(&self) -> Result<usize, VfsError> {
        Ok(self.current()?.pages)
    }
}
