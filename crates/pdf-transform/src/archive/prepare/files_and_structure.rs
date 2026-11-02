//! The files and structure family of the preparations: the structure tree a `/MarkInfo`
//! `/Marked` flag promises, ISO 19005-2 section 6.7.2.2's subject.
//!
//! It is a family of its own because it is a judgement about the file's logical structure as a
//! whole rather than about any one page, packet or font, and the rows that read it are
//! `decision/files_and_structure`'s, section 6 of `doc/pdfa-blocking-issues.md`.

use super::super::decision::Because;
use super::{NO_CATALOG, NOT_ASKED_FOR};
use pdf_syntax::Document;
use pdf_syntax::object::Dictionary;

/// Why `/MarkInfo` with `/Marked true` is not written into a file with no structure tree.
///
/// `doc/pdf-a-conversion-limits.md` section 5.1, and it is a policy refusal rather than a gap:
/// the flag asserts that the file follows ISO 32000's Tagged PDF conventions, and only the tree
/// itself can make that true. ISO 19005-2 section 6.7.1 advises writers against adding structural
/// information not present in the source solely to achieve conformance, so declining here is the
/// standard's own counsel rather than this converter being strict.
const NO_STRUCTURE_TREE: &str = "ISO 19005-2 section 6.7.2.2's Marked flag says the file follows \
     the base standard's Tagged PDF conventions, and this document states no StructTreeRoot for \
     it to be true of. This converter will not invent a structure tree — deciding that a run of \
     glyphs is a heading and what an image depicts is authoring rather than converting, and \
     section 6.7.1 of that part advises writers against it — so the flag is not written either. \
     PDF/A-2u and PDF/A-2b ask nothing of logical structure and this document may well reach \
     both";

/// Whether the file already carries the structure tree a `/MarkInfo` would be a claim about.
///
/// `doc/pdf-a-conversion-limits.md` section 5.1: the converter will not invent a structure tree,
/// and writing `/Marked true` over a file that has none would be the same act in one key — a
/// claim that the file follows §14.8's conventions, made by the converter rather than
/// demonstrated by the file. So this is the fence rather than a debt, and says so.
pub(super) fn structure_tree(
    wanted: bool,
    document: &Document,
    catalog: Option<&Dictionary>,
) -> Result<(), Because> {
    match (wanted, catalog) {
        (true, Some(catalog)) if !document.get_key(catalog, "StructTreeRoot").is_null() => Ok(()),
        (true, Some(_)) => Err(Because::TheFence(NO_STRUCTURE_TREE)),
        (true, None) => Err(Because::NotBuiltYet(NO_CATALOG)),
        (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
    }
}
