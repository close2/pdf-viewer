//! The directory answers: what a directory lists and what a path's attributes are.
//!
//! [`Vfs::list`] and [`Vfs::stat`] are RFC 0003 section 5.1's reads that return no file bytes.
//! Both ask the layout first and the document second, and a size they report is the cache's
//! (RFC 0003 section 5.5: no virtual file is stat'd before it is generated).

use crate::error::VfsError;
use crate::inventory::{
    attachments, image_names, last_component, locate_in, metadata_stream, page_names, width,
};
use crate::layout::{Generator, Kind, Route};
use crate::{Attributes, Current, DirEntry, Vfs, canonical_path};
use crate::{layout, path};

impl Vfs {
    /// What is in a directory.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchPath`] for a path the layout does not name or the document does not
    /// have, [`VfsError::NotADirectory`] for a file, and the backing's or the worker's own.
    pub fn list(&self, path: &str) -> Result<Vec<DirEntry>, VfsError> {
        let current = self.current()?;
        let (route, captures) = locate_in(&current, path, &self.config.resolutions, &self.cache)?;
        let directory = canonical_path(path)?;
        if route.kind != Kind::Directory {
            return Err(VfsError::NotADirectory(path.to_owned()));
        }
        let mut entries = self.entries_of(&current, path, route, &captures)?;
        // A write in flight is *in* the tree: `cp` stats what it has just written, a file
        // manager shows the copy arriving, and a directory that named nothing there would fail
        // the copy it had accepted. `crate::commit` has the argument.
        for staged in self.pending() {
            let Some(name) = staged
                .path
                .strip_prefix(&directory)
                .and_then(|rest| rest.strip_prefix('/'))
                .filter(|rest| !rest.contains('/'))
            else {
                continue;
            };
            if !entries.iter().any(|entry| entry.name == name) {
                entries.push(DirEntry {
                    name: name.to_owned(),
                    kind: Kind::File,
                });
            }
        }
        if entries.len() > self.config.max_entries {
            return Err(VfsError::TooManyEntries {
                path: path.to_owned(),
                count: entries.len(),
                ceiling: self.config.max_entries,
            });
        }
        Ok(entries)
    }

    /// What one directory's own generator names, before the writes in flight join it.
    fn entries_of(
        &self,
        current: &Current,
        path: &str,
        route: &'static Route,
        captures: &path::Captures,
    ) -> Result<Vec<DirEntry>, VfsError> {
        let entries = match route.generator {
            Generator::Root => layout::children("/")
                .into_iter()
                .map(|child| DirEntry {
                    name: last_component(child.pattern),
                    kind: child.kind,
                })
                .collect(),
            Generator::PageOrdinals => page_names(current, "pdf"),
            Generator::Resolutions => self
                .config
                .resolutions
                .iter()
                .map(|dpi| DirEntry {
                    name: path::resolution_name(*dpi),
                    kind: Kind::Directory,
                })
                .collect(),
            Generator::RenderOrdinals => page_names(current, "png"),
            Generator::ImagePageOrdinals => (1..=current.pages)
                .map(|page| DirEntry {
                    name: path::page_name_stem(page, width(current)),
                    kind: Kind::Directory,
                })
                .collect(),
            Generator::ImageInventory => {
                let page = captures.page.ok_or(VfsError::NoSuchPath(path.to_owned()))?;
                // Already in the extraction's own order: a `BTreeMap`'s keys are sorted, and the
                // inventory is those keys.
                image_names(&self.cache, current, page)?
                    .iter()
                    .map(|name| DirEntry {
                        name: name.clone(),
                        kind: Kind::File,
                    })
                    .collect()
            }
            Generator::TextOrdinals => {
                let mut entries = page_names(current, "txt");
                entries.push(DirEntry {
                    name: String::from("document.txt"),
                    kind: Kind::File,
                });
                entries
            }
            Generator::AttachmentInventory => attachments(current)?
                .iter()
                .map(|embedded| DirEntry {
                    name: embedded.file_name.clone(),
                    kind: Kind::File,
                })
                .collect(),
            Generator::MetaNames => {
                let mut entries = vec![
                    DirEntry {
                        name: String::from("info.json"),
                        kind: Kind::File,
                    },
                    DirEntry {
                        name: String::from("outline.json"),
                        kind: Kind::File,
                    },
                ];
                // `xmp.xml` is listed only where §14.3.2's stream exists, because its content is
                // the stream's own bytes and there are none to invent. `info.json` and
                // `outline.json` are always listed: both are answers this program composes, and
                // a document that states neither an `/Info` nor an `/Outlines` has an empty
                // answer rather than no answer.
                if metadata_stream(&self.cache, current)?.is_some() {
                    entries.push(DirEntry {
                        name: String::from("xmp.xml"),
                        kind: Kind::File,
                    });
                }
                entries
            }
            Generator::ExtractedPage
            | Generator::RenderedPage
            | Generator::ExtractedImage
            | Generator::PageText
            | Generator::DocumentText
            | Generator::ExtractedAttachment
            | Generator::Information
            | Generator::MetadataStream
            | Generator::Outline => return Err(VfsError::NotADirectory(path.to_owned())),
        };
        Ok(entries)
    }

    /// What a path is, and — for a file — how big.
    ///
    /// **A file is generated here.** RFC 0003 section 5.5: "\[r]ule: no virtual file is stat'd
    /// before it is generated. `stat` on `pages/0007.pdf` generates (or finds cached) the bytes
    /// and reports the true size." The cache is what stops a `cp` paying for it twice.
    ///
    /// # Errors
    ///
    /// As [`Vfs::open`], plus [`VfsError::NoSuchPath`].
    pub fn stat(&self, path: &str) -> Result<Attributes, VfsError> {
        let current = self.current()?;
        if let Some(staged) = self.staged_at(path) {
            return Ok(Attributes {
                kind: Kind::File,
                size: Some(staged.len()),
                generation: staged.generation,
            });
        }
        let (route, _) = locate_in(&current, path, &self.config.resolutions, &self.cache)?;
        if route.kind == Kind::Directory {
            return Ok(Attributes {
                kind: Kind::Directory,
                size: None,
                generation: current.generation,
            });
        }
        // A size this generation has already produced, answered without producing it again.
        // RFC 0003 section 5.5's rule is that a `stat` may not *estimate* — "an under-estimate
        // silently truncates a page" — and a length taken off the bytes themselves is not an
        // estimate whether or not those bytes are still in the cache. `Cache::sizes` has what a
        // mount by hand measured this to be worth.
        let canonical = canonical_path(path)?;
        if let Some(size) = self.cache.size_of(current.generation, &canonical) {
            return Ok(Attributes {
                kind: Kind::File,
                size: Some(size),
                generation: current.generation,
            });
        }
        let handle = self.open(path)?;
        Ok(Attributes {
            kind: Kind::File,
            size: Some(handle.len()),
            generation: handle.generation,
        })
    }
}
