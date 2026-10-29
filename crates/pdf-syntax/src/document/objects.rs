//! Object resolution: the trailer, the catalogue, one object by number, and one entry of a
//! dictionary, each followed through §7.3.10's references and remembered once parsed.

use super::{Document, MAX_REFERENCE_DEPTH, read, write};
use crate::error::{SyntaxError, SyntaxResult};
use crate::object::{Dictionary, Name, Object, ObjectId};
use crate::xref::{Location, XrefTable};

/// What [`Document::get_key_of`] found in the object cache before it let the lock go.
///
/// Three answers rather than two, because "the object has no such entry" and "the object is
/// not the kind of thing that has entries" are different statements and its caller needs both.
enum Held {
    /// The entry as the dictionary states it, still unresolved. [`Object::Null`] where the
    /// dictionary has no such key, which is what §7.3.9 makes an omitted entry.
    Entry(Object),
    /// The object is in hand and is not a dictionary, so it has no entries at all.
    NotADictionary,
    /// Nothing is in hand: the object has not been parsed yet, or it is itself a reference
    /// and following §7.3.10's chain is [`Document::resolve`]'s job rather than this lock's.
    Unread,
}

impl Document {
    /// Returns the trailer dictionary.
    #[must_use]
    pub fn trailer(&self) -> &Dictionary {
        self.xref.trailer()
    }

    /// Returns the cross-reference table.
    #[must_use]
    pub fn xref(&self) -> &XrefTable {
        &self.xref
    }

    /// Returns the document catalogue.
    ///
    /// # Errors
    ///
    /// [`SyntaxError::TrailerMissing`] when the trailer has no `/Root`, or it does not
    /// resolve to a dictionary. Without a catalogue there is no page tree, so this is fatal
    /// rather than recoverable.
    pub fn catalog(&self) -> SyntaxResult<Dictionary> {
        let root = self
            .trailer()
            .get("Root")
            .cloned()
            .ok_or(SyntaxError::TrailerMissing { key: "/Root" })?;

        self.resolve(&root)
            .as_dict()
            .cloned()
            .ok_or(SyntaxError::TrailerMissing {
                key: "/Root (not a dictionary)",
            })
    }

    /// Fetches an indirect object, returning [`Object::Null`] if it cannot be read.
    ///
    /// Null rather than an error because the specification says a reference to a
    /// non-existent object *is* null. A missing object is therefore ordinary, not
    /// exceptional, and forcing every caller to handle a `Result` for it would bury the
    /// cases that matter.
    #[must_use]
    pub fn get(&self, id: ObjectId) -> Object {
        if let Some(cached) = read(&self.cache).get(&id.number) {
            return cached.clone();
        }

        // Loading an object can need other objects: an indirect `/Length`, an indirect
        // `/Filter`, or the object stream a compressed object lives in. A file may point
        // any of those at the object being loaded, and the resulting recursion is bounded
        // by nothing in the parser, so it is bounded here. Null is what §7.3.10 gives a
        // reference that resolves to nothing, which is what a cycle amounts to.
        if !self.begin_loading(id.number) {
            return Object::Null;
        }
        let object = self.load(id).unwrap_or(Object::Null);
        self.end_loading(id.number);

        write(&self.cache).insert(id.number, object.clone());
        object
    }

    /// Records that this thread has begun loading `number`, or reports a cycle.
    ///
    /// # Why the set is per thread
    ///
    /// It is a guard on the *call stack*, not on the document: `get` is re-entrant because
    /// loading an object can need an indirect `/Length`, an indirect `/Filter` or the object
    /// stream it lives in, and a file may point any of those back at the object being loaded.
    /// A set shared between threads would answer §7.3.10's null to a second thread merely
    /// because a first happened to be reading the same object — a wrong answer produced by
    /// timing, which is the one kind this program must not have. Two threads loading one
    /// object duplicate the parse instead, and the cache makes that the last time.
    pub(super) fn begin_loading(&self, number: u32) -> bool {
        write(&self.loading)
            .entry(std::thread::current().id())
            .or_default()
            .insert(number)
    }

    /// Records that this thread has finished loading `number`.
    ///
    /// Empties are removed so that the map is bounded by the threads *currently* inside
    /// [`Self::get`] rather than by every thread that has ever touched the document.
    pub(super) fn end_loading(&self, number: u32) {
        let mut loading = write(&self.loading);
        let thread = std::thread::current().id();
        if let Some(numbers) = loading.get_mut(&thread) {
            numbers.remove(&number);
            if numbers.is_empty() {
                loading.remove(&thread);
            }
        }
    }

    /// Resolves an object if it is a reference, following chains.
    ///
    /// A reference to a reference is unusual but legal. The chain is bounded by
    /// `MAX_REFERENCE_DEPTH`, and a cycle resolves to null rather than looping.
    #[must_use]
    pub fn resolve(&self, object: &Object) -> Object {
        let mut current = object.clone();
        for _ in 0..MAX_REFERENCE_DEPTH {
            match current {
                Object::Reference(id) => current = self.get(id),
                other => return other,
            }
        }
        Object::Null
    }

    /// Looks up a key in a dictionary and resolves the result.
    ///
    /// The common operation by a wide margin: nearly every value in a PDF may be indirect,
    /// so a bare `dict.get` is almost always a bug waiting to happen.
    #[must_use]
    pub fn get_key(&self, dict: &Dictionary, key: &str) -> Object {
        dict.get(key)
            .map_or(Object::Null, |object| self.resolve(object))
    }

    /// Looks up a key stated by a **name the document itself supplies**, and resolves the result.
    ///
    /// [`Self::get_key`] is for a key ISO 32000-2 names — `/Type`, `/Kids`, `/CharProcs` — which
    /// is an ASCII literal in this source. This one is for the other half: a key whose bytes came
    /// out of the file, such as the glyph name a Type 3 `/Encoding` maps a code to or the
    /// appearance state an annotation's `/AS` selects. Those must be probed with their own bytes,
    /// because §7.3.5 makes two names one name only when "the resulting sequences of bytes are …
    /// an exact binary match" — and a name carrying a byte no UTF-8 sequence begins survives no
    /// trip through `str`.
    #[must_use]
    pub fn get_key_by_name(&self, dict: &Dictionary, key: &Name) -> Object {
        dict.get_by_name(key)
            .map_or(Object::Null, |object| self.resolve(object))
    }

    /// Looks up one key **inside** an indirect object, without copying the rest of it.
    ///
    /// [`Self::get`] hands back a clone of the whole object, because that is the only thing a
    /// value behind a lock can be handed back as. Where a caller wants one entry of a large
    /// dictionary that is only a step on the way to somewhere else, the copy is the cost.
    /// ISO 32000-2 §7.7.3.2's `/Kids` is "an array of indirect references", so a page tree's
    /// walk steps over its neighbours, and asking each of them for Table 30's `/Count` through
    /// `get` deep-copies a page dictionary — its `/Annots`, its `/Resources`, all of it — to
    /// read one number. Over one sweep of ISO 32000-2 that was 17.1% of the whole search;
    /// ADR 0330 has the A/B.
    ///
    /// `None` says the object is **not a dictionary**, which is a different answer from
    /// [`Object::Null`] and is why this returns an `Option` where [`Self::get_key`] does not:
    /// a caller that must tell "this node has no such entry" from "this is not a node" cannot
    /// do it from a null, and the page tree is exactly such a caller — a `/Kids` entry naming
    /// something that is not a dictionary is not a page and must not be counted as one.
    ///
    /// The answer is [`Self::get_key`]'s in every other respect: the entry is resolved, and an
    /// absent one is [`Object::Null`].
    #[must_use]
    pub fn get_key_of(&self, id: ObjectId, key: &str) -> Option<Object> {
        self.stated_key_of(id, key)
            .map(|stated| self.resolve(&stated))
    }

    /// One entry inside an indirect object as the file states it, a reference left unfollowed.
    ///
    /// [`Self::get_key_of`]'s answer before §7.3.10's chain is followed: `None` where the object
    /// is not a dictionary, [`Object::Null`] where the entry is absent, and otherwise the entry
    /// itself. It exists for a caller that shares what an entry names among everything naming
    /// the same object, which ISO 32000-2 §7.8.3 describes for a page's resources:
    ///
    /// > If the content streams of multiple pages require exactly the same set of resources, a
    /// > single Resources dictionary may be shared between them by using indirect references.
    ///
    /// Only the reference says that the pages name one object rather than equal ones (ADR 2588).
    #[must_use]
    pub fn stated_key_of(&self, id: ObjectId, key: &str) -> Option<Object> {
        // The guard is dropped before anything is resolved, because resolving can load another
        // object and that takes this same lock to write.
        let held = {
            let cache = read(&self.cache);
            match cache.get(&id.number) {
                Some(Object::Reference(_)) | None => Held::Unread,
                Some(object) => object.as_dict().map_or(Held::NotADictionary, |dict| {
                    Held::Entry(dict.get(key).cloned().unwrap_or(Object::Null))
                }),
            }
        };
        match held {
            Held::Entry(value) => Some(value),
            Held::NotADictionary => None,
            // Either nothing has parsed this object yet, or it is itself a reference and
            // §7.3.10's chain is `resolve`'s to follow. Both are the ordinary path, and the
            // copy it makes is made once per object rather than once per lookup.
            Held::Unread => self
                .resolve(&Object::Reference(id))
                .as_dict()
                .map(|dict| dict.get(key).cloned().unwrap_or(Object::Null)),
        }
    }

    /// Loads an object from wherever the cross-reference table says it is.
    pub(super) fn load(&self, id: ObjectId) -> Option<Object> {
        let Some(location) = self.xref.location(id.number) else {
            // The table's own header said an entry for this number was there and the entry
            // itself was not read, so nothing has said the number names nothing — which is
            // what `XrefTable::declared_and_unread` separates from an absent entry. The
            // object's own header is then the file's one surviving statement about it, read
            // exactly as `load_by_header` reads it for an entry that was disproved.
            return self
                .xref
                .declared_and_unread(id.number)
                .then(|| self.load_by_header(id))
                .flatten();
        };
        match location {
            Location::Offset(offset) => {
                // A table pointing at the wrong object is a real corruption. Returning
                // another object's contents under this number would corrupt the document
                // graph silently, so what is at the offset is used only when it says it is
                // this object — and where it does not, the file's own headers are asked.
                match self.parse_at(offset) {
                    Some((found, object)) if found.number == id.number => Some(object),
                    _ => self.load_by_header(id),
                }
            }
            Location::InStream { stream, index } => {
                let expanded = self.expand_object_stream(stream)?;
                // Objects in a stream are addressed by their own number, and `index` is
                // only a hint about ordering, so the number is authoritative.
                expanded
                    .get(&id.number)
                    .cloned()
                    .or_else(|| expanded.values().nth(index as usize).cloned())
            }
        }
    }
}
