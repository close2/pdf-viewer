//! The structure this reader recovered, written out as the structure of the archive (ADR 2502).
//!
//! # The reading
//!
//! ISO 32000-2 §7.5.5's Table 15 makes the trailer's `/Root` "( Required; shall be an indirect
//! reference )" to the catalog, and §7.7.2's Table 29 makes the catalog's `/Pages` "( Required;
//! shall be an indirect reference )" to the root of the page tree. A file that breaks either is
//! one this tree still draws: `pdf_model::Pages` asks every object the file's cross-reference
//! information names whether it declares itself a page (Table 31's required `/Type`), and draws
//! those in ascending object number. Objects that information does not name are not asked, which
//! is why a source stating more pages than were drawn is refused (see the last section). **The standard
//! defines no recovery for a missing catalog or page tree.** Its Annex C.4 (informative, "File
//! Limits") says only that a reader of a damaged cross-reference table "may attempt to rebuild the
//! table by scanning all the objects in the file", which is the table's reconstruction and not the
//! tree's. Finding the pages by their own declarations is this reader's choice, and writing the
//! pages it found as the archive's page tree is a second choice of the same kind, made here and
//! recorded as the loss `recovered-structure`. What §7.7.3.2 does say bears on the second choice:
//!
//! > PDF processors shall not be required to preserve the existing structure of the page tree.
//!
//! # What is written, and what is not
//!
//! The pages are the producer's: every page dictionary, content stream and resource is carried as
//! the source states it, apart from the two entries a page owes its tree. Table 31's `/Parent`
//! names the new root, and the four entries §7.7.3.4 makes inheritable (`/Resources`,
//! `/MediaBox`, `/CropBox`, `/Rotate`) are copied onto a page that states none, from the nearest
//! ancestor the page's old `/Parent` chain reaches, because the new root states none of them. The
//! catalog is the producer's dictionary where the trailer's `/Root` resolves to one, with `/Type`
//! and `/Pages` stated; otherwise it holds those two entries and nothing else. **No mark, no page
//! and no content stream is composed**: the construction states the objects the reader drew from.
//!
//! The result is a whole file, reopened as a document, and the conversion runs on that document.
//! The reopened document must pass the three checks that declined the source, which is what makes
//! "recovered" a statement about the output and not about the intention.
//!
//! # Where it stops
//!
//! A source this does not rebuild is refused by name: one the reader draws no page of, and one
//! that states more pages than the reader reached (a linearization parameter dictionary's `/N`, or
//! the `/Count` of a page tree root the catalog still names) where the caller has not authorised
//! [`super::Loss::PageNotHeld`].
//!
//! A `/Kids` entry naming no object — a direct `null`, or a reference §7.3.10 makes a reference to
//! the null object because the file defines no such object — is no page, so it is not written and
//! the count it took is no page lost (`doc/questions/A727`, `doc/adr/2596`). An entry naming an
//! object the cross-reference information places, which this reader reads as no dictionary, is a
//! page the producer wrote and the file does not hold: the loss `page-not-held`, with the count
//! stated and the count carried written into the report and the file's history.
//!
//! # Protection and signatures are the source's
//!
//! The rebuilt file is unencrypted and carries no byte range a signature covered, and the
//! conversion that goes on over it reads both from the source (`doc/adr/2523`): the claim the
//! encryption made is [`super::SourceProtection::of`] over the source, the loss
//! [`super::Loss::Encryption`] is decided for `file-structure/no-encryption` as for any
//! encrypted source, and each signature is reported against the source's bytes while its value is
//! removed from the objects written.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use pdf_archive::Target;
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};
use pdf_syntax::serialize::{Assembly, Form, ObjectStreams, Options, Streams, serialize};
use pdf_syntax::{Document, Limits};

use crate::Refusal;
use crate::split::INHERITABLE;

/// How a recovered catalog was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveredCatalog {
    /// The producer's catalog dictionary, with `/Type` and `/Pages` stated.
    Kept,
    /// A catalog stating `/Type` and `/Pages` only, because the trailer names no dictionary.
    Written,
}

/// What a conversion rebuilt of a document's structure, per document.
///
/// Reported beside the loss `recovered-structure` and recorded in the output's own
/// `xmpMM:History` (ADR 2502).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredStructure {
    /// The refusal's own sentence: which clause the source did not satisfy.
    pub because: String,
    /// How many pages the new root lists, each the producer's.
    pub pages: usize,
    /// How many of them this reader found by their own `/Type /Page` declaration rather than by
    /// walking a page tree; `None` where it walked one.
    pub scanned: Option<usize>,
    /// How the catalog was written.
    pub catalog: RecoveredCatalog,
    /// The pages whose dictionary this reader read in part, each written as the entries it read.
    pub read_in_part: Vec<PartlyRead>,
    /// How many `/Kids` entries named no object and were not written (`doc/adr/2596`).
    pub naming_nothing: usize,
    /// The pages the source states and does not hold, where the caller authorised
    /// [`super::Loss::PageNotHeld`]; `None` where the archive holds every page the source states.
    pub not_held: Option<NotHeld>,
}

/// The loss [`super::Loss::PageNotHeld`] as one conversion took it: the count the source states and
/// the count the archive holds (`doc/questions/A727`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotHeld {
    /// The statement the count is read from: the page tree root's `/Count` or a linearization
    /// parameter dictionary's `/N`.
    pub stated_by: &'static str,
    /// The number of pages it states.
    pub stated: usize,
    /// The number of pages the archive holds, each the producer's.
    pub carried: usize,
}

/// A page whose dictionary stops being readable, written as the entries the reader drew it from.
///
/// ISO 32000-2 §7.3.7 states a dictionary's extent only as its closing `>>`, so a reader that
/// never reaches it cannot know how many entries the producer wrote; `pdf_model::Pages` draws the
/// page from the entries before the damage and says so (ADR 0784). The archive states those
/// entries and no others, and names the page, the count and the byte where reading stopped, so
/// that what the producer wrote after them is a stated loss rather than a silent one
/// (`doc/adr/2551`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartlyRead {
    /// The page's number, counting from one in the order this reader draws them.
    pub page: usize,
    /// How many entries were read whole and are written.
    pub entries: usize,
    /// The byte offset in the source at which reading stopped.
    pub stopped_at: usize,
}

impl RecoveredStructure {
    /// What was rebuilt, as one sentence for the report and the file's own history.
    #[must_use]
    pub fn sentence(&self) -> String {
        let catalog = match self.catalog {
            RecoveredCatalog::Kept => {
                "the producer's catalog kept, with /Type and a new /Pages stated"
            }
            RecoveredCatalog::Written => "a catalog written afresh, stating /Type and /Pages only",
        };
        let found = match self.scanned {
            Some(count) => format!(
                "{count} of them found by their own /Type /Page declaration, in ascending object \
                 number"
            ),
            None => "walked from the page tree the file states".to_owned(),
        };
        let mut partly = String::new();
        for read in &self.read_in_part {
            // Writing into a `String` cannot fail.
            let _ = write!(
                partly,
                " Page {}'s dictionary stops being readable at byte {} of the source, and is \
                 written as the {} {} before it, which this reader drew the page from; \
                 whatever its producer wrote after them is not in the archive.",
                read.page,
                read.stopped_at,
                read.entries,
                if read.entries == 1 {
                    "entry"
                } else {
                    "entries"
                }
            );
        }
        if self.naming_nothing > 0 {
            let _ = write!(
                partly,
                " {} /Kids {} named no object (§7.3.10's null) and {} not written, since no page \
                 is there.",
                self.naming_nothing,
                if self.naming_nothing == 1 {
                    "entry"
                } else {
                    "entries"
                },
                if self.naming_nothing == 1 {
                    "is"
                } else {
                    "are"
                }
            );
        }
        if let Some(not_held) = &self.not_held {
            let _ = write!(
                partly,
                " page-not-held: {} states {} pages and the archive holds {}; the {} the file does \
                 not hold where its cross-reference information places {} not in the archive, and \
                 nothing is drawn in {} place.",
                not_held.stated_by,
                not_held.stated,
                not_held.carried,
                not_held.stated.saturating_sub(not_held.carried),
                if not_held.stated.saturating_sub(not_held.carried) == 1 {
                    "it is"
                } else {
                    "them are"
                },
                if not_held.stated.saturating_sub(not_held.carried) == 1 {
                    "its"
                } else {
                    "their"
                },
            );
        }
        format!(
            "recovered-structure: {catalog}, and a single page tree node listing the {} page(s) \
             this reader drew, {found}; each page is the producer's, with /Parent restated and \
             the inheritable entries it stated no value for copied from its old ancestors.{partly} \
             Declined as a source because: {}",
            self.pages, self.because
        )
    }
}

/// A source rebuilt: the file written, reopened, and what it states.
pub(super) struct Recovered {
    /// The rebuilt file, opened.
    pub(super) document: Document,
    /// What was rebuilt.
    pub(super) note: RecoveredStructure,
}

/// Rebuilds the structure of a document this program declined as a source.
///
/// `declined` is the refusal that declined it; its sentence is carried into the note, and
/// `page_not_held` is whether the caller authorised [`super::Loss::PageNotHeld`]. The rebuilt
/// file states the source's version, or `target`'s where the source states none
/// ([`super::header::target_version`], `doc/adr/2558`): it is the file the conversion goes on to
/// write, and the source's header line is recorded from the source.
///
/// # Errors
///
/// [`Refusal::Reconstructed`] naming why the source is not rebuilt (the module's last section, or
/// a rebuilt file that still fails the checks that declined the source), and
/// [`Refusal::Assembly`] where the numbering is spent or the rebuilt file cannot be opened.
pub(super) fn rebuild(
    document: &Document,
    declined: &Refusal,
    target: Target,
    page_not_held: bool,
) -> Result<Recovered, Refusal> {
    let because = declined.to_string();
    build(document, because.clone(), target, page_not_held).map_err(|refusal| match refusal {
        Refusal::Reconstructed(why) => Refusal::Reconstructed(format!(
            "{because}; it is not written out as the structure this reader recovered, because \
             {why} (ADR 2502)"
        )),
        other => other,
    })
}

/// [`rebuild`], with its refusals worded as the reason alone.
fn build(
    document: &Document,
    because: String,
    target: Target,
    page_not_held: bool,
) -> Result<Recovered, Refusal> {
    let tree = Pages::new(document);
    let scanned = tree.found_by_scanning();
    let (pages, read_in_part) = pages_to_carry(&tree)?;
    let children = super::page_tree::children_not_held(document);
    let not_held = refuse_a_count_the_reader_fell_short_of(
        document,
        pages.len(),
        children.naming_nothing,
        page_not_held,
    )?;

    let kept_catalog = document.catalog().ok();
    let mut assembly = Assembly::new(vec![document]);
    let assembly_error = |error: pdf_syntax::AssemblyError| Refusal::Assembly(error.to_string());
    let catalog_at = assembly.reserve().map_err(assembly_error)?;
    let root_at = assembly.reserve().map_err(assembly_error)?;
    // Every page takes its number before any is built, so that a reference from one page's
    // annotation to another page of the document names that page and not null.
    let page_count = pages.len();
    let mut placed = Vec::with_capacity(page_count);
    for page in pages {
        let slot = match &page {
            Carried::Object(id) => assembly.replace(0, *id),
            Carried::Direct(_) => assembly.reserve(),
        }
        .map_err(assembly_error)?;
        placed.push((page, slot));
    }
    let mut handed = Handed::new();
    for (source, slot) in &placed {
        let page = build_page(&mut assembly, document, source, root_at, &mut handed)?;
        assembly.place(*slot, page).map_err(assembly_error)?;
    }
    let mut catalog = Dictionary::new();
    if let Some(kept) = &kept_catalog {
        for (key, value) in kept.iter() {
            if matches!(key.as_bytes(), b"Type" | b"Pages") {
                continue;
            }
            let carried = carry(&mut assembly, document, value, 0)?;
            if !matches!(carried, Object::Null) {
                catalog.insert(key.clone(), carried);
            }
        }
    }
    catalog.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Catalog"[..])),
    );
    catalog.insert(Name::new(&b"Pages"[..]), Object::Reference(root_at));
    let root = page_tree_root(
        placed
            .iter()
            .map(|(_, slot)| Object::Reference(*slot))
            .collect(),
    );
    assembly
        .place(root_at, Object::Dictionary(root))
        .and_then(|()| assembly.place(catalog_at, Object::Dictionary(catalog)))
        .map_err(assembly_error)?;
    assembly.set_root(catalog_at);
    if let Some(info) = document
        .trailer()
        .get("Info")
        .and_then(Object::as_reference)
        .filter(|info| document.get(*info).as_dict().is_some())
    {
        let mapped = crate::optimize::copy_closure(&mut assembly, document, info, true)
            .map_err(assembly_error)?;
        assembly.set_info(Some(mapped));
    }

    let version = document
        .version()
        .unwrap_or_else(|| super::header::target_version(target));
    let options = Options {
        form: Form::of(document),
        object_streams: ObjectStreams::Disable,
        streams: Streams::Carry,
    };
    let mut bytes = Vec::new();
    serialize(&assembly, version, options, &mut bytes)
        .map_err(|error| Refusal::Assembly(error.to_string()))?;
    let reopened = Document::open_with_limits(bytes, Limits::DEFAULT)
        .map_err(|error| Refusal::Assembly(format!("the rebuilt file does not open: {error}")))?;
    check_what_declined_the_source(&reopened, page_count)?;
    Ok(Recovered {
        document: reopened,
        note: RecoveredStructure {
            because,
            pages: page_count,
            scanned,
            catalog: if kept_catalog.is_some() {
                RecoveredCatalog::Kept
            } else {
                RecoveredCatalog::Written
            },
            read_in_part,
            naming_nothing: children.naming_nothing,
            not_held,
        },
    })
}

/// A page of the source the rebuild carries.
enum Carried {
    /// An object of the source.
    Object(ObjectId),
    /// A dictionary written directly in its parent's `/Kids`: it names its parent, or states the
    /// entries the reader resolved for it.
    Direct(Dictionary),
}

/// The page objects the new root lists, in the order this reader draws them.
///
/// A page whose dictionary this reader parsed whole is carried as the object it is. One that stops
/// being readable is carried as the entries the reader drew it from, with the values of the
/// inheritable entries in force stated on it, and is named in the second list ([`PartlyRead`]). A
/// page written directly in its parent's `/Kids` is carried too, as an object of its own, because
/// Table 30 makes a node's `/Kids` an array of indirect references.
///
/// A page object is carried once, at the first index the reader draws it at. A tree whose `/Count`
/// states more pages than its `/Kids` hold has the reader skip a subtree by the count it states and
/// land on the next subtree's first page again, so one object answers two indices, or answer an
/// index with no page at all; either is the reader's reach past what the file holds and no page of
/// the producer's, so it is not carried, and the stated count is then answered by
/// [`refuse_a_count_the_reader_fell_short_of`] against the pages carried.
fn pages_to_carry(tree: &Pages<'_>) -> Result<(Vec<Carried>, Vec<PartlyRead>), Refusal> {
    let mut pages = Vec::new();
    let mut carried_objects = BTreeSet::new();
    let mut read_in_part = Vec::new();
    for (index, page) in tree.every().into_iter().enumerate() {
        let number = index.saturating_add(1);
        let Some(page) = page else {
            continue;
        };
        if page.id.is_some_and(|id| !carried_objects.insert(id)) {
            continue;
        }
        if let Some(damage) = &page.damaged_dictionary {
            read_in_part.push(PartlyRead {
                page: number,
                entries: damage.entries,
                stopped_at: damage.stopped_at,
            });
            pages.push(Carried::Direct(with_resolved_entries(page)));
            continue;
        }
        // A page written directly in its parent's /Kids is given an object of its own, as Table 30
        // asks of every child. One that names its parent has its inheritable entries read from the
        // nodes above it; one that does not has the values the reader's descent resolved for it
        // stated on the page itself (§7.7.3.4), since the reader reaches it through the `/Kids`
        // that holds it whether or not it names the node back.
        let names_parent = page.dict.get("Parent").and_then(Object::as_reference);
        pages.push(match page.id {
            Some(id) => Carried::Object(id),
            None if names_parent.is_some() => Carried::Direct(page.dict),
            None => Carried::Direct(with_resolved_entries(page)),
        });
    }
    if pages.is_empty() {
        return Err(not_rebuilt("this reader draws no page of it"));
    }
    Ok((pages, read_in_part))
}

/// A direct page that names no `/Parent`, with the inheritable entries the reader resolved for it
/// stated on the page.
///
/// ISO 32000-2 §7.7.3.4 makes `/Resources`, `/MediaBox`, `/CropBox` and `/Rotate` inheritable. The
/// walk that found the page applied them from the `/Kids` arrays it descended through, so the
/// [`pdf_model::Page`] carries the values in effect; an entry the file states nowhere is not
/// invented (a media box the reader guessed, a crop box it defaulted, a rotation of 0).
fn with_resolved_entries(page: pdf_model::Page) -> Dictionary {
    let mut dict = page.dict;
    let rectangle = |sides: [f32; 4]| {
        Object::Array(
            sides
                .iter()
                .map(|side| Object::Real(f64::from(*side)))
                .collect(),
        )
    };
    if dict.get("Resources").is_none() && page.resources.iter().next().is_some() {
        dict.insert(
            Name::new(&b"Resources"[..]),
            Object::Dictionary(std::sync::Arc::unwrap_or_clone(page.resources)),
        );
    }
    if dict.get("MediaBox").is_none() && page.substituted_media_box.is_none() {
        dict.insert(Name::new(&b"MediaBox"[..]), rectangle(page.media_box));
    }
    if dict.get("CropBox").is_none() && page.crop_box_defined {
        dict.insert(Name::new(&b"CropBox"[..]), rectangle(page.crop_box));
    }
    if dict.get("Rotate").is_none() && page.rotate != 0 {
        dict.insert(
            Name::new(&b"Rotate"[..]),
            Object::Integer(i64::from(page.rotate)),
        );
    }
    dict
}

/// Answers a source that states more pages than this reader reached: refused by name, or, where
/// the caller authorised [`super::Loss::PageNotHeld`], the counts the loss is reported with.
///
/// The reader finds pages through what the file's cross-reference information names, and a file
/// whose later sections it does not reach would be written as the pages it did reach and nothing
/// else, with no word that it had more. Two statements of a count are read, each only to compare
/// with what was drawn: a linearization parameter dictionary's `/N` (ISO 32000-2 §F.3.3, Table
/// F.1: "( Required ) The number of pages in the document."), and the `/Count` of the page tree's
/// root where the catalog still names it (Table 30). Nothing else in a file states a count; a
/// source that states none is rebuilt on the pages the reader drew, which is the loss
/// `recovered-structure`.
///
/// `naming_nothing` is how many `/Kids` entries name no object
/// ([`super::page_tree::ChildrenNotHeld::naming_nothing`]). Each is no leaf, and Table 30 makes
/// the `/Kids` arrays what "definitively determines the number of descendant pages", so a count
/// that exceeds the pages drawn by no more than those entries states no page the file lacks.
fn refuse_a_count_the_reader_fell_short_of(
    document: &Document,
    drawn: usize,
    naming_nothing: usize,
    page_not_held: bool,
) -> Result<Option<NotHeld>, Refusal> {
    let as_count = |value: Option<i64>| value.and_then(|n| usize::try_from(n).ok());
    let linearized = document.xref().object_numbers().find_map(|number| {
        let object = document.get(ObjectId::new(number, 0));
        let dict = object.as_dict()?;
        dict.get("Linearized")?;
        as_count(dict.get("N").and_then(Object::as_integer))
    });
    let root_count = document
        .catalog()
        .ok()
        .and_then(|catalog| catalog.get("Pages").cloned())
        .map(|pages| match pages {
            Object::Reference(id) => document.get(id),
            other => other,
        })
        .and_then(|root| as_count(root.as_dict()?.get("Count").and_then(Object::as_integer)));
    let mut not_held: Option<NotHeld> = None;
    for (stated, whose) in [
        (linearized, "its linearization parameter dictionary's /N"),
        (root_count, "its page tree root's /Count"),
    ] {
        let Some(stated) = stated.filter(|stated| stated.saturating_sub(naming_nothing) > drawn)
        else {
            continue;
        };
        if !page_not_held {
            return Err(not_rebuilt(&format!(
                "{whose} states {stated} pages where this reader reaches {drawn}, and the \
                 rebuilt file would hold those {drawn} and say nothing of the rest; authorise \
                 page-not-held to write the {drawn} with the two counts stated"
            )));
        }
        if not_held.is_none_or(|held| stated > held.stated) {
            not_held = Some(NotHeld {
                stated_by: whose,
                stated,
                carried: drawn,
            });
        }
    }
    Ok(not_held)
}

/// The refusal of a source this module does not rebuild, worded as `Refusal::Reconstructed`.
fn not_rebuilt(why: &str) -> Refusal {
    Refusal::Reconstructed(why.to_owned())
}

/// The rebuilt file must pass the three checks that declined the source, and must draw the pages
/// the source's reader drew, found by its page tree.
fn check_what_declined_the_source(rebuilt: &Document, pages: usize) -> Result<(), Refusal> {
    let root = crate::optimize::catalog_of(rebuilt)?;
    crate::optimize::refuse_a_document_only_recovery_reads(rebuilt, root)?;
    super::page_tree::refuse_a_page_tree_node_with_no_children(rebuilt, root)?;
    let tree = Pages::new(rebuilt);
    if tree.found_by_scanning().is_some() || tree.len() != pages {
        return Err(not_rebuilt(&format!(
            "the rebuilt file's page tree reaches {} page(s) where the source's reader drew {pages}",
            tree.len()
        )));
    }
    Ok(())
}

/// What an old parent node hands down, by the node a page names and the entry: the value as the
/// rebuild states it, which is always a reference (or `None` where no ancestor states the entry).
///
/// A value an ancestor states directly is placed once as an object of its own and every page that
/// inherits it names that object. §7.7.3.4 says "All values shall be inherited as-is, without
/// merging", and a reference to the object that holds the value changes no meaning, where a copy
/// per page would make the output as large as the page count times the value (a bound: a few
/// hundred pages under one node with a large direct `/Resources` are a few hundred times that
/// dictionary).
type Handed<'a> = BTreeMap<(ObjectId, &'a str), Option<Object>>;

/// The rebuild's one page tree node: ISO 32000-2 §7.7.3.2 Table 30's three required entries, its
/// `/Count` the kids it lists, each a page.
fn page_tree_root(kids: Vec<Object>) -> Dictionary {
    let mut root = Dictionary::new();
    root.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Pages"[..])),
    );
    root.insert(
        Name::new(&b"Count"[..]),
        Object::Integer(i64::try_from(kids.len()).unwrap_or(i64::MAX)),
    );
    root.insert(Name::new(&b"Kids"[..]), Object::Array(kids));
    root
}

/// One page of the rebuild: the source's dictionary, `/Parent` naming the new root, the entries
/// §7.7.3.4 makes inheritable stated where the page stated none, every reference mapped.
fn build_page(
    assembly: &mut Assembly<'_>,
    document: &Document,
    source: &Carried,
    root: ObjectId,
    handed: &mut Handed<'static>,
) -> Result<Object, Refusal> {
    let dict = match source {
        Carried::Object(id) => match document.get(*id) {
            Object::Dictionary(dict) => dict,
            _ => {
                return Err(not_rebuilt(&format!(
                    "object {} is not a dictionary",
                    id.number
                )));
            }
        },
        Carried::Direct(dict) => dict.clone(),
    };
    let mut out = Dictionary::new();
    for (key, value) in dict.iter() {
        if key.as_bytes() == b"Parent" {
            continue;
        }
        let carried = carry(assembly, document, value, 0)?;
        if !matches!(carried, Object::Null) {
            out.insert(key.clone(), carried);
        }
    }
    let old_parent = dict.get("Parent").and_then(Object::as_reference);
    for key in INHERITABLE {
        if out.get(key).is_some() {
            continue;
        }
        let Some(parent) = old_parent else { continue };
        let slot = (parent, key);
        let stated = if let Some(done) = handed.get(&slot) {
            done.clone()
        } else {
            let found = match source {
                Carried::Object(id) => crate::split::inherited(document, *id, key),
                Carried::Direct(_) => crate::split::inherited_from(document, parent, key),
            };
            let stated = match found {
                Some(Object::Reference(id)) => {
                    Some(carry(assembly, document, &Object::Reference(id), 0)?)
                }
                Some(value) => {
                    let carried = carry(assembly, document, &value, 0)?;
                    let at = assembly
                        .reserve()
                        .map_err(|error| Refusal::Assembly(error.to_string()))?;
                    assembly
                        .place(at, carried)
                        .map_err(|error| Refusal::Assembly(error.to_string()))?;
                    Some(Object::Reference(at))
                }
                None => None,
            }
            .filter(|value| !matches!(value, Object::Null));
            handed.insert(slot, stated.clone());
            stated
        };
        if let Some(value) = stated {
            out.insert(Name::new(key.as_bytes()), value);
        }
    }
    out.insert(Name::new(&b"Parent"[..]), Object::Reference(root));
    // ISO 32000-2 §7.7.3.3's Table 31 makes a page object's `/Type` required, and `Page` for a page
    // object (a template, the other value, is never a leaf of the tree). Every page this lists is
    // a leaf of the tree it writes, so one whose dictionary stated no type is stated a page, the
    // structure the reader drew it as (`doc/adr/2558`).
    if out.get("Type").is_none() {
        out.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"Page"[..])),
        );
    }
    Ok(Object::Dictionary(out))
}

/// A value of the source with every reference in it mapped into the assembly, the objects they
/// name copied with everything those reach.
///
/// A reference to nothing is §7.3.10's null and is dropped by the caller. The walk is bounded by
/// the depth the parser admits, so a value nested past it is refused rather than followed.
fn carry(
    assembly: &mut Assembly<'_>,
    document: &Document,
    value: &Object,
    depth: usize,
) -> Result<Object, Refusal> {
    if depth >= crate::optimize::MAX_WALK_DEPTH {
        return Err(Refusal::Assembly(
            "a value is nested deeper than the parser admits".to_owned(),
        ));
    }
    let deeper = depth.saturating_add(1);
    Ok(match value {
        Object::Reference(id) => {
            if let Some(mapped) = assembly.copied(0, *id) {
                Object::Reference(mapped)
            } else if document.get(*id) == Object::Null {
                Object::Null
            } else {
                Object::Reference(
                    crate::optimize::copy_closure(assembly, document, *id, true)
                        .map_err(|error| Refusal::Assembly(error.to_string()))?,
                )
            }
        }
        Object::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(carry(assembly, document, item, deeper)?);
            }
            Object::Array(out)
        }
        Object::Dictionary(dict) => {
            let mut out = Dictionary::new();
            for (key, item) in dict.iter() {
                let carried = carry(assembly, document, item, deeper)?;
                if !matches!(carried, Object::Null) {
                    out.insert(key.clone(), carried);
                }
            }
            Object::Dictionary(out)
        }
        other => other.clone(),
    })
}
