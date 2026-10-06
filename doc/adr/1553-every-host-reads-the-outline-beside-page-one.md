# 1553 — Every host reads the outline beside page one

Status: accepted. Session 1359. Completes ADR 1543 section 3's second bullet; supersedes nothing.
Context: `CLAUDE.md` principle 2 ("[a]nything not needed to show page one is deferred");
ISO 32000-2 §12.3.3 (the outline), §7.7.3 (the page tree); `viewer-core` rule 4; ADRs 0182, 0247,
1531, 1539, 1543; trap 104.
Code: `crates/viewer-gtk/src/host.rs` and `crates/viewer-qt/src/host.rs` (`prepare`,
`take_the_preparation`, `preparing`, `READING_THE_OUTLINE`); `crates/viewer-confined/src/worker.rs`
(`Preparing`); `crates/viewer-ui/src/bin/quorra-confined.rs` (the caption's section);
`crates/viewer-ffi/src/{abi,session,events}.rs`, `include/quorra.h`, `c/open_a_page.c`;
`tools/drive-windows.sh` (`01-section`).
Tests: `viewer-ffi`'s `a_preparation_read_on_another_thread_gives_page_one_its_section` and the C
program's `prepared:` line; `viewer-confined`'s
`the_worker_reads_the_outline_beside_page_one_and_its_next_answer_names_the_section`.

## Decision

`Viewer::preparation` → `Preparation::run` → `Viewer::prepared` is taken by every host, each in its
own shape, so that no first use of the outline or the placed page tree is on a thread that draws.

- **GTK and Qt** ask at the join, before the open's events are answered, and run it on a thread. GTK
  looks for the answer on its main loop at `viewer_host::drawing::POLL`; Qt folds the look into the
  drawing timer `drawing_wait` already arms. While it is out, the contents panel says that it is
  reading the outline instead of asking `Query::Outline` on the toolkit's thread, and is built
  again when the answer lands. A first page turn before then still reads on first use (ADR 1543).
- **The confined worker is the host there**, so it hands the core the thread: started after an
  open's anticipation, inside the confinement, holding a handle to the immutable file. Nothing
  crosses unasked, so what `prepared` raises is owed to the next events frame; a `Query::Outline`
  arriving first waits for the thread rather than reading a second time, since that thread started
  first. `quorra-confined`'s caption now shows `Event::PageChanged`'s section, which it dropped.
- **The C ABI** gains `quorra_preparation_take`, `quorra_preparation_run` (any thread; consumes),
  `quorra_prepared_hand_back` (consumes; returns the events), two `_free`s, and
  `quorra_event_page_section`, without which a C caller could not see what the preparation is for.
  The names are not `quorra_preparation`/`quorra_prepared`: C has one namespace for a typedef and a
  function, and the compiler said so. Six symbols, no struct by value, no event kind:
  `QUORRA_ABI_VERSION` and `QUORRA_EVENT_KIND_COUNT` do not move (ADR 0247 section 4).

## Measured

The drive reads `01-section`, `page 1 of 3 — Chapter one` in the title, in all four windows. The
cost moved is ADR 1543's: on ISO 32000-2, 100.6 M instructions of outline and page tree that a
first page turn or a panel would otherwise have read on the thread that draws.
