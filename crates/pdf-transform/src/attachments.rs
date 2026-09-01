//! `attachments` — §7.11.4's embedded files, listed or extracted, RFC 0002 section 6.6's read
//! direction.
//!
//! Plumbing over readers that already ship. `pdf_model::attachment::attachments` reads two of
//! the homes an embedded file has — the catalog's `/EmbeddedFiles` name tree (§7.7.4) and the
//! catalog's `/AF` associated files (§14.13) — deduplicated by stream, exactly as the viewer's
//! files panel lists them. The third home is §12.5.6.15's file attachment annotation, which
//! "contains a reference to a file, which typically shall be embedded in the PDF file", under
//! Table 187's required `/FS` and reachable from no tree: every page's `/Annots` is walked for
//! `/Subtype /FileAttachment` and `pdf_model::attachment::of_annotation` reads each — the
//! viewer's own reader, which is what makes the annotation's `/Contents` the description, as
//! that clause's one `shall` requires. A file the tree names *and* an annotation carries is one
//! file, deduplicated by stream as the first two homes already are, and listed once under the
//! home that came first: the tree's. An annotation's file is listed with its page.
//!
//! The order is the document's: the tree's files, then `/AF`'s, then the annotations' page by
//! page in `/Annots` order, so an ordinal names the same file on every run.
//!
//! # The write direction, on §7.5.6 alone
//!
//! `--attach` (pdftk's `attach_files`) is RFC 0002 section 6.6's "smallest consumer of a
//! writer", and the writer it consumes is the one this tree already has: §7.5.6's incremental
//! update, `pdf_syntax::write::incremental_update`, the path the viewer saves a filled field
//! or an added annotation through (ADRs 0100, 0121). Nothing of the source is rewritten —
//! "changes shall be appended to the end of the file, leaving its original contents intact" —
//! so the output is the source's bytes, byte for byte, and then three new objects and one
//! replaced one:
//!
//! - §7.11.4's embedded file stream (Table 44's `/Type /EmbeddedFile`, Table 45's `/Params`
//!   with `/Size` and `/CheckSum` from the bytes, and the dates only where the caller stated
//!   one — RFC 0002 section 9: no clock);
//! - §7.11.3's file specification dictionary, indirect because Table 43 requires it where
//!   `/EF` is present, with `/F` and `/UF` both the name, `/EF` naming the stream under both
//!   keys, and `/Desc` where given;
//! - a new root for §7.7.4's `/EmbeddedFiles` name tree — every entry the old tree held, as
//!   the tree stated it, plus the new one, in one `/Names` node sorted as §7.9.6 requires;
//! - and whichever object held the tree, pointed at the new root: the old root's object where
//!   the tree was indirect, the name dictionary's where that was, and the catalog otherwise.
//!
//! **The whole tree is rewritten as one node rather than one leaf edited in place, and that is
//! a choice with a cost.** §7.9.6 permits it — "[i]f the root node has a Names entry, it shall
//! be the only node in the tree" — and it makes the update the same three objects whatever
//! shape the producer chose; the cost is a document with thousands of embedded files paying
//! for all of them in one array, which no document in the corpus has. The values are kept as
//! the old leaves stated them, references included, so no file specification is copied.
//!
//! `pdf_syntax::Document` stays immutable: the update is a map of objects beside it, exactly
//! as `ViewState::save` builds one, and the document is read, never changed. The edit is
//! Table 22's bit 4 — see [`crate::Operation::Modify`]. ADR 0802.
//!
//! # The third home, written: `--to-page`
//!
//! The same embedded file stream and file specification, filed by a §12.5.6.15 file attachment
//! annotation on a page instead of by the name tree — the home a table of data "can use … to
//! link to a spreadsheet file based on that data". Table 187's `/FS` is the specification, its
//! `/Name` one of the four the clause names ("PDF writers should include this entry"), and the
//! description goes in the annotation's `/Contents`, which the clause's one `shall` makes the
//! text a reader shows: "Interactive PDF processors shall use this entry rather than the
//! optional Desc entry". The page's `/Annots` is rewritten where it is — the array's own object
//! where the page references one, the page otherwise — for the reason `ViewState::save` gives:
//! inlining a referenced array would leave the old object in the file saying something else.
//! No `/AP` is written: this tree synthesises the four icons (`pdf_model::icon`), and a stream
//! this crate drew would be a second artwork for the same clause. Where the annotation sits when
//! nobody says is a choice the standard leaves open, and [`OnPage::rect`] states it. ADR 0803.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::Arc;

use pdf_model::Pages;
use pdf_model::attachment::{Attachment, attachments, of_annotation};
use pdf_syntax::{Date, Dictionary, Document, Name, Object, ObjectId, Stream, tree};

use crate::json::Value;
use crate::pattern::{Fill, Pattern};
use crate::{Declined, Listed, Origin, Output, Refusal, Report, Sinks, Warning};

/// §7.11.4's embedded files.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachmentsPlan {
    /// Which source.
    pub source: usize,
    /// What to do with them.
    pub action: Action,
}

/// What an attachments plan does.
///
/// `PartialEq` without `Eq`, because [`OnPage`]'s rectangle is four reals.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Inventory only.
    List,
    /// Every embedded file, each named by the pattern — `%t` is the file's own name,
    /// sanitised, and `%d` its ordinal.
    SaveAll {
        /// How the outputs are named.
        names: Pattern,
    },
    /// One embedded file, by the name the document files it under or its own file name.
    Save {
        /// The name asked for.
        name: String,
        /// How the output is named; `%t` is the file's own name.
        names: Pattern,
    },
    /// A file written into the document as a new §7.11.4 embedded file, filed in §7.7.4's
    /// `/EmbeddedFiles` tree, by §7.5.6's incremental update. The one output is the whole
    /// updated document.
    Attach {
        /// The file's bytes.
        payload: Payload,
        /// The name it is filed under — Table 32's key, which "should match the value of F or
        /// UF", and so Table 43's `/F` and `/UF` as well.
        name: String,
        /// Table 43's `/Desc`, where the caller has one.
        description: Option<String>,
        /// Table 45's `/CreationDate` and `/ModDate`, both this, where the caller states one.
        /// None otherwise: this crate has no clock, and the same attachment is the same bytes
        /// on every run (RFC 0002 section 9).
        date: Option<Date>,
        /// How the output is named; `%t` is the attached file's name.
        names: Pattern,
        /// Filed by a §12.5.6.15 annotation on a page rather than by the name tree.
        on_page: Option<OnPage>,
    },
    /// An embedded file taken out of §7.7.4's `/EmbeddedFiles` tree by §7.5.6's incremental
    /// update: the tree rewritten without the entry, and the objects the entry alone reached
    /// marked free in the new cross-reference section. The one output is the whole updated
    /// document.
    Remove {
        /// The name the tree files it under.
        name: String,
        /// How the output is named; `%t` is the removed file's name.
        names: Pattern,
    },
}

/// Where a `--to-page` attachment's annotation goes.
#[derive(Debug, Clone, PartialEq)]
pub struct OnPage {
    /// The page, counted from 1 as a person counts.
    pub page: usize,
    /// Table 166's `/Rect`, `[x0 y0 x1 y1]` in the page's user space, where the caller stated
    /// one.
    ///
    /// **Where nobody states one, a 20-unit square 20 units in from the crop box's upper-left
    /// corner**, and that is a choice the standard leaves open: §12.5.6.15 says where the file
    /// goes and nothing about where the icon does. Twenty units is the side this tree draws a
    /// text annotation's icon at, and the upper-left is where a reader's eye starts on a page
    /// in a left-to-right script; a page rotated by `/Rotate` keeps the same user-space corner,
    /// which a caller who minds states a rectangle for.
    pub rect: Option<[f32; 4]>,
    /// Table 187's `/Name`, one of `Graph`, `PushPin`, `Paperclip` and `Tag`; `PushPin` — the
    /// table's default — where none is given, and written either way, because the table asks
    /// writers to include the entry.
    pub icon: Option<String>,
}

impl OnPage {
    /// §12.5.6.15's four icon names, which are the only ones this tree draws.
    pub const ICONS: [&'static str; 4] = ["Graph", "PushPin", "Paperclip", "Tag"];

    /// Table 187's default.
    pub const DEFAULT_ICON: &'static str = "PushPin";

    /// The side of the square placed where no rectangle is stated, in user-space units.
    pub const DEFAULT_SIDE: f32 = 20.0;
}

/// The bytes of a file to attach, which print as their length rather than themselves.
#[derive(Clone, PartialEq, Eq)]
pub struct Payload(Arc<[u8]>);

impl Payload {
    /// The file.
    pub fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self(bytes.into())
    }

    /// Its bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Payload({} bytes)", self.0.len())
    }
}

/// One embedded file, as the inventory describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentEntry {
    /// Which source.
    pub source: usize,
    /// The name the document files it under — the name-tree key, or the file specification's
    /// name where it came by `/AF`.
    pub name: String,
    /// Table 43's `/UF` or `/F`, the file's own name.
    pub file_name: Option<String>,
    /// Table 43's `/Desc`.
    pub description: Option<String>,
    /// Table 45's `/Subtype`, the media type.
    pub media_type: Option<String>,
    /// Table 46's `/Size`, the uncompressed length the file states.
    pub size: Option<i64>,
    /// Table 46's `/CreationDate`, as the file spells it.
    pub created: Option<String>,
    /// Table 46's `/ModDate`.
    pub modified: Option<String>,
    /// Table 43's `/AFRelationship`.
    pub relationship: String,
    /// The page whose §12.5.6.15 annotation carries it, counted from 1, where that is the
    /// home it was found in.
    pub page: Option<usize>,
}

impl AttachmentEntry {
    /// The entry from the reader's own record.
    fn of(source: usize, attachment: &Attachment, page: Option<usize>) -> Self {
        Self {
            source,
            page,
            name: attachment.name.clone(),
            file_name: attachment.file_name.clone(),
            description: attachment.description.clone(),
            media_type: attachment.media_type.clone(),
            size: attachment.size,
            created: attachment.created.clone(),
            modified: attachment.modified.clone(),
            relationship: format!("{:?}", attachment.relationship),
        }
    }

    /// The entry as JSON.
    pub(crate) fn to_json(&self) -> Value {
        Value::Object(vec![
            ("kind".to_owned(), Value::text("attachment")),
            ("source".to_owned(), Value::count(self.source)),
            ("name".to_owned(), Value::text(self.name.clone())),
            (
                "file_name".to_owned(),
                Value::optional(self.file_name.clone()),
            ),
            (
                "description".to_owned(),
                Value::optional(self.description.clone()),
            ),
            (
                "media_type".to_owned(),
                Value::optional(self.media_type.clone()),
            ),
            (
                "size".to_owned(),
                self.size.map_or(Value::Null, Value::Integer),
            ),
            ("created".to_owned(), Value::optional(self.created.clone())),
            (
                "modified".to_owned(),
                Value::optional(self.modified.clone()),
            ),
            (
                "relationship".to_owned(),
                Value::text(self.relationship.clone()),
            ),
            ("page".to_owned(), Value::optional_count(self.page)),
        ])
    }
}

/// Runs the verb.
pub(crate) fn run(
    plan: &AttachmentsPlan,
    document: &Document,
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<(), Refusal> {
    let all = every_home(document);
    match &plan.action {
        Action::List => {
            report.listed.extend(all.iter().map(|(attachment, page)| {
                Listed::Attachment(AttachmentEntry::of(plan.source, attachment, *page))
            }));
            Ok(())
        }
        Action::SaveAll { names } => {
            if !names.distinguishes(all.len()) && !names.names_a_title() {
                return Err(Refusal::Pattern(format!(
                    "{} files would be written and the output name {:?} has neither %d nor %t \
                     to tell them apart",
                    all.len(),
                    names.to_string()
                )));
            }
            let count = all.len();
            for (at, (attachment, page)) in all.iter().enumerate() {
                save(
                    plan,
                    document,
                    sinks,
                    names,
                    at.saturating_add(1),
                    count,
                    attachment,
                    *page,
                    report,
                )?;
            }
            Ok(())
        }
        Action::Save { name, names } => {
            let (attachment, page) = all
                .iter()
                .find(|(attachment, _)| {
                    attachment.name == *name || attachment.file_name.as_deref() == Some(name)
                })
                .ok_or_else(|| Refusal::NoSuchAttachment {
                    at: plan.source,
                    name: name.clone(),
                })?;
            save(
                plan, document, sinks, names, 1, 1, attachment, *page, report,
            )
        }
        Action::Attach {
            payload,
            name,
            description,
            date,
            names,
            on_page,
        } => attach(
            plan,
            document,
            sinks,
            &Attaching {
                payload,
                name,
                description: description.as_deref(),
                date: *date,
                names,
                on_page: on_page.as_ref(),
            },
            report,
        ),
        Action::Remove { name, names } => remove(plan, document, sinks, name, names, report),
    }
}

/// What one attach plan states, borrowed.
struct Attaching<'a> {
    /// The file.
    payload: &'a Payload,
    /// Its filing name.
    name: &'a str,
    /// Table 43's `/Desc`.
    description: Option<&'a str>,
    /// Table 45's dates.
    date: Option<Date>,
    /// The output's name.
    names: &'a Pattern,
    /// A page's annotation rather than the tree.
    on_page: Option<&'a OnPage>,
}

/// Appends §7.5.6's update carrying one new embedded file, and writes the whole file.
fn attach(
    plan: &AttachmentsPlan,
    document: &Document,
    sinks: &dyn Sinks,
    attaching: &Attaching<'_>,
    report: &mut Report,
) -> Result<(), Refusal> {
    let at = plan.source;
    let catalog = document
        .catalog()
        .map_err(|error| Refusal::Unopenable { at, error })?;
    // §7.5.5 makes `/Root` "an indirect reference", and the catalog is the fallback holder of
    // the tree, so an object number for it is needed before anything is built.
    let Some(Object::Reference(root_id)) = document.trailer().get("Root").cloned() else {
        return Err(Refusal::Update {
            at,
            error: pdf_syntax::write::UpdateError::NoRoot,
        });
    };

    // Table 32's key is a text string; §7.9.6 compares keys "on a simple byte-by-byte basis".
    let key = pdf_syntax::text_string::encode_text_string(attaching.name);

    let mut next = next_object_number(document);
    let mut fresh = || {
        let id = ObjectId {
            number: next,
            generation: 0,
        };
        next = next.saturating_add(1);
        id
    };
    let stream_id = fresh();
    let specification_id = fresh();

    let mut replacements: BTreeMap<ObjectId, Object> = BTreeMap::new();
    replacements.insert(
        stream_id,
        embedded_file_stream(attaching.payload.bytes(), attaching.date),
    );
    replacements.insert(
        specification_id,
        file_specification(&key, stream_id, attaching.description),
    );

    if let Some(on_page) = attaching.on_page {
        let annotation_id = fresh();
        file_on_page(
            document,
            at,
            on_page,
            annotation_id,
            specification_id,
            attaching.description,
            &mut replacements,
        )?;
    } else {
        let TreeState {
            names_entry,
            names_dict,
            tree_entry,
            mut entries,
        } = TreeState::read(document, &catalog);
        if entries.iter().any(|(existing, _)| *existing == key) {
            return Err(Refusal::AttachmentExists {
                at,
                name: attaching.name.to_owned(),
            });
        }
        // §7.9.6: "[t]he keys shall be sorted in lexical order", and "[s]horter keys shall
        // appear before longer ones beginning with the same byte sequence" — which is what a
        // byte vector's own ordering does.
        entries.push((key, Object::Reference(specification_id)));
        let tree_id = fresh();
        replacements.insert(tree_id, tree_root(entries));
        point_holder_at_tree(
            &mut replacements,
            tree_id,
            Holder {
                catalog,
                root_id,
                names_entry,
                names_dict,
                tree_entry,
            },
        );
    }

    let bytes = pdf_syntax::write::incremental_update(document, &replacements)
        .map_err(|error| Refusal::Update { at, error })?;

    let expanded = attaching.names.expand(&Fill {
        ordinal: 1,
        count: 1,
        page: None,
        label: None,
        title: Some(attaching.name),
    });
    sinks
        .open(&expanded.name)
        .and_then(|mut sink| sink.write_all(&bytes).and_then(|()| sink.flush()))
        .map_err(|error| Refusal::Sink {
            name: expanded.name.clone(),
            error,
        })?;
    report.outputs.push(Output {
        name: expanded.name,
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        sanitised: expanded.sanitised,
        origin: Origin::Updated {
            source: at,
            attached: attaching.name.to_owned(),
        },
    });
    Ok(())
}

/// Appends §7.5.6's update that takes one entry out of the `/EmbeddedFiles` tree, and writes
/// the whole file.
///
/// The tree is rewritten as one node without the entry — the same shape [`attach`] writes, for
/// the same reason — and the objects the entry alone reached are marked free in the new
/// section: the file specification, where the tree held it by reference, and the embedded file
/// streams its `/EF` names. **Alone** is the condition: §7.11.4.1 gives an embedded file more
/// than one home, and a stream that an `/AF` entry or a page's annotation still reaches is not
/// deleted by the tree letting go of it, so it is left in use and the report says so. What is
/// freed stays in the file byte for byte — §7.5.6: "[d]eleted objects shall be left unchanged
/// in the PDF file, but shall be marked as deleted by means of their cross-reference entries" —
/// and `pdf_syntax::write::incremental_update_freeing` says which of §7.5.4's two mechanisms an
/// update can use and why.
fn remove(
    plan: &AttachmentsPlan,
    document: &Document,
    sinks: &dyn Sinks,
    name: &str,
    names: &Pattern,
    report: &mut Report,
) -> Result<(), Refusal> {
    let at = plan.source;
    let catalog = document
        .catalog()
        .map_err(|error| Refusal::Unopenable { at, error })?;
    let Some(Object::Reference(root_id)) = document.trailer().get("Root").cloned() else {
        return Err(Refusal::Update {
            at,
            error: pdf_syntax::write::UpdateError::NoRoot,
        });
    };
    let TreeState {
        names_entry,
        names_dict,
        tree_entry,
        entries,
    } = TreeState::read(document, &catalog);
    let key = pdf_syntax::text_string::encode_text_string(name);
    let Some(position) = entries.iter().position(|(existing, _)| *existing == key) else {
        return Err(Refusal::NoSuchAttachment {
            at,
            name: name.to_owned(),
        });
    };
    let mut entries = entries;
    let (_, value) = entries.remove(position);

    // The objects this entry alone reached: the specification where the leaf held a
    // reference, and the streams under its `/EF` — unless another home still reaches one.
    let mut freed: Vec<ObjectId> = Vec::new();
    let specification = document.resolve(&value);
    if let Some(specification) = specification.as_dict() {
        let mut streams: Vec<ObjectId> = Vec::new();
        if let Some(embedded) = document.get_key(specification, "EF").as_dict() {
            for entry_key in ["F", "UF", "DOS", "Mac", "Unix"] {
                if let Some(Object::Reference(id)) = embedded.get(entry_key)
                    && !streams.contains(id)
                {
                    streams.push(*id);
                }
            }
        }
        let elsewhere = other_homes(document, &streams);
        if elsewhere.is_empty() {
            if let Object::Reference(id) = value {
                freed.push(id);
            }
            freed.extend(streams);
        } else {
            report.warnings.push(Warning {
                source: at,
                page: None,
                detail: format!(
                    "{name}: its file specification and stream are left in use, because \
                     another home in the document still reaches the stream ({elsewhere})"
                ),
            });
        }
    }

    let mut replacements: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let tree_id = ObjectId {
        number: next_object_number(document),
        generation: 0,
    };
    replacements.insert(tree_id, tree_root(entries));
    point_holder_at_tree(
        &mut replacements,
        tree_id,
        Holder {
            catalog,
            root_id,
            names_entry,
            names_dict,
            tree_entry,
        },
    );
    // The old root is what the holder rewrite reuses where the tree was indirect, so it is
    // never both replaced and freed; a specification the tree held inline is not an object.
    freed.retain(|id| !replacements.contains_key(id));

    let bytes = pdf_syntax::write::incremental_update_freeing(document, &replacements, &freed)
        .map_err(|error| Refusal::Update { at, error })?;

    let expanded = names.expand(&Fill {
        ordinal: 1,
        count: 1,
        page: None,
        label: None,
        title: Some(name),
    });
    sinks
        .open(&expanded.name)
        .and_then(|mut sink| sink.write_all(&bytes).and_then(|()| sink.flush()))
        .map_err(|error| Refusal::Sink {
            name: expanded.name.clone(),
            error,
        })?;
    report.outputs.push(Output {
        name: expanded.name,
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        sanitised: expanded.sanitised,
        origin: Origin::Updated {
            source: at,
            attached: name.to_owned(),
        },
    });
    Ok(())
}

/// Which of the other two homes — the catalog's `/AF`, a page's annotation — still reach one
/// of these streams, as a sentence, or empty where none does.
///
/// By object identity rather than by pointer: the tree's own entry is being let go of, and
/// what matters is whether any *other* reference to the stream remains.
fn other_homes(document: &Document, streams: &[ObjectId]) -> String {
    let mut homes: Vec<String> = Vec::new();
    if let Ok(catalog) = document.catalog()
        && let Some(associated) = document.get_key(&catalog, "AF").as_array()
        && associated.iter().any(|entry| {
            document
                .resolve(entry)
                .as_dict()
                .is_some_and(|spec| references_one_of(document, spec, streams))
        })
    {
        homes.push("the catalog's /AF".to_owned());
    }
    let pages = Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        for annotation in pdf_model::retrieval::annotations(document, &page) {
            if document
                .get_key(&annotation, "FS")
                .as_dict()
                .is_some_and(|spec| references_one_of(document, spec, streams))
            {
                homes.push(format!("page {}'s annotation", index.saturating_add(1)));
            }
        }
    }
    homes.join(", ")
}

/// Whether a file specification's `/EF` names any of these streams.
fn references_one_of(
    document: &Document,
    specification: &Dictionary,
    streams: &[ObjectId],
) -> bool {
    document
        .get_key(specification, "EF")
        .as_dict()
        .is_some_and(|embedded| {
            embedded
                .iter()
                .any(|(_, entry)| matches!(entry, Object::Reference(id) if streams.contains(id)))
        })
}

/// §12.5.6.15's annotation, and the page's `/Annots` rewritten to reach it.
///
/// Table 187's three entries — `/Subtype /FileAttachment`, the required `/FS`, and `/Name` —
/// with Table 166's `/Rect`, `/P` ("an indirect reference to the page object with which this
/// annotation is associated") and `/Contents` where there is a description. `/Annots` is
/// appended to, because §12.5.2 makes the array's order the drawing order and a file added last
/// belongs on top of what was there.
fn file_on_page(
    document: &Document,
    at: usize,
    on_page: &OnPage,
    annotation_id: ObjectId,
    specification_id: ObjectId,
    description: Option<&str>,
    replacements: &mut BTreeMap<ObjectId, Object>,
) -> Result<(), Refusal> {
    let pages = Pages::new(document);
    let page = on_page
        .page
        .checked_sub(1)
        .and_then(|index| pages.get(index))
        .ok_or(Refusal::NoSuchPage {
            at,
            page: on_page.page,
            count: pages.len(),
        })?;
    // A page without an object number is one `Pages` recovered by scanning, and the writer
    // refuses such a document before this is reached; the `Recovered` reason is the true one.
    let page_id = page.id.ok_or(Refusal::Update {
        at,
        error: pdf_syntax::write::UpdateError::Recovered,
    })?;
    let icon = on_page.icon.as_deref().unwrap_or(OnPage::DEFAULT_ICON);
    if !OnPage::ICONS.contains(&icon) {
        return Err(Refusal::Pattern(format!(
            "--icon takes Graph, PushPin, Paperclip or Tag, not {icon:?}"
        )));
    }
    let [x0, y0, x1, y1] = on_page.rect.unwrap_or_else(|| {
        let [left, _, _, top] = page.crop_box;
        let inset = OnPage::DEFAULT_SIDE;
        [
            left + inset,
            top - 2.0 * inset,
            left + 2.0 * inset,
            top - inset,
        ]
    });

    let mut annotation = Dictionary::new();
    annotation.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Annot"[..])),
    );
    annotation.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"FileAttachment"[..])),
    );
    annotation.insert(
        Name::new(&b"Rect"[..]),
        Object::Array(
            [x0, y0, x1, y1]
                .into_iter()
                .map(|value| Object::Real(f64::from(value)))
                .collect(),
        ),
    );
    annotation.insert(Name::new(&b"P"[..]), Object::Reference(page_id));
    annotation.insert(Name::new(&b"FS"[..]), Object::Reference(specification_id));
    annotation.insert(
        Name::new(&b"Name"[..]),
        Object::Name(Name::new(icon.as_bytes())),
    );
    if let Some(description) = description {
        annotation.insert(
            Name::new(&b"Contents"[..]),
            Object::String(pdf_syntax::text_string::encode_text_string(description).into()),
        );
    }
    replacements.insert(annotation_id, Object::Dictionary(annotation));

    match page.dict.get("Annots").cloned() {
        Some(Object::Reference(array_id)) => {
            let mut entries = match document.get(array_id) {
                Object::Array(entries) => entries,
                _ => Vec::new(),
            };
            entries.push(Object::Reference(annotation_id));
            replacements.insert(array_id, Object::Array(entries));
        }
        other => {
            let mut entries = match other {
                Some(Object::Array(entries)) => entries,
                _ => Vec::new(),
            };
            entries.push(Object::Reference(annotation_id));
            let mut dict = page.dict.clone();
            dict.insert(Name::new(&b"Annots"[..]), Object::Array(entries));
            replacements.insert(page_id, Object::Dictionary(dict));
        }
    }
    Ok(())
}

/// One `/Names` node holding every entry, in §7.9.6's order.
fn tree_root(mut entries: Vec<(Vec<u8>, Object)>) -> Object {
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    let mut names_array = Vec::with_capacity(entries.len().saturating_mul(2));
    for (key, value) in entries {
        names_array.push(Object::String(key.into()));
        names_array.push(value);
    }
    let mut root = Dictionary::new();
    root.insert(Name::new(&b"Names"[..]), Object::Array(names_array));
    Object::Dictionary(root)
}

/// Where §7.7.4's `/EmbeddedFiles` tree is now: the name dictionary as the catalog states it,
/// the tree as the name dictionary states it, and the entries the tree's leaves hold, values
/// as stated.
struct TreeState {
    /// The catalog's `/Names` entry, unresolved.
    names_entry: Option<Object>,
    /// The name dictionary it names, where it names one.
    names_dict: Option<Dictionary>,
    /// The name dictionary's `/EmbeddedFiles` entry, unresolved.
    tree_entry: Option<Object>,
    /// Every key and value in the tree, values as the leaves state them.
    entries: Vec<(Vec<u8>, Object)>,
}

impl TreeState {
    /// Reads the tree's state out of the catalog.
    fn read(document: &Document, catalog: &Dictionary) -> Self {
        let resolve = |object: &Object| document.resolve(object);
        let names_entry = catalog.get("Names").cloned();
        let names_dict = names_entry
            .as_ref()
            .map(resolve)
            .and_then(|object| object.as_dict().cloned());
        let tree_entry = names_dict
            .as_ref()
            .and_then(|names| names.get("EmbeddedFiles").cloned());
        let entries = tree_entry
            .as_ref()
            .map(resolve)
            .and_then(|object| {
                object
                    .as_dict()
                    .map(|root| tree::name_entries(root, &resolve))
            })
            .unwrap_or_default();
        Self {
            names_entry,
            names_dict,
            tree_entry,
            entries,
        }
    }
}

/// The objects that can hold the tree, from the outermost in.
struct Holder {
    /// The catalog, as read.
    catalog: Dictionary,
    /// Its object number, from the trailer's `/Root`.
    root_id: ObjectId,
    /// The catalog's `/Names` entry, unresolved.
    names_entry: Option<Object>,
    /// The name dictionary, where there is one.
    names_dict: Option<Dictionary>,
    /// The name dictionary's `/EmbeddedFiles` entry, unresolved.
    tree_entry: Option<Object>,
}

/// Points whichever object held the tree at the new root, rewriting the nearest indirect
/// object so that as little as possible is said twice: the old root's number where the tree
/// was indirect, the name dictionary's where that was, and the catalog's otherwise.
fn point_holder_at_tree(
    replacements: &mut BTreeMap<ObjectId, Object>,
    tree_id: ObjectId,
    holder: Holder,
) {
    let Holder {
        mut catalog,
        root_id,
        names_entry,
        names_dict,
        tree_entry,
    } = holder;
    match (tree_entry, names_entry) {
        (Some(Object::Reference(old_root)), _) => {
            // The new root takes the old root's number, so nothing above it changes.
            let root = replacements.remove(&tree_id).unwrap_or(Object::Null);
            replacements.insert(old_root, root);
        }
        (_, Some(Object::Reference(names_id))) => {
            let mut names = names_dict.unwrap_or_default();
            names.insert(Name::new(&b"EmbeddedFiles"[..]), Object::Reference(tree_id));
            replacements.insert(names_id, Object::Dictionary(names));
        }
        _ => {
            let mut names = names_dict.unwrap_or_default();
            names.insert(Name::new(&b"EmbeddedFiles"[..]), Object::Reference(tree_id));
            catalog.insert(Name::new(&b"Names"[..]), Object::Dictionary(names));
            replacements.insert(root_id, Object::Dictionary(catalog));
        }
    }
}

/// The first object number nothing in the file uses.
///
/// Both of §7.5.5's answers are asked and the larger wins, for the reason `ViewState::save`
/// gives: Table 15's `/Size` "shall be 1 greater than the highest object number defined in the
/// PDF file", and tens of corpus documents write a cross-reference entry past their own
/// `/Size`. Trusting the stated number alone would put a new object on an existing one's
/// number and silently replace it.
fn next_object_number(document: &Document) -> u32 {
    let highest = document.xref().object_numbers().max().unwrap_or_default();
    let stated = document
        .trailer()
        .get("Size")
        .and_then(Object::as_integer)
        .and_then(|size| u32::try_from(size).ok())
        .unwrap_or_default();
    highest.saturating_add(1).max(stated)
}

/// §7.11.4's embedded file stream, Tables 44 and 45.
///
/// Unfiltered: the bytes are the file's, and a compression this crate chose would be a second
/// decision in a writer whose one job is to say what was attached. `/CheckSum` is Table 45's —
/// "[t]he checksum shall be calculated by applying the standard MD5 message-digest algorithm
/// (defined in Internet RFC 1321) to the bytes of the embedded file stream" — and `/Size` its
/// "size of the uncompressed embedded file, in bytes".
fn embedded_file_stream(bytes: &[u8], date: Option<Date>) -> Object {
    let mut params = Dictionary::new();
    params.insert(
        Name::new(&b"Size"[..]),
        Object::Integer(i64::try_from(bytes.len()).unwrap_or(i64::MAX)),
    );
    params.insert(
        Name::new(&b"CheckSum"[..]),
        Object::String(<md5::Md5 as md5::Digest>::digest(bytes).to_vec().into()),
    );
    if let Some(date) = date {
        let spelled = Object::String(pdf_date(date).into_bytes().into());
        params.insert(Name::new(&b"CreationDate"[..]), spelled.clone());
        params.insert(Name::new(&b"ModDate"[..]), spelled);
    }
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"EmbeddedFile"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(bytes.len()).unwrap_or(i64::MAX)),
    );
    dict.insert(Name::new(&b"Params"[..]), Object::Dictionary(params));
    Object::Stream(Arc::new(Stream {
        dict,
        data: bytes.into(),
        decryption_failed: false,
    }))
}

/// §7.11.3's file specification dictionary, Table 43, for a file embedded under both of
/// Table 43's names.
///
/// `/F` and `/UF` are both written, as the table asks — "[t]he UF entry should be used in
/// addition to the F entry" — and both are the filing name; `/EF` names the one stream under
/// both keys, which is "a subset of the F and UF keys corresponding to the entries by those
/// names". `/Type` is required "if an EF, EP or RF entry is present".
fn file_specification(name: &[u8], stream: ObjectId, description: Option<&str>) -> Object {
    let mut embedded = Dictionary::new();
    embedded.insert(Name::new(&b"F"[..]), Object::Reference(stream));
    embedded.insert(Name::new(&b"UF"[..]), Object::Reference(stream));
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Filespec"[..])),
    );
    dict.insert(Name::new(&b"F"[..]), Object::String(name.into()));
    dict.insert(Name::new(&b"UF"[..]), Object::String(name.into()));
    dict.insert(Name::new(&b"EF"[..]), Object::Dictionary(embedded));
    if let Some(description) = description {
        dict.insert(
            Name::new(&b"Desc"[..]),
            Object::String(pdf_syntax::text_string::encode_text_string(description).into()),
        );
    }
    Object::Dictionary(dict)
}

/// §7.9.4's date string, `D:YYYYMMDDHHmmSSOHH'mm'`, every field written.
///
/// The zone is written only where the date states one, and `Z` is written with the two zero
/// fields the clause's grammar places after it.
fn pdf_date(date: Date) -> String {
    let mut text = format!(
        "D:{:04}{:02}{:02}{:02}{:02}{:02}",
        date.year, date.month, date.day, date.hour, date.minute, date.second
    );
    match date.offset {
        None => {}
        Some(0) => text.push_str("Z00'00'"),
        Some(minutes) => {
            let sign = if minutes < 0 { '-' } else { '+' };
            let absolute = minutes.unsigned_abs();
            let _ = write!(text, "{sign}{:02}'{:02}'", absolute / 60, absolute % 60);
        }
    }
    text
}

/// Reads a date a caller typed, in ISO 8601's `YYYY-MM-DDTHH:MM:SS` with an optional `Z` or
/// `±HH:MM` — the form RFC 0002 section 9's `--date` names.
///
/// Only that form: a date this program writes into somebody's file should be one the caller
/// spelled in full, and a partial date would be this crate defaulting fields on their behalf.
#[must_use]
pub fn parse_iso_8601(text: &str) -> Option<Date> {
    let (stamp, zone) = match text.find(['Z', '+']) {
        Some(at) => (&text[..at], Some(&text[at..])),
        None => match text.rfind('-') {
            // The date's own hyphens sit before the `T`; a zone's sits after it.
            Some(at) if text.find('T').is_some_and(|t| at > t) => (&text[..at], Some(&text[at..])),
            _ => (text, None),
        },
    };
    let (date, time) = stamp.split_once('T')?;
    let mut date_fields = date.split('-');
    let year: i32 = date_fields.next()?.parse().ok()?;
    let month: u8 = date_fields.next()?.parse().ok()?;
    let day: u8 = date_fields.next()?.parse().ok()?;
    if date_fields.next().is_some() {
        return None;
    }
    let mut time_fields = time.split(':');
    let hour: u8 = time_fields.next()?.parse().ok()?;
    let minute: u8 = time_fields.next()?.parse().ok()?;
    let second: u8 = time_fields.next()?.parse().ok()?;
    if time_fields.next().is_some() {
        return None;
    }
    let offset = match zone {
        None => None,
        Some("Z") => Some(0),
        Some(zone) => {
            let (sign, rest) = zone.split_at(1);
            let (hours, minutes) = rest.split_once(':')?;
            let hours: i16 = hours.parse().ok()?;
            let minutes: i16 = minutes.parse().ok()?;
            let total = hours.checked_mul(60)?.checked_add(minutes)?;
            Some(if sign == "-" {
                total.checked_neg()?
            } else {
                total
            })
        }
    };
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    Some(Date {
        year,
        month,
        day,
        hour,
        minute,
        second,
        offset,
    })
}

/// Most annotation-borne files listed from one document, beside the reader's own bound on
/// the tree's: a page a producer fills with a thousand attachment icons is one making a reader
/// work.
const MAX_ANNOTATION_FILES: usize = 4096;

/// Every embedded file in every home, each stream once, with the page where the home is an
/// annotation.
fn every_home(document: &Document) -> Vec<(Attachment, Option<usize>)> {
    let mut all: Vec<(Attachment, Option<usize>)> = attachments(document)
        .into_iter()
        .map(|attachment| (attachment, None))
        .collect();
    let pages = Pages::new(document);
    let mut from_annotations = 0_usize;
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        for annotation in pdf_model::retrieval::annotations(document, &page) {
            if from_annotations >= MAX_ANNOTATION_FILES {
                return all;
            }
            let subtype = document.get_key(&annotation, "Subtype");
            if subtype.as_name().and_then(|name| name.as_str()) != Some("FileAttachment") {
                continue;
            }
            let Some(attachment) = of_annotation(document, &annotation) else {
                continue;
            };
            // One payload filed in two homes is one file, and the tree's entry is the one
            // kept: the streams share an `Arc` because the document caches resolved objects by
            // identity, the argument `pdf_model::attachment::attachments` already rests on.
            if all
                .iter()
                .any(|(seen, _)| Arc::ptr_eq(&seen.stream, &attachment.stream))
            {
                continue;
            }
            from_annotations = from_annotations.saturating_add(1);
            all.push((attachment, Some(index.saturating_add(1))));
        }
    }
    all
}

/// Decodes and writes one embedded file, accounting for it in the report.
///
/// A stream this reader refuses is a per-file refusal (exit 4) and the next file is still
/// written; a sink that fails is the machine's and ends the run (exit 2).
#[expect(
    clippy::too_many_arguments,
    reason = "the two callers differ only in the ordinal and the count, and a struct for nine \
              things used once would name the same nine"
)]
fn save(
    plan: &AttachmentsPlan,
    document: &Document,
    sinks: &dyn Sinks,
    names: &Pattern,
    ordinal: usize,
    count: usize,
    attachment: &Attachment,
    page: Option<usize>,
    report: &mut Report,
) -> Result<(), Refusal> {
    // `%t` is the file's own name where it has one and the filing name otherwise: what a person
    // saving "every attachment into a directory" expects to see.
    let title = attachment.file_name.as_deref().unwrap_or(&attachment.name);
    let expanded = names.expand(&Fill {
        ordinal,
        count,
        page: None,
        label: None,
        title: Some(title),
    });
    let declined = |detail: String| Declined {
        source: plan.source,
        page,
        subject: expanded.name.clone(),
        detail,
    };
    let decoded = match document.decoded_stream_data_reported(&attachment.stream) {
        Ok(decoded) => decoded,
        Err(refusal) => {
            // `StreamRefusal` has no `Display`, like `Unsupported`; its `Debug` names the variant.
            report.refused.push(declined(format!("{refusal:?}")));
            return Ok(());
        }
    };
    if let Some(damage) = &decoded.damage {
        report.warnings.push(Warning {
            source: plan.source,
            page,
            detail: format!(
                "{}: the embedded file's stream is damaged ({damage:?})",
                attachment.name
            ),
        });
    }
    if let Some(false) = attachment.checksum_matches(&decoded.data) {
        report.warnings.push(Warning {
            source: plan.source,
            page,
            detail: format!(
                "{}: the bytes do not match Table 46's /CheckSum",
                attachment.name
            ),
        });
    }
    sinks
        .open(&expanded.name)
        .and_then(|mut sink| sink.write_all(&decoded.data).and_then(|()| sink.flush()))
        .map_err(|error| Refusal::Sink {
            name: expanded.name.clone(),
            error,
        })?;
    report.outputs.push(Output {
        name: expanded.name.clone(),
        bytes: u64::try_from(decoded.data.len()).unwrap_or(u64::MAX),
        sanitised: expanded.sanitised,
        origin: Origin::Attachment {
            source: plan.source,
            name: attachment.name.clone(),
        },
    });
    Ok(())
}
