# 576 — The name that came off by a run

**The finding**: the largest quorra bump this tree has taken — 121 commits — is
character-identical on the lane the gate runs by default, and everything it delivers is on the two
lanes that lane does not exercise: a refusal gone at 4× on both, and §10.7.4's disappearing thin
mark closed on the device coverage lane.

Date: 2026-08-18. Argued in
[ADR 0411](../adr/0411-the-name-that-came-off-by-a-run.md).

Touched: `Cargo.lock` (two pins), `crates/render-quorra/tests/corpus.rs` (`REFUSED_AT_FOUR` and
its doc comment), `doc/conformance/ledger.toml` (§10.7.4), `doc/QUORRA_UPGRADE.md` (a section for
`cad50156`), `doc/adr/0411-…`, this file.

## What the release was

`eada81ec` → `cad50156`: 121 commits, 92 non-merge, fourteen new ADRs of theirs, 205 files and
32 000 lines. Most of it is the library's own tests, notes and module splits. Four things reach
this tree — a clipped mark's coverage tile bounded by its chain's own device box (their ADR 0057),
a thin mark diverted to the processor lane (0070), a soft mask read as a knockout element's
opacity rather than its shape (0066), and a group inside a knockout group refused by name (0069) —
plus `SceneError::InvalidImageAlpha`, `RenderError::ViewportTransformTooLarge`, three new counters
and a limit, and a `quorra-pages` crate this tree does not depend on.

**It cost no source change to take.** The seventh consecutive bump of which that is true, and the
largest by a long way: nothing was removed, renamed or resignatured.

One lock line moved that is nobody's request: six packages' `windows-sys` re-resolved from 0.61.2
to 0.52.0, both versions already present and both still present. Windows-only, so it was checked
with `doc/verify.md`'s cross-target runs under `-D warnings` for `x86_64-pc-windows-msvc` and
`aarch64-apple-darwin`, all clean, and `cargo deny check` is clean on the new graph.

## Both lanes, both scales, both revisions

Run here side by side in one sitting on the real Radeon 890M under RADV, `--profile gates`:

| lane, scale | `eada81ec` | `cad50156` |
|---|---|---|
| scale 1, `cpu` — the §2 gate | 932 / 23 / 2 / 17 | 932 / 23 / 2 / 17 |
| scale 1, `gpu` | 930 / 25 / 2 / 17 | **932 / 23 / 2 / 17** |
| scale 4, `cpu` | 937 / 11 / 4 / 22 | **938 / 11 / 3 / 22** |
| scale 4, `gpu` | 938 / 10 / 4 / 22 | **939 / 10 / 3 / 22** |

The `gpu` scale-1 columns were `diff`ed line by line rather than compared as totals: exactly two
lines leave — `bug1883609.pdf` and `vertical.pdf` — and every other line is character-identical.
At 4× on the same lane `issue12295.pdf` moves toward the oracle without reaching it, which totals
alone would have called null.

Everything else in `doc/todo/02` §2 ran and is green: fmt, clippy, 2143 nextest tests, the
doctest, the corpus gate at 974 documents and 66 incomplete, the oracle at 906/67/786, the two
text-extraction gates at 98.26%, dates, XMP, JPEG 2000 and the five conformance gates over 8762
citations and 842 quotations.

## The two things worth remembering

**A name comes off a ratchet by a run.** Three separate upstream messages had said
`bug1703683_page2_reduced.pdf` could leave `REFUSED_AT_FOUR`, and the doc comment on that constant
was written by an earlier round so that the departure would be expected rather than rediscovered.
None of them took the name off, correctly: at `eada81ec` the refusal was the right behaviour of
the revision this tree depended on (ADR 0402 decision 3). This round is the run — the scale-4 CPU
lane failed loudly, naming the one element of the difference — and the page then rendered on its
own and **agreed with the CPU oracle**, which is the check that matters, because a page moving
from *refused* to *drawn wrong* looks identical in a refusal list.

**The counts converge and the sets do not.** After the change both coverage lanes differ from the
CPU oracle on 23 pages at page scale, and they are not the same 23: the device lane alone on
`bug1863910.pdf` and `issue16500.pdf`, the processor lane alone on `bug1743245.pdf` and
`issue21068.pdf`. Neither of the first two carries a moved line in either column, so it is a
residue the thin-mark condition does not reach rather than one it caused. §10.7.4's ledger row
records the four names as where a round wanting the lanes to converge starts.

## One stale claim, in the brief this round was told to read

`doc/RENDER_LIBRARY.md` §4.1 said `render-quorra` "has not yet expanded `Command::Shaped` into the
pair, and until it does it refuses the command by name". `Scene::shaped` has emitted the `DestOut`
and `Plus` halves since the four-hundred-and-fifty-sixth session (ADR 0291) — a hundred and twenty
rounds of a false sentence, in the one document this project hands to the team writing its
renderer. Corrected, together with quorra's ADR 0066 amendment to what `DestOut` is weighted by.
The document is read at the start of a quorra round and written by nobody, and the round that made
the sentence false was not a quorra round.

## The question answered back

Upstream's `doc/api-change-image-alpha.md` asks this tree whether `SceneError` should be
`#[non_exhaustive]`. The answer is yes for `SceneError` and `RenderError` and **no for
`SurfaceProblem`**: the first two are open-ended vocabularies of refusals this tree reports
through a `#[from]` and a catch-all, and the third is matched over all five variants with no
wildcard on purpose, because a swapchain state is a decision rather than a report and a sixth one
appearing should stop the build. The rule: mark the enum whose variants a caller reports; leave
exhaustive the enum whose variants a caller decides on.
