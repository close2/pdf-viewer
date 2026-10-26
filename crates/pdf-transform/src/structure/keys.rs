//! The parent-tree keys the carry hands out (ISO 32000-2 §14.7.5.4) and the plan's small answers.

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::Carry;

impl Carry {
    /// The `/StructParents` this output page states, where the carry gave it one.
    pub(crate) fn page_key(&self, placed: ObjectId) -> Option<i64> {
        self.page_keys.get(&placed).copied()
    }

    /// A fresh §14.7.5.4 key for an object that is a content item in its own right.
    ///
    /// Called as the closure walk reaches an annotation or an `XObject` stating `/StructParent`,
    /// so the key it is given is a function of the order the output's objects are numbered in —
    /// deterministic, which is RFC 0002 section 9's first layer.
    ///
    /// `None` where the source's parent tree has no entry for the object's key, or where the
    /// entry names an element this output does not hold: the caller then writes **no**
    /// `/StructParent` at all. A key into a tree the output does state, naming nothing, is the
    /// outcome ADR 0831 named as worse than an absent one — it would tell an assistive processor
    /// that the content has a parent element and then hand it nothing.
    pub(crate) fn object_key(&mut self, document: &Document, at: usize, old: i64) -> Option<i64> {
        let entry = Self::source_entry(document, old)?;
        let id = entry.as_reference()?;
        if !self.placed.contains_key(&(at, id)) {
            return None;
        }
        let key = self.next_key;
        self.next_key = self.next_key.saturating_add(1);
        self.object_keys.push((at, old, key));
        Some(key)
    }

    /// The output's `/StructTreeRoot`, for the catalog.
    pub(crate) fn root(&self) -> ObjectId {
        self.root
    }

    /// Table 353's mark information dictionary, where any flag is worth stating.
    ///
    /// Table 353 gives all three a default of `false`, so a dictionary of three false flags says
    /// exactly what saying nothing says; the entry is written only where one of them is true.
    pub(crate) fn mark_info(&self) -> Option<Object> {
        if !self.marked && !self.user_properties && !self.suspects {
            return None;
        }
        let mut dict = Dictionary::new();
        for (key, flag) in [
            ("Marked", self.marked),
            ("UserProperties", self.user_properties),
            ("Suspects", self.suspects),
        ] {
            if flag {
                dict.insert(Name::new(key.as_bytes()), Object::Boolean(true));
            }
        }
        Some(Object::Dictionary(dict))
    }
}
