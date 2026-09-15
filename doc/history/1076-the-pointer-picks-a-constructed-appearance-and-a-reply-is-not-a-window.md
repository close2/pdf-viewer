# 1076 — The pointer picks a constructed appearance, and a reply is not a window

§12.5's last executable rows. Two residues, both about something a clause conditions on a pointer and then leaves unsaid;
a third confirmed as the printer's and left alone. ADR 1090.

**§12.5.6.19 — Table 192's `/RI`, `/IX`, `/RC` and `/AC` are drawn.** The reason this row gave for refusing them — "a
constructed appearance is one stream where §12.5.5 gives a stored one three" — named a capability rather than a clause, and
`view::Appearance` has carried the pointer state since session 132. The table conditions its six icon and caption entries on
the same three conditions §12.5.5 states for `/N`, `/R` and `/D`, so `appearance::icon_entries` and `caption_entries` select
among them by that state; `widget` takes the whole `AnnotationView` and `field_text` an `Appearance`. Table 192's scoping is
kept: `/RC` and `/AC` are "push-button fields only" while `/CA` "may be used with any type of button field". **Where the
state's entry is absent the normal one is drawn** — a decision: Table 192 states no fallback, §12.5.5 states one for the
same three appearances, and reading it literally would erase a button's picture the moment a pointer crossed it. Three tests,
each watched fail with its rule removed: `annotations.rs::table_192s_three_icons_are_chosen_by_what_the_pointer_is_doing`
(three coloured form `XObject`s, one pixel per state, `/H /N` so the default `I` does not invert them), and in
`variable_text.rs` `::table_192s_three_captions_…` (one, ten and twenty letters, as an extent) and
`::a_check_box_keeps_its_own_caption_under_every_pointer_state`. Row stays `partial` on `/TP`'s four side codes, Q63's shape.

**§12.5.6.2 — the threading `shall` is implemented.** Table 172's `/RT` `R`: replies are not displayed "individually but
together in the form of threaded comments". `popup::thread_window` climbs `/IRT` while the annotation is a reply and answers
the window of the **highest** ancestor that states a `/Popup`; `popup::popups` folds the rest in as `popup::Comment`s carrying
depth, author and text, and `popup::popup_of` answers the same window, so §12.5.1's activation of a reply still reaches
something. A reply whose chain opens no window keeps its own, and no window is ever its own host — which is what a looping
`/IRT` produces. Four tests in `popup.rs`, three calibrated by planting the rule's negation. Row stays `partial` on `/RC`'s
XFA formatting. **One expired claim corrected**: the note said the accessibility half was owed, and §14.9.3 has read an
annotation's `/Contents` since session 66.

**§12.5.6.22 — confirmed and left.** Its residue is exactly the two bullets after the EXAMPLE, page tiling and n-up, each opening "shall be printed" and conditioned on a selection this program never makes.

**Census** (`push_button_census`, extended with §12.5.5's sub-dictionary; 963 of 974 documents open, 833 widgets): `/AP /N`
515, **`/AP /R` 0**, `/AP /D` 130, **0** stating `/R` or `/D` with no `/N`; `/RC` 0, `/AC` 0, `/RI` 0, `/IX` 0; `/TP` 2, both
with their own `/AP /N`. Nothing in the corpus reaches the new construction, so `raster_golden` moved 0 — predicted before it
ran. `annotation_group_census` gained the thread-head column: over ISO 32000-2's own PDF, 1752 replies, 1752 naming a
`/Popup`, **1401** whose head names one, chain depth 4. Those 351 are why the highest window hosts, not the head.

**Boundary** (`doc/ui-boundary.md`): `PopupWindow::replies`, one field and no message — no host reads `/IRT`, and the fold
changes which windows `Query::Popups` answers at all. Carried through `viewer-confined`'s wire, `viewer-ui`'s `draw_thread`,
`viewer-gtk`'s box per reply, `viewer-qt` via `viewer_host::popup::thread`, and three C entry points (`quorra_popup_reply_count`, `_object`, `_text`) since a C caller cannot fail to compile. `QUORRA_EVENT_KIND_COUNT` and `QUORRA_ABI_VERSION` unmoved; the two hand-written counts moved 179 → 182, 165 → 168.

**Gates**, all exit 0. Tier 1: `cargo fmt --all --check`; `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`; `cargo nextest run --workspace` — 4851 passed, 37 skipped; `cargo test --workspace --doc`; both `fuzz/` lines; `cargo test -p conformance` — 251 passed. Tier 2: `raster_golden` — held 974, moved 0, unheld 0, left 0.
