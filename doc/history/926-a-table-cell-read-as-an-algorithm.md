# 926 — A table cell read as an algorithm, and 104 GB nobody had swept

2026-09-04. Argued in
[ADR 0892](../adr/0892-a-type-3-fonts-encoding-may-be-a-name.md) (a Type 3 font's `/Encoding` may
be a name) and
[ADR 0893](../adr/0893-a-font-dictionary-that-is-the-null-object.md) (a `/Font` entry that is
§7.3.10's null, held rather than guessed).
[`doc/questions/Q35`](../questions/Q35-a-font-the-file-does-not-carry.md) is the half a round
should not decide for itself — asked in this session as `Q27`, and renumbered by session 934
because round 927 took the same number on a branch this one could not see.

Merged: `round-922` — the launch-path gate, `CLAUDE.md` principle 2's four numbers plus the fifth
it makes a gate of its own, ADRs 0884 and 0885.

Touched, beyond the merge: `crates/pdf-model/src/type3.rs`,
`crates/pdf-model/tests/type3.rs`, `doc/conformance/ledger.toml` (§9.6.4, §9.6.5.3),
`doc/checks/fixed-documents.toml` (four rows), `doc/todo/03-more-corpora.md` (§48), two ADRs, one
`Q` file, this file.

## 1. The merge, and the one conflict

`tools/state.sh`, in the same `section_vfs` two previous merges had already touched. At the merge
base the section ran `-p pdf-vfs --test awkward_classes`, a target session 919 had deleted. Round
922 repointed it at `viewer-confined`; round 924, on `main`, deleted the line and wrote down why.
`main`'s resolution is taken, on three grounds rather than on its date: `doc/todo/02` §2's sequence
— which `tools/conformance/tests/sandbox_gates.rs` reads — names no such line and `state.sh` runs
that sequence; `doc/verify.md` owns the invocation and says the walk is owed by a round that
touches the confinement rather than by every round; and 922's line would have run a corpus walk
with neither `cargo build -p viewer-confined --bins` in front of it (trap 10) nor
`tools/bounded.sh` around it, both of which `doc/verify.md`'s invocation has. `section_launch` is
taken whole from 922.

Every other file auto-merged and each was read rather than trusted. The ledger is untouched by
`round-922` — checked with `git diff main -- doc/conformance/ledger.toml`, which is empty — so the
row-by-row reading the instruction asks for had nothing to find there; what it did find is the
`state.sh` conflict above, which is the third merge in a row where the auto-merge and the correct
answer differ.

## 2. The gates on the merged result, and one false failure of the kind §2 predicts verbatim

The whole sequence, 29 lines. Twenty-eight green. `foreign_corpus` exited 101 on

```
bookmarks: bug1997343.pdf: §14.7.5.4: mupdf resolves the source page's parent-tree entry to
90 entries and ours to 79
```

which is, to the document and the lane, the failure `doc/todo/02` §2 records from session 914 and
tells a round to re-run alone before diagnosing. The run's own log says it had waited for a
neighbour's gate binary thirty seconds earlier. Re-run by itself: **exit 0**, `bookmarks: poppler
identical 5, mupdf identical 5, §14.7 shapes agreed 3, §14.7 faults: 0`. A contended foreign reader
resolves less structure, and that reads exactly like a defect in our carry.

The launch line ran separately, as its own section of that file asks, four times at one-minute
load averages from 12 down to 1.9. `tools/state.sh launch` prints the figures and this file does
not (ADR 0281). What belongs here is the verdict: **twenty-seven of the twenty-eight figures are
inside their bands and one is not** — `doc/PDF20_AN001-BPC.pdf`'s cold open — and this round did
not move that band, because it could not tell a coarse probe from a swept filesystem from a
regression. All three hypotheses, the readings that support each and the two cheap experiments that
would separate them are in [`doc/todo/42`](../todo/42-the-launch-path.md); the second of them is
this round's own doing, since §5a's sweep took the directory the gate's cold-read copy is made
beside.

**Also touched by that section: `doc/checks/fixed-documents.toml` grew four rows**, one per
document ADR 0892 fixed, and two of them take a band narrower than the file's usual ±1.0 and say
why — a five-line table on an A4 sheet is worth 0.739 of a level and twelve text operations on a
newspaper page are worth 0.080, so the usual width would pin nothing they are about.

## 3. The build directory, swept

`doc/todo/02` §5a asks for this past a hundred gigabytes and it stood at **229 GB**; two rounds had
declined it in a row to avoid forcing a cold rebuild on a neighbour, which is right while a
neighbour is building and becomes an excuse if nobody takes it. **104 GiB reclaimed, to 125 GB.**

Removed: the main checkout's `debug` (65 GB), `release` and `gates`, which is §5a's own command; and
four directories no checkout on this disk names — `quorra-main`, `quorra-mask-round`,
`quorra-a21540`, `probes-round` — plus three small ones in the same position, `hayro`, `jpxprobe`
and `pdfref-survey`, none touched since August. `tools/worktree.sh list` is what says which is
whose, and every one of the seven was checked against every `.cargo/config.toml` under
`/home/cl/projects` before it went.

Left: `pdfv-r922`, `pdfv-r923` and `pdfv-r925`, whose worktrees are live; `quorra` (45 GB), which
`/home/cl/projects/render-lib/.cargo/config.toml` names and which belongs to another project;
`target/tmp/`, which holds the reference-render cache §5a says never to take; and the main
checkout's three cross-target directories (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`,
`aarch64-apple-darwin`, 10.3 GB together), which are `doc/verify.md`'s cross-target checks and are
not §5a's named subdirectories.

**The cold rebuild cost 133 s** for §5's whole release set, because `sccache` had every crate. That
is worth writing down: §5a's own estimate of "about three minutes" is right, and the reason two
rounds declined the sweep was a cost the wrapper had already paid.

**And the headroom lasted about an hour.** Rounds 927 to 930 opened while this one ran, and by the
time it committed the root was at **282 GB** again — four new build directories of 20 to 30 GB
apiece, every one of them live. So the honest arithmetic of this sweep is that **31 GiB of the 104
was dead** (seven directories no checkout names) and 73 was the main checkout's own profiles, which
grow back on the next build; a sweep is a recurring cost whose rate is set by how many rounds run
at once, not a fix. §5a says all three of those now.

**And the sweep removed a hazard as a side effect**, which is `doc/oracle-and-corpus.md` §3d's
fourth rule: `<target>/release/examples/` is where session 908 found a `pdf-sandbox-worker` ten
hours behind its tree, searched ahead of the fresh one and refreshed by nothing. There is no such
copy now, and the ranking below was taken with `release/pdf-sandbox-worker` built in the same
minute as the example.

## 4. The improvement lane, which had had no round since 908

`corpus-cache/tika-issue-tracker/batch5/pdfminer.six`, 123 documents. The survey line, the ranking
and what each row is are [`doc/todo/03` §48](../todo/03-more-corpora.md); what belongs here is the
shape of the two findings.

**The head by ink was not the finding, and two instruments said so before the page did.**
`pdfminer.six-29-0.pdf` is 1.06 levels below the interval `pdftoppm` and `mutool draw` bracket, and
step 6's ladder does not converge — ours flat at 9.23 over four resolutions, poppler flat at 12.61,
mupdf drifting *up*. A difference map is uniform over every glyph and empty everywhere else, and
`pdffonts` closes it: six of the seven fonts are not embedded and five are not §9.6.2.2's standard
14, so the three renderers are measuring their own substitutes' stem widths. `doc/todo/21`'s
standing population, held.

**The finding was one row down and it is a reading of §9.6.4.** A Type 3 font stating
`/Encoding /WinAnsiEncoding` was refused outright, because Table 110's cell — "[a]n encoding
dictionary whose Differences array shall specify the complete character encoding for this font" —
had been read as the shape of the lookup. Step a) of the same clause is the half addressed to a
processor and it delegates to §9.6.5, whose General subclause permits a name. ADR 0892; four
documents of 24 324 gain their text and the pages are line for line what `pdftoppm` draws. The same
before/after over the 65 944-document crawl diffs to **zero lines** and finds no witness at all,
which is session 908's argument about which corpus can rank which defect, measured again.

**And the two documents the ranking could not settle are now a decision rather than a silence.**
A `/Font` entry that is §7.3.10's null object: `mutool draw` substitutes and produces a legible
letter on one of them, `pdftoppm` substitutes and produces solid black blocks over an engineering
drawing on the other, and each declines on the page the other guesses at. §9.5 makes every input to
a substitution an entry of the dictionary that is missing. The refusal is kept and argued (ADR
0893), and whether a reader should offer the guess anyway is `Q35`.
