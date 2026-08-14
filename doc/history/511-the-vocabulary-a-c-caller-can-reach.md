# 511 — The vocabulary a C caller can reach, and the flag that was obeyed backwards

**Finding.** Three items of surface were owed on `doc/todo/30` and one of them was a misreading.
Table 229 bit 26's `RadiosInUnison` was recorded in two ledger rows, one todo file and ADR 0235 as
"crosses and is not obeyed, because turning on every button of a set that shares an on state is a
decision for whatever handles the press". Reading the clause found that sentence wrong in **both**
directions at once. The flag being *set* was already obeyed by construction — `/V` is a name and a
widget is on when its `/AP /N` holds a stream under it, so two widgets sharing a name have always
gone on together, which is §12.7.5.2.4's NOTE, obeyed by code that had never read the bit. What was
*not* obeyed is the flag being clear: §12.7.5.2.3 requires that "at most one radio button in a field
shall be set at a time", and this tree turned them all on. **The requirement is stated in the check
box subclause, attached to `/Opt`**, which is why a round reading §12.7.5.2.4 for a radio button's
flag would have found a NOTE describing the exception and no requirement at all. Read the clause the
entry is defined in, not only the clause the feature is named after.

**The other two items were smaller than the file thought.** The C ABI's remaining sixty-eight symbols
are shims, and adding two thirds of an ABI in one round moved `PDFV_EVENT_KIND_COUNT` **not at all**
— a `Command` is a symbol and only an `Event` is a number, which is the third demonstration in three
rounds of the property ADR 0247's first shape was chosen for. And ADR 0245's scale question needed
no message: `Query::Fields` already gives the rectangles, the toolkit gives the minimums, and the
one piece missing was arithmetic.

**Date.** 2026-08-14.
**ADR.** [0346](../adr/0346-the-vocabulary-a-c-caller-can-reach.md).

## The three, in the file's own order

**The ABI: 43 entry points → 111.** `doc/todo/30`'s list is closed — `Command::Pointer` and
`Command::Select` with the selection, the caret and §12.5.6.6's drag; `Query::Fields` and all four
`Edit`s; `Command::Save` and `Command::Extract` with a **byte** accessor apiece; §8.11.4.3's layers
and §7.11.4's files as a second flattened panel; §12.4.4's clock and its transitions; and
`Command::Restrict`, `Command::Delegate` and `Command::Present`. Four things the shape had to say
that it had not said before, each in ADR 0346:

- `PDFV_EVENT_KIND_COUNT` is 16 before and after, and the round was asked to expect it to move;
- two enumerations are *answered with* rather than pushed — `ControlKind` and `RowKind` — so each
  has its own count and name and neither joins `pdfv_abi_check`, whose signature is the one thing
  every compiled caller already depends on;
- `PDFV_EVENT_SEARCHED` **had been missing since the four-hundred-and-fourteenth moved the count**,
  and `header_and_library_agree.rs` could not see it because it compares the header against a
  hand-written map: a constant absent from both sides agrees with itself;
- sixteen of Tables 227, 229, 231 and 233's booleans cross as one `uint32_t`, because a third
  by-value struct is the only change this ABI cannot make cheaply.

`viewer-host` gained its fourth consumer and wanted no change: a C caller's `PDFV_CONTROL_*` **is**
`viewer_host::ControlKind`, which is ADR 0246's third decision tested rather than repeated.

**Table 229 bit 26.** `Field::an_earlier_button_answers_to` is the fix, in `replacement_state`, so
`Field::is_on` and `appearance::appearance_state` reach it through one function and the description a
host reads cannot disagree with the picture the page draws — ADR 0235's finding was that those two
paths go wrong *differently*. Which button stays on is a **documented choice** (the first in
`/Kids`), because the clause states none and a file that gave two buttons one name cannot say; Table
230's positional `/AP` names are the standard's own instrument for a producer that wants them
distinguishable. The rule binds a value this reader replaced and not the file's own `/AS`, which
§12.7.5.2.3 gives precedence.

**The scale a form host draws at.** `viewer_host::ControlFit` is the piece that did not exist: a
control's minimum does not change with the page's magnification and its `/Rect` does, in proportion,
so the magnification at which everything fits is the current one times the worst ratio. `viewer-gtk`
lost its private copy, gained the arithmetic, and binds `w` to `Zoom::Scale`. **No message was
added.** Qt still measures in `cpp/window.cpp`, so feeding the shared arithmetic there is a `cxx`
bridge change, and that is what is left of the item.

## Measured, not assumed

**§12.7.5.2.4's population, over all 1293 documents this tree can reach** — 964 openable of pdf.js's
974 and 329 of the four corpora's 337, `cargo run --release -p pdf-model --example
field_flag_census`:

| | widgets | documents |
|---|---|---|
| bit 26 on a `Btn` (`RadiosInUnison`) | **0** | 0 |
| bit 26 on a `Tx` (`RichText`) | **0** | 0 |
| a radio field whose widgets share an `/AP /N` on state, flag **set** | **0** fields | — |
| the same, flag **clear** | **0** fields | — |

The first two rows were not answerable before this round: bit 26 was one census row called
`RadiosInUnison/RichText` because Table 226's `/FT` was prose beside the count rather than a filter.
It is a filter now, which makes every type-specific row accurate. The last two are the population a
flag count cannot see — a document can exercise the clause without setting the bit, because the
bit's clear case is a requirement too — and both are zero, so the corpus and the oracle are blind to
this change in either direction. Trap 8's instrument is what was used instead: a pair of fixtures
differing in one bit, and both new assertions were **checked by deleting the rule**. Both fail; the
three describing the pre-existing behaviour keep passing, which is the evidence that the flag-set
half really was already right.

**The form scale, under `Xvfb` on `160F-2019.pdf`:**

```
11 of 76 control(s) wider than their /Rect (worst +85 on 120 px), 76 taller (worst +22 on 12 px);
  every control fits at 3.278, which `w` sends
note: fitting §12.7's controls at 3.278
 0 of 76 control(s) wider than their /Rect (worst +0 on 0 px), 0 taller (worst +0 on 0 px);
  every control fits at this magnification
```

The counts before the key press are ADR 0245's own numbers to the pixel, which is what says the move
into `viewer-host` measures the same thing.

## Gates

Every one green, on the sequence `doc/todo/02` §2 states. `cargo fmt --all --check` clean, `cargo
clippy --workspace --all-targets` silent of lints. **1852 tests**, 15 skipped; doctests unchanged.
**The C compile test ran rather than skipping** — `viewer-ffi::a_c_program_drives_the_abi PASS
[11.767s]` — which matters this round more than usual, because it is the only thing that puts a C
compiler and a linker in front of sixty-eight new declarations.

| gate | verdict |
|---|---|
| corpus | 974 documents in 4.5 s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **62 incomplete**, 0 slow |
| oracle | 1794 pages: **905 agrees, 68 contradicted, 786 ambiguous**, 19 no render |
| text extraction | **10967/11161 matched words in bounds (98.26%)**, 507 of 974 judged; pdf.js and PDFBox halves both pass |
| dates | 1514 of 1545 conform to §7.9.4 (97.99%) |
| xmp | 318 of 319 read, 3191 properties |
| jpeg 2000 | 14 codestreams byte-identical to OpenJPEG |
| quorra | 956 pages: 917 agree, 37 differ, 2 refused, 18 not comparable |
| conformance | 5 tests pass, every table reference names a table the standard has |

**The corpus and the oracle were run because `RadiosInUnison` is appearance code**, and the census
above is why they could not have moved: no document in the corpus states a radio field the rule can
reach. They are reported as unmoved rather than assumed to be.

**Trap 10a's tell reads 0.0% and it is not a regression.** This worktree's `CARGO_TARGET_DIR` is its
own, so `target/tmp/pdfref-cache` was empty and all 6176 reference renders were produced fresh —
1121 s of `pdftoppm`, `mutool` and `gs`. A hit rate of zero on a *fresh* cache directory is the
instrument working; the number to watch is a low rate on a warm one.

## §5, run here and owed again on `main`

The six binaries and `libviewer_ffi.so` are built in release and installed into this worktree's
`target/`, which is what verifies that the new code survives `lto = "fat"` as well as putting them
where the section says. **They are not the ones a person's shell finds**: this round's build
directory is inside the worktree, so `target/` here is the worktree's and nothing in it survives the
merge. §5 for `main` belongs to whoever merges this, along with §2's whole sequence — which is that
section's own rule and not a courtesy.

**Touched.** `crates/viewer-ffi/src/kinds.rs` (nine enumerations, and the rule at the top about which
kind gets a count), `crates/viewer-ffi/src/form.rs` (new), `crates/viewer-ffi/src/shapes.rs` (new),
`crates/viewer-ffi/src/panels.rs` (`Panel` beside `Outline`), `crates/viewer-ffi/src/session.rs` (the
new commands and queries), `crates/viewer-ffi/src/events.rs` (eight accessors and Table 164's
numbers), `crates/viewer-ffi/src/abi.rs` (68 entry points), `crates/viewer-ffi/src/lib.rs`,
`crates/viewer-ffi/include/pdf_viewer.h` (by hand, as the header always is),
`crates/viewer-ffi/c/open_a_page.c` (the new surface, and §12.7's form on a fixture),
`crates/viewer-ffi/tests/*` (the three counts and the fixture that gives the C program a form),
`crates/pdf-model/src/appearance.rs` (`an_earlier_button_answers_to`, `holds_state`),
`crates/pdf-model/tests/radios_in_unison.rs` (new),
`crates/pdf-model/examples/field_flag_census.rs` (bit 26 by field type, and §12.7.5.2.4's own
population), `crates/viewer-host/src/fit.rs` (new), `crates/viewer-host/src/lib.rs`,
`crates/viewer-gtk/src/host.rs` (the shared `ControlFit`, the magnification, `w`),
`doc/conformance/ledger.toml` (§12.7.5.2, §12.7.5.2.1, §12.7.5.2.3, §12.7.5.2.4),
`doc/todo/30-a-native-host.md`, `doc/todo/README.md`, `doc/ui-boundary.md`, `doc/crate-map.md`,
`doc/HANDOVER.md`, `doc/running-the-viewer.md`, `doc/adr/0346-*` (new), this file.
