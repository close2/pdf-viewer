//! ISO 19005's conversion chosen after launch, as the card this window draws for itself (ADR 1966).
//!
//! **The restrictions card's shape**: a keyboard-driven card over the dimmed page, because this
//! window has no window manager behind its chrome. What it offers is
//! [`viewer_host::archiving::Choosing`] unchanged — the six targets and the shipped profiles, each
//! with the one sentence its file states — which is the list the GTK and Qt dialogues offer too.

use pdf_render::DisplayList;

use super::{
    CARD_INK, CARD_PADDING, CARD_PAPER, Chrome, DIMMED, RESTRICTION_CURSOR, Style, TEXT_SIZE,
    dimmed, rectangle,
};

/// How wide the card is, in logical pixels: room for the longest profile sentence on one line.
const ARCHIVE_WIDTH: f32 = 640.0;

/// The lines above the targets: the title and the sentence about what a profile is.
const HEADING_LINES: usize = 2;

/// The keys, said on the card's last line.
const KEYS: &str = "← → target   ·   ↑ ↓ profile   ·   Enter to convert   ·   Escape to close";

/// The card, and the choice being made on it.
#[derive(Debug, Clone, Default)]
pub struct ArchiveCard {
    /// The choice, while the card is over the page.
    choosing: Option<viewer_host::archiving::Choosing>,
}

/// What a press on the card did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchivePress {
    /// The press chose a target or a profile, and the card is to be drawn again.
    Chose,
    /// The press was on the card and chose nothing.
    Nothing,
}

impl ArchiveCard {
    /// Whether the card is over the page.
    #[must_use]
    pub const fn shown(&self) -> bool {
        self.choosing.is_some()
    }

    /// Puts the card up with a choice as it begins.
    pub fn show(&mut self, choosing: viewer_host::archiving::Choosing) {
        self.choosing = Some(choosing);
    }

    /// Takes the card down, with whatever was chosen on it: the person's answer, on Enter.
    pub fn take(&mut self) -> Option<viewer_host::archiving::Choosing> {
        self.choosing.take()
    }

    /// Moves the target by `by`; answers whether anything changed.
    pub fn move_target(&mut self, by: isize) -> bool {
        self.choosing
            .as_mut()
            .is_some_and(|choosing| choosing.move_target(by))
    }

    /// Moves the profile by `by`; answers whether anything changed.
    pub fn move_profile(&mut self, by: isize) -> bool {
        self.choosing
            .as_mut()
            .is_some_and(|choosing| choosing.move_profile(by))
    }

    /// A press at `at`, in the window's device pixels: a target's cell or a profile's row becomes
    /// the one chosen. Choosing to convert stays Enter's, so a stray click converts nothing.
    pub fn press(&mut self, at: (f32, f32), width: u32, height: u32, scale: f32) -> ArchivePress {
        let Some(choosing) = self.choosing.as_mut() else {
            return ArchivePress::Nothing;
        };
        let layout = Layout::of(choosing.profiles().len(), width, height, scale);
        let (x, y) = at;
        let inner = x - (layout.left + layout.pad);
        if inner < 0.0 || inner >= layout.inner() {
            return ArchivePress::Nothing;
        }
        let from_top = y - (layout.top + layout.pad);
        if from_top < 0.0 {
            return ArchivePress::Nothing;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a non-negative distance in pixels divided by a line height: a small count"
        )]
        let line = (from_top / layout.line) as usize;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "a non-negative offset across the card divided by a sixth of it: 0 to 5"
        )]
        let cell = (inner / (layout.inner() / choosing.targets().len() as f32)) as usize;
        let chose = if line == Layout::TARGETS {
            choosing.set_target(cell)
        } else if let Some(row) = line.checked_sub(Layout::PROFILES) {
            choosing.set_profile(row / 2)
        } else {
            false
        };
        if chose {
            ArchivePress::Chose
        } else {
            ArchivePress::Nothing
        }
    }

    /// The card, in device pixels of the window.
    #[must_use]
    pub fn draw(
        &self,
        chrome: &Chrome,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Option<DisplayList> {
        let choosing = self.choosing.as_ref()?;
        let layout = Layout::of(choosing.profiles().len(), width, height, scale);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window's extent in pixels, which is thousands"
        )]
        let mut list = dimmed(width as f32, height as f32);
        rectangle(
            &mut list,
            (layout.left, layout.top, layout.wide, layout.tall),
            CARD_PAPER,
        );
        let left = layout.left + layout.pad;
        chrome.text(
            &mut list,
            viewer_host::archiving::CHOOSING_TITLE,
            (left, layout.baseline(0)),
            layout.size,
            BOLD,
            CARD_INK,
        );
        chrome.text(
            &mut list,
            viewer_host::archiving::CHOOSING,
            (left, layout.baseline(1)),
            layout.size,
            Style::default(),
            DIMMED,
        );
        draw_targets(&mut list, chrome, choosing, &layout);
        draw_profiles(&mut list, chrome, choosing, &layout);
        chrome.text(
            &mut list,
            KEYS,
            (
                left,
                layout.baseline(Layout::keys(choosing.profiles().len())),
            ),
            layout.size,
            Style::default(),
            DIMMED,
        );
        Some(list)
    }
}

/// Bold, for a title, a profile's name and the chosen target.
const BOLD: Style = Style {
    bold: true,
    italic: false,
};

/// The targets' line: six cells across the card, the chosen one marked.
fn draw_targets(
    list: &mut DisplayList,
    chrome: &Chrome,
    choosing: &viewer_host::archiving::Choosing,
    layout: &Layout,
) {
    #[expect(clippy::cast_precision_loss, reason = "six targets")]
    let cell = layout.inner() / choosing.targets().len() as f32;
    let baseline = layout.baseline(Layout::TARGETS);
    let mut x = layout.left + layout.pad;
    for (index, target) in choosing.targets().iter().enumerate() {
        let on = index == choosing.target_index();
        if on {
            rectangle(
                list,
                (x, baseline - layout.size, cell, layout.line),
                RESTRICTION_CURSOR,
            );
        }
        chrome.text(
            list,
            &target.to_string(),
            (x + layout.pad / 3.0, baseline),
            layout.size,
            if on { BOLD } else { Style::default() },
            CARD_INK,
        );
        x += cell;
    }
}

/// The profiles' lines: each name over its sentence, the chosen one marked.
fn draw_profiles(
    list: &mut DisplayList,
    chrome: &Chrome,
    choosing: &viewer_host::archiving::Choosing,
    layout: &Layout,
) {
    let left = layout.left + layout.pad;
    for (index, offered) in choosing.profiles().iter().enumerate() {
        let first = Layout::profile(index);
        if index == choosing.profile_index() {
            rectangle(
                list,
                (
                    layout.left,
                    layout.baseline(first) - layout.size,
                    layout.wide,
                    layout.line * 2.0,
                ),
                RESTRICTION_CURSOR,
            );
        }
        chrome.text(
            list,
            offered.name,
            (left, layout.baseline(first)),
            layout.size,
            BOLD,
            CARD_INK,
        );
        chrome.text(
            list,
            &offered.sentence,
            (left, layout.baseline(first.saturating_add(1))),
            layout.size,
            Style::default(),
            DIMMED,
        );
    }
}

/// Where the card and its lines are, for a window this size: [`ArchiveCard::draw`] and
/// [`ArchiveCard::press`] read the one answer, so a row is pressed where it is drawn.
#[derive(Debug, Clone, Copy)]
struct Layout {
    /// The card's left edge.
    left: f32,
    /// The card's top edge.
    top: f32,
    /// The card's width.
    wide: f32,
    /// The card's height.
    tall: f32,
    /// One line's height.
    line: f32,
    /// The text's size.
    size: f32,
    /// The card's padding.
    pad: f32,
}

impl Layout {
    /// The line the targets are on: under the heading and a blank line.
    const TARGETS: usize = HEADING_LINES + 1;

    /// The first profile's line: under the targets and a blank line. Each profile is two lines,
    /// its name and its sentence.
    const PROFILES: usize = Self::TARGETS + 2;

    /// The first line of the profile at `index`.
    const fn profile(index: usize) -> usize {
        Self::PROFILES.saturating_add(index.saturating_mul(2))
    }

    /// The keys' line, under the last profile and a blank line.
    const fn keys(profiles: usize) -> usize {
        Self::profile(profiles).saturating_add(1)
    }

    /// The baseline of the line at `index`.
    fn baseline(&self, index: usize) -> f32 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a line count on one card, which is tens"
        )]
        let index = index as f32;
        self.top + self.pad + self.size + self.line * index
    }

    /// The card for `profiles` profiles, centred in a window this size.
    fn of(profiles: usize, width: u32, height: u32, scale: f32) -> Self {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window's extent in pixels, which is thousands"
        )]
        let (window_wide, window_tall) = (width as f32, height as f32);
        let size = TEXT_SIZE * scale;
        let pad = CARD_PADDING * scale;
        let line = size * 1.5;
        let wide = ARCHIVE_WIDTH * scale;
        // The profiles' lines, a blank line, and the keys.
        let lines = Self::keys(profiles).saturating_add(1);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a line count on one card, which is tens"
        )]
        let tall = pad * 2.0 + line * lines as f32;
        Self {
            left: (window_wide - wide).max(0.0) / 2.0,
            top: (window_tall - tall).max(0.0) / 2.0,
            wide,
            tall,
            line,
            size,
            pad,
        }
    }

    /// The width inside the padding.
    fn inner(&self) -> f32 {
        (self.wide - self.pad * 2.0).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{ArchiveCard, ArchivePress, Layout};

    /// A press on a target's cell and on a profile's sentence chooses them, a press on the
    /// heading chooses nothing, and what is chosen is what Enter takes.
    #[test]
    fn a_press_chooses_where_the_card_draws_and_enter_takes_it() {
        let mut card = ArchiveCard::default();
        card.show(viewer_host::archiving::Choosing::new());
        let (width, height, scale) = (1000, 800, 1.0);
        let profiles = viewer_host::archiving::offered().len();
        let layout = Layout::of(profiles, width, height, scale);
        let line_at = |index: usize| {
            #[expect(clippy::cast_precision_loss, reason = "a small line index")]
            let index = index as f32;
            layout.top + layout.pad + layout.line * (index + 0.5)
        };
        let fourth_cell = layout.left + layout.pad + layout.inner() / 6.0 * 3.5;
        assert_eq!(
            card.press((fourth_cell, line_at(0)), width, height, scale),
            ArchivePress::Nothing,
            "the heading chooses nothing"
        );
        assert_eq!(
            card.press(
                (fourth_cell, line_at(Layout::TARGETS)),
                width,
                height,
                scale
            ),
            ArchivePress::Chose
        );
        let second_sentence = line_at(Layout::PROFILES + 3);
        assert_eq!(
            card.press((layout.left + 40.0, second_sentence), width, height, scale),
            ArchivePress::Chose
        );
        let chosen = card.take().expect("shown").chosen();
        assert_eq!(chosen.target.to_string(), "PDF/A-4");
        assert_eq!(chosen.profile, "as-if-printed");
        assert!(!card.shown());
    }
}
