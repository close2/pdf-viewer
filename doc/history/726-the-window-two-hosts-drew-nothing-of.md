# 726 — The window two hosts drew nothing of

The ninth UI round, on the project owner's standing ask. ADR 0613.

## What it took, and why those three

`tools/state.sh windows` was run first, as the briefing asked, and its reading — ADR 0603's, one line
per unreached variant marked *debt* or *not a debt* — was the work list. Three of the five debts were
taken, in ADR 0509's own order (*what a reader can do and cannot do here*, then *what costs no new
message*, then *what makes the level-hosts decision checkable*):

- **§12.5.6.14's popup windows in both native hosts.** A comment on a page was invisible in two
  windows of three, with corpus witnesses to drive.
- **§12.5.6.5's link cursor in both native hosts.** A reader could not see a link was there until
  after clicking it, in two windows of three.
- **`AccessibilityNode::lines` across the C ABI**, which `doc/todo/30` had priced at "two accessors"
  and nobody had checked.

Left standing, and named with their reasons by the script rather than here: §14.7's tree on the
native hosts' own accessibility interfaces with §9.10.2's readback beside it (`doc/todo/31`'s, and
the largest), and §12.3.5's collection. §12.5.6.6's free text stays refused by name and is
`doc/todo/33`'s.

**No message was added**, the thirteenth time since the six-hundred-and-seventh.

## What is new

- `viewer_host::popup` — the title bar's two texts, the body, the upright box, and the one refusal
  for a window with no area. `viewer-ui` adopted it and lost three private derivations.
- `viewer_host::geometry::bounds` — written identically in both native hosts already, and the popup
  would have been its third and fourth caller.
- `viewer_host::trace::Topic::Pointer` — because the one thing this round could not photograph is a
  cursor.
- `viewer-gtk`: a `GtkFrame` per open window in a layer of its own, and the link cursor on the page
  layer.
- `viewer-qt`: `QtPopup` and a `PopupWindow : QFrame`, plus `QtUpdate::popups` and `QtUpdate::cursor`
  with `over_link` beside it — Rust still never calls a Qt object.
- `viewer-ffi`: `pdfv_structure_lines`, `pdfv_structure_line`, `pdfv_structure_character`.
- Trap 19, in `doc/traps/the-interactive-loop.md`.

## What was measured, and on what

- Driven under `Xvfb`, `pr7352.pdf`, photographed in **all three** hosts: the same red title bar from
  Table 166's `/C`, the same `/T`, the same `2016-05-25 12:40` from one `viewer_host::stamp`, the
  same wrapped `/Contents`.
- `issue14438.pdf`: `6 of 6 §12.5.6.14 popup window(s) placed` in both native hosts.
- The link cursor: the pointer swept down page 5 of `ISO_32000-2_sponsored_EC3.pdf` makes both native
  hosts report crossing into and out of the activation region on the `pointer` topic. **The cursor
  itself was not photographed** — `xwd` does not capture the pointer and there is no compositor — and
  saying so is better than a code check dressed as a measurement.
- `examples/open_annotation_census` over the pdf.js corpus: **7 open popups on 2 documents.**

## Four things that were wrong, and how each was found

- **The GTK popups made the document decide the window's width.** A `GtkFixed` measures its children;
  `issue14438.pdf` states six windows beside its page; `--trace` shows the page area walking 509 →
  1229 device pixels in nine frames. Nothing on the screen looked wrong. Trap 19.
- **The first version of the trace line could not have said so**: it fired on windows *placed*, so a
  document whose windows all had zero area printed the same silence as one with none. It fires on
  what the answer held (trap 11).
- **`tools/state.sh windows`' `Popups` reason said "[s]even of the corpus's documents"** where the
  population is seven windows on two. The row is deleted with the debt, so the durable fix went into
  the instrument: `open_annotation_census` counted such popups into its totals and named no document
  holding one.
- **`doc/todo/02` §5 named a literal build directory**, which in a worktree round is a *neighbour's*.
  Three rebuild-install-run cycles measured another branch's binary and printed nothing for a feature
  that was working — trap 15's own subject, reached through an instruction. §5 derives the directory
  now, and so does `tools/state.sh disk`.

## Gates

The whole §2 sequence, on a quiet machine, after the workers were built (trap 10). Formatting, lint
under `RUSTFLAGS="-D warnings"`, the workspace tests, the doctests, the fuzz targets' `check`, the
corpus, the oracle, the three text lines, both censuses, dates, xmp, jpeg2000, quorra's corpus,
`fixed_documents` and `cargo test -p conformance` — all green. §4's `--bin quotations` and `--bin
pointers` show only their standing false positives. §5's binaries are installed, from this
worktree's own build directory.

`doc/conformance/ledger.toml` §12.5.6.14, §12.5.6.5 and §14.7 all carry what this round did.
