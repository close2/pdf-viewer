//! Annex O's `search`, as a bar a person types into and a colour over what it found.
//!
//! Two questions with two scopes, which is why the bar is not one loop: `Command::Find` reads the
//! *document* one page per step and this window pumps the steps, while `Query::Find` answers for
//! the page on the screen out of a readback that already exists. The first is what the note says
//! and the second is what the yellow shows.

use viewer_core::{Answer, Command, Find, FindDirection, Query};
use winit::keyboard::{Key, NamedKey};

use crate::app::App;
use crate::trace::Topic;

/// How long a search goes between repaints of the find bar's progress count.
///
/// A choice with a measurement behind it — see [`App::searched`], which is where the numbers are —
/// and it belongs to this host alone: a native host repaints when its toolkit says to. Ten a
/// second is above what a person reads a moving count at and below what a present costs here,
/// so the progress indicator stays a fraction of the search rather than a multiple of it.
pub(crate) const SEARCH_PROGRESS: std::time::Duration = std::time::Duration::from_millis(100);

impl App {
    /// The find bar, where it is open.
    ///
    /// Across the whole window rather than only the page area, which is where both native hosts
    /// put theirs: a search is about the document and not about the page pane.
    pub(crate) fn find_list(&self, width: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        self.find.draw(chrome, width, scale)
    }

    /// Every occurrence of the find bar's string on the page being shown.
    ///
    /// Asked on every frame while the bar is open, because it is a query over a readback that
    /// already exists — `Query::Find` is this page and `Command::Find` is the document, and only
    /// the second interprets anything. Drawn *under* the selection, so that the occurrence a
    /// person is on reads as the one the selection colour is over.
    pub(crate) fn matches_list(
        &self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        if !self.find.shown || self.find.needle.is_empty() {
            return None;
        }
        let Answer::Found(occurrences) = self.viewer.query(Query::Find(&self.find.needle)) else {
            return None;
        };
        let mut quads: Vec<[f32; 8]> = occurrences.into_iter().flatten().collect();
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same
        // one addition `selection_list` makes.
        for quad in &mut quads {
            for x in quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        crate::overlays::highlight_list(&quads, MATCH, width, height)
    }

    /// A step of the search reported: what it found, and how much is left to read.
    ///
    /// The sentence goes into the bar rather than only onto stdout, because a person watching a
    /// thousand-page document being read is the reason a step is one page.
    pub(crate) fn searched(
        &mut self,
        found: Option<viewer_core::Found>,
        remaining: usize,
        wrapped: bool,
    ) {
        self.pages_left = remaining;
        self.find.note = match found {
            Some(found) => format!(
                "page {}{}",
                found.page.saturating_add(1),
                if wrapped { ", wrapped" } else { "" }
            ),
            None if remaining == 0 => "not in this document".to_owned(),
            None => format!("{remaining} page(s) left"),
        };
        if found.is_some() || remaining == 0 {
            println!(
                "note: search for {:?} — {}",
                self.find.needle, self.find.note
            );
            // What the search left behind, which is the legible half of `viewer-core`'s readback
            // budget: a bound nobody can read is a bound nobody can check. Printed at the end of
            // a search rather than per step, because it changes by one entry a step and the flood
            // is what `--trace`'s topics exist to avoid (ADR 0227).
            if let Some(held) = self.viewer.readback_cache(crate::DOCUMENT) {
                self.trace.say(
                    Topic::Search,
                    format_args!(
                        "search: readback cache holds {} page(s), {} of {} bytes,                          {} hit(s), {} miss(es), {} evicted",
                        held.pages,
                        held.bytes,
                        held.budget,
                        held.hits,
                        held.misses,
                        held.evicted
                    ),
                );
            }
        }
        // **Not on every step**, and the interval is measured rather than chosen. A redraw here
        // is a whole window presented, and under `Xvfb` with lavapipe that is about 13 ms — so
        // repainting once per page made a 1023-page sweep **19.25 s** of presenting a bar whose
        // text changes by one digit (ADR 0250). This was once every 16 *steps* until the
        // four-hundred-and-twentieth session gave `viewer-core` a readback cache and a step
        // stopped costing 5.7 ms: a repeated sweep of ISO 32000-2 is 7.27 ms of searching, and
        // 64 presents of a progress count made it **0.51 s** in the window. A step count is a
        // proxy for time that was calibrated against one step cost; the clock is the thing it was
        // a proxy for, and it costs the same on a cold sweep and nothing on a warm one (ADR 0256).
        let due = self
            .searched_at
            .is_none_or(|last| last.elapsed() >= SEARCH_PROGRESS);
        if found.is_some() || remaining == 0 {
            self.searched_at = None;
            self.redraw();
        } else if due {
            self.searched_at = Some(std::time::Instant::now());
            self.redraw();
        }
    }

    /// A key press while the find bar is open. Answers whether the bar took it.
    ///
    /// Every printable key is a character of the string, which is what makes this the first
    /// branch in the key handler rather than the last. Enter is *next* and shift-Enter is
    /// *previous* — one keystroke for each direction, which is what every find bar has —
    /// and Escape closes the bar and forgets the plan.
    pub(crate) fn find_key(&mut self, key: &Key<&str>) -> bool {
        match key {
            Key::Named(NamedKey::Escape) => {
                self.find.toggle();
                self.pages_left = 0;
                self.dispatch(Command::Find(Find::Stop));
            }
            Key::Named(NamedKey::Enter) => {
                if self.find.needle.is_empty() {
                    return true;
                }
                let needle = self.find.needle.clone();
                let direction = if self.shift {
                    FindDirection::Backward
                } else {
                    FindDirection::Forward
                };
                self.dispatch(Command::Find(Find::Start { needle, direction }));
            }
            Key::Named(NamedKey::Backspace) => {
                self.find.backspace();
                self.find.note.clear();
            }
            Key::Named(NamedKey::Space) => {
                self.find.typed(" ");
                self.find.note.clear();
            }
            Key::Character(text) if !text.is_empty() => {
                self.find.typed(text);
                self.find.note.clear();
            }
            // A key with no character and no meaning here — an arrow, a function key. Taken
            // anyway: while the bar has the keyboard, letting one through to turn the page would
            // move the document out from under the search a person is typing.
            _ => {}
        }
        self.redraw();
        true
    }
}

/// The colour every occurrence of a search string is washed in.
///
/// A paler yellow than [`overlays::SELECTION`](crate::overlays)'s blue and multiplied over the
/// page for the same reason: the glyphs underneath have to stay readable. **The colour is a
/// choice** — the standard describes no find bar and says nothing about what a match looks like —
/// and it is chosen to be a different *hue* from the selection rather than a different weight of
/// it, so that "where else the word is" and "which one you are on" cannot be confused at a glance.
/// The two native hosts made the other choice, one hue at two alphas, because a platform hands
/// them a selection colour and no second one; this host has no theme to ask and so may pick both.
const MATCH: pdf_render::Color = pdf_render::Color {
    r: 1.0,
    g: 0.87,
    b: 0.35,
    a: 1.0,
};
