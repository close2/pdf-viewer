//! File attachments a person adds or removes: the log of what was attached and what was detached,
//! and the reading of the attachments the document and that log make together.

use pdf_syntax::{Document, Object};

use super::{Added, AttachRefusal, Detached, Filed, Filing, FilingHome, ViewState};

impl ViewState {
    /// §7.11.4: puts a file into the document, in one of §7.11.4.1's two homes.
    ///
    /// Nothing is written: the stream and the specification are allocated their numbers and
    /// built at save time by [`Self::save`], through `crate::attachment::filing` — the same
    /// writer `pdf-transform attachments --attach` uses. Where the home is a page, the
    /// §12.5.6.15 annotation is added to [`Self::additions`] at once, so the page draws its icon
    /// before anything is saved and the same code writes it that writes every other added
    /// annotation. Table 187's `/Name` is the table's default, `PushPin`.
    ///
    /// # Errors
    ///
    /// [`AttachRefusal::NameTaken`] where the document's tree — less what this viewing session
    /// detached — or this viewing session already files something under the name.
    pub fn attach(&mut self, document: &Document, filing: Filing) -> Result<(), AttachRefusal> {
        let key = pdf_syntax::text_string::encode_text_string(&filing.name);
        if self.attachment_named(document, &filing.name) {
            return Err(AttachRefusal::NameTaken);
        }
        let stream = self.allocate(document);
        let specification = self.allocate(document);
        let annotation = match filing.home {
            FilingHome::Document => None,
            FilingHome::Page { page, rect } => {
                let id = self.allocate(document);
                let dict = crate::attachment::filing::file_attachment_annotation(
                    page,
                    rect,
                    specification,
                    crate::attachment::filing::DEFAULT_ICON,
                    filing.description.as_deref(),
                );
                self.added.push(Added { id, page, dict });
                Some(id)
            }
        };
        self.filed.push(Filed {
            filing,
            key,
            stream,
            specification,
            annotation,
        });
        Ok(())
    }

    /// §7.11.4: takes a file out of the document, by the name it is filed under.
    ///
    /// A file this session attached is simply forgotten — its annotation too, where it had one.
    /// A file the document's own tree names is left out of the tree the next save writes, and
    /// what that entry alone reached is marked free then; the decision about what "alone" means
    /// is `crate::attachment::filing::freed_by_removing`'s and the same one the transform makes.
    pub fn detach(&mut self, document: &Document, name: &str) -> Detached {
        let key = pdf_syntax::text_string::encode_text_string(name);
        if let Some(at) = self.filed.iter().position(|filed| filed.key == key) {
            let filed = self.filed.remove(at);
            if let Some(annotation) = filed.annotation {
                self.added.retain(|added| added.id != annotation);
            }
            self.release_numbers_if_nothing_is_held();
            return Detached::Filed;
        }
        if self.unfiled.contains(&key) {
            return Detached::Nothing;
        }
        let Ok(catalog) = document.catalog() else {
            return Detached::Nothing;
        };
        let tree =
            crate::attachment::filing::Tree::read(document, &catalog, &|id| document.get(id));
        if tree.holds(&key) {
            self.unfiled.push(key);
            return Detached::Unfiled;
        }
        Detached::Nothing
    }

    /// Whether anything — the document's tree less what was detached, or this viewing session —
    /// files a file under this name.
    #[must_use]
    pub fn attachment_named(&self, document: &Document, name: &str) -> bool {
        let key = pdf_syntax::text_string::encode_text_string(name);
        if self.filed.iter().any(|filed| filed.key == key) {
            return true;
        }
        if self.unfiled.contains(&key) {
            return false;
        }
        document.catalog().is_ok_and(|catalog| {
            crate::attachment::filing::Tree::read(document, &catalog, &|id| document.get(id))
                .holds(&key)
        })
    }

    /// §7.11.4's embedded files as this state has them: the document's own list, less what was
    /// detached, plus what was attached to the document as a whole.
    ///
    /// The list a panel shows, so it answers for the same home the document's own list answers for
    /// — the tree, and the catalog's `/AF` beside it — and not for a file this viewing session put
    /// on a *page*: that one is where §12.5.6.15 put it, drawn as its icon, and after a save and a
    /// reopen it would be in the home the tree does not list (ADR 0295). A list that showed it
    /// before the save and not after would be a list that changed under a person who changed
    /// nothing.
    ///
    /// An attached file's record is the one [`crate::attachment::read`] would build from the
    /// objects a save writes — the same stream, so that extracting it decodes the same bytes.
    #[must_use]
    pub fn attachments(&self, document: &Document) -> Vec<crate::attachment::Attachment> {
        let mut out: Vec<crate::attachment::Attachment> = crate::attachment::attachments(document)
            .into_iter()
            .filter(|attachment| {
                let key = pdf_syntax::text_string::encode_text_string(&attachment.name);
                !self.unfiled.contains(&key)
            })
            .collect();
        for filed in &self.filed {
            if !matches!(filed.filing.home, FilingHome::Document) {
                continue;
            }
            let Object::Stream(stream) = crate::attachment::filing::embedded_file_stream(
                &filed.filing.bytes,
                filed.filing.media_type.as_deref(),
                None,
            ) else {
                continue;
            };
            out.push(crate::attachment::Attachment {
                name: filed.filing.name.clone(),
                file_name: Some(filed.filing.name.clone()),
                description: filed.filing.description.clone(),
                media_type: filed.filing.media_type.clone(),
                size: i64::try_from(filed.filing.bytes.len()).ok(),
                created: None,
                modified: None,
                checksum: Some(
                    <md5::Md5 as md5::Digest>::digest(filed.filing.bytes.bytes()).to_vec(),
                ),
                relationship: crate::attachment::Relationship::Unspecified,
                // A file this session attached carries neither: Table 43's `/Thumb` is a
                // miniature a producer made of a file it already had, and `/EP` says the payload
                // is encrypted with a handler this program does not have. Filing writes neither.
                thumbnail: None,
                payload: None,
                stream,
            });
        }
        out
    }

    /// Every file this session attached, in order — what a save writes.
    #[must_use]
    pub fn attached(&self) -> &[Filed] {
        &self.filed
    }

    /// The names of the document's own files this session detached, in order.
    pub fn detached(&self) -> impl Iterator<Item = String> + '_ {
        self.unfiled.iter().map(|key| pdf_syntax::text_string(key))
    }

    /// Forgets every attach and every detach.
    ///
    /// The fourth of the clears a replay starts from, beside [`Self::clear_all_fields`],
    /// [`Self::clear_all_additions`] and [`Self::clear_all_free_text`]. The annotations that
    /// filed page attachments are additions and go with the second; this takes the files.
    pub fn clear_all_attachments(&mut self) {
        self.filed.clear();
        self.unfiled.clear();
        self.release_numbers_if_nothing_is_held();
    }
}
