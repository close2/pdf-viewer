//! What a run is handed: the site, the field, the script, the event, the fields the realm is
//! told of, and the budget.

use std::time::Duration;

use pdf_model::view::{DocumentState, FieldState, ScriptEvent, ScriptSite, WindowView};

/// One script to run at one of §12.6.3's sites or the document's open.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// Where the script runs.
    pub site: ScriptSite,
    /// §12.7.4.2's fully qualified name of the event's field, or empty where it has none.
    pub field: String,
    /// The name a document-level script carries in Table 32's tree; empty at every other site.
    pub label: String,
    /// Table 221's `/JS`, as text.
    pub script: String,
    /// What the event hands the script.
    pub event: Event,
    /// Every field whose state changed since the realm last heard; the realm replaces its record
    /// of each by name before the script runs (ADR 1602).
    pub fields: Vec<FieldState>,
    /// The zero-based page the event happens on.
    pub page: u32,
    /// How many pages the document has.
    pub pages: u32,
    /// Whether the view state holds work no save has written: `this.dirty`.
    pub dirty: bool,
    /// The document as a whole, where it changed since the realm last heard; the realm replaces
    /// its record with it (ADR 1626).
    pub document: Option<DocumentState>,
    /// The window's view of the document, which the realm reads as `this.zoom`, `this.zoomType`
    /// and `this.layout` (ADR 1736).
    pub view: WindowView,
    /// The moment `Date` answers, in milliseconds since 1970-01-01T00:00:00Z — the same value for
    /// the whole run, so that a script cannot time the host (RFC 0008 section 4.2).
    pub moment: u64,
    /// The offset `Date` reads local time with, in seconds east of UTC: the host's to state, since
    /// the engine reads no time zone of its own.
    pub utc_offset_seconds: i32,
}

impl Request {
    /// The request a view state's event asks for, at `moment` and `utc_offset_seconds`.
    ///
    /// The one translation from what `pdf_model::view` hands a runner to what crosses to the
    /// engine — a runner in this process and one across a process boundary build the same request
    /// from the same event.
    #[must_use]
    pub fn of(event: &ScriptEvent<'_>, moment: u64, utc_offset_seconds: i32) -> Self {
        Self {
            site: event.site,
            field: event.field.to_owned(),
            label: event.label.to_owned(),
            script: event.script.to_owned(),
            event: Event {
                value: event.value.to_owned(),
                change: event.change.to_owned(),
                selection_start: utf16_offset(event.value, event.selection.0),
                selection_end: utf16_offset(event.value, event.selection.1),
                will_commit: event.will_commit,
                commit_key: event
                    .commit_key
                    .map_or(0, pdf_model::view::CommitKey::number),
                field_full: event.field_full,
                change_ex: event.change_ex.to_owned(),
                source: event.source.to_owned(),
                shift: event.keys.shift,
                modifier: event.keys.modifier,
                // The reference makes `keyDown` a list box's or a combo box's keystroke's alone; a
                // host sets the arrows only with one, so every other site reads it false.
                key_down: event.keys.arrows
                    && event.site == ScriptSite::Field(pdf_model::aform::Trigger::Keystroke),
                rich_value: event.rich_value.to_owned(),
            },
            fields: event.fields.to_vec(),
            page: u32::try_from(event.page).unwrap_or(u32::MAX),
            pages: u32::try_from(event.pages).unwrap_or(u32::MAX),
            dirty: event.dirty,
            document: event.document.cloned(),
            view: event.view,
            moment,
            utc_offset_seconds,
        }
    }
}

/// Adobe's `event` properties a request carries.
///
/// Offsets are in UTF-16 code units, which is what an ECMAScript string index counts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is one of the reference's boolean event properties, carried as it is named"
)]
pub struct Event {
    /// `event.value`: the field's text before the event.
    pub value: String,
    /// `event.change`: what a keystroke inserts, replacing the selection.
    pub change: String,
    /// `event.selStart`.
    pub selection_start: u32,
    /// `event.selEnd`.
    pub selection_end: u32,
    /// `event.willCommit`.
    pub will_commit: bool,
    /// `event.commitKey`: 0 where the event is not a commit's, else 1 to 3 (ADR 1626).
    pub commit_key: u8,
    /// `event.fieldFull`.
    pub field_full: bool,
    /// `event.changeEx`.
    pub change_ex: String,
    /// The name of `event.source`'s field — the field whose change a calculation answers — or
    /// empty.
    pub source: String,
    /// `event.shift` (ADR 1762).
    pub shift: bool,
    /// `event.modifier` (ADR 1762).
    pub modifier: bool,
    /// `event.keyDown`: whether an arrow key made a choice field's selection (ADR 1762).
    pub key_down: bool,
    /// Table 228's `/RV` of the event's field, which `event.richValue` reads, or empty (ADR 1762).
    pub rich_value: String,
}

/// Every ceiling one run is held to (ADR 1590 gives each number its reason).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// Longest a run may take, measured between the engine's execution slices.
    pub wall: Duration,
    /// Most of the engine's own cost units ("clock cycles", Boa's name) a run may spend.
    pub steps: u64,
    /// The engine's loop-iteration limit, per call frame.
    pub loop_iterations: u64,
    /// The engine's recursion limit.
    pub recursion: u32,
    /// The engine's value-stack limit, in values.
    pub stack: u32,
    /// Most elements a built-in may be asked to iterate or allocate for one array.
    pub elements: u64,
    /// Longest string, in UTF-16 code units, a built-in may be asked to build from its arguments.
    pub string_units: u64,
    /// Largest `ArrayBuffer`, in bytes.
    pub buffer_bytes: u64,
    /// Deepest a script's brackets may nest, counted before it is parsed: Boa's parser recurses
    /// once per level and has no depth limit of its own (ADR 1602).
    pub nesting: u32,
    /// Most native stack, in bytes, [`crate::depth::estimate`] may say a script's parse and
    /// compilation would need: half the worker's 8 MiB thread, the rest left to what runs beneath
    /// a string compiled at run time (ADR 1626).
    pub depth: u64,
}

impl Budget {
    /// The budget a field event runs under.
    pub const FIELD_EVENT: Self = Self {
        wall: Duration::from_millis(100),
        steps: 40_000_000,
        loop_iterations: 100_000,
        recursion: 512,
        stack: 10 * 1024,
        elements: 1 << 20,
        string_units: 1 << 24,
        buffer_bytes: 16 << 20,
        nesting: 128,
        depth: 4 << 20,
    };
}

impl Default for Budget {
    fn default() -> Self {
        Self::FIELD_EVENT
    }
}

/// The UTF-16 offset of byte offset `at` in `text`, the byte moved down to a character boundary.
///
/// Tier 0 counts a selection in bytes and an ECMAScript string counts it in UTF-16 code units; a
/// stale or hostile offset past the end or inside a character is clamped, never trusted.
#[must_use]
pub fn utf16_offset(text: &str, at: usize) -> u32 {
    let mut boundary = at.min(text.len());
    while !text.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    let units = text
        .get(..boundary)
        .map_or(0, |head| head.encode_utf16().count());
    u32::try_from(units).unwrap_or(u32::MAX)
}

/// The byte offset of UTF-16 offset `at` in `text`: the start of the character holding that unit,
/// or the end.
#[cfg(feature = "engine")]
pub(crate) fn byte_offset(text: &str, at: u32) -> usize {
    let mut units = 0_u64;
    for (index, character) in text.char_indices() {
        let next = units.saturating_add(u64::try_from(character.len_utf16()).unwrap_or(2));
        if next > u64::from(at) {
            return index;
        }
        units = next;
    }
    text.len()
}
