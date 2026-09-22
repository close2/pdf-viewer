//! `preserve` where the content already is: the sites whose clause leaves one way to conform that
//! takes nothing out of the document.
//!
//! `doc/rfc/0007` section 4.6.1 separates two `preserve` mechanisms, a page appended to the
//! document and an attachment, and both *move* what a removal would otherwise take away. At the
//! two sites here nothing has to move. `doc/adr/1285` is the argument; each site in brief:
//!
//! - **ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2** require an annotation's `/F` to
//!   set `Print` and clear `Hidden`, `Invisible`, `NoView` and `ToggleNoView`.
//!   `doc/pdf-a-conversion-limits.md` section 3.7 gives such an annotation two futures, removal
//!   and being shown, and makes removal the default. `preserve` is the other: the annotation stays
//!   and its flags become [`pdf_archive::flags_permitting`]'s value. The population is the one
//!   the removal is prepared over, so this module adds only the report row.
//! - **ISO 19005-2 section 6.2.9.2 and ISO 19005-4 section 6.2.8.2** forbid a reference `XObject`,
//!   and ISO 32000-2 §8.10.4.1 makes the form carrying the `Ref` entry a proxy:
//!
//!   > This form XObject shall serve as a proxy that should be processed by a PDF processor when
//!   > the referenced content is not available.
//!
//!   An archive holds no other file, so the proxy is what its reader draws already, and removing
//!   the entry leaves every reader drawing it. What goes is the pointer, and the report names
//!   the file and page it pointed at.
//!
//! Neither needs anything recorded in `xmpMM:History`. The permissions that make a record a
//! condition, `doc/questions/A55` for a derived file and `doc/adr/1014` for an appended page,
//! are about content this program made or placed, and neither site has any. The report is where
//! each change is named, as for the removals these replace.

use std::collections::BTreeSet;

use pdf_archive::Target;
use pdf_model::file_spec::FileSpec;
use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

use crate::json::Value;

use super::decision::Because;
use super::prepare::RemovedAnnotation;

/// The requirement whose `preserve` shows an annotation its producer hid.
pub(super) const PRINTABLE_AND_VISIBLE: &str = "annotations/printable-and-visible";

/// The requirement whose `preserve` keeps a reference `XObject`'s proxy.
pub(super) const NO_REFERENCE_XOBJECTS: &str = "graphics/no-reference-xobjects";

/// What an operator answering the flag site with `preserve` is agreeing to.
pub(super) const SHOWN: &str = "each annotation whose flags ISO 19005 forbids stays on its page \
     and is shown on screen and on paper, including one its producer hid or kept off one of the \
     two; the report names every annotation shown and the flags it had";

/// What an operator answering the reference `XObject` site with `preserve` is agreeing to.
pub(super) const PROXIED: &str = "each reference XObject keeps the proxy its producer drew and \
     stops pointing at the page of another file it would otherwise import, so a reader that could \
     reach that file draws the proxy as well; the report names every file and page that was \
     named";

/// Why a reference `XObject` held somewhere other than in a stream of its own is not proxied.
const PROXY_THAT_IS_NOT_A_STREAM: &str = "a form XObject naming another file through a Ref entry \
     is held inside another object rather than being a stream of its own, and ISO 32000-2 \
     \u{a7}7.3.8.1 makes every stream an indirect object. Which dictionary the requirement was \
     reported at is then not something this rewrite can act on without guessing";

/// One annotation kept on its page with the flags the requirement asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownAnnotation {
    /// The annotation object.
    pub at: ObjectId,
    /// The zero-based page it is on.
    pub page: usize,
    /// Its `/Subtype`, or the words for an annotation that states none.
    pub subtype: String,
    /// The `/F` its producer wrote.
    pub stated: i64,
    /// The `/F` this conversion wrote.
    pub written: i64,
}

impl ShownAnnotation {
    /// The row for an annotation the removal would have taken, where the flags are why.
    pub(super) fn of(hidden: &RemovedAnnotation) -> Option<Self> {
        let stated = hidden.flags?;
        Some(Self {
            at: hidden.at,
            page: hidden.page,
            subtype: hidden.subtype.clone(),
            stated,
            written: pdf_archive::flags_permitting(stated),
        })
    }

    /// One shown annotation as JSON.
    pub(super) fn to_json(&self) -> Value {
        Value::Object(vec![
            ("page".to_owned(), Value::count(self.page)),
            ("subtype".to_owned(), Value::text(self.subtype.clone())),
            (
                "object".to_owned(),
                Value::text(format!("{} {}", self.at.number, self.at.generation)),
            ),
            ("stated_flags".to_owned(), Value::Integer(self.stated)),
            ("written_flags".to_owned(), Value::Integer(self.written)),
        ])
    }
}

/// One reference `XObject` whose `Ref` entry this conversion removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxiedReference {
    /// The form `XObject`.
    pub at: ObjectId,
    /// The file Table 95's `/F` names, as §7.11's file specification reads, where it reads.
    pub file: Option<String>,
    /// The page Table 95's `/Page` names: an index as a number, or a page label as text.
    pub page: Option<String>,
}

impl ProxiedReference {
    /// One proxied reference as JSON.
    pub(super) fn to_json(&self) -> Value {
        Value::Object(vec![
            (
                "object".to_owned(),
                Value::text(format!("{} {}", self.at.number, self.at.generation)),
            ),
            (
                "file".to_owned(),
                self.file.clone().map_or(Value::Null, Value::text),
            ),
            (
                "page".to_owned(),
                self.page.clone().map_or(Value::Null, Value::text),
            ),
        ])
    }
}

/// The reference `XObject`s whose `Ref` entry goes, and what each named.
#[derive(Debug)]
pub(super) struct ReferenceXObjects {
    /// The form `XObject` streams to rewrite.
    pub(super) at: BTreeSet<ObjectId>,
    /// One row per form, for the report.
    pub(super) proxied: Vec<ProxiedReference>,
}

/// The reference `XObject`s the requirement reported, read off the document once.
///
/// The population is [`pdf_archive::reference_xobjects`]'s, which is the requirement's own walk
/// read as a list, so the rewrite acts on exactly what the requirement found.
pub(super) fn prepare_reference_xobjects(
    document: &Document,
    target: Target,
) -> Result<ReferenceXObjects, Because> {
    let mut at = BTreeSet::new();
    let mut proxied = Vec::new();
    for id in pdf_archive::reference_xobjects(document, target) {
        let Object::Stream(stream) = document.get(id) else {
            return Err(Because::NotBuiltYet(PROXY_THAT_IS_NOT_A_STREAM));
        };
        let stated = document.get_key(&stream.dict, "Ref");
        let reference = stated.as_dict();
        let file = reference.and_then(|dict| {
            let specification = document.get_key(dict, "F");
            let parsed = FileSpec::parse(document, &specification)?;
            parsed.url().or_else(|| parsed.display_name())
        });
        let page = reference.and_then(|dict| match document.get_key(dict, "Page") {
            Object::Integer(index) => Some(index.to_string()),
            Object::String(label) => Some(pdf_syntax::text_string(&label)),
            _ => None,
        });
        at.insert(id);
        proxied.push(ProxiedReference { at: id, file, page });
    }
    Ok(ReferenceXObjects { at, proxied })
}
