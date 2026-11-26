//! The trailer's document information dictionary, read the one way every rewriting writer reads
//! it — ISO 32000-2 §14.3.3 and §7.5.5.
//!
//! Table 15 printed `/Info` as "( Optional; shall be an indirect reference )", and Errata
//! Collection 3's Issue #106 (approved) struck "; shall be an indirect reference": a trailer that
//! states the dictionary directly is a valid file. A writer that asks the entry for a reference
//! alone drops that dictionary, and with it the `/ModDate` Table 349 makes "( Required if
//! `PieceInfo` is present in the document catalog dictionary; otherwise optional; PDF 1.1 )" beside
//! a catalog it carries whole. So `optimize`, `redact`, `split`, the archive converter's two
//! rewriters and `update`'s `SetInformation` read it here, and each carries a direct dictionary
//! as well as an indirect one (ADR 3719).
//!
//! A carried direct dictionary is written as an indirect object of its own: Table 15 no longer
//! asks for one, and none forbids it, so the output states the same dictionary in the shape every
//! writer here already writes.

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Object, ObjectId};
use pdf_syntax::serialize::{Assembly, AssemblyError};

/// The deepest a direct dictionary's nesting is followed when its references are carried.
const MAX_DEPTH: usize = 64;

/// How a source's trailer states its information dictionary.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Information {
    /// An indirect reference to an object that is a dictionary.
    Indirect(ObjectId),
    /// The dictionary itself, written in the trailer.
    Direct(Dictionary),
}

impl Information {
    /// The dictionary, resolved.
    pub(crate) fn dictionary(&self, document: &Document) -> Dictionary {
        match self {
            Self::Indirect(id) => document.get(*id).as_dict().cloned().unwrap_or_default(),
            Self::Direct(dict) => dict.clone(),
        }
    }
}

/// The information dictionary `document`'s trailer states, direct or indirect; `None` where it
/// states none, or names an object that is not a dictionary (§7.3.10 makes a reference to a
/// missing object a null, and a null states no dictionary).
pub(crate) fn stated(document: &Document) -> Option<Information> {
    match document.trailer().get("Info")? {
        Object::Reference(id) => document
            .get(*id)
            .as_dict()
            .is_some()
            .then_some(Information::Indirect(*id)),
        Object::Dictionary(dict) => Some(Information::Direct(dict.clone())),
        _ => None,
    }
}

/// The information dictionary as the output will hold it, before it has a number.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Carried {
    /// The source's indirect dictionary, already copied to this number.
    Placed(ObjectId),
    /// The source's direct dictionary, with every reference in it in the output's numbering.
    Restated(Dictionary),
}

impl Carried {
    /// The number the output's trailer names: the copy's, or a new object holding the restated
    /// dictionary.
    ///
    /// # Errors
    ///
    /// [`AssemblyError::TooManyObjects`] where the new object has no number left.
    pub(crate) fn place(self, assembly: &mut Assembly<'_>) -> Result<ObjectId, AssemblyError> {
        match self {
            Self::Placed(id) => Ok(id),
            Self::Restated(dict) => assembly.add(Object::Dictionary(dict)),
        }
    }
}

/// Carries `document`'s information dictionary with the writer's own copy: `copy` copies one
/// indirect object with whatever it reaches and answers its output number, or `None` where the
/// writer does not carry it (a reference it drops is then written as null, §7.3.10).
///
/// `Ok(None)` where the trailer states no dictionary.
///
/// # Errors
///
/// Whatever `copy` answers.
pub(crate) fn carry<E>(
    document: &Document,
    mut copy: impl FnMut(ObjectId) -> Result<Option<ObjectId>, E>,
) -> Result<Option<Carried>, E> {
    match stated(document) {
        None => Ok(None),
        Some(Information::Indirect(id)) => Ok(copy(id)?.map(Carried::Placed)),
        Some(Information::Direct(dict)) => {
            let restated = restate(&Object::Dictionary(dict), &mut copy, 0)?;
            Ok(restated.as_dict().cloned().map(Carried::Restated))
        }
    }
}

/// `value` with each reference it holds copied through `copy`.
fn restate<E>(
    value: &Object,
    copy: &mut impl FnMut(ObjectId) -> Result<Option<ObjectId>, E>,
    depth: usize,
) -> Result<Object, E> {
    if depth >= MAX_DEPTH {
        return Ok(Object::Null);
    }
    Ok(match value {
        Object::Reference(id) => copy(*id)?.map_or(Object::Null, Object::Reference),
        Object::Array(items) => Object::Array(
            items
                .iter()
                .map(|item| restate(item, copy, depth.saturating_add(1)))
                .collect::<Result<_, E>>()?,
        ),
        Object::Dictionary(dict) => {
            let mut out = Dictionary::new();
            for (key, entry) in dict.iter() {
                out.insert(key.clone(), restate(entry, copy, depth.saturating_add(1))?);
            }
            Object::Dictionary(out)
        }
        other => other.clone(),
    })
}
