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
//! The write direction (`--attach`, pdftk's `attach_files`) is the smallest consumer of a writer
//! and is the serializer round's.

use std::io::Write as _;
use std::sync::Arc;

use pdf_model::Pages;
use pdf_model::attachment::{Attachment, attachments, of_annotation};
use pdf_syntax::Document;

use crate::json::Value;
use crate::pattern::{Fill, Pattern};
use crate::{Declined, Listed, Origin, Output, Refusal, Report, Sinks, Warning};

/// §7.11.4's embedded files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentsPlan {
    /// Which source.
    pub source: usize,
    /// What to do with them.
    pub action: Action,
}

/// What an attachments plan does.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    }
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
