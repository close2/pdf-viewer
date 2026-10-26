//! The file reads: asking whether an operation would be allowed, opening a virtual file, and
//! producing its bytes.
//!
//! [`Vfs::open`] returns a [`crate::Handle`] over bytes the cache holds; [`Vfs::consult`] and
//! [`Vfs::answer`] are `CLAUDE.md` principle 3's question asked before a copy starts.

use pdf_transform::Consulted;

use crate::error::VfsError;
use crate::inventory::{attachments, extract_images, locate_in, metadata_stream, width};
use crate::layout::{Generator, Kind, Route};
use crate::path;
use crate::worker::{Answer, Query, Worker};
use crate::{Current, Handle, Verb, Vfs, canonical_path, mismatch};

impl Vfs {
    /// **Would this operation be restricted, and why** — the question a host puts to a person
    /// before committing to the operation.
    ///
    /// `CLAUDE.md` principle 3's four levels are `off`, `on`, *ask before the operation* and
    /// *warn before the operation*, and "a refusal that cannot become an 'ask' is the thing to
    /// avoid". RFC 0003 section 6 makes the *ask* level unaskable from inside: the decision is
    /// taken where the document is, which is a process with no channel to a person by
    /// construction. So the question crosses instead — this call — and the answer comes back
    /// through [`Vfs::answer`]; the operation is then issued exactly as it always was, and the
    /// yes is spent by the query that performs it (ADR 0874).
    ///
    /// A face with somewhere to put the question — KIO's `messageBox`, a window's dialogue, a
    /// terminal — puts it. A face with nowhere does not call this at all and gets
    /// [`crate::worker::WorkerError::Unanswerable`] from the operation, which is what a mount gets and
    /// why the sentence names the poverty rather than hiding it.
    ///
    /// The level is the mount's own ([`crate::Config::policy`]), so a tree at `off` answers
    /// `Consulted::Proceed` for everything and a face that always asks first costs one round
    /// trip and no dialogue.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchPath`] for a path the layout does not name, and the worker's own where
    /// the document cannot be opened at all.
    pub fn consult(&self, path: &str, verb: Verb) -> Result<Consulted, VfsError> {
        let current = self.current()?;
        // A write is asked about *before* the file exists — `cp new.pdf pages/0004.pdf` names a
        // position rather than an entry — so the write side is located the way `Vfs::create`
        // locates it and the other two the way a read does. Getting this wrong would make every
        // insertion's consultation a `NoSuchPath`, which is the shape of a face that cannot ask.
        let (route, _) = match verb {
            Verb::Write => Self::locate_for_write(&current, &canonical_path(path)?)?,
            Verb::Read | Verb::Delete => {
                locate_in(&current, path, &self.config.resolutions, &self.cache)?
            }
        };
        // The layout table is what says which operation a path's verb performs, because it is
        // the one place a path's meaning is stated; `tests/a_write.rs` holds it to
        // `pdf_transform::Plan::operation`'s own answer rather than to this call.
        let operation = match verb {
            Verb::Read => route.generator.operation(),
            Verb::Write => route.write.on_write.operation(),
            Verb::Delete => route.write.on_delete.operation(),
        };
        let Some(operation) = operation else {
            // A verb this tree will not perform, or a listing that reads nothing the standard
            // restricts: there is no operation, so there is nothing to consult about. The
            // refusal such a path earns is the layout's own and is unrelated to policy.
            return Ok(Consulted::Proceed);
        };
        let consulted = match current.worker.ask(&Query::Consult { operation })? {
            Answer::Consulted(consulted) => consulted,
            other => return Err(VfsError::Worker(mismatch(&other, "a consultation"))),
        };
        if matches!(consulted, Consulted::Ask { .. }) {
            current.worker.putting(operation);
        } else {
            current.worker.nothing_outstanding();
        }
        Ok(consulted)
    }

    /// The person's answer to the question [`Vfs::consult`] last put — the second of ADR 0874's
    /// two round trips.
    ///
    /// `true` means *do it*: the next operation that performs the operation asked about runs at
    /// `pdf_model::restriction::Level::Off`, which is the level `CLAUDE.md` says "shall always
    /// be possible" and is what a person consenting to one operation has chosen for it. The
    /// consent is spent by that one query and by no other, and it goes when the generation goes.
    ///
    /// `false` forgets the question and does nothing else, because a question declined is
    /// neither the document doing something nor this program refusing — `viewer_core`'s own rule
    /// for `Command::Answer`.
    ///
    /// Answers whether there was a question outstanding to answer. `false` there is a face's
    /// defect rather than a person's: nothing asked, or the document moved underneath the mount
    /// while the dialogue was up and the generation that was asked about is gone.
    ///
    /// # Errors
    ///
    /// The backing's, where the generation cannot be read.
    pub fn answer(&self, proceed: bool) -> Result<bool, VfsError> {
        Ok(self.current()?.worker.answered(proceed))
    }

    /// Materialises a virtual file and hands back a handle onto its bytes.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchPath`], [`VfsError::IsADirectory`], and the worker's own refusal where
    /// the content cannot be produced — a codec the confined worker does not have, a page the
    /// rasteriser declined. Loud in every case (trap 5).
    pub fn open(&self, path: &str) -> Result<Handle, VfsError> {
        let current = self.current()?;
        if let Some(staged) = self.staged_at(path) {
            return Ok(staged);
        }
        let (route, captures) = locate_in(&current, path, &self.config.resolutions, &self.cache)?;
        if route.kind == Kind::Directory {
            return Err(VfsError::IsADirectory(path.to_owned()));
        }
        let canonical = canonical_path(path)?;
        if let Some(bytes) = self.cache.get(current.generation, &canonical) {
            return Ok(Handle {
                path: canonical,
                generation: current.generation,
                bytes,
            });
        }
        self.generated
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let bytes = self.generate(&current, route, &captures, &canonical)?;
        Ok(Handle {
            path: canonical.clone(),
            generation: current.generation,
            bytes: self.cache.put(current.generation, &canonical, bytes),
        })
    }

    /// One virtual file's bytes.
    fn generate(
        &self,
        current: &Current,
        route: &'static Route,
        captures: &path::Captures,
        canonical: &str,
    ) -> Result<Vec<u8>, VfsError> {
        let missing = || VfsError::NoSuchPath(canonical.to_owned());
        let page = || captures.page.ok_or_else(missing);
        let answer = match route.generator {
            Generator::ExtractedPage => {
                current.worker.ask(&Query::ExtractPage { page: page()? })?
            }
            Generator::RenderedPage => current.worker.ask(&Query::RenderPage {
                page: page()?,
                dpi: captures.dpi.ok_or_else(missing)?,
            })?,
            Generator::ExtractedImage => {
                let page = page()?;
                let name = captures.name.clone().ok_or_else(missing)?;
                return extract_images(&self.cache, current, page)?
                    .get(&name)
                    .map(|kept| kept.to_vec())
                    .ok_or_else(missing);
            }
            Generator::PageText => current.worker.ask(&Query::PageText { page: page()? })?,
            Generator::DocumentText => return self.document_text(current),
            Generator::ExtractedAttachment => {
                let name = captures.name.clone().ok_or_else(missing)?;
                let document_name = attachments(current)?
                    .iter()
                    .find(|embedded| embedded.file_name == name)
                    .map(|embedded| embedded.document_name.clone())
                    .ok_or_else(missing)?;
                current.worker.ask(&Query::ExtractAttachment {
                    name: document_name,
                })?
            }
            Generator::Information => current.worker.ask(&Query::Information)?,
            Generator::MetadataStream => {
                return metadata_stream(&self.cache, current)?
                    .map(|bytes| bytes.to_vec())
                    .ok_or_else(missing);
            }
            Generator::Outline => current.worker.ask(&Query::Outline)?,
            Generator::Root
            | Generator::PageOrdinals
            | Generator::Resolutions
            | Generator::RenderOrdinals
            | Generator::ImagePageOrdinals
            | Generator::ImageInventory
            | Generator::TextOrdinals
            | Generator::AttachmentInventory
            | Generator::MetaNames => {
                return Err(VfsError::IsADirectory(canonical.to_owned()));
            }
        };
        match answer {
            Answer::Bytes(bytes) => Ok(bytes),
            Answer::Absent => Err(missing()),
            other => Err(VfsError::Worker(mismatch(&other, "bytes"))),
        }
    }

    /// `text/document.txt`: every page's readback in page order.
    ///
    /// Separated by U+000C FORM FEED, which is a stated choice rather than a clause: the standard
    /// says nothing about how a concatenation of pages' text is delimited, and the form feed is
    /// what every text extractor a script would otherwise be piped through emits between pages.
    /// `CLAUDE.md`'s rule for a silence applies — a documented choice, said to be a choice.
    ///
    /// Each page goes through the per-page cache on the way, so a caller that greps
    /// `document.txt` and then reads a page pays for that page once.
    fn document_text(&self, current: &Current) -> Result<Vec<u8>, VfsError> {
        let width = width(current);
        let mut out: Vec<u8> = Vec::new();
        for page in 1..=current.pages {
            let path = format!("/text/{}", path::page_name(page, width, "txt"));
            let bytes = if let Some(bytes) = self.cache.get(current.generation, &path) {
                bytes
            } else {
                let Answer::Bytes(bytes) = current.worker.ask(&Query::PageText { page })? else {
                    return Err(VfsError::Worker(mismatch(&Answer::Absent, "bytes")));
                };
                self.cache.put(current.generation, &path, bytes)
            };
            if page > 1 {
                out.push(0x0c);
            }
            out.extend_from_slice(&bytes);
        }
        Ok(out)
    }
}
