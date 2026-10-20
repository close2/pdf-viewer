//! The repair of a cross-reference entry the file disproves: an object found by its own header.

use std::sync::Arc;

use super::{Document, read, write};
use crate::file::NoRoom;
use crate::object::{Object, ObjectId};
use crate::xref::{Location, XrefTable};

impl Document {
    /// Finds an object by its own header, where the table's entry for it was disproved.
    ///
    /// # Why this is a repair and not a guess
    ///
    /// §7.5.4 says a cross-reference entry's offset gives "the number of bytes from the
    /// beginning of the PDF file to the beginning of the object", and §7.3.10 makes an
    /// indirect object begin with its own number:
    ///
    /// > The definition of an indirect object in a PDF file shall consist of its object number
    /// > and generation number (separated by white-space), followed by the value of the object
    /// > bracketed between the keywords obj and endobj
    ///
    /// So the file states where object 3 is twice, and where the two statements disagree the
    /// object's own header is the one written next to the bytes it describes. Taking it is
    /// the same move [`crate::xref::rebuild`] makes for a whole table and
    /// `pdf_model::Pages::new` makes for a page tree that walks to nothing: a recovery from
    /// the file's own declarations, never from another reader's behaviour.
    ///
    /// # What it deliberately does not do
    ///
    /// It runs where an entry *exists* and is disproved, and — since ADR 0789 — where the
    /// section's own header declared an entry this reader never read
    /// ([`crate::xref::XrefTable::declared_and_unread`]). It does **not** run for a number the
    /// file did not mention at all, or mentioned as **free**: that names nothing, because
    /// §7.5.6 makes a deletion the most recent statement about an object, and this reader does
    /// not resurrect objects its own file deleted (ADR 0100).
    /// Scanning for a header there would undo exactly that. The caller's two questions —
    /// [`crate::xref::XrefTable::location`] first, then `declared_and_unread` — are what keep
    /// the three cases apart.
    ///
    /// # Cost
    ///
    /// One linear scan of the file, memoised, and only for a document that has already been
    /// found to be wrong about one of its objects. Two of the 974 corpus documents reach it.
    pub(super) fn load_by_header(&self, id: ObjectId) -> Option<Object> {
        let headers = self.object_headers();
        let Location::Offset(offset) = headers.location(id.number)? else {
            return None;
        };
        let (found, object) = self.parse_at(offset)?;
        if found.number != id.number {
            return None;
        }
        write(&self.misfiled).insert(id.number);
        Some(object)
    }

    /// The file's own object headers, scanned once and remembered.
    pub(super) fn object_headers(&self) -> Arc<XrefTable> {
        if let Some(headers) = read(&self.headers).as_ref() {
            return Arc::clone(headers);
        }
        let scanned = Arc::new(
            self.whole_for_a_scan()
                .map(|input| crate::xref::scan_for_objects(input, self.limits))
                .unwrap_or_default(),
        );
        *write(&self.headers) = Some(Arc::clone(&scanned));
        scanned
    }

    /// The whole file for a scan, or `None` where the file is on disk and the process cannot
    /// hold it — recorded once, for [`Self::scan_refused`].
    ///
    /// Every scan in this type goes through here, so that "this reader could not scan the file"
    /// is one fact in one place rather than a silence in three. In memory the answer is always
    /// the bytes.
    pub(super) fn whole_for_a_scan(&self) -> Option<&[u8]> {
        match self.bytes.whole() {
            Ok(input) => Some(input),
            Err(no_room) => {
                // Two threads refusing at once record one refusal, which is the same refusal.
                let _ = self.scan_refused.set(no_room);
                None
            }
        }
    }

    /// Whether a scan of the file was refused because the process could not hold it whole.
    ///
    /// `None` for every document held in memory and for every document on disk whose
    /// cross-reference table was right about each object it was asked for: a scan is what runs
    /// when the table is disproved — by an offset that names no object, by a `/Root` that leads
    /// nowhere, by a page tree that reaches no page — and reading the file whole is what a scan
    /// is. A document this answers `Some` for has been read as far as its table was right, and
    /// no further: an object the scan would have found is [`Object::Null`] here, and a host
    /// that wants to say why asks this (ADR 0809).
    #[must_use]
    pub fn scan_refused(&self) -> Option<NoRoom> {
        self.scan_refused.get().copied()
    }
}
