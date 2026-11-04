//! The pages a conversion adds or edits to carry what its target would otherwise lose: the
//! preserved packet and annotation appearances, the statement of what a conversion removed, the
//! attached files laid out as text, and the producer's marks relocated onto the page they were on.
//!
//! These are one family because they are the constructions `CLAUDE.md`'s authoring amendments
//! place on the near side — a page composed solely of content the document already holds, and a
//! producer's own marks written onto a page the producer wrote — and they share what makes those
//! constructions safe: every mark on such a page is read out of the file or is the fixed
//! vocabulary naming a requirement and a loss, and every page laid out is bounded before any is.

use super::super::ArchivePlan;
use super::super::decision::{Because, Loss};
use super::super::fonts::Substitutes;
use super::super::preserve::{self, Composed};
use super::super::report::{Preserved, SetIn};
use super::super::sites;
use super::super::statement;
use super::super::tagged;
use super::{
    ATTACHMENT_REQUIREMENTS, Cleaned, ForbiddenAnnotations, Fresh, NOT_A_PACKET, NOT_ASKED_FOR,
    PACKET_REQUIREMENTS, RemovedAnnotation, SCHEMA_REQUIREMENT, Spare,
};
use pdf_archive::MisusedProperty;
use pdf_archive::Target;
use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};
use std::collections::{BTreeMap, BTreeSet};

/// How many removed properties the `xmpMM:History` entry names before it stops listing them.
///
/// A bound rather than a reading (principle 3): a packet may state hundreds of properties and the
/// provenance entry is prose. The conversion report names every one of them whatever this is.
const MOST_NAMED: usize = 16;

/// The pages a configured `preserve` remedy appends, or why none are.
///
/// **Nothing is composed for a conversion that asked for none** — `doc/adr/0947`'s first rule
/// applied to a remedy — so a caller naming no preservation never loads a font or measures a page.
///
/// Two sites reach a page, and they are the two kinds of content `preserve` knows:
///
/// - ISO 19005-2 section 6.6.2.3.1's removal is about to edit a packet, so the packet **as the
///   producer wrote it** is appended: the whole packet rather than the properties alone, because a
///   value cut down to what a report can print is not the value, and because the packet is what
///   `doc/rfc/0007` section 2 names — *instead of losing metadata it could be appended or prefixed
///   as an extra page*;
/// - either section 6.3.1 row's removal is about to take an annotation off a page, so its **normal
///   appearance** is appended: the form `XObject` the producer wrote, invoked where §12.5.5 puts
///   it. `doc/adr/1099` is the argument.
#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "each argument is one preparation this composition reads and none of them can be \
              bundled without a struct built at exactly one call site; and the body is one \
              sequence per kind of content a page carries, whose order the comments state"
)]
pub(super) fn the_preserved_pages(
    plan: &ArchivePlan,
    document: &Document,
    input: &pdf_archive::Report,
    failed: &BTreeSet<&'static str>,
    properties: Result<&Cleaned, &Because>,
    fresh: Result<&Fresh, &Because>,
    forbidden: Result<&ForbiddenAnnotations, &Because>,
    embedded: Result<&sites::EmbeddedFiles, &Because>,
    has_output_intent: bool,
    remedies: &super::super::remedies::Remedies,
    recovery: super::super::recovered::Stated<'_>,
    substitutes: Option<&Substitutes>,
    runs: &[super::super::WrittenRun],
    rebuilt: Option<&super::super::parent_tree::Rebuilt>,
    information: &[super::RemovedInformation],
    spare: &mut Spare,
) -> Result<Composed, Because> {
    let appended = &remedies.appended;
    let asked = |site: &'static str| {
        plan.preservations
            .iter()
            .any(|preservation| preservation.site == site && preservation.by_page)
            && failed.contains(site)
    };
    // ISO 19005-2 section 6.6.2.1's three rows, whose rewrite replaces a packet rather than
    // editing it: the bytes the producer wrote stop being metadata, so `preserve` keeps them as
    // content. One keep per replaced stream, under the first of the three sites the operator
    // answered and this document failed, because one page of bytes cannot be split between rows
    // that all named the same stream.
    let replaced_sites: Vec<&'static str> = PACKET_REQUIREMENTS
        .into_iter()
        .filter(|site| asked(site))
        .collect();
    let replaced_site = replaced_sites.first().copied();
    // **A packet no page can lay out takes the row's second step** (`doc/adr/2678`), the
    // embedded-file rows' mechanism (`doc/adr/2536`): with `discard` or `append-statement` it is
    // removed and named for itself, and every other packet is still laid out.
    let packet_row = replaced_site.and_then(|site| {
        plan.preservations
            .iter()
            .find(|preservation| preservation.site == site)
    });
    let discard_packets = packet_row.is_some_and(|preservation| {
        preservation.no_page_carries == super::super::RemedyKind::Discard
    });
    let mut removed_packets: Vec<super::RemovedPacket> = Vec::new();
    let mut omitted_characters: Vec<super::OmittedCharacter> = Vec::new();
    let replaced = match replaced_site {
        // The replacement is what makes the file conform and the page is what keeps what it
        // replaces, so a document whose packets cannot be replaced gets neither — the property
        // site's own construction, for its reason.
        Some(site) => {
            let fresh = fresh.map_err(|because| *because)?;
            let mut packets: Vec<(ObjectId, std::sync::Arc<[u8]>)> = Vec::new();
            for at in fresh.packets.keys() {
                let Object::Stream(stream) = document.get(*at) else {
                    return Err(Because::NotBuiltYet(NOT_A_PACKET));
                };
                // The one place a preservation gives up rather than refusing the conversion:
                // there are no bytes to lay out, so there is nothing this page could carry, and
                // the stream is replaced with nothing preserved. Named in the report all the
                // same, because the replacement itself is reported per stream.
                let Some(bytes) = document.decoded_stream_data(&stream) else {
                    continue;
                };
                // The test is the page's own: `preserve` lays a packet out as UTF-8 text and
                // refuses `NOT_TEXT` on anything else, so exactly what it would refuse is handed on.
                if discard_packets && std::str::from_utf8(&bytes).is_err() {
                    removed_packets.push(super::RemovedPacket {
                        site,
                        at: *at,
                        length: stream.data.len(),
                        filters: filters_stated(document, &stream.dict),
                        because: super::metadata::PACKET_NOT_TEXT,
                        stated_on: None,
                    });
                    continue;
                }
                // **The same step, one character at a time** (`doc/adr/2682`, `A778`): a C0
                // control character has no glyph for the page to set, so the step that removes a
                // packet no page can lay out leaves just that character off the page and names it,
                // and the rest of the packet is laid out as the producer wrote it. Without the
                // step the character keeps the face search's refusal, `NO_FACE`.
                if discard_packets && let Ok(text) = std::str::from_utf8(&bytes) {
                    let (kept, omitted) = super::metadata::without_controls(site, *at, text);
                    if !omitted.is_empty() {
                        omitted_characters.extend(omitted);
                        packets.push((*at, kept.into_bytes().into()));
                        continue;
                    }
                }
                packets.push((*at, bytes));
            }
            packets
        }
        None => Vec::new(),
    };
    let packets_stated = packet_row
        .and_then(|preservation| preservation.statement.as_ref())
        .filter(|_| !removed_packets.is_empty())
        .map(|asked| (removed_packets.as_slice(), asked));
    let omitted_stated = packet_row
        .and_then(|preservation| preservation.statement.as_ref())
        .filter(|_| !omitted_characters.is_empty())
        .map(|asked| (omitted_characters.as_slice(), asked));
    // **An information entry the packet has no form for takes its row's second step**
    // (`doc/adr/2681`), the same mechanism again: the information preparation removed it, and the
    // page states it where that step is `append-statement`.
    let information_site = information.first().map(|entry| entry.site);
    let information_stated = information_site
        .and_then(|site| {
            plan.preservations
                .iter()
                .find(|preservation| preservation.site == site)
        })
        .and_then(|preservation| preservation.statement.as_ref())
        .map(|asked| (information, asked));
    let packets = if asked(SCHEMA_REQUIREMENT) {
        // The removal is what makes the file conform; the pages are what keep what it removes. A
        // document whose packet cannot be cut gets neither, and the reason is the cut's own.
        let cleaned = properties.map_err(|because| *because)?;
        let mut packets: Vec<(ObjectId, std::sync::Arc<[u8]>)> = Vec::new();
        for at in cleaned.packets.keys() {
            let Object::Stream(stream) = document.get(*at) else {
                return Err(Because::NotBuiltYet(NOT_A_PACKET));
            };
            let Some(bytes) = document.decoded_stream_data(&stream) else {
                return Err(Because::NotBuiltYet(NOT_A_PACKET));
            };
            packets.push((*at, bytes));
        }
        packets
    } else {
        Vec::new()
    };
    let annotation_site = SUBTYPE_REQUIREMENTS
        .into_iter()
        .find(|site| asked(site))
        .map(|site| {
            // **Before the relocation is planned, not after.** `doc/adr/1163`: marks put back onto
            // the producer's own page are drawn by operators no marked-content sequence brackets,
            // which §14.8.2.2.1 makes an artifact in a tagged document — and the marks are the
            // producer's real content. Naming a structure type for them is what this conversion
            // will not do, so the refusal is the same one an appended page of marks gets.
            if tagged::describes_its_content(document) {
                return Err(Because::NotBuiltYet(tagged::MARKS_IN_A_TAGGED_DOCUMENT));
            }
            // The removal is what makes the file conform and the marks are what a preserve keeps,
            // so a document whose annotations cannot be removed gets neither — the packet site's
            // own construction, for its reason.
            let removals = forbidden.map_err(|because| *because)?;
            let marks = marks_of(document, removals)?;
            relocate_marks(document, site, removals, &marks, spare).map(|plan| (site, plan))
        })
        .transpose()?;
    // ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9: the files the removal takes out, each
    // laid out as pages where this reader can lay it out and answered by the row's `on-failure`
    // where it cannot (`doc/adr/2536`).
    // `derive:<tool>:append` reaches the same files by the derivation rather than a `preserve`
    // row: each was converted by `super::super::derived`, and its pages are appended here.
    let attachment_site = ATTACHMENT_REQUIREMENTS.into_iter().find(|site| {
        asked(site) || (failed.contains(site) && appended.values().any(|file| file.site == *site))
    });
    let discard_attachments = attachment_site.is_some_and(|site| {
        plan.preservations.iter().any(|preservation| {
            preservation.site == site
                && preservation.no_page_carries == super::super::RemedyKind::Discard
        })
    });
    let mut attachments = match attachment_site {
        Some(site) => {
            let files = embedded.map_err(|because| *because)?;
            attachments_to_lay_out(
                document,
                site,
                files,
                discard_attachments,
                (appended, &remedies.drawn),
            )?
        }
        None => AttachmentsToLayOut::default(),
    };
    // **`append-statement`** (`doc/rfc/0014` section 4, `A706`): the loss `discard` takes, then
    // one page listing every object it took. A row whose first step is the statement removes
    // every file, as `discard` does; one whose statement follows `preserve:append` lists the files
    // no page carried.
    let stated = ATTACHMENT_REQUIREMENTS.into_iter().find_map(|site| {
        plan.preservations
            .iter()
            .find(|preservation| preservation.site == site && failed.contains(site))
            .and_then(|preservation| preservation.statement.as_ref())
            .map(|asked| (site, asked))
    });
    if let Some((site, _)) = stated
        && attachment_site != Some(site)
    {
        let files = embedded.map_err(|because| *because)?;
        if files.each.len() > statement::MOST_STATED_OBJECTS {
            return Err(Because::NotBuiltYet(statement::TOO_MANY_STATED));
        }
        attachments
            .removed
            .extend(
                files
                    .each
                    .iter()
                    .map(|(at, name)| super::super::report::AttachmentOutcome {
                        site,
                        object: *at,
                        name: name.clone(),
                        annotations: Vec::new(),
                        answer: super::super::report::AttachmentAnswer::Removed(
                            super::super::report::DISCARDED_BY_THE_ROW.to_owned(),
                        ),
                        stated_on: None,
                    }),
            );
    }
    let graphics = stated_graphics(plan, document, input, failed);
    let fonts = stated_fonts(plan, substitutes);
    let statement_text = statement_of(
        document,
        plan.target,
        stated.map(|(_, asked)| (attachments.removed.as_slice(), asked)),
        &graphics,
        packets_stated,
        omitted_stated,
        information_stated,
        fonts.as_ref(),
        runs,
        recovery,
    )?;
    if packets.is_empty()
        && replaced.is_empty()
        && annotation_site.is_none()
        && attachments.texts.is_empty()
        && attachments.derived.is_empty()
        && attachments.drawn.is_empty()
        && statement_text.is_none()
    {
        // Every file the row was asked about fell to its `on-failure`: nothing is composed, and
        // the outcome per file is the whole of what the preservation did. And every packet a
        // packet row replaces held no bytes to lay out: the replacement is the remedy, reported
        // per stream, and no page is owed (`doc/adr/2676`).
        if !attachments.removed.is_empty() || replaced_site.is_some() {
            let mut composed = Composed {
                written: Vec::new(),
                pages: Vec::new(),
                labels: None,
                carried: Vec::new(),
                relocations: BTreeMap::new(),
                structure: None,
                attachments: attachments.removed,
                linked_annotations: BTreeMap::new(),
                statement: None,
                removed_packets,
                removed_information: information.to_vec(),
                omitted_characters,
            };
            link_the_annotations(&mut composed, embedded);
            return Ok(composed);
        }
        return Err(Because::NotBuiltYet(NOT_ASKED_FOR));
    }
    let mut keeps: Vec<preserve::Keep<'_>> = packets
        .iter()
        .map(|(at, bytes)| preserve::Keep {
            site: SCHEMA_REQUIREMENT,
            subject: format!(
                "the XMP metadata packet object {} {} holds, as the producer wrote it",
                at.number, at.generation
            ),
            content: preserve::Kept::Text(bytes),
            declined: None,
        })
        .collect();
    if let Some(site) = replaced_site {
        keeps.extend(replaced.iter().map(|(at, bytes)| preserve::Keep {
            site,
            subject: format!(
                "the XMP metadata packet object {} {} held, as the producer wrote it",
                at.number, at.generation
            ),
            content: preserve::Kept::Text(bytes),
            declined: None,
        }));
    }
    // **The relocations are decided before the appended pages are composed**, because a mark that
    // could not be relocated onto the producer's own page (`doc/adr/1123`'s two refusals) falls
    // back to the appended page, which is `doc/adr/1099`'s mechanism kept whole. So the fallback
    // marks join the keeps `compose` lays out, carrying the sentence that says why.
    let first_attachment = keeps.len();
    keeps.extend(
        attachments
            .texts
            .iter()
            .map(|(site, at, name, bytes)| preserve::Keep {
                site,
                subject: format!(
                    "the embedded file {name} (file specification object {} {}), its text as the \
                     file holds it",
                    at.number, at.generation
                ),
                content: preserve::Kept::Text(bytes),
                declined: None,
            }),
    );
    // Each derived file's pages, one keep per page, after the texts: a site is answered by a
    // `preserve` row or by a derivation and never both, so at most one of the two lists is full.
    let first_derived = keeps.len();
    if !attachments.derived.is_empty() && tagged::describes_its_content(document) {
        return Err(Because::NotBuiltYet(
            super::super::derived::DERIVED_PAGES_IN_A_TAGGED_DOCUMENT,
        ));
    }
    let mut imported: Vec<(usize, Vec<(ObjectId, Object)>)> = Vec::new();
    for (_, at, name, bytes) in &attachments.derived {
        let pages = super::super::derived::import_pages(
            document,
            bytes,
            document.limits(),
            preserve::MOST_PAGES,
            spare,
        )?;
        let count = pages.pages.len();
        for (index, marks) in pages.pages.into_iter().enumerate() {
            keeps.push(preserve::Keep {
                site: attachments.derived_site(),
                subject: format!(
                    "page {} of the PDF derived from the embedded file {name} (file \
                     specification object {} {}) and converted by this converter",
                    index.saturating_add(1),
                    at.number,
                    at.generation
                ),
                content: preserve::Kept::Marks(marks),
                declined: None,
            });
        }
        imported.push((count, pages.written));
    }
    // Each drawn file's pages after the derived ones (`super::super::drawn`): a PDF's imported as a
    // derived file's are, an image's one page of its own. A file whose pages cannot be written
    // takes the row's `on-failure` like any other file no page carries.
    let first_drawn = keeps.len();
    let mut drawn_laid: Vec<DrawnLaid> = Vec::new();
    for (site, at, name, file) in &attachments.drawn {
        let pages = match &file.drawn {
            super::super::remedies::Drawn::Pdf(bytes) => super::super::derived::import_pages(
                document,
                bytes,
                document.limits(),
                preserve::MOST_PAGES,
                spare,
            ),
            super::super::remedies::Drawn::Image(image) => {
                super::super::drawn::image_page(document, image, spare)
            }
        };
        let pages = match pages {
            Ok(pages) => pages,
            Err(because) if discard_attachments => {
                attachments
                    .removed
                    .push(super::super::report::AttachmentOutcome {
                        site,
                        object: *at,
                        name: name.clone(),
                        annotations: Vec::new(),
                        answer: super::super::report::AttachmentAnswer::Removed(format!(
                            "no page carries it, because {}",
                            because.sentence()
                        )),
                        stated_on: None,
                    });
                continue;
            }
            Err(because) => return Err(because),
        };
        let count = pages.pages.len();
        let image = matches!(file.drawn, super::super::remedies::Drawn::Image(_));
        for (index, marks) in pages.pages.into_iter().enumerate() {
            keeps.push(preserve::Keep {
                site,
                subject: if image {
                    format!(
                        "the image the embedded file {name} (file specification object {} {}) \
                         holds, drawn at one unit per sample",
                        at.number, at.generation
                    )
                } else {
                    format!(
                        "page {} of the PDF the embedded file {name} (file specification object \
                         {} {}) holds, converted by this converter",
                        index.saturating_add(1),
                        at.number,
                        at.generation
                    )
                },
                content: preserve::Kept::Marks(marks),
                declined: None,
            });
        }
        drawn_laid.push((site, *at, name.clone(), count, pages.written));
    }
    let drawn_keeps = keeps.len().saturating_sub(first_drawn);
    let relocation = annotation_site.map(|(site, plan)| {
        for (marks, subject, declined) in plan.fallback {
            keeps.push(preserve::Keep {
                site,
                subject,
                content: preserve::Kept::Marks(marks),
                declined: Some(declined),
            });
        }
        (plan.relocations, plan.written, plan.rows)
    });
    // Last, so that its row is the last `compose` writes and the pages it takes are the last.
    let statement_site = stated
        .map(|(site, _)| site)
        .or_else(|| packets_stated.and(replaced_site))
        .or_else(|| omitted_stated.and(replaced_site))
        .or_else(|| information_stated.and(information_site))
        .or_else(|| graphics.first().map(|entry| entry.site))
        .or_else(|| fonts.as_ref().map(|_| super::super::config::STATED_FONTS))
        .or_else(|| (!runs.is_empty()).then_some(super::super::config::STATED_RUNS))
        .or_else(|| recovery.structure.map(|_| RECOVERED_SITE))
        .or_else(|| recovery.not_held.map(|_| NOT_HELD_SITE));
    if let (Some(site), Some(text)) = (statement_site, statement_text.as_ref()) {
        keeps.push(statement_keep(site, text));
    }
    let laid_out = attachments.texts.len();
    // The text keeps a fallback composition leaves out, so that the rows after them are found.
    let mut texts_dropped = 0;
    let before = spare.next;
    let mut composed = match preserve::compose(document, &keeps, has_output_intent, rebuilt, spare)
    {
        Ok(composed) => composed,
        // **A file no page can carry takes `on-failure`, and so does one whose page cannot be
        // composed** (`doc/adr/2536`): with `discard`, the files set as text and the files drawn
        // are removed and named with the composition's own sentence, and the rest of what the
        // plan preserves is composed again without them. The numbers the failed attempt took
        // are given back first; those the drawn files' imports took stay unused.
        Err(because) if discard_attachments && (laid_out > 0 || drawn_keeps > 0) => {
            spare.next = before;
            let stated_keep = statement_text
                .as_ref()
                .map(|_| keeps.len().saturating_sub(1));
            let mut others: Vec<preserve::Keep<'_>> = keeps
                .into_iter()
                .enumerate()
                .filter(|(index, _)| {
                    let text = (first_attachment..first_attachment.saturating_add(laid_out))
                        .contains(index);
                    let drawn =
                        (first_drawn..first_drawn.saturating_add(drawn_keeps)).contains(index);
                    !text && !drawn && Some(*index) != stated_keep
                })
                .map(|(_, keep)| keep)
                .collect();
            let taken = attachments
                .texts
                .iter()
                .map(|(site, at, name, _)| (*site, *at, name.clone()))
                .chain(
                    drawn_laid
                        .drain(..)
                        .map(|(site, at, name, _, _)| (site, at, name)),
                );
            for (site, at, name) in taken {
                attachments
                    .removed
                    .push(super::super::report::AttachmentOutcome {
                        site,
                        object: at,
                        name,
                        annotations: Vec::new(),
                        answer: super::super::report::AttachmentAnswer::Removed(format!(
                            "no page carries it, because {}",
                            because.sentence()
                        )),
                        stated_on: None,
                    });
            }
            // The statement is written again over every file now removed, the ones the failed
            // composition took with them included.
            let restated = statement_of(
                document,
                plan.target,
                stated.map(|(_, asked)| (attachments.removed.as_slice(), asked)),
                &graphics,
                packets_stated,
                omitted_stated,
                information_stated,
                fonts.as_ref(),
                runs,
                recovery,
            )?;
            if let (Some(site), Some(text)) = (statement_site, restated.as_ref()) {
                others.push(statement_keep(site, text));
            }
            if others.is_empty() {
                let mut composed = Composed {
                    written: Vec::new(),
                    pages: Vec::new(),
                    labels: None,
                    carried: Vec::new(),
                    relocations: BTreeMap::new(),
                    structure: None,
                    attachments: attachments.removed,
                    linked_annotations: BTreeMap::new(),
                    statement: None,
                    removed_packets,
                    removed_information: information.to_vec(),
                    omitted_characters,
                };
                link_the_annotations(&mut composed, embedded);
                return Ok(composed);
            }
            let composed = preserve::compose(document, &others, has_output_intent, rebuilt, spare)?;
            attachments.texts.clear();
            texts_dropped = laid_out;
            composed
        }
        Err(because) => return Err(because),
    };
    let mut stated_information = None;
    if statement_site.is_some() && statement_text.is_some() {
        let files_stated = stated.is_some() && !attachments.removed.is_empty();
        let mut sites: Vec<&'static str> = stated
            .filter(|_| files_stated)
            .map(|(site, _)| site)
            .into_iter()
            .collect();
        if let (true, Some(site)) = (
            packets_stated.is_some() || omitted_stated.is_some(),
            replaced_site,
        ) {
            sites.push(site);
        }
        if let (Some(_), Some(site)) = (information_stated, information_site) {
            sites.push(site);
        }
        for entry in &graphics {
            if !sites.contains(&entry.site) {
                sites.push(entry.site);
            }
        }
        if fonts.is_some() {
            sites.push(super::super::config::STATED_FONTS);
        }
        if !runs.is_empty() {
            sites.push(super::super::config::STATED_RUNS);
        }
        if recovery.structure.is_some() {
            sites.push(RECOVERED_SITE);
        }
        if recovery.not_held.is_some() {
            sites.push(NOT_HELD_SITE);
        }
        let files = if files_stated {
            attachments.removed.len()
        } else {
            0
        };
        let listed = super::super::report::Listed {
            removed_objects: files
                .saturating_add(packets_stated.map_or(0, |(removed, _)| removed.len())),
            omitted_characters: omitted_stated.map_or(0, |(omitted, _)| omitted.len()),
            removed_keys: information_stated.map_or(0, |(removed, _)| removed.len()),
            halftone_entries: graphics.len(),
            font_records: fonts.as_ref().map_or(0, statement::FontsStated::listed),
            run_records: runs.len(),
            recovery_records: recovery.entries(),
        };
        let page = the_statement_page(&mut composed, sites, listed)?;
        if files_stated {
            for outcome in &mut attachments.removed {
                outcome.stated_on = Some(page);
            }
        }
        if packets_stated.is_some() {
            for packet in &mut removed_packets {
                packet.stated_on = Some(page);
            }
        }
        if omitted_stated.is_some() {
            for omitted in &mut omitted_characters {
                omitted.stated_on = Some(page);
            }
        }
        if information_stated.is_some() {
            stated_information = Some(page);
        }
    }
    composed.removed_packets = removed_packets;
    composed.omitted_characters = omitted_characters;
    composed.removed_information = information
        .iter()
        .map(|entry| super::RemovedInformation {
            stated_on: stated_information,
            ..entry.clone()
        })
        .collect();
    // Each laid-out file's pages, read off the keep that carried it: `compose` writes one
    // `Preserved` row per keep and in the keeps' order.
    composed.attachments = attachments.removed;
    for (row, (site, at, name, _)) in composed
        .carried
        .iter()
        .skip(first_attachment)
        .zip(&attachments.texts)
    {
        composed
            .attachments
            .push(super::super::report::AttachmentOutcome {
                site,
                object: *at,
                name: name.clone(),
                annotations: Vec::new(),
                answer: super::super::report::AttachmentAnswer::AppendedAsPages(row.pages.clone()),
                stated_on: None,
            });
    }
    let mut rows = composed
        .carried
        .iter()
        .skip(first_derived.saturating_sub(texts_dropped));
    for ((site, at, name, _), (count, written)) in attachments.derived.iter().zip(imported) {
        let pages: Vec<usize> = rows
            .by_ref()
            .take(count)
            .flat_map(|row| row.pages.iter().copied())
            .collect();
        composed.written.extend(written);
        composed
            .attachments
            .push(super::super::report::AttachmentOutcome {
                site,
                object: *at,
                name: name.clone(),
                annotations: Vec::new(),
                answer: super::super::report::AttachmentAnswer::AppendedAsPages(pages),
                stated_on: None,
            });
    }
    for (site, at, name, count, written) in drawn_laid {
        let pages: Vec<usize> = rows
            .by_ref()
            .take(count)
            .flat_map(|row| row.pages.iter().copied())
            .collect();
        composed.written.extend(written);
        composed
            .attachments
            .push(super::super::report::AttachmentOutcome {
                site,
                object: at,
                name,
                annotations: Vec::new(),
                answer: super::super::report::AttachmentAnswer::AppendedAsPages(pages),
                stated_on: None,
            });
    }
    composed
        .attachments
        .sort_by_key(|outcome| (outcome.object.number, outcome.object.generation));
    link_the_annotations(&mut composed, embedded);
    // **One act answers all three rows, and each of them gets a row saying so.** A stream's
    // packet is laid out once however many of ISO 19005-2 section 6.6.2.1's rules it broke, so
    // the pages are the same pages — and `super::super::configured` asks, per requirement, whether the
    // preservation carried anything for it. A row left out would refuse a requirement this
    // conversion did answer.
    let also: Vec<Preserved> = composed
        .carried
        .iter()
        .filter(|row| Some(row.site) == replaced_site)
        .flat_map(|row| {
            replaced_sites.iter().skip(1).map(|site| Preserved {
                site,
                ..row.clone()
            })
        })
        .collect();
    composed.carried.extend(also);
    if let Some((relocations, written, rows)) = relocation {
        composed.written.extend(written);
        composed.carried.extend(rows);
        composed.relocations = relocations;
    }
    Ok(composed)
}

/// Points each file attachment annotation that named a file at what became of the file.
///
/// §12.5.6.15's annotation stays on its page with the producer's appearance, as a §12.5.6.5 link
/// to the first page its file was laid out on, or to nothing where its file was removed — either
/// way it names no file, which is what the embedded-file requirement asks (`doc/adr/2537`). The
/// appended pages are in page order, so the first page any row carries is the first appended.
fn link_the_annotations(
    composed: &mut Composed,
    embedded: Result<&sites::EmbeddedFiles, &Because>,
) {
    let Ok(files) = embedded else {
        return;
    };
    let first = composed
        .carried
        .iter()
        .flat_map(|row| row.pages.iter().copied())
        .min();
    for (annotation, named) in &files.annotations {
        let Some(outcome) = composed
            .attachments
            .iter_mut()
            .find(|outcome| outcome.object == *named)
        else {
            continue;
        };
        let page = match &outcome.answer {
            super::super::report::AttachmentAnswer::AppendedAsPages(pages) => pages
                .first()
                .zip(first)
                .and_then(|(page, first)| page.checked_sub(first))
                .and_then(|index| composed.pages.get(index))
                .copied(),
            super::super::report::AttachmentAnswer::Removed(_) => None,
        };
        outcome.annotations.push(*annotation);
        composed.linked_annotations.insert(*annotation, page);
    }
}

/// The statement's text over the files an embedded-file row removed, the entries the halftone
/// rows took, the packets a packet row removed (`doc/adr/2678`), the document information entries
/// an information row removed (`doc/adr/2681`), the pages photographed and faces embedded with a loss under the font row
/// (`doc/adr/2600`), the runs the operator row wrote as numbers (`doc/adr/2638`), the rebuilt
/// structure (`doc/adr/2669`) and the pages the source states and the file does not hold, or
/// `None` where none of the eight lists anything. The structure and the pages not held are
/// the last entries.
#[expect(
    clippy::too_many_arguments,
    reason = "one argument per kind of entry the page lists, each a different preparation's \
              record; bundling them would be a struct built at exactly the two call sites"
)]
fn statement_of(
    document: &Document,
    target: Target,
    files: Option<(
        &[super::super::report::AttachmentOutcome],
        &super::super::config::Statement,
    )>,
    graphics: &[statement::GraphicsEntry<'_>],
    packets: Option<(&[super::RemovedPacket], &super::super::config::Statement)>,
    omitted: Option<(&[super::OmittedCharacter], &super::super::config::Statement)>,
    information: Option<(
        &[super::RemovedInformation],
        &super::super::config::Statement,
    )>,
    fonts: Option<&statement::FontsStated<'_>>,
    runs: &[super::super::WrittenRun],
    recovery: super::super::recovered::Stated<'_>,
) -> Result<Option<String>, Because> {
    let files = files.filter(|(removed, _)| !removed.is_empty());
    if files.is_none()
        && packets.is_none()
        && omitted.is_none()
        && information.is_none()
        && graphics.is_empty()
        && fonts.is_none()
        && runs.is_empty()
        && recovery.is_empty()
    {
        return Ok(None);
    }
    let listed: Vec<&super::super::report::AttachmentOutcome> = files
        .map(|(removed, _)| removed.iter().collect())
        .unwrap_or_default();
    let mut text = statement::text(
        document,
        target,
        files.map(|(_, asked)| (listed.as_slice(), asked)),
        graphics,
    )
    .map_err(Because::NotBuiltYet)?;
    let mut before = listed.len().saturating_add(graphics.len());
    if let Some((removed, asked)) = packets {
        statement::packet_entries(&mut text, target, before, removed, asked)
            .map_err(Because::NotBuiltYet)?;
        before = before.saturating_add(removed.len());
    }
    if let Some((omitted, asked)) = omitted {
        statement::omitted_entries(&mut text, target, before, omitted, asked)
            .map_err(Because::NotBuiltYet)?;
        before = before.saturating_add(omitted.len());
    }
    if let Some((removed, asked)) = information {
        statement::information_entries(&mut text, target, before, removed, asked)
            .map_err(Because::NotBuiltYet)?;
        before = before.saturating_add(removed.len());
    }
    if let Some(fonts) = fonts {
        statement::font_entries(&mut text, target, fonts, before).map_err(Because::NotBuiltYet)?;
        before = before.saturating_add(fonts.listed());
    }
    if !runs.is_empty() {
        statement::runs_entry(&mut text, target, before, runs).map_err(Because::NotBuiltYet)?;
        before = before.saturating_add(runs.len());
    }
    if let Some(structure) = recovery.structure {
        statement::recovered_entry(&mut text, before, structure).map_err(Because::NotBuiltYet)?;
        before = before.saturating_add(1);
    }
    if let Some(not_held) = recovery.not_held {
        statement::not_held_entry(&mut text, before, not_held).map_err(Because::NotBuiltYet)?;
    }
    Ok(Some(text))
}

/// The filters a stream's dictionary states, in their written form and in order (ISO 32000-2
/// Table 5's `Filter`, a name or an array of names); an entry that is neither is written as the
/// statement page's fixed words for it rather than read as no filter.
fn filters_stated(document: &Document, dict: &pdf_syntax::object::Dictionary) -> Vec<String> {
    let written = |object: &Object| {
        document.resolve(object).as_name().map_or_else(
            || "an entry that is not a name".to_owned(),
            |name| format!("/{}", name.escaped()),
        )
    };
    match dict.get("Filter").map(|filter| document.resolve(filter)) {
        None => Vec::new(),
        Some(Object::Array(filters)) => filters.iter().map(written).collect(),
        Some(filter) => vec![written(&filter)],
    }
}

/// Whether the font row's statement page may be composed: its row names `append-statement`, and
/// the fonts' preparation will run, because the font row failed or a content-stream row of
/// `super::super::fenced` failed where the profile authorised `page-as-image`.
pub(super) fn fonts_may_be_stated(plan: &ArchivePlan, failed: &BTreeSet<&'static str>) -> bool {
    plan.preservations.iter().any(|preservation| {
        preservation.site == super::super::config::STATED_FONTS && preservation.statement.is_some()
    }) && (failed.contains(super::super::config::STATED_FONTS)
        || (plan.authorised.grants(Loss::PageAsImage)
            && super::super::fenced::ROWS
                .iter()
                .any(|row| failed.contains(row))))
}

/// Whether a `preserve` row may lay a metadata packet out on an appended page: a packet row or the
/// property row answered by page and failed by this document, the two sites
/// [`the_preserved_pages`] keeps a packet's bytes for as [`preserve::Kept::Text`]
/// (`doc/adr/2676`). An embedded file's text is not asked about here: its row also derives and
/// draws, and those pages bring their own families through `Remedies`.
pub(super) fn packets_may_be_preserved(
    plan: &ArchivePlan,
    failed: &BTreeSet<&'static str>,
) -> bool {
    PACKET_REQUIREMENTS
        .into_iter()
        .chain([SCHEMA_REQUIREMENT])
        .any(|site| {
            failed.contains(site)
                && plan
                    .preservations
                    .iter()
                    .any(|preservation| preservation.site == site && preservation.by_page)
        })
}

/// Whether the operator row's statement page may be composed: its row names `append-statement`,
/// the row failed and the profile authorised `content-as-drawn`, so the runs are written
/// (`doc/adr/2638`).
pub(super) fn runs_may_be_stated(plan: &ArchivePlan, failed: &BTreeSet<&'static str>) -> bool {
    plan.preservations.iter().any(|preservation| {
        preservation.site == super::super::config::STATED_RUNS && preservation.statement.is_some()
    }) && failed.contains(super::super::config::STATED_RUNS)
        && plan.authorised.grants(Loss::ContentAsDrawn)
}

/// The runs the operator row's statement lists, where [`runs_may_be_stated`] and they were
/// written: each is the report's own record, so the page lists exactly what the report names.
pub(super) fn stated_runs<'a>(
    plan: &ArchivePlan,
    failed: &BTreeSet<&'static str>,
    drawn: Option<&'a super::super::as_drawn::Drawn>,
) -> &'a [super::super::WrittenRun] {
    match drawn {
        Some(drawn) if runs_may_be_stated(plan, failed) => &drawn.runs,
        _ => &[],
    }
}

/// What the font row states, where its row names `append-statement` and the fonts' preparation
/// photographed a page or embedded a face with a loss (`doc/adr/2600`).
///
/// Every record is the report's own — `Substitutes::photographed` and `Substitutes::done` become
/// the report's `photographed` and `substituted` unchanged — so the page lists exactly what the
/// report names. A page photographed for a fault of its content stream is listed too: the font
/// row's `discard` is what authorised `page-as-image` in the profile.
fn stated_fonts<'a>(
    plan: &'a ArchivePlan,
    substitutes: Option<&'a Substitutes>,
) -> Option<statement::FontsStated<'a>> {
    let asked = plan
        .preservations
        .iter()
        .find(|preservation| preservation.site == super::super::config::STATED_FONTS)
        .and_then(|preservation| preservation.statement.as_ref())?;
    let substitutes = substitutes?;
    let stated = statement::FontsStated {
        pages: &substitutes.photographed,
        faces: &substitutes.done,
        asked,
    };
    (stated.listed() > 0).then_some(stated)
}

/// What the statement page's record names a `page-not-held` entry under: the loss word, because
/// no requirement's row states it (`doc/adr/2596`).
const NOT_HELD_SITE: &str = "page-not-held";

/// What the statement page's record names a `recovered-structure` entry under, for the same reason
/// (`doc/adr/2669`).
const RECOVERED_SITE: &str = "recovered-structure";

/// Every entry a halftone row whose loss is stated takes, in `STATED_GRAPHICS`'s order and each
/// population's own (`doc/adr/2562`).
///
/// Read from the same `super::super::halftones` population the rewrite acts on, so the page lists
/// exactly the objects the output changes. A population that refuses is the rewrite's refusal at
/// its own site, and nothing is stated for it here.
fn stated_graphics<'a>(
    plan: &'a ArchivePlan,
    document: &Document,
    input: &pdf_archive::Report,
    failed: &BTreeSet<&'static str>,
) -> Vec<statement::GraphicsEntry<'a>> {
    let mut out = Vec::new();
    for site in super::super::config::STATED_GRAPHICS {
        if !failed.contains(site) {
            continue;
        }
        let Some(asked) = plan
            .preservations
            .iter()
            .find(|preservation| preservation.site == site)
            .and_then(|preservation| preservation.statement.as_ref())
        else {
            continue;
        };
        let population = match site {
            "graphics/no-halftone-name" => super::super::halftones::removal(document, input),
            "graphics/halftone-type-is-one-or-five" => {
                super::super::halftones::defaults(document, input)
            }
            "graphics/no-halftone-origin-in-a-graphics-state" => {
                super::super::halftones::origins(document, input)
            }
            _ => continue,
        };
        let Ok(population) = population else {
            continue;
        };
        out.extend(
            population
                .at
                .into_iter()
                .map(|object| statement::GraphicsEntry {
                    site,
                    object,
                    asked,
                }),
        );
    }
    out
}

/// The keep that lays out a statement's text on ADR 1014's appended page.
fn statement_keep<'a>(site: &'static str, text: &'a str) -> preserve::Keep<'a> {
    preserve::Keep {
        site,
        subject: statement::SUBJECT.to_owned(),
        content: preserve::Kept::Text(text.as_bytes()),
        declined: None,
    }
}

/// Takes the statement's row off what `compose` carried and makes it the statement's page.
///
/// The statement is the last keep, so its row is the last one `compose` wrote; the pages it took
/// are bounded by [`statement::MOST_STATEMENT_PAGES`]. Returns the first of them.
fn the_statement_page(
    composed: &mut Composed,
    sites: Vec<&'static str>,
    listed: super::super::report::Listed,
) -> Result<usize, Because> {
    let row = composed
        .carried
        .pop()
        .filter(|row| row.subject == statement::SUBJECT)
        .ok_or(Because::NotBuiltYet(statement::NOTHING_STATED))?;
    if row.pages.len() > statement::MOST_STATEMENT_PAGES {
        return Err(Because::NotBuiltYet(statement::STATEMENT_TOO_MANY_PAGES));
    }
    let first = row
        .pages
        .first()
        .copied()
        .ok_or(Because::NotBuiltYet(statement::NOTHING_STATED))?;
    composed.statement = Some(super::super::report::StatementPage {
        sites,
        pages: row.pages,
        listed,
        face: row.face,
    });
    Ok(first)
}

/// How many embedded files one conversion lays out as pages before the rest take `on-failure`.
///
/// `CLAUDE.md` principle 3's explicit bound, checked before any file is decoded: the name tree is
/// the producer's, and a document naming ten thousand files costs ten thousand outcomes rather
/// than ten thousand layouts. [`preserve`]'s own page budget bounds what the ones taken can cost.
pub(in crate::archive) const MOST_ATTACHMENTS_AS_PAGES: usize = 16;

/// The longest file, in decoded bytes, laid out as text.
///
/// A bound checked before the layout, so that a hostile attachment cannot make `wrapped` hold
/// every line of a file whose pages [`preserve`]'s budget would refuse anyway: sixty-four pages
/// of about sixty lines of about eighty characters is some three hundred thousand, and 256 KiB
/// sits under it with every character a byte.
const MOST_BYTES_AS_TEXT: usize = 256 * 1024;

/// Why an embedded file is not laid out because it is not text.
const ATTACHMENT_NOT_TEXT: &str = "its bytes are not UTF-8 text free of control characters, nor \
     a PDF, PNG or JPEG file, and this reader lays out as pages only an attachment it can set as \
     the characters it holds or draw as the pages or the image it holds";

/// Why an embedded file is not laid out because its stream does not decode.
const ATTACHMENT_NOT_DECODED: &str = "its embedded file stream does not decode, so there are no \
     bytes a page could carry";

/// Why an embedded file is not laid out because it is longer than the bound.
const ATTACHMENT_TOO_LONG: &str = "it is longer than the 256 KiB this converter lays out as text \
     pages (MOST_BYTES_AS_TEXT)";

/// Why an embedded file past the count bound is not laid out.
const ATTACHMENT_PAST_THE_COUNT: &str = "the document names more embedded files than the sixteen \
     this converter lays out as pages in one conversion (MOST_ATTACHMENTS_AS_PAGES)";

/// Why an embedded file nothing here can lay out stops a row whose `on-failure` is `stop`.
const ATTACHMENT_NOT_LAID_OUT: &str = "an embedded file this target does not admit cannot be \
     laid out as pages (it is neither UTF-8 text nor a PDF, PNG or JPEG file this reader draws, \
     does not decode, is text longer than 256 KiB, or is past the sixteenth), and the row's \
     on-failure is stop; on-failure = \"discard\" removes such a file \
     and names it, one answer per file (doc/adr/2536)";

/// One drawn file whose pages were written: its row, its specification, its name, how many
/// pages it brought and the objects they reach.
type DrawnLaid = (
    &'static str,
    ObjectId,
    String,
    usize,
    Vec<(ObjectId, Object)>,
);

/// The files a remedy answered before the layout: each derivation's converted PDF, and each
/// drawn file or the sentence saying why it is not drawn, by file specification.
type AnsweredFiles<'a> = (
    &'a BTreeMap<ObjectId, super::super::remedies::AppendedFile>,
    &'a BTreeMap<ObjectId, Result<super::super::remedies::DrawnFile, String>>,
);

/// The embedded files one conversion lays out, and the ones it removes instead.
#[derive(Default)]
struct AttachmentsToLayOut<'a> {
    /// Each file laid out as text: its row, its specification, its name and its bytes.
    texts: Vec<(&'static str, ObjectId, String, std::sync::Arc<[u8]>)>,
    /// Each file whose derived PDF's pages are appended (`derive:<tool>:append`): its row, its
    /// specification, its name and the converted file.
    derived: Vec<(&'static str, ObjectId, String, &'a [u8])>,
    /// Each file drawn as pages (`super::super::drawn`): its row, its specification, its name and what
    /// is drawn.
    drawn: Vec<(
        &'static str,
        ObjectId,
        String,
        &'a super::super::remedies::DrawnFile,
    )>,
    /// Each file no page carries, with the sentence saying why.
    removed: Vec<super::super::report::AttachmentOutcome>,
}

/// Sorts the files a removal takes out into those laid out as pages and those removed.
///
/// **One answer per file** (`doc/adr/2536`): the row is the site's, and the outcome is each
/// object's. A file this reader cannot set as text takes the row's `on-failure` — `discard`
/// removes it and names it, and `stop` refuses the conversion with [`ATTACHMENT_NOT_LAID_OUT`].
fn attachments_to_lay_out<'a>(
    document: &Document,
    site: &'static str,
    files: &sites::EmbeddedFiles,
    discard: bool,
    (appended, drawn): AnsweredFiles<'a>,
) -> Result<AttachmentsToLayOut<'a>, Because> {
    let mut out = AttachmentsToLayOut::default();
    // A derivation answers every file of its site or none (`super::super::remedies`), so a file it did
    // not convert is a refusal of the site rather than a file for the text layout.
    if appended.values().any(|file| file.site == site) {
        for (at, name) in &files.each {
            let file = appended
                .get(at)
                .ok_or(Because::NotBuiltYet(DERIVED_FILE_NOT_CONVERTED))?;
            out.derived.push((site, *at, name.clone(), &file.bytes));
        }
        return Ok(out);
    }
    for (index, (at, name)) in files.each.iter().enumerate() {
        let refused = if index >= MOST_ATTACHMENTS_AS_PAGES {
            Some(ATTACHMENT_PAST_THE_COUNT.to_owned())
        } else if let Some(file) = drawn.get(at) {
            // A PDF or an image, answered before the layout because the colour it paints in
            // decided the output intent (`super::super::remedies`).
            match file {
                Ok(file) => {
                    out.drawn.push((site, *at, name.clone(), file));
                    None
                }
                Err(sentence) => Some(sentence.clone()),
            }
        } else {
            match attached_bytes(document, *at) {
                None => Some(ATTACHMENT_NOT_DECODED.to_owned()),
                Some(bytes) if bytes.len() > MOST_BYTES_AS_TEXT => {
                    Some(ATTACHMENT_TOO_LONG.to_owned())
                }
                Some(bytes) if !reads_as_text(&bytes) => Some(ATTACHMENT_NOT_TEXT.to_owned()),
                Some(bytes) => {
                    out.texts.push((site, *at, name.clone(), bytes));
                    None
                }
            }
        };
        if let Some(because) = refused {
            if !discard {
                return Err(Because::NotBuiltYet(ATTACHMENT_NOT_LAID_OUT));
            }
            out.removed.push(super::super::report::AttachmentOutcome {
                site,
                object: *at,
                name: name.clone(),
                annotations: Vec::new(),
                answer: super::super::report::AttachmentAnswer::Removed(format!(
                    "no page carries it, because {because}"
                )),
                stated_on: None,
            });
        }
    }
    Ok(out)
}

impl AttachmentsToLayOut<'_> {
    /// The row the derived files answer; every file of one site carries the same one.
    fn derived_site(&self) -> &'static str {
        self.derived
            .first()
            .map_or(ATTACHMENT_REQUIREMENTS[0], |(site, ..)| site)
    }
}

/// Why a derivation's site is refused where one of its files was not converted.
const DERIVED_FILE_NOT_CONVERTED: &str = "an embedded file this requirement failed at has no \
     converted derivation, so the pages derived from the others are not appended alone";

/// The decoded bytes of the file one specification embeds: §7.11.4's `/EF`, `/UF` before `/F`.
///
/// Decoded under the document's own limits, so the filters' budgets bound what this reads.
pub(in crate::archive) fn attached_bytes(
    document: &Document,
    specification: ObjectId,
) -> Option<std::sync::Arc<[u8]>> {
    let object = document.get(specification);
    let dict = object.as_dict()?;
    let files = document.get_key(dict, "EF");
    let files = files.as_dict()?;
    ["UF", "F"].into_iter().find_map(|key| {
        let Object::Stream(stream) = document.get_key(files, key) else {
            return None;
        };
        document.decoded_stream_data(&stream)
    })
}

/// Whether bytes are text this reader can set as the characters they hold.
///
/// UTF-8, not empty, and free of every control character but the four a text file lays out by —
/// tab, line feed, form feed and carriage return — so that a binary file whose bytes happen to
/// be valid UTF-8 is not set as a page of invisible characters.
fn reads_as_text(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && std::str::from_utf8(bytes).is_ok_and(|text| {
            text.chars()
                .all(|c| !c.is_control() || matches!(c, '\t' | '\n' | '\u{c}' | '\r'))
        })
}

/// What relocating a page's forbidden-annotation marks decided: the on-page edits and the fallbacks.
///
/// `doc/adr/1123`. The marks that could be put back where §12.5.5 had them are in [`Self::rows`]
/// and drive [`Self::relocations`] and [`Self::written`]; the marks a refusal sent to an appended
/// page instead are in [`Self::fallback`], each with the sentence naming its refusal.
struct RelocationPlan {
    /// The producer pages the rewrite edits, keyed by page object.
    relocations: BTreeMap<ObjectId, preserve::RelocatedPage>,
    /// The `q`-prepend and closing content streams the walk adds, in the source's numbering.
    written: Vec<(ObjectId, Object)>,
    /// The report rows for marks relocated onto the producer's own page.
    rows: Vec<Preserved>,
    /// The marks a refusal sent to an appended page: the marks, the report subject, the reason.
    fallback: Vec<(preserve::Marks, String, &'static str)>,
}

/// One removed annotation's report subject, named as the report names it.
fn annotation_subject(removed: &RemovedAnnotation) -> String {
    format!(
        "the normal appearance of the {} annotation object {} {} held on page {}, as the producer \
         wrote it",
        removed.subtype,
        removed.at.number,
        removed.at.generation,
        removed.page.saturating_add(1)
    )
}

/// Decides, per page, which forbidden-annotation marks relocate onto the producer's page.
///
/// **The construction `doc/adr/1123` builds.** Marks are grouped by the page they were on, because
/// the `q`-prepend, the closing stream and the `/Contents` array are one edit per page however many
/// appearances land on it. A page whose producer content is unbalanced or too deeply nested, or
/// whose `/Contents` this cannot wrap, sends all of its marks to an appended page; a single
/// appearance a remaining unhidden annotation would sit over sends only itself.
fn relocate_marks(
    document: &Document,
    site: &'static str,
    removals: &ForbiddenAnnotations,
    marks: &[(&RemovedAnnotation, preserve::Marks)],
    spare: &mut Spare,
) -> Result<RelocationPlan, Because> {
    let tree = pdf_model::Pages::new(document);
    let mut plan = RelocationPlan {
        relocations: BTreeMap::new(),
        written: Vec::new(),
        rows: Vec::new(),
        fallback: Vec::new(),
    };
    // The marks in the order `marks_of` produced them — page order — grouped by page index so that
    // one page's edit is decided once. `BTreeMap` keeps the groups in page order for the report.
    let mut by_page: BTreeMap<usize, Vec<&(&RemovedAnnotation, preserve::Marks)>> = BTreeMap::new();
    for entry in marks {
        by_page.entry(entry.0.page).or_default().push(entry);
    }
    for (page_index, group) in by_page {
        let fall_back = |plan: &mut RelocationPlan, reason: &'static str| {
            for (removed, marks) in group.iter().map(|entry| (entry.0, entry.1.clone())) {
                plan.fallback
                    .push((marks, annotation_subject(removed), reason));
            }
        };
        // A page the model cannot resolve to an object of its own cannot be the one this edits, so
        // its marks take the appended page. This is `doc/adr/1099`'s mechanism, unchanged.
        let Some(page) = tree.get(page_index) else {
            fall_back(&mut plan, PAGE_NOT_RESOLVED);
            continue;
        };
        let Some(page_id) = page.id else {
            fall_back(&mut plan, PAGE_NOT_RESOLVED);
            continue;
        };
        // The `q`/`Q` count must be taken over *all* of the producer's content, so a part that
        // does not decode — an `LZWDecode` stream the reader does not implement, a damaged one —
        // would leave the count short and the closing stream unbalanced. That page's marks take an
        // appended page rather than a miscounted relocation (trap 5: an unsupported input stays
        // loud). §7.8.2 concatenates the parts into the one sequence §8.4.2 is about.
        let (content, issues) = page.content_with_report(document);
        if !issues.is_empty() {
            fall_back(&mut plan, CONTENT_DID_NOT_DECODE);
            continue;
        }
        let depth = match preserve::producer_open_depth(document, &content, &page.resources) {
            Ok(depth) => depth,
            Err(Because::NotBuiltYet(reason)) => {
                fall_back(&mut plan, reason);
                continue;
            }
            Err(other) => return Err(other),
        };
        let original = page.dict.get("Contents").cloned().unwrap_or(Object::Null);
        let remaining = remaining_unhidden_rects(document, &page, &removals.at);
        let mut on_page: Vec<preserve::OnPage> = Vec::new();
        let mut relocated_rows: Vec<Preserved> = Vec::new();
        let mut taken: BTreeSet<Vec<u8>> = BTreeSet::new();
        for (removed, marks) in group.iter().map(|entry| (entry.0, entry.1.clone())) {
            let Some(rect) = annotation_rect(document, removed.at) else {
                // §12.5.5 fits the appearance to the /Rect, so a mark whose landing rectangle
                // cannot be read is one whose overlap this cannot judge — it takes the appended page.
                plan.fallback
                    .push((marks, annotation_subject(removed), NO_RECTANGLE_TO_JUDGE));
                continue;
            };
            let rect = preserve::normalise_rect(rect);
            if remaining
                .iter()
                .any(|other| preserve::overlaps(rect, *other))
            {
                plan.fallback.push((
                    marks,
                    annotation_subject(removed),
                    preserve::REMAINING_ANNOTATION_OVER_THE_MARKS,
                ));
                continue;
            }
            let name = preserve::free_xobject_name(document, &page.resources, &mut taken);
            on_page.push(preserve::OnPage {
                appearance: marks.appearance,
                placement: marks.placement,
                name,
            });
            relocated_rows.push(Preserved {
                site,
                subject: annotation_subject(removed),
                pages: vec![page_index],
                placement: preserve::PLACEMENT_ON_PAGE,
                face: SetIn::NoText,
                declined: None,
            });
        }
        if on_page.is_empty() {
            continue;
        }
        let (relocated, written) = preserve::relocate_a_page(
            document,
            depth,
            &original,
            &page.resources,
            &on_page,
            spare,
        )?;
        plan.relocations.insert(page_id, relocated);
        plan.written.extend(written);
        plan.rows.extend(relocated_rows);
    }
    Ok(plan)
}

/// The normalised rectangles of the annotations that remain on a page and would draw over marks.
///
/// The population `doc/adr/1120` section 4 fixes: every annotation the removal leaves behind
/// (`removed` is [`ForbiddenAnnotations::at`]), that §12.5.3's flags do not hide — clear of both
/// `Hidden` (bit 2) and `NoView` (bit 6) — with a rectangle to read. The standard states no
/// painting order among annotations, so this is the whole of what a converter can know is over the
/// marks, and it over-refuses rather than guess an order.
fn remaining_unhidden_rects(
    document: &Document,
    page: &pdf_model::Page,
    removed: &BTreeSet<ObjectId>,
) -> Vec<[f32; 4]> {
    let Some(annots) = document
        .get_key(&page.dict, "Annots")
        .as_array()
        .map(<[Object]>::to_vec)
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in &annots {
        let Some(id) = entry.as_reference() else {
            continue;
        };
        if removed.contains(&id) {
            continue;
        }
        let Some(dict) = document.get(id).as_dict().cloned() else {
            continue;
        };
        // §12.5.3's Table 167: bit 2 is `Hidden` and bit 6 is `NoView`. An annotation set to
        // either draws nothing a reader sees, so it is not a mark the relocated appearance can go
        // under. An absent or non-integer `/F` states no flags and is neither.
        let flags = document.get_key(&dict, "F").as_integer().unwrap_or(0);
        if flags & HIDDEN != 0 || flags & NO_VIEW != 0 {
            continue;
        }
        if let Some(rect) = annotation_rect(document, id) {
            out.push(preserve::normalise_rect(rect));
        }
    }
    out
}

/// One annotation's `/Rect` as four finite numbers, or `None` where it states none this can read.
fn annotation_rect(document: &Document, id: ObjectId) -> Option<[f32; 4]> {
    let dict = document.get(id).as_dict().cloned()?;
    let rect = document.get_key(&dict, "Rect");
    let values = rect.as_array()?;
    if values.len() != 4 {
        return None;
    }
    let mut out = [0.0_f32; 4];
    for (slot, value) in out.iter_mut().zip(values) {
        let number = document.resolve(value).as_number()?;
        if !number.is_finite() {
            return None;
        }
        // A `/Rect` is device-independent points; the cast to the geometry the overlap test uses
        // loses no coordinate any page states at the scale annotations are placed.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a rectangle coordinate in points, compared at a rectangle's own scale"
        )]
        let number = number as f32;
        *slot = number;
    }
    Some(out)
}

/// §12.5.3's Table 167 `Hidden` flag: bit 2, counting from 1.
const HIDDEN: i64 = 1 << 1;

/// §12.5.3's Table 167 `NoView` flag: bit 6, counting from 1.
const NO_VIEW: i64 = 1 << 5;

/// Why a page with no object identity of its own takes the appended page for its marks.
const PAGE_NOT_RESOLVED: &str = "the page the annotation was on could not be resolved to an object \
     this conversion can add content to, so its marks take an appended page";

/// Why a mark whose landing rectangle cannot be read takes the appended page.
const NO_RECTANGLE_TO_JUDGE: &str = "the annotation states no rectangle ISO 32000-2 \u{a7}12.5.5 \
     could fit its appearance to, so whether a remaining annotation would sit over the relocated \
     marks cannot be judged, and they take an appended page";

/// Why a page whose own content does not fully decode takes the appended page for its marks.
const CONTENT_DID_NOT_DECODE: &str = "part of the producer's own page content does not decode — a \
     filter this reader does not implement, or a damaged stream — so the q/Q balance ISO 32000-2 \
     \u{a7}8.4.2 requires cannot be counted over all of it, and the marks take an appended page \
     rather than a relocation whose closing stream might be unbalanced";

/// The three requirements whose refusal a page of preserved marks answers: one removal takes the
/// annotations all three report (`doc/adr/2051` section 4).
const SUBTYPE_REQUIREMENTS: [&str; 4] = [
    "annotations/subtype-defined-in-iso-32000-1",
    "annotations/subtype-defined-in-iso-32000-2",
    "annotations/three-dimensional-only-in-engineering-files",
    "annotations/file-attachment-only-in-embedded-file-files",
];

/// Why an annotation with no normal appearance stream is refused rather than dropped quietly.
///
/// The operator asked for what the removal would lose to be kept, and for this annotation there is
/// nothing in the file to keep: §12.5.5 makes the normal appearance the marks a reader draws, and
/// an annotation stating none has no marks of its own. Constructing one from the annotation's
/// appearance characteristics is what `pdf_model::appearance` does for the requirement that *asks*
/// for a dictionary; doing it here would mean preserving a picture this program drew and calling
/// it the producer's, which `doc/questions/A48` forbids and `doc/adr/1014` bounds. So the document
/// is refused, naming the annotation, rather than converted with the operator's request silently
/// unmet.
const NO_APPEARANCE_TO_PRESERVE: &str = "this configuration answers an annotation ISO 19005 does \
     not admit by keeping its normal appearance on an appended page, and one of the annotations to \
     be removed states no normal appearance stream — so there are no marks of the producer's to \
     keep. The report names it. Authorising the loss with --authorise forbidden-annotation removes \
     it without a page, which loses nothing that was ever drawn";

/// Why an appearance whose placement §12.5.5 cannot compute is refused.
const NO_PLACEMENT: &str = "an annotation to be removed states a normal appearance and neither it \
     nor the stream states a rectangle ISO 32000-2 \u{a7}12.5.5's algorithm can map onto, so there \
     is nowhere on a page to put the marks that is the producer's choice rather than this \
     program's";

/// Why an appearance on a page this converter cannot measure is refused.
const NO_PAGE_FOR_THE_MARKS: &str = "an annotation to be removed sits on a page whose own boxes \
     this converter cannot read, and a page of preserved marks states the boxes of the page the \
     annotation was on. Inventing a paper size would put the producer's marks somewhere the \
     producer did not";

/// Each removed annotation's normal appearance, with §12.5.5's matrix and its page's geometry.
fn marks_of<'a>(
    document: &Document,
    removals: &'a ForbiddenAnnotations,
) -> Result<Vec<(&'a RemovedAnnotation, preserve::Marks)>, Because> {
    let tree = pdf_model::Pages::new(document);
    let mut out = Vec::new();
    for removed in &removals.removed {
        let appearance = removed
            .appearance
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_TO_PRESERVE))?;
        let annotation = document
            .get(removed.at)
            .as_dict()
            .cloned()
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_TO_PRESERVE))?;
        let stream = document
            .get(appearance)
            .as_stream()
            .map(|stream| stream.dict.clone())
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_TO_PRESERVE))?;
        let placement = pdf_model::appearance::placement(document, &annotation, &stream)
            .ok_or(Because::NotBuiltYet(NO_PLACEMENT))?;
        let page = tree
            .get(removed.page)
            .ok_or(Because::NotBuiltYet(NO_PAGE_FOR_THE_MARKS))?;
        if page.substituted_media_box.is_some() {
            return Err(Because::NotBuiltYet(NO_PAGE_FOR_THE_MARKS));
        }
        out.push((
            removed,
            preserve::Marks {
                appearance,
                placement: [
                    placement.a,
                    placement.b,
                    placement.c,
                    placement.d,
                    placement.e,
                    placement.f,
                ],
                media_box: page.media_box,
                crop_box: page.crop_box,
                rotate: i64::from(page.rotate),
            },
        ));
    }
    Ok(out)
}

/// The `parameters` field of the removal's recorded action: what went, by name.
///
/// Empty where nothing was removed, which is what keeps a document needing no removal from
/// carrying an entry saying so.
pub(super) fn names_removed(removed: &[MisusedProperty]) -> String {
    if removed.is_empty() {
        return String::new();
    }
    let named: Vec<&str> = removed
        .iter()
        .take(MOST_NAMED)
        .map(|property| property.spelled.as_str())
        .collect();
    let rest = removed.len().saturating_sub(named.len());
    let tail = if rest == 0 {
        String::new()
    } else {
        format!(", and {rest} more")
    };
    format!(
        "these properties were removed because the predefined schema each names does not define \
         the value it held, which ISO 19005-2 clause 6.6.2.3.1 requires: {}{tail}. The conversion \
         report names each one, its namespace and the value that was there",
        named.join(", ")
    )
}

/// The parameters a discarded container's recorded action states, naming what went with it.
pub(super) fn names_discarded(removed: &[MisusedProperty]) -> String {
    if removed.is_empty() {
        return String::new();
    }
    let named: Vec<&str> = removed
        .iter()
        .take(MOST_NAMED)
        .map(|property| property.spelled.as_str())
        .collect();
    let rest = removed.len().saturating_sub(named.len());
    let tail = if rest == 0 {
        String::new()
    } else {
        format!(", and {rest} more")
    };
    format!(
        "an extension schema container that left out a field ISO 19005-2 clause 6.6.2.3.3 \
         requires, and that the file states nowhere, was discarded with every property it \
         described: {}{tail}. The conversion report names each one, its namespace and the value \
         that was there",
        named.join(", ")
    )
}
