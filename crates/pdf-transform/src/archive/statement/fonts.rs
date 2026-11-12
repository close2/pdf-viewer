//! The statement's entries for the font row: each page this conversion photographed and each face
//! it embedded with a loss, as the report records them (`doc/rfc/0014` section 4,
//! `doc/questions/A717`, `doc/adr/2600`).
//!
//! `fonts/font-programs-embedded` resolves to one `discard` carrying the font bundle, and the
//! report already holds that bundle one record at a time: a [`PhotographedPage`] for each page
//! replaced by a photograph of itself, and a [`SubstitutedFont`] for each face embedded with a
//! loss. The page lists those records and nothing else, so each fact on it is of one of three
//! kinds: computed from the source (a page's number, a font's `/BaseFont` and resource name, an
//! optional content group's `/Name`, how many structure leaves named the page), a constant of this
//! program (the resolution, a requirement's identifier), or this file's fixed line for a loss word.

use std::fmt::Write as _;

use pdf_archive::{Target, table};

use super::super::Loss;
use super::super::config::{STATED_FONTS, Statement};
use super::super::fonts::{PhotographedPage, SubstitutedFont};
use super::{MOST_STATED_OBJECTS, MOST_STATEMENT_BYTES, STATEMENT_TOO_LONG, TOO_MANY_STATED};

/// What the font row's statement lists: the report's own records, with what the row says was
/// tried before the loss.
pub(in crate::archive) struct FontsStated<'a> {
    /// Every page the conversion photographed, in the report's order.
    pub(in crate::archive) pages: &'a [PhotographedPage],
    /// Every face the conversion embedded; those whose embedding lost nothing are not listed.
    pub(in crate::archive) faces: &'a [SubstitutedFont],
    /// What the row says was tried before the loss.
    pub(in crate::archive) asked: &'a Statement,
}

impl FontsStated<'_> {
    /// The faces the page lists: those whose embedding incurred a loss.
    fn faces_lost(&self) -> impl Iterator<Item = &SubstitutedFont> {
        self.faces.iter().filter(|face| !face.losses.is_empty())
    }

    /// How many entries the page gives the row: one per page photographed and one per face lost.
    pub(in crate::archive) fn listed(&self) -> usize {
        self.pages.len().saturating_add(self.faces_lost().count())
    }
}

/// Why a loss the font row's page has no line for is not stated.
const NOT_A_FONT_LOSS: &str = "the statement of what this conversion removed was asked to list a \
     loss the font row's page has no line for, so no statement page is composed";

/// The one line each loss of the font bundle is described by on the page.
///
/// Fixed here rather than taken from [`Loss::describe`], whose sentences speak to the person
/// running a conversion; the page speaks to a reader of the archive, about the page or the font in
/// front of it.
const fn line(loss: Loss) -> Option<&'static str> {
    Some(match loss {
        Loss::TextAsImage => {
            "the page is a photograph of itself as this viewer drew it, and the text of a font no \
             reader can name is blank as this viewer left it"
        }
        Loss::PageAsImage => {
            "the page is a photograph of itself as this viewer drew it, so the text it held can no \
             longer be selected or searched"
        }
        Loss::TaggingOfPhotographedPages => {
            "the structure tree's leaves that named the page were taken out of the tree with the \
             page's content"
        }
        Loss::OptionalContentOfPhotographedPages => {
            "the page was drawn in the configuration this viewer shows, so each optional content \
             group its content named is fixed in the state it was drawn in"
        }
        Loss::MachineFace => {
            "the font is drawn in the shapes of a face this program ships, at the widths the file \
             states, where this viewer drew it from a face of its own machine"
        }
        Loss::SubstituteStretch => {
            "the font is drawn in a face this program ships at that face's own widths, where this \
             viewer narrowed it to the widths the file states"
        }
        Loss::GlyphsFromToUnicode => {
            "the font's codes are drawn with the glyphs a face holds for the characters its \
             ToUnicode map states, the face this program ships or the face of this machine's \
             the viewer drew it from"
        }
        Loss::LayoutFromThisViewer => {
            "the file states no widths for the font, and the widths written are the layout this \
             viewer drew it with"
        }
        _ => return None,
    })
}

/// Writes the font row's entries after the `listed_before` the statement already holds, numbered
/// on from them, or returns the bound the text would pass.
///
/// Each `/BaseFont`, resource name and group name is the file's bytes, written under the
/// statement's `<U+XXXX>` rule ([`super::escaped`]), and each is held to
/// [`MOST_STATEMENT_BYTES`] before it is written.
pub(in crate::archive) fn entries(
    out: &mut String,
    target: Target,
    stated: &FontsStated<'_>,
    listed_before: usize,
) -> Result<(), &'static str> {
    if listed_before.saturating_add(stated.listed()) > MOST_STATED_OBJECTS {
        return Err(TOO_MANY_STATED);
    }
    let requirement = table::requirements().find(|requirement| requirement.id == STATED_FONTS);
    let citation = requirement
        .and_then(|requirement| requirement.clauses.citation(target))
        .unwrap_or_default();
    let asks = requirement.map_or("", |requirement| requirement.asks);
    let mut index = listed_before;
    for page in stated.pages {
        index = index.saturating_add(1);
        heading(out, index, &citation, asks)?;
        // Writing into a `String` cannot fail, so each `write!` result is discarded deliberately.
        let _ = writeln!(
            out,
            "   Page: {}, photographed at {} dpi",
            page.page, page.dots_per_inch
        );
        losses(out, page.losses().into_iter())?;
        named(
            out,
            "Fonts not drawn",
            page.fonts.iter().map(String::as_str),
        )?;
        named(
            out,
            "Content stream requirements answered",
            page.requirements.iter().map(String::as_str),
        )?;
        if page.structure_leaves > 0 {
            let _ = writeln!(
                out,
                "   Structure tree leaves taken out: {}",
                page.structure_leaves
            );
        }
        if let Some(fixed) = &page.optional_content {
            out.push_str("   Optional content groups fixed as drawn:");
            if fixed.groups.is_empty() {
                out.push_str(" none named");
            }
            for (at, group) in fixed.groups.iter().enumerate() {
                within(out, &group.name)?;
                let _ = write!(
                    out,
                    "{} \"{}\" {}",
                    if at == 0 { "" } else { "," },
                    super::escaped(&group.name),
                    group.state_word()
                );
            }
            if fixed.left_out > 0 {
                let _ = write!(out, " and {} more", fixed.left_out);
            }
            out.push('\n');
        }
        tried(out, stated.asked);
    }
    for face in stated.faces_lost() {
        index = index.saturating_add(1);
        heading(out, index, &citation, asks)?;
        within(out, &face.requested)?;
        within(out, &face.resource)?;
        let _ = writeln!(
            out,
            "   Font: {} (resource {})",
            super::escaped(&face.requested),
            super::escaped(&face.resource)
        );
        losses(out, face.losses.iter())?;
        tried(out, stated.asked);
    }
    if out.len() > MOST_STATEMENT_BYTES {
        return Err(STATEMENT_TOO_LONG);
    }
    Ok(())
}

/// An entry's number, requirement and `asks` sentence, once the text so far is within its bound.
fn heading(out: &mut String, index: usize, citation: &str, asks: &str) -> Result<(), &'static str> {
    if out.len() > MOST_STATEMENT_BYTES {
        return Err(STATEMENT_TOO_LONG);
    }
    let _ = write!(
        out,
        "\n{index}. Requirement: {STATED_FONTS} ({citation})\n   Asks: {asks}\n"
    );
    Ok(())
}

/// One `Loss:` line per loss, each its word and this file's line for it.
fn losses(out: &mut String, losses: impl Iterator<Item = Loss>) -> Result<(), &'static str> {
    for loss in losses {
        let line = line(loss).ok_or(NOT_A_FONT_LOSS)?;
        let _ = writeln!(out, "   Loss: {}, {line}", loss.word());
    }
    Ok(())
}

/// A labelled list of the file's names, written only where it names any.
fn named<'a>(
    out: &mut String,
    label: &str,
    names: impl Iterator<Item = &'a str>,
) -> Result<(), &'static str> {
    let mut any = false;
    for name in names {
        within(out, name)?;
        let _ = write!(
            out,
            "{}{}",
            if any {
                ", ".to_owned()
            } else {
                format!("   {label}: ")
            },
            super::escaped(name)
        );
        any = true;
    }
    if any {
        out.push('\n');
    }
    Ok(())
}

/// What the row says was tried before the loss, as the embedded-file entry states it.
fn tried(out: &mut String, asked: &Statement) {
    match &asked.before {
        Some(step) => {
            let _ = writeln!(out, "   Tried before: {step}");
        }
        None => out.push_str("   Tried before: no step\n"),
    }
    for (step, why) in &asked.skipped {
        let _ = writeln!(out, "   Not admitted at this target: {step}, because {why}");
    }
}

/// Whether the text so far and one more of the file's names, whose escaped form is at most eight
/// bytes a byte, still fit the statement's bound.
fn within(out: &str, name: &str) -> Result<(), &'static str> {
    if out.len() > MOST_STATEMENT_BYTES || name.len() > MOST_STATEMENT_BYTES / 8 {
        return Err(STATEMENT_TOO_LONG);
    }
    Ok(())
}
