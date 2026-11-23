//! The named resource entries ISO 19005 section 6.2.2's exemption takes out of the requirements,
//! removed where the exemption withdrew a finding (ADR 3410).
//!
//! Both parts' section 6.2.2 close with the same sentence: a named resource present in a resources
//! dictionary whose name the associated content stream never references is not used for rendering
//! and is exempt from the part's requirements (part 4 keeps its object-syntax subclauses, part 2
//! under `TechNote` 0010 A010 likewise). `pdf_archive` applies it, so a finding at an object reached
//! only through such an entry is withdrawn, and ADR 3394 holds that withdrawal to be the clause's
//! reading. A validator that does not apply the exemption still fails the file, and the entry draws
//! nothing: so where the exemption withdrew a finding, the output does not carry the entry at all,
//! and the objects only it reached go with it, because the writer copies what the converted
//! document reaches. No mark moves and nothing a reader is shown is lost: a content stream that
//! never names a resource cannot draw it.
//!
//! **Only entries no content stream in the file names at all are removed**
//! ([`pdf_archive::Exempt::removable_entries`]), which is narrower than the exemption: a resources
//! dictionary one stream states and a page inherits besides is associated with the stream alone,
//! and an entry the page uses is not one to take away. Nothing is removed where no finding was
//! withdrawn, because then nothing in the file fails on the entry's account.

use std::collections::BTreeMap;

use pdf_archive::{Entry, Examination, Target};
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

/// The entries to remove, by the object each is written in, in the source's numbering.
#[derive(Debug, Default)]
pub(super) struct Removal {
    /// Each object's entries: the run of entries within it that leads to the category
    /// dictionary, and the resource name keyed there.
    pub(super) at: BTreeMap<ObjectId, Vec<(Vec<Entry>, Name)>>,
}

impl Removal {
    /// The entries, worded for the report: the resource name, and the object it was written in.
    pub(super) fn described(&self) -> Vec<String> {
        self.at
            .iter()
            .flat_map(|(object, entries)| {
                entries.iter().map(move |(through, name)| {
                    let category = through
                        .iter()
                        .rev()
                        .find_map(|entry| match entry {
                            Entry::Key(key) => Some(key.escaped()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    format!(
                        "/{} in object {} {}{}",
                        name.escaped(),
                        object.number,
                        object.generation,
                        if category.is_empty() {
                            String::new()
                        } else {
                            format!(", its /{category} dictionary")
                        }
                    )
                })
            })
            .collect()
    }
}

/// The entries to remove, or `None` where the exemption withdrew no finding of `input`'s or there
/// is no entry it is safe to remove.
pub(super) fn removal(
    document: &Document,
    input: &pdf_archive::Report,
    target: Target,
) -> Option<Removal> {
    if input
        .judgements
        .iter()
        .all(|judgement| judgement.withdrawn == 0)
    {
        return None;
    }
    let examination = Examination::new(document, target);
    let mut removal = Removal::default();
    for entry in examination.exempt().removable_entries(&examination) {
        removal
            .at
            .entry(entry.object)
            .or_default()
            .push((entry.through, entry.name));
    }
    (!removal.at.is_empty()).then_some(removal)
}

/// `dict` with the entry `name` removed from the dictionary `through` leads to inside it, or
/// `None` where `through` leads nowhere or the entry is not there.
pub(super) fn without(dict: &Dictionary, through: &[Entry], name: &Name) -> Option<Dictionary> {
    let Some((first, rest)) = through.split_first() else {
        dict.get_by_name(name)?;
        let mut kept = Dictionary::new();
        for (key, value) in dict.iter().filter(|(key, _)| *key != name) {
            kept.insert(key.clone(), value.clone());
        }
        return Some(kept);
    };
    let Entry::Key(key) = first else {
        return None;
    };
    let inner = within(dict.get_by_name(key)?, rest, name)?;
    let mut out = dict.clone();
    out.insert(key.clone(), inner);
    Some(out)
}

/// The same for a value inside a dictionary: a dictionary or an array the run passes through.
fn within(value: &Object, through: &[Entry], name: &Name) -> Option<Object> {
    match value {
        Object::Dictionary(dict) => without(dict, through, name).map(Object::Dictionary),
        Object::Array(items) => {
            let (Entry::Index(at), rest) = through.split_first()? else {
                return None;
            };
            let inner = within(items.get(*at)?, rest, name)?;
            let mut out = items.clone();
            *out.get_mut(*at)? = inner;
            Some(Object::Array(out))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use pdf_archive::Entry;
    use pdf_syntax::{Dictionary, Name, Object};

    use super::without;

    fn named(bytes: &[u8]) -> Name {
        Name::new(bytes)
    }

    /// A page's direct `/Resources /Properties` dictionary loses the one entry and keeps the rest.
    #[test]
    fn the_entry_is_removed_where_the_run_leads_and_nothing_else_moves() {
        let mut properties = Dictionary::new();
        properties.insert(named(b"MC0"), Object::Integer(1));
        properties.insert(named(b"MC1"), Object::Integer(2));
        let mut resources = Dictionary::new();
        resources.insert(named(b"Properties"), Object::Dictionary(properties));
        let mut page = Dictionary::new();
        page.insert(named(b"Resources"), Object::Dictionary(resources));
        let through = [
            Entry::Key(named(b"Resources")),
            Entry::Key(named(b"Properties")),
        ];
        let out = without(&page, &through, &named(b"MC0")).expect("the entry is there");
        let Some(Object::Dictionary(resources)) = out.get("Resources") else {
            panic!("resources kept");
        };
        let Some(Object::Dictionary(properties)) = resources.get("Properties") else {
            panic!("properties kept");
        };
        assert!(properties.get("MC0").is_none());
        assert_eq!(properties.get("MC1"), Some(&Object::Integer(2)));
        assert!(
            without(&page, &through, &named(b"MC9")).is_none(),
            "nothing to remove"
        );
    }
}
