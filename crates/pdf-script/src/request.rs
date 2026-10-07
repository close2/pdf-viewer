//! What a run is handed: the trigger, the field, the script, the event, and the budget.

use std::time::Duration;

use pdf_model::aform::Trigger;

/// One script to run, at one of Table 199's triggers, on one field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Which trigger fired. The bridge carries `/K` and `/F`; a request for `/V` or `/C` is
    /// answered with a refusal naming the trigger (ADR 1591).
    pub trigger: Trigger,
    /// §12.7.4.2's fully qualified name of the field the event is on.
    pub field: String,
    /// Table 221's `/JS`, as text.
    pub script: String,
    /// What the event hands the script.
    pub event: Event,
    /// The moment `Date` answers, in milliseconds since 1970-01-01T00:00:00Z — the same value for
    /// the whole run, so that a script cannot time the host (RFC 0008 section 4.2).
    pub moment: u64,
    /// The offset `Date` reads local time with, in seconds east of UTC: the host's to state, since
    /// the engine reads no time zone of its own.
    pub utc_offset_seconds: i32,
}

/// Adobe's `event` properties the bridge carries, as Table 199's `/K` and `/F` raise them.
///
/// Offsets are in UTF-16 code units, which is what an ECMAScript string index counts.
#[derive(Debug, Clone, PartialEq, Eq)]
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
