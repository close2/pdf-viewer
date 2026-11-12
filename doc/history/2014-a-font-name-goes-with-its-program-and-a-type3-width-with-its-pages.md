# 2014 — A font name goes with its program, and a Type 3 width with its pages

Session 2014, a single round, PDF/A lane's worktree: R03-photograph's remaining documents under `as-if-printed`,
grouped by cause, largest group first. ADRs 2846, 2847.

**The premise.** Half true. R03 alone holds 7 documents at 2b and 10 at 4, and none is a page the reader does not
draw whole: each is a fenced row the photograph was never offered for. By cause: a font name in GBK or EUC-KR
bytes (`issue2128r`, `issue3521`, `issue5010`; three more beside R10/R11), a Type 3 width (`303226`,
`issue19634`, `issue19971`, part 4), a MediaBox outside part 2's range (`boundingBox_invalid`, `issue19517`), a
Private Use `ActualText` (`sakula`, `smaskdim`), an operator in a widget's appearance (`hesr13613`), a transfer
function (`issue6931_reduced`). The eight govdocs files are the not-drawn-whole ones and stop at R08 and R06b too.
HEAD reproduced every census line under all eight profiles at both parts.

**Built.** ADR 2846 (`archive/fonts/names.rs`, a branch of `decide`): once every route has run, a Type 0 font
whose `/BaseFont` is not UTF-8 is named as ISO 32000-2 Table 119 composes it from its descendant, which the
pack, machine-face and re-keying routes already name as the embedded program (Tables 117, 120); the row is
answered at the substitution's losses where every place holds valid names or is no longer reached from the
catalog. ADR 2847 (`archive/fenced.rs`): the Type 3 width row, reported at its font, is placed on the pages whose
resources reach the font (every object `Named` holds), a font an annotation reaches keeping the fence; the wide
gate photographs them. Ledger 9.7.6.1 stays `implemented`, its note, code and test extended.

**Freed** (with the CJK pack): `issue2128r` under all eight profiles at 2b and 4; `303226`, `issue19634`,
`issue19971` at 4 under `as-if-printed` and `state-every-loss`. `issue3521` converts under those two only without
the pack (its page photographed); with it, its three unshown fonts keep the row. `issue20489` moves to R11.
**Pixels** (CPU, `/OutputIntents` renamed): `issue2128r` under `as-shown` differs by 1 level in 1 pixel at 72 dpi,
under `as-if-printed` (the pack's face, `machine-face`) in 1968 glyph pixels; a photographed page differs by
resampling at 72 dpi and is identical at the photograph's 300 dpi (`303226` 0 pixels, `issue19634` 1 level).

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0 on `pdf-transform` (101 first: CamelCase in paraphrased
clauses); nextest of `pdf-transform` and `pdf-archive` 0 (1520 passed, 26 skipped); the pack test 0; four new
tests each fail under a plant (one run); `cargo test -p conformance` 0 after `git add -N`; archive tier 0 and 0;
censuses against round 2010 (`cmp.py`): shipped 7 NEW, stating 9 NEW (the freed files above), 0 GONE, every
other output identical masked (5397, 4803); veraPDF 1.30.3: 82 runs over new outputs PASS; `rank.py --table`:
R03 7/10 to 6/6, sections 2 and 9 replaced; `tools/state.sh --round 2014 docs` 0. No tier 2: no renderer.

**Left.** MediaBox (a `/UserUnit` bracket the fence closes), `ActualText`, the appearance operator, the transfer
function (ADR 1964's stop), `issue5010` (the program's own name), and `/Widths` written into `d0` (ADR 2847 §3).
