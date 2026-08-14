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
//! panels.** [`Outline`] shipped in the four-hundred-and-eleventh session with three accessors of
//! its own, and a C entry point cannot change shape; §8.11.4.3's layers and §7.11.4's files arrived
//! later and want a fourth thing the outline never needed — *what acting on the row does*, which
//! for those two is a switch and an extraction rather than an activation. So they cross as
//! [`Panel`], with the row's action named, and the outline keeps the functions a caller already
//! compiled against. Both are `viewer_host::PanelRow` underneath, flattened the same way.

use viewer_host::{PanelRow, RowAction, attachment_rows, layer_rows, outline_rows};

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
    /// The object `pdfv_activate` names, and its generation.
    ///
    /// Both, because §7.3.10 makes an indirect reference two numbers and a viewer that dropped
    /// the second would name a different object in a file that reused a number.
    object: (u32, u16),
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

    /// The object number and generation `pdfv_activate` takes for the row.
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
    /// Table 99's `/Locked`, for [`RowKind::Toggle`]: "[t]he state of a locked group cannot be
    /// changed through the user interface of an interactive PDF processor."
    locked: bool,
    /// The `/EmbeddedFiles` key `pdfv_extract` takes, for [`RowKind::Extract`].
    name: String,
}

impl Panel {
    /// §8.11.4.3's `/Order`, flattened.
    #[must_use]
    pub fn of_layers(layers: &[viewer_core::Layer]) -> Self {
        Self::of(&layer_rows(layers))
    }

    /// §7.11.4's embedded files, flattened. Flat already, because a name tree is a mapping.
    #[must_use]
    pub fn of_attachments(attachments: &[pdf_model::attachment::Attachment]) -> Self {
        Self::of(&attachment_rows(attachments))
    }

    /// Flattens rows `viewer_host::panel` built.
    fn of(rows: &[PanelRow]) -> Self {
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
        });
        push_rows(&row.children, depth.saturating_add(1), into);
    }
}
