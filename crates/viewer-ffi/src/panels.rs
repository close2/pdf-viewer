//! §12.3.3's outline, flattened so that a C caller can hold it.
//!
//! **A tree is the one shape a C ABI cannot hand over as itself**, and this is where that is
//! paid. `viewer-gtk` puts `viewer_host::panel`'s rows into a `GtkTreeListModel` and `viewer-qt`
//! into a `QAbstractItemModel`; a C caller has no model, so what crosses is the depth-first
//! sequence with a **depth** on each row — which is exactly what `viewer-qt` builds internally,
//! because `QAbstractItemModel` must answer for any node at any moment (ADR 0246). The second
//! host having already needed the flattening is why this is a finding rather than a compromise.
//!
//! The rows come from `viewer_host::panel`, unchanged: a native host on this boundary is mostly
//! not toolkit code (ADR 0246 decision 3), and a C host is a native host.

//! **Three answers, two handles, and the split is a fact about the ABI rather than about the
//! panels.** [`Outline`] has three accessors of its own from the first ABI (ADR 0247), and a C
//! entry point cannot change shape; §8.11.4.3's layers and §7.11.4's files want a fourth thing the
//! outline never needed — *what acting on the row does*, which for those two is a switch and an
//! extraction rather than an activation. So they cross as [`Panel`], with the row's action named,
//! and the outline keeps the functions a caller already compiled against. Both are
//! `viewer_host::PanelRow` underneath, flattened the same way.

use viewer_host::{PanelRow, RowAction, attachment_rows, external_rows, layer_rows, outline_rows};

use crate::kinds::RowKind;
use crate::status::Status;

/// §12.3.3's outline, depth first, with a depth on every row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outline {
    /// One entry per item, parent before children.
    rows: Vec<Row>,
}

/// One outline item.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    /// Table 151's `/Title`: "the text that shall be displayed on the screen for this item".
    title: String,
    /// How far in it is, zero at the top level.
    depth: u32,
    /// Whether the sign of `/Count` asks for it to start open.
    expanded: bool,
    /// The object `quorra_activate` names, and its generation.
    ///
    /// Both, because §7.3.10 makes an indirect reference two numbers and a viewer that dropped
    /// the second would name a different object in a file that reused a number.
    object: (u32, u16),
    /// Table 151's `/C` and Table 152's flags.
    style: OutlineStyle,
    /// The languages the title is in, as runs: [`PanelRow::languages`].
    languages: Vec<(usize, String)>,
}

/// How an outline row's text is drawn: Table 151's `/C` and Table 152's two flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlineStyle {
    /// `/C` as `0xRRGGBB`, eight bits a component (`viewer_host::ItemStyle::ink`); `0` is the
    /// table's default black.
    pub rgb: u32,
    /// Table 152's bit 1, "display the item in italic".
    pub italic: bool,
    /// Table 152's bit 2, "display the item in bold".
    pub bold: bool,
}

impl Outline {
    /// Flattens what [`viewer_core::Answer::Outline`] answered with.
    #[must_use]
    pub fn of(outline: &pdf_model::outline::Outline) -> Self {
        let mut rows = Vec::new();
        push_rows(&outline_rows(outline), 0, &mut rows);
        Self { rows }
    }

    /// How many rows there are, counting every level.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// One row's title.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row.
    pub fn title(&self, row: usize) -> Result<&str, Status> {
        self.rows
            .get(row)
            .map(|row| row.title.as_str())
            .ok_or(Status::OutOfRange)
    }

    /// The BCP 47 tag of the language one row's title opens in — ISO 32000-2 §14.9.2.1 makes the
    /// catalog's `/Lang` the language of "outline entries", and a §7.9.2.2.2 escape at the
    /// title's start overrides it — or the empty string where neither states one.
    /// [`Self::language_run`] reads an escape later in the title.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row.
    pub fn language(&self, row: usize) -> Result<&str, Status> {
        self.rows
            .get(row)
            .map(|row| opening_language(&row.languages))
            .ok_or(Status::OutOfRange)
    }

    /// One of the runs of language a row's title is in: the byte of the title it starts at, and
    /// its BCP 47 tag — the catalog's `/Lang` (§14.9.2.1) and each §7.9.2.2.2 escape inside the
    /// title from where it stands (`pdf_model::structure::language_runs`).
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row, or the row has fewer runs.
    pub fn language_run(&self, row: usize, run: usize) -> Result<(usize, &str), Status> {
        language_run(
            &self.rows.get(row).ok_or(Status::OutOfRange)?.languages,
            run,
        )
    }

    /// How far in the row is, zero at the top level.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row.
    pub fn depth(&self, row: usize) -> Result<u32, Status> {
        self.rows
            .get(row)
            .map(|row| row.depth)
            .ok_or(Status::OutOfRange)
    }

    /// Whether the document asked for the row to start open — the sign of §12.3.3's `/Count`.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row.
    pub fn expanded(&self, row: usize) -> Result<bool, Status> {
        self.rows
            .get(row)
            .map(|row| row.expanded)
            .ok_or(Status::OutOfRange)
    }

    /// The colour and style the row's text is drawn in.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row.
    pub fn style(&self, row: usize) -> Result<OutlineStyle, Status> {
        self.rows
            .get(row)
            .map(|row| row.style)
            .ok_or(Status::OutOfRange)
    }

    /// The object number and generation `quorra_activate` takes for the row.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row.
    pub fn object(&self, row: usize) -> Result<(u32, u16), Status> {
        self.rows
            .get(row)
            .map(|row| row.object)
            .ok_or(Status::OutOfRange)
    }
}

/// §8.11.4.3's layers or §7.11.4's embedded files, depth first, with what each row *does*.
///
/// The outline's three accessors answer a title, a depth and an object, which is all §12.3.3's
/// items ever have. These two do not fit that: a layer's row carries Table 99's `/Locked` and
/// whether the group is on, and an attachment's carries a name rather than an object. So the row
/// says which of `viewer_host::RowAction`'s four it is, and the caller reads the payload the action
/// names — an object for the first two, a string for the third, nothing for a heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    /// One entry per row, parent before children.
    rows: Vec<PanelEntry>,
}

/// One row of a panel.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PanelEntry {
    /// What the row says.
    label: String,
    /// A second line, where the answer carries something worth showing beside the label.
    detail: Option<String>,
    /// How far in it is, zero at the top level.
    depth: u32,
    /// Whether the document asked for it to start open.
    expanded: bool,
    /// Which of the four actions it is.
    kind: RowKind,
    /// The object, for [`RowKind::Activate`] and [`RowKind::Toggle`]; `(0, 0)` otherwise, which
    /// §7.5.4 reserves for the head of the free list and is therefore never a document's object.
    object: (u32, u16),
    /// Whether the group is on, for [`RowKind::Toggle`].
    on: bool,
    /// Table 99's `/Locked`, for [`RowKind::Toggle`]: "\[t]he state of a locked group cannot be
    /// changed through the user interface of an interactive PDF processor."
    locked: bool,
    /// The `/EmbeddedFiles` key `quorra_extract` takes, for [`RowKind::Extract`].
    name: String,
    /// The languages the label is in, as runs: [`PanelRow::languages`], which a layer panel's
    /// rows fill (§14.9.2.1's "names for optional content groups").
    languages: Vec<(usize, String)>,
}

impl Panel {
    /// §8.11.4.3's `/Order`, flattened.
    #[must_use]
    pub fn of_layers(layers: &[viewer_core::Layer]) -> Self {
        Self::of_rows(&layer_rows(layers))
    }

    /// §7.11.4's embedded files, flattened. Flat already, because a name tree is a mapping.
    #[must_use]
    pub fn of_attachments(attachments: &[pdf_model::attachment::Attachment]) -> Self {
        Self::of_rows(&attachment_rows(attachments))
    }

    /// §7.11.4's embedded files, then §14.13.2's associated files outside the document, each an
    /// [`RowKind::Inert`] row that names the file and its relationship and offers no save
    /// (`viewer_host::external_rows`, ADR 2799). No sentence for an empty list: an empty panel is
    /// what [`Self::of_attachments`] has always handed a C caller.
    #[must_use]
    pub fn of_files(
        attachments: &[pdf_model::attachment::Attachment],
        external: &[pdf_model::attachment::ExternalAssociatedFile],
    ) -> Self {
        let mut rows = attachment_rows(attachments);
        rows.extend(external_rows(external));
        Self::of_rows(&rows)
    }

    /// Flattens rows `viewer_host::panel` built.
    ///
    /// **Public, and that is the finding rather than a convenience** (ADR 0576). §12.4.3's article
    /// threads and §14.3.3's properties need no shape of their own: `viewer_host::article_rows` and
    /// `viewer_host::property_rows` already produce exactly this, because the two native hosts
    /// wanted them as rows and a C caller is a native host (ADR 0246 decision 3). Two queries cost
    /// one `pub` between them.
    #[must_use]
    pub fn of_rows(rows: &[PanelRow]) -> Self {
        let mut out = Vec::new();
        push_panel(rows, 0, &mut out);
        Self { rows: out }
    }

    /// How many rows there are, counting every level.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// One row's label, and the second line beside it where the answer carries one.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row.
    pub fn text(&self, row: usize, detail: bool) -> Result<&str, Status> {
        let entry = self.rows.get(row).ok_or(Status::OutOfRange)?;
        Ok(if detail {
            entry.detail.as_deref().unwrap_or_default()
        } else {
            entry.label.as_str()
        })
    }

    /// How far in the row is, and whether it starts open.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row.
    pub fn depth(&self, row: usize) -> Result<(u32, bool), Status> {
        self.rows
            .get(row)
            .map(|row| (row.depth, row.expanded))
            .ok_or(Status::OutOfRange)
    }

    /// Which of the four actions the row is, and everything the action carries.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row.
    pub fn action(&self, row: usize) -> Result<(RowKind, (u32, u16), bool, bool), Status> {
        self.rows
            .get(row)
            .map(|row| (row.kind, row.object, row.on, row.locked))
            .ok_or(Status::OutOfRange)
    }

    /// The `/EmbeddedFiles` key for a [`RowKind::Extract`] row, and `""` for any other.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row.
    pub fn name(&self, row: usize) -> Result<&str, Status> {
        self.rows
            .get(row)
            .map(|row| row.name.as_str())
            .ok_or(Status::OutOfRange)
    }

    /// One of the runs of language a row's label is in, as [`Outline::language_run`] reads an
    /// outline row's: the byte of the label it starts at, and its BCP 47 tag.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such row, or the row has fewer runs.
    pub fn language_run(&self, row: usize, run: usize) -> Result<(usize, &str), Status> {
        language_run(
            &self.rows.get(row).ok_or(Status::OutOfRange)?.languages,
            run,
        )
    }
}

/// The language a text opens in: the run that starts at its first byte, or `""` where none does.
fn opening_language(languages: &[(usize, String)]) -> &str {
    match languages.first() {
        Some((0, tag)) => tag,
        _ => "",
    }
}

/// One run of [`PanelRow::languages`], or [`Status::OutOfRange`] past the last.
fn language_run(languages: &[(usize, String)], run: usize) -> Result<(usize, &str), Status> {
    languages
        .get(run)
        .map(|(start, tag)| (*start, tag.as_str()))
        .ok_or(Status::OutOfRange)
}

/// Walks the tree depth first, recording each row's depth and what acting on it does.
fn push_panel(rows: &[PanelRow], depth: u32, into: &mut Vec<PanelEntry>) {
    for row in rows {
        let (kind, object, on, locked, name) = match &row.action {
            RowAction::Activate(id) => (
                RowKind::Activate,
                (id.number, id.generation),
                false,
                false,
                String::new(),
            ),
            RowAction::Toggle { group, on, locked } => (
                RowKind::Toggle,
                (group.number, group.generation),
                *on,
                *locked,
                String::new(),
            ),
            RowAction::Extract { name } => (RowKind::Extract, (0, 0), false, false, name.clone()),
            RowAction::Inert => (RowKind::Inert, (0, 0), false, false, String::new()),
        };
        into.push(PanelEntry {
            label: row.label.clone(),
            detail: row.detail.clone(),
            depth,
            expanded: row.expanded,
            kind,
            object,
            on,
            locked,
            name,
            languages: row.languages.clone(),
        });
        push_panel(&row.children, depth.saturating_add(1), into);
    }
}

/// Walks the tree depth first, recording each row's depth.
fn push_rows(rows: &[PanelRow], depth: u32, into: &mut Vec<Row>) {
    for row in rows {
        into.push(Row {
            title: row.label.clone(),
            depth,
            expanded: row.expanded,
            // An outline row's action is always `Activate` (`viewer_host::panel::row_of_item`),
            // and the other three exist for the layer and attachment trees this ABI does not yet
            // carry. Naming object zero for them is not a fallback but a refusal a caller can
            // see: §7.5.4 reserves number zero for the head of the free list, so it is never an
            // object a document states, and `Command::Activate` on it does nothing and says
            // nothing — which is the right answer for a caller that named the wrong thing.
            object: match &row.action {
                RowAction::Activate(id) => (id.number, id.generation),
                RowAction::Toggle { .. } | RowAction::Extract { .. } | RowAction::Inert => (0, 0),
            },
            style: OutlineStyle {
                rgb: row.style.map_or(0, |style| {
                    let [red, green, blue] = style.ink;
                    u32::from_be_bytes([0, red, green, blue])
                }),
                italic: row.style.is_some_and(|style| style.italic),
                bold: row.style.is_some_and(|style| style.bold),
            },
            languages: row.languages.clone(),
        });
        push_rows(&row.children, depth.saturating_add(1), into);
    }
}

#[cfg(test)]
mod tests {
    use pdf_model::outline::{Item, Outline as Items};
    use pdf_syntax::ObjectId;

    use super::{Outline, OutlineStyle};

    /// Table 151's `/C` and Table 152's flags reach `quorra_outline_style`'s row, `/C` as
    /// `0xRRGGBB` from the `DeviceRGB` components rounded to eight bits.
    #[test]
    fn an_outline_rows_colour_and_style_reach_the_abi() {
        let items = Items {
            items: vec![Item {
                id: ObjectId::new(10, 0),
                title: "Chapter".to_owned(),
                title_languages: Vec::new(),
                destination: None,
                open: false,
                italic: true,
                bold: false,
                colour: [1.0, 0.5, 0.0],
                children: Vec::new(),
            }],
            stated_count: None,
            language: None,
        };
        assert_eq!(
            Outline::of(&items).style(0),
            Ok(OutlineStyle {
                rgb: 0x00ff_8000,
                italic: true,
                bold: false,
            })
        );
    }

    /// §14.9.2.1 and §7.9.2.2.2: a layer row's runs of language reach `quorra_panel_language_run`
    /// one by one, and the read past the last answers out of range; an outline row's opening run
    /// is what `quorra_outline_language` answers.
    #[test]
    fn a_rows_runs_of_language_reach_the_abi() {
        use crate::status::Status;
        let panel = super::Panel::of_layers(&[viewer_core::Layer::Group {
            group: ObjectId::new(7, 0),
            name: Some("Plan Walls".to_owned()),
            languages: vec![(0, "de-DE".to_owned()), (5, "en".to_owned())],
            on: true,
            locked: false,
        }]);
        assert_eq!(panel.language_run(0, 0), Ok((0, "de-DE")));
        assert_eq!(panel.language_run(0, 1), Ok((5, "en")));
        assert_eq!(panel.language_run(0, 2), Err(Status::OutOfRange));
        assert_eq!(panel.language_run(1, 0), Err(Status::OutOfRange));

        let items = Items {
            items: vec![Item {
                id: ObjectId::new(10, 0),
                title: "Kapitel Intro".to_owned(),
                title_languages: vec![(8, "en".to_owned())],
                destination: None,
                open: false,
                italic: false,
                bold: false,
                colour: [0.0; 3],
                children: Vec::new(),
            }],
            stated_count: None,
            language: Some("de".to_owned()),
        };
        let outline = Outline::of(&items);
        assert_eq!(outline.language(0), Ok("de"));
        assert_eq!(outline.language_run(0, 1), Ok((8, "en")));
        assert_eq!(outline.language_run(0, 2), Err(Status::OutOfRange));
    }
}
