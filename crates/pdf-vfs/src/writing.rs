//! The writes that land as §7.5.6's incremental update: a POSIX write's `create`, `write_at`,
//! `flush` and `release`, and the whole-file `write`, `remove`, `create_directory` and `rename`.
//!
//! The transaction's state is [`crate::commit`]'s; what is here is each verb's path through the
//! layout, and [`Vfs::flush`], which holds the mount's one lock across reading the generation,
//! computing the update, replacing the file and building the next generation.

use std::sync::Arc;

use crate::commit::{Abandoned, Committed, Pending, Staged, StagedId};
use crate::error::{Refused, VfsError};
use crate::inventory::{attachments, locate_in, safe_name, spells};
use crate::layout::{Reason, Route, Write};
use crate::path;
use crate::worker::{Answer, Query, Worker, WorkerError};
use crate::{Current, Handle, Serving, Verb, Vfs, canonical_path, mismatch, refusal_for};

impl Vfs {
    /// Copying a file into the tree, whole, in one call.
    ///
    /// The transaction of [`Vfs::create`], [`Vfs::write_at`] and [`Vfs::flush`] with nothing
    /// between them — what a KIO `put` is, because "KIO's verb is already transactional"
    /// (RFC 0003 section 5.4), and what a test writes.
    ///
    /// # Errors
    ///
    /// As [`Vfs::create`] and [`Vfs::flush`].
    pub fn write(&self, path: &str, bytes: &[u8]) -> Result<Committed, VfsError> {
        let id = self.create(path)?;
        let staged = self.write_at(id, 0, bytes);
        let committed = staged.and_then(|_| self.flush(id));
        self.release(id);
        committed
    }

    /// Begins a write to `path`, and hands back the token the rest of it is addressed by.
    ///
    /// **Everything that can be decided before the bytes arrive is decided here**, because a
    /// `create` is where a file manager's error message comes from: the layout's refusals, the
    /// names this directory can file, and the position a page may be inserted at. What is left
    /// for [`Vfs::flush`] is what needs the bytes — whether they are a document — and the
    /// document's own assertions over its reader, which are asked once at the transform seam.
    ///
    /// # Errors
    ///
    /// [`VfsError::Refused`] where the layout refuses the destination by design or declares a
    /// meaning nothing implements, [`VfsError::NoSuchPath`] for a path the layout does not name,
    /// [`VfsError::Unnameable`] for a name this directory cannot file, [`VfsError::AlreadyFiled`]
    /// for an embedded file whose name §7.7.4's tree already holds, and the backing's or the
    /// worker's own.
    pub fn create(&self, path: &str) -> Result<StagedId, VfsError> {
        let mut held = self.held();
        let current = self.current_in(&mut held)?;
        let canonical = canonical_path(path)?;
        let (route, captures) = Self::locate_for_write(&current, &canonical)?;
        let token = held.next_token;
        held.next_token = held.next_token.saturating_add(1);
        let foreign_edits = held.foreign_edits;
        held.staged.insert(
            token,
            Staged {
                path: canonical,
                route,
                captures,
                generation: current.generation,
                foreign_edits,
                bytes: Vec::new(),
                touched: false,
                committed: false,
            },
        );
        Ok(StagedId(token))
    }

    /// Writes into a staged file at an offset, as `pwrite(2)` does.
    ///
    /// A gap is filled with zero bytes, which is what a sparse write to a real file produces; a
    /// caller that seeks past the end and writes has made a file that long.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchWrite`] for a token nothing answers to, and [`VfsError::TooLarge`] past
    /// [`crate::Config::max_staged_bytes`].
    pub fn write_at(&self, id: StagedId, offset: u64, bytes: &[u8]) -> Result<usize, VfsError> {
        let mut held = self.held();
        let ceiling = self.config.max_staged_bytes;
        let staged = held
            .staged
            .get_mut(&id.0)
            .ok_or(VfsError::NoSuchWrite(id.0))?;
        let from = usize::try_from(offset).map_err(|_| VfsError::TooLarge {
            path: staged.path.clone(),
            reached: usize::MAX,
            ceiling,
        })?;
        let end = from.saturating_add(bytes.len());
        if end > ceiling {
            return Err(VfsError::TooLarge {
                path: staged.path.clone(),
                reached: end,
                ceiling,
            });
        }
        staged.touched = true;
        if staged.bytes.len() < end {
            staged.bytes.resize(end, 0);
        }
        staged
            .bytes
            .get_mut(from..end)
            .ok_or(VfsError::NoSuchWrite(id.0))?
            .copy_from_slice(bytes);
        staged.committed = false;
        Ok(bytes.len())
    }

    /// Cuts a staged file to `length`, as `ftruncate(2)` does.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchWrite`], and [`VfsError::TooLarge`] for a length past the ceiling.
    pub fn truncate(&self, id: StagedId, length: u64) -> Result<(), VfsError> {
        let mut held = self.held();
        let ceiling = self.config.max_staged_bytes;
        let staged = held
            .staged
            .get_mut(&id.0)
            .ok_or(VfsError::NoSuchWrite(id.0))?;
        let to = usize::try_from(length)
            .ok()
            .filter(|to| *to <= ceiling)
            .ok_or_else(|| VfsError::TooLarge {
                path: staged.path.clone(),
                reached: usize::try_from(length).unwrap_or(usize::MAX),
                ceiling,
            })?;
        staged.bytes.resize(to, 0);
        staged.touched = true;
        staged.committed = false;
        Ok(())
    }

    /// Commits a staged write, which is the moment RFC 0003 section 5.4 names: a FUSE write
    /// buffers, and validation and commit happen on `flush`, whose error return reaches the
    /// application's `close()` — `release` reaches nobody, which is why it is only cleanup.
    ///
    /// A second `flush` with nothing written since the first does nothing and says so with the
    /// same answer, because the kernel issues one per `close(2)` and a file opened twice is
    /// closed twice.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchWrite`], [`VfsError::Changed`] where the document moved under the write,
    /// and the worker's own — including [`crate::worker::WorkerError::Restricted`] and
    /// [`crate::worker::WorkerError::Unanswerable`], which are `CLAUDE.md` principle 3's two levels that
    /// do not proceed.
    pub fn flush(&self, id: StagedId) -> Result<Committed, VfsError> {
        let mut held = self.held();
        let (path, mut route, mut captures, generation, foreign_edits, bytes, touched, committed) = {
            let staged = held.staged.get(&id.0).ok_or(VfsError::NoSuchWrite(id.0))?;
            (
                staged.path.clone(),
                staged.route,
                staged.captures.clone(),
                staged.generation,
                staged.foreign_edits,
                staged.bytes.clone(),
                staged.touched,
                staged.committed,
            )
        };
        let current = self.current_in(&mut held)?;
        // Nothing to do, twice over. `committed` is the second `close(2)` of a file the kernel
        // opened twice; `!touched` is a handle opened for writing that nobody wrote to or
        // truncated, which [`Staged::touched`] says is not a write at all. Both mark the write
        // done so that [`Vfs::release`] does not then announce an abandonment.
        if committed || !touched {
            if let Some(staged) = held.staged.get_mut(&id.0) {
                staged.committed = true;
            }
            return Ok(Committed {
                path,
                meaning: route.write.on_write,
                from: current.generation,
                to: current.generation,
                pages: current.pages,
                warnings: Vec::new(),
            });
        }
        if current.generation != generation {
            // **Whose edit moved it decides, and so does what the name means.**
            //
            // A *foreign* edit is `ESTALE` without exception: RFC 0003 section 5.4 makes the
            // backing file the single source of truth, and committing over somebody else's
            // update could discard it. [`Staged::foreign_edits`] is how that question is asked
            // exactly, across any number of transitions.
            //
            // Our own commit is a different thing, and it used to be refused as though it were
            // the same one — two writes in flight at once in a single mount, the first
            // committing, the second `ESTALE` (ADR 0864). What decides there is whether the
            // *name* still means what it meant. An embedded file's name and the information
            // dictionary are identities, so they still do. A page's ordinal is a **position** —
            // RFC 0003 section 5.2: "[o]rdinal names are positions, not identities … after any
            // write, the next listing renumbers" — so an insertion staged before a commit that
            // renumbered would land somewhere nobody asked for, and it stays `ESTALE`.
            let rebasable = foreign_edits == held.foreign_edits
                && matches!(
                    route.write.on_write,
                    Write::EmbedFile | Write::SetInformation
                );
            if !rebasable {
                return Err(VfsError::Changed { path });
            }
            // Re-asked rather than reused, so that every check `create` made — the name §7.7.4's
            // tree already holds above all — is made again against the generation this is about
            // to be committed to.
            let (fresh_route, fresh_captures) = Self::locate_for_write(&current, &path)?;
            route = fresh_route;
            captures = fresh_captures;
            if let Some(staged) = held.staged.get_mut(&id.0) {
                staged.route = route;
                staged.captures = captures.clone();
                staged.generation = current.generation;
            }
        }
        let query = match route.write.on_write {
            Write::InsertPages => Query::InsertPages {
                at: captures
                    .page
                    .ok_or_else(|| VfsError::NoSuchPath(path.clone()))?,
                document: bytes,
            },
            Write::EmbedFile => Query::Attach {
                name: captures
                    .name
                    .clone()
                    .ok_or_else(|| VfsError::NoSuchPath(path.clone()))?,
                bytes,
            },
            Write::SetInformation => Query::SetInformation { json: bytes },
            // `create` refused every other mapping before a byte was accepted, so this is a row
            // that changed under a staged write rather than a caller's mistake.
            Write::DeletePage | Write::RemoveAttachment | Write::Refused(_) => {
                return Err(refusal_for(&path, Verb::Write)?.into());
            }
        };
        let outcome = self.apply_update(&mut held, &current, &path, &query)?;
        if let Some(staged) = held.staged.get_mut(&id.0) {
            staged.committed = true;
        }
        Ok(Committed {
            meaning: route.write.on_write,
            ..outcome
        })
    }

    /// Drops a staged write.
    ///
    /// `Some` where it was never committed, which is a write that did not happen: the document
    /// is byte for byte what it was, and [`Abandoned::sentence`] is what a face logs so that the
    /// name its listing showed is explained rather than discovered.
    pub fn release(&self, id: StagedId) -> Option<Abandoned> {
        let mut held = self.held();
        let staged = held.staged.remove(&id.0)?;
        (!staged.committed).then(|| Abandoned {
            path: staged.path,
            size: u64::try_from(staged.bytes.len()).unwrap_or(u64::MAX),
            meaning: staged.route.write.on_write,
        })
    }

    /// A handle onto a write in flight at this path, where there is one.
    ///
    /// The generation is the one the write was created against, which is what makes the property
    /// RFC 0003 section 5.4 states hold for a staged file too: what a reader is handed is one
    /// generation's worth of bytes and never a splice.
    pub(crate) fn staged_at(&self, path: &str) -> Option<Handle> {
        let canonical = canonical_path(path).ok()?;
        let held = self.held();
        let staged = held
            .staged
            .values()
            .find(|staged| !staged.committed && staged.path == canonical)?;
        Some(Handle {
            path: canonical,
            generation: staged.generation,
            bytes: Arc::from(staged.bytes.as_slice()),
        })
    }

    /// The writes in flight, as a listing shows them.
    #[must_use]
    pub fn pending(&self) -> Vec<Pending> {
        self.held()
            .staged
            .iter()
            .filter(|(_, staged)| !staged.committed)
            .map(|(token, staged)| Pending {
                id: StagedId(*token),
                path: staged.path.clone(),
                size: u64::try_from(staged.bytes.len()).unwrap_or(u64::MAX),
                meaning: staged.route.write.on_write,
            })
            .collect()
    }

    /// Deleting a path.
    ///
    /// **Not staged, because a deletion has no bytes**: `unlink(2)` is one call and this is one
    /// transaction, so it validates and commits together and its error is the caller's own.
    ///
    /// # Errors
    ///
    /// [`VfsError::Refused`] where the layout refuses the deletion, [`VfsError::NoSuchPath`],
    /// and the worker's own.
    pub fn remove(&self, path: &str) -> Result<Committed, VfsError> {
        let mut held = self.held();
        let current = self.current_in(&mut held)?;
        let canonical = canonical_path(path)?;
        let (route, captures) =
            locate_in(&current, &canonical, &self.config.resolutions, &self.cache)?;
        let query = match route.write.on_delete {
            Write::DeletePage => Query::DeletePage {
                page: captures
                    .page
                    .ok_or_else(|| VfsError::NoSuchPath(canonical.clone()))?,
            },
            // The *document's* own name for it, which is what the worker files it under; the
            // tree lists it under a name a directory can hold, and `attachments` is the map
            // between the two.
            Write::RemoveAttachment => {
                let listed = captures
                    .name
                    .clone()
                    .ok_or_else(|| VfsError::NoSuchPath(canonical.clone()))?;
                Query::Detach {
                    name: attachments(&current)?
                        .iter()
                        .find(|embedded| embedded.file_name == listed)
                        .map(|embedded| embedded.document_name.clone())
                        .ok_or_else(|| VfsError::NoSuchPath(canonical.clone()))?,
                }
            }
            Write::InsertPages | Write::EmbedFile | Write::SetInformation | Write::Refused(_) => {
                return Err(refusal_for(&canonical, Verb::Delete)?.into());
            }
        };
        let outcome = self.apply_update(&mut held, &current, &canonical, &query)?;
        Ok(Committed {
            meaning: route.write.on_delete,
            ..outcome
        })
    }

    /// The whole commit: ask the worker, check §7.5.6's property, replace the file, and step the
    /// generation on — all under the lock the caller already holds.
    fn apply_update(
        &self,
        held: &mut Serving,
        current: &Current,
        path: &str,
        query: &Query,
    ) -> Result<Committed, VfsError> {
        let Answer::Written { bytes, warnings } = current.worker.ask(query)? else {
            return Err(VfsError::Worker(mismatch(
                &Answer::Absent,
                "an updated document",
            )));
        };
        self.check_appended(&bytes)?;
        self.backing
            .commit(&bytes)
            .map_err(|error| VfsError::Backing {
                document: self.backing.describe(),
                error,
            })?;
        let from = current.generation;
        // The old generation goes now and the new one is built before the lock is let go, so no
        // operation can see a tree that belongs to neither — and `ours` is what makes the next
        // one `Provenance::Ours` rather than looking like somebody else's edit.
        held.current = None;
        held.ours = self.backing.generation().ok();
        let next = self.current_in(held)?;
        Ok(Committed {
            path: path.to_owned(),
            meaning: Write::Refused(Reason::LayoutIsNotWritable),
            from,
            to: next.generation,
            pages: next.pages,
            warnings,
        })
    }

    /// ISO 32000-2 §7.5.6, checked against the file rather than trusted:
    ///
    /// > When updating a PDF file incrementally, changes shall be appended to the end of the
    /// > file, leaving its original contents intact.
    ///
    /// So the document the worker computed begins with the document on disk, byte for byte, and
    /// the broker can say so **without reading either as a PDF** — which is what keeps this on
    /// its side of RFC 0003 section 6's line. A worker that answered with anything else has
    /// either been compromised or has a defect, and either way its answer is not written.
    fn check_appended(&self, bytes: &[u8]) -> Result<(), VfsError> {
        /// How much is compared at a time, so that a large document is not held twice.
        const WINDOW: usize = 1 << 16;

        let original = self.backing.bytes().map_err(|error| VfsError::Backing {
            document: self.backing.describe(),
            error,
        })?;
        let length = original.len();
        let refuse = |detail: &str| {
            VfsError::Worker(WorkerError::Refused(format!(
                "the update this worker computed does not have the document as its prefix ({detail}), \
             and §7.5.6 makes an update \"appended to the end of the file, leaving its original \
             contents intact\"; nothing was written"
            )))
        };
        if bytes.len() < length {
            return Err(refuse("it is shorter than the file"));
        }
        let mut at = 0;
        while at < length {
            let end = at.saturating_add(WINDOW).min(length);
            let window = original.read(at..end);
            if bytes.get(at..end) != Some(window.as_ref()) {
                return Err(refuse("they differ inside the file's own bytes"));
            }
            at = end;
        }
        Ok(())
    }

    /// The row a path names, for a **write** — which admits two paths a read does not.
    ///
    /// A `pages/NNNN.pdf` one past the last page, because RFC 0003 section 5.2 makes an ordinal a
    /// position and appending is inserting at the position after the end; and an
    /// `attachments/NAME` the document does not file, because that is what embedding a file *is*.
    /// Everything else is the read's own answer.
    pub(crate) fn locate_for_write(
        current: &Current,
        path: &str,
    ) -> Result<(&'static Route, path::Captures), VfsError> {
        let missing = || VfsError::NoSuchPath(path.to_owned());
        let (route, captures) = path::resolve(path).ok_or_else(missing)?;
        match route.write.on_write {
            Write::InsertPages => {
                let page = captures.page.ok_or_else(missing)?;
                // One past the end appends; nothing further out is a position, and a listing
                // never showed it.
                if page == 0 || page > current.pages.saturating_add(1) {
                    return Err(missing());
                }
                if !spells(current, path, page) {
                    return Err(missing());
                }
            }
            Write::EmbedFile => {
                let name = captures.name.clone().ok_or_else(missing)?;
                // The tree lists an embedded file under a name a directory can hold, and a read
                // is looked up by that name; a file filed under a name the listing would show
                // differently is a file `ls` could not name. So the two have to agree, and a
                // name that would be changed is refused rather than quietly renamed.
                if safe_name(&name) != name {
                    return Err(VfsError::Unnameable {
                        path: path.to_owned(),
                        detail: format!(
                            "§7.7.4's tree would file this as {:?}, which is not the name asked \
                             for; a directory cannot hold every byte a name tree can",
                            safe_name(&name)
                        ),
                    });
                }
                if attachments(current)?
                    .iter()
                    .any(|embedded| embedded.file_name == name)
                {
                    return Err(VfsError::AlreadyFiled {
                        path: path.to_owned(),
                    });
                }
            }
            Write::SetInformation => {}
            Write::DeletePage | Write::RemoveAttachment | Write::Refused(_) => {
                return Err(refusal_for(path, Verb::Write)?.into());
            }
        }
        Ok((route, captures))
    }

    /// Creating a directory.
    ///
    /// # Errors
    ///
    /// As [`Vfs::write`]; every directory in this tree is the document's own shape.
    pub fn create_directory(&self, path: &str) -> Result<(), VfsError> {
        let _ = canonical_path(path)?;
        Err(Refused::ByDesign {
            path: path.to_owned(),
            reason: Reason::LayoutIsNotWritable,
        }
        .into())
    }

    /// Renaming within the tree.
    ///
    /// **Refused outright and not merely unbuilt**, which is RFC 0003 section 5.3's first
    /// refusal: "\[r]ename semantics under position-names are ambiguous (is `mv 0007 0002` 'before
    /// old 0002' or 'become new 0002'?), and a file manager's drag-reorder emits rename storms
    /// this tree cannot make atomic."
    ///
    /// # Errors
    ///
    /// Always [`Refused::ByDesign`] with [`Reason::ReorderIsAmbiguous`].
    pub fn rename(&self, from: &str, to: &str) -> Result<(), VfsError> {
        Err(Refused::ByDesign {
            path: format!("{from} -> {to}"),
            reason: Reason::ReorderIsAmbiguous,
        }
        .into())
    }
}
