//! `attachments` — §7.11.4's embedded files, listed or extracted, RFC 0002 section 6.6's read
//! direction.
//!
//! Plumbing over a reader that already ships: `pdf_model::attachment::attachments` reads both
//! homes an embedded file has — the catalog's `/EmbeddedFiles` name tree (§7.7.4) and the
//! catalog's `/AF` associated files (§14.13) — deduplicated by stream, exactly as the viewer's
//! files panel lists them. Per-page file-attachment annotations (§12.5.6.15) are the third home
//! and are not enumerated this round; `doc/todo/57`.
//!
//! The write direction (`--attach`, pdftk's `attach_files`) is the smallest consumer of a writer
//! and is the serializer round's.

use std::io::Write as _;

use pdf_model::attachment::{Attachment, attachments};
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
}

impl AttachmentEntry {
    /// The entry from the reader's own record.
    fn of(source: usize, attachment: &Attachment) -> Self {
        Self {
            source,
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
    let all = attachments(document);
    match &plan.action {
        Action::List => {
            report.listed.extend(all.iter().map(|attachment| {
                Listed::Attachment(AttachmentEntry::of(plan.source, attachment))
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
            for (at, attachment) in all.iter().enumerate() {
                save(
                    plan,
                    document,
                    sinks,
                    names,
                    at.saturating_add(1),
                    count,
                    attachment,
                    report,
                )?;
            }
            Ok(())
        }
        Action::Save { name, names } => {
            let attachment = all
                .iter()
                .find(|attachment| {
                    attachment.name == *name || attachment.file_name.as_deref() == Some(name)
                })
                .ok_or_else(|| Refusal::NoSuchAttachment {
                    at: plan.source,
                    name: name.clone(),
                })?;
            save(plan, document, sinks, names, 1, 1, attachment, report)
        }
    }
}

/// Decodes and writes one embedded file, accounting for it in the report.
///
/// A stream this reader refuses is a per-file refusal (exit 4) and the next file is still
/// written; a sink that fails is the machine's and ends the run (exit 2).
#[expect(
    clippy::too_many_arguments,
    reason = "the two callers differ only in the ordinal and the count, and a struct for eight \
              things used once would name the same eight"
)]
fn save(
    plan: &AttachmentsPlan,
    document: &Document,
    sinks: &dyn Sinks,
    names: &Pattern,
    ordinal: usize,
    count: usize,
    attachment: &Attachment,
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
        page: None,
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
            page: None,
            detail: format!(
                "{}: the embedded file's stream is damaged ({damage:?})",
                attachment.name
            ),
        });
    }
    if let Some(false) = attachment.checksum_matches(&decoded.data) {
        report.warnings.push(Warning {
            source: plan.source,
            page: None,
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
