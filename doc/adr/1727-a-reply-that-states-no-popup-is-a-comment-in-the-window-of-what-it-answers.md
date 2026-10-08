# 1727 — A reply that states no popup is a comment in the window of what it answers

Session 1445. Status: **accepted** and **built** in `pdf_model::popup` and so in every window that
draws a thread. Builds on ADRs 1090 (a reply's window is folded into its thread's), 1723 (a text
note with no popup opens its own window) and 1720 (a window's state is the later writer's).
Context: ISO 32000-2 §12.5.6.2, Table 172 (`/IRT`, `/RT`, `/Popup`), Table 171.
Code: `crates/pdf-model/src/popup.rs` (`popupless_reply`, `states_its_own_window`; `popups` folds
both kinds of reply; `thread_window` climbs to a note's own window); `crates/pdf-model/src/submission.rs`
(`is_markup`, made visible to the crate); `crates/viewer-gtk/src/host.rs` (a window's column in a
scrolled window). Tests: `viewer-core`'s
`a_reply_that_states_no_popup_is_a_comment_in_the_window_of_what_it_answers`;
`tools/drive-windows.sh` step 65 in the three windows.

## 1. The premise, and what the clause asks

Table 172's `/RT` value `R`: "The annotation is considered a reply to the annotation specified by
IRT . Interactive PDF processors shall not display replies to an annotation individually but
together in the form of threaded comments." `/Popup` is optional in the same table, and `popups`
read a reply only through a popup it stated, so a reply without one was shown nowhere — the
clause's `shall` is about how replies are shown, not a licence to show some of them.

## 2. The decisions

- **Every markup annotation that replies and states no `/Popup` is a comment**, whatever its
  subtype: `/IRT` and `/RT` are Table 172's, the entries of every markup annotation (Table 171's
  `Markup` column), so a highlight answering a note is as much a reply as a text note is. Its text is
  read as a parentless popup's, with a person's retyping in its place; only a text annotation's
  `/Open` counts, for `opens_with_the_page`'s reason.
- **Its window is the highest ancestor that states one**, which now includes a text note's own
  window (ADR 1723): the climb that hangs a thread on a popup hangs it on the note at the head of
  the chain where that note states none. §12.5.1's activation of the reply follows the same climb.
- **A reply with nothing to be shown together with is shown nowhere**, rather than in a window of
  its own: displaying it individually is what the clause forbids, and a reply that does state a
  popup already keeps its own window only because that window is the file's.
- **A person's or a script's write to the thread's window stands over a folded reply's `/Open`**,
  since each `/Open` is the file's opinion of the first frame (ADR 1720).

## 3. What building it found

`quorra-gtk` placed a window's text in an overlay child, which GTK allocates the height it prefers
at its narrowest width (trap 132): a thread was laid out with every word on a line of its own and
the replies fell below the window's edge, clipped. The column now sits in a scrolled window that
scrolls nothing, prefers no height, and is handed the window's width.
