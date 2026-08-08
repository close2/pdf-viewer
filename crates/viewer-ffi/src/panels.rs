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

use viewer_host::{PanelRow, RowAction, outline_rows};

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
