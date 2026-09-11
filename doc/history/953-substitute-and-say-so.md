# 953 — Substitute and say so, and a route the verb cannot take

Date: 2026-09-11. ADR: 0953.
Files: `crates/pdf-font/src/restate.rs` (new), `src/standard.rs`, `crates/pdf-transform/src/archive/`.

§4.9's default, which the owner overturned this project's original position to get: a font the file
never embedded is substituted and embedded rather than refused, because a substitute is what a
reader already sees.

**§4.9 states three routes for the metrics and the third moves every line of the document.** Route
1 keeps a metric-compatible face's own advances; route 2 restates the program's advances to the
numbers the file already states; route 3 — rewriting `/Widths` to match the substitute — is the one
that repositions text, because `/Widths` is what §9.2.4 positions glyphs by. **Route 3 is not
reachable**, established three ways rather than asserted: the corpus sweep gained a property that
the whole set of `/Widths`, `/W`, `/DW`, `/DW2`, `/W2` and `/MissingWidth` statements in an output
equals its source's — which caught a real bug on its first run, because `/W` is *also* a
cross-reference stream's key; the four section 6.2.11.5 witnesses render byte for byte identical at
100 dpi either side; and every glyph of all twenty compiled-in faces is restated one at a time and
its drawn path compared.

**Two rows stopped being debts and became the fence.**
`embedded-programs-define-every-glyph-shown` and `no-notdef-glyph-shown` had sat under
`NOT_BUILT_YET` — a promise nothing will keep — because both predicates ask their question only of
a font whose *own* program the file carries, which is §2.2's refusal rather than a gap.

`restate` is real font surgery: the sfnt path rebuilds `hmtx` and `hhea` with every checksum and
`head.checkSumAdjustment` recomputed, and the CFF path does leading-width-operand surgery **with
subroutine inlining**, because a subroutinised charstring states its width inside the subr and
prepending a second would flip the parity the rule turns on.

The re-validation net earned its keep again: a substitution leaves `/CharSet` and `/CIDSet`
describing a program the file never carried, which section 6.2.11.4.2 requires to be complete.
Stage three found it, exactly as ADR 0947 predicted it would.

**What a shipped font family would buy, counted rather than assumed**: two documents, Latin,
wanting an OFL sans in **CFF** form because §9.9's Table 124 admits only a CFF program in an
`MMType1` descriptor. Four more are blocked by *composite* substitution rather than a missing face,
so a Noto-CJK family is necessary and not sufficient for the Japanese pair. No font was downloaded.
