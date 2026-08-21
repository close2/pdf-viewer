# 639 — The round that left no silence

Fifth merge round, four branches, two conflicts and both of them the ordinary kind — two rounds
appending to the same file and two claiming the same section number. Nothing had to be composed
this time, which is worth recording after 634: the arrangement is not always adversarial.

## What was merged

`round-635`, `round-636`, `round-637`, `round-638`, branched from `95830400`.

`doc/checks/fixed-documents.toml` took both appends — **31 rows over nine sessions** now. `doc/todo/03`
had two section 24s; 635's keeps the number and 636's becomes §25.

## The sequence, whole, quiet machine

| | |
|---|---|
| `fmt`, `clippy --workspace --all-targets` under `-D warnings` | clean, silent |
| `nextest --workspace` | **2355 passed, 17 skipped** |
| doctests, `-p conformance` | clean (157 + 5 + 1) |
| corpus | 974 documents, 68 incomplete |
| oracle | 1794 pages — 907 agrees, 66 contradicted, 786 ambiguous |
| `render-quorra` | 957 pages — 932 agree, 23 differ, 2 refused |
| **`fixed_documents`** | **31 checked, 0 absent** |
| text extraction, both censuses, dates, XMP, JPEG 2000 | clean |
| `cargo deny check` | advisories, bans, licenses, sources ok |

## The ledger has no `silent` row

875 rows: **436 implemented, 222 partial, 18 reported, 78 inapplicable, 8 writer-side, 113
out-of-scope, 0 unreviewed.**

Four rounds ago 632 put the project's first `silent` row in the ledger by reading §11.7.5.2
honestly, and `doc/HANDOVER.md` calls that status the one worth hunting: a requirement this program
fails **without saying so**. 637 closed it — and the two numbers that moved beside it are the
round's real result rather than the closure. **`reported` 17 → 18** is §11.7.5.2 itself, now loud.
**`implemented` 437 → 436** is §10.5, which 637 demoted on the way: enumerating every place a
transfer function is applied showed what was *not* on the list — a shading's colours pass through
neither route, so a `sh` or shading-pattern fill under a stated `/TR` was drawn unmapped, silently,
inside a row that said `implemented`.

**Closing one silence found another one clause over.** That is the argument for the status existing
at all, and it is why a `silent` row must never be left `silent` with nothing added.

## What the four rounds were, in one line each

- **635** finished what 631 and 633 began from opposite ends: three more of §7.4's filters now
  *derive* an inline image's end rather than searching for it. Its most valuable result is a
  **negative** one — all 1 272 438 `CCITTFaxDecode` images already agreed with the search, so half
  the population was guessed at correctly and is now derived rather than lucky.
- **636** ranked ten more archives and found a catalogue cover drawn as its own complement: §7.4.9
  ignores a `/Decode` array only where `/ColorSpace` is **absent**, and both the code and the ledger
  row had read that condition as being about the *filter*. Its negative head is **the shallowest any
  chunk has produced** — deepest −10.2 against −112.6, −84.2, −43.5 and −20.3 before it.
- **637** closed the `silent` row, as above.
- **638** built the presentation window in all three hosts, and overturned the reasoning that had
  kept it open: §12.4.4 requires nothing of one, but §7.7.2's Table 29 and §12.2's Table 147 do —
  *"Full-screen mode, with no menu bar, window controls, or any other window visible"*. The silence
  was real and in the wrong clause.

## Two instrument findings this batch produced, both now in `doc/environment.md`

- **`xwd` returns stale content for a window that has not repainted.** 638 got four byte-identical
  captures across a rebuild and two different renderers, and nearly wrote a code comment claiming a
  defect nobody had seen.
- **A round's edits can land in the main worktree** if absolute paths are typed. 637's first five
  did; it captured them as a patch, applied them to its own tree and reverse-applied them in `main`
  **before anything was built**, and said so. `main` was verified clean at `?? .claude/` before this
  merge began.

## Owed

- **CI**: 630's two fixes — the `-u` linker argument for a runner with no `lld`, and Miri's
  fifty-minute test — are on `main` and have still never faced a run. The token cannot push.
- **The owner's session**: `tmp/pi.pdf`, for 628.
- **`doc/todo/37` lead 2**, which 633 declined to claim without a trace it could not take, still
  wants a quiet machine.
- **13 944 crawled documents unranked**, 52 of 145 archives done.
