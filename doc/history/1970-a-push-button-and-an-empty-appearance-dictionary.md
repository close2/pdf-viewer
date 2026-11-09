# 1970 — A push button with no appearance, and an appearance dictionary stating no entry

Session 1970, a single round, PDF/A lane's worktree. R16, two sites (ADRs 2751, 2752).

**The premise.** Half. Under `as-if-printed` R16 was the only refusal of four: `160F-2019` and
`checkbox_no_appearance` (a button with no `/AP`), `sakula` (no `N`, at 2b only; R03 at 4) and
`poppler-395-0-fuzzed` (signature and a partial construction). But the two button documents are two
kinds: `160F`'s seven are push buttons (Table 229 bit 17) drawing nothing, the check box pair is
not; and `sakula`'s `/AP` is not empty in the file but `<< /N 0 0 R >>`, null by §7.3.10.

**Built.** ADR 2751: a push button with no `/AP` is constructed (Table 192, as every widget) and
written as the one entry of a subdictionary under ADR 2319's `Normal`, `/AS` selecting it; a check
box or radio button keeps the refusal, in a sentence naming those two. ADR 2752:
`pdf_archive::states_an_appearance` reads an `/AP` of no entries or only null ones (§7.3.7) as
none, for both appearance rows; such an `/AP` is taken away (`AppearanceStates::emptied`), and
the annotation then owes what an annotation with no `/AP` owes.

**Freed** under `as-if-printed`: `160F-2019` at 2b and 4, `sakula` at 2b. **Pixels** (CPU, 72 dpi):
`160F` against its source at most 1, 0 with `/OutputIntents` renamed; `sakula` with the entry blanked
from the source draws all 49 pages identically, and its output's differences are the profile's
named font substitutions.

**Tests** (`archive/button_appearances.rs`, five). Plants: a push button's `/N` written as a stream
fails the first (the output settles only on a second pass); no check-box refusal fails the second;
no `emptied` branch fails the third and fourth; the empty-only reading in `pdf-archive` fails the
third and fifth. One existing case gains a `/D` beside its null `/N`, keeping that sentence's shape.

**Census** (against round 1969): shipped 3 NEW (the three above), 0 GONE, 0 CHANGED; stating 0 NEW,
0 GONE, 1 CHANGED, `digitally_signed_3D_Portfolio` under `state-every-loss` at 2b, whose statement
page no longer lists `annotations/appearance-dictionary-present` among its embedded PDF's refusals.
R16 now frees 2 / 2 and stops 14 / 17. veraPDF 1.30.3 passes the three CLI and three census outputs.

**Left of R16**: the check box with no `/AP` (its removal touches its field), hidden widgets under
`as-shown`, Popup pairs, appended-page appearances under a structure tree, a partial field
construction, and the XFA forms' non-printing signatures.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on pdf-transform and pdf-archive; nextest
pdf-transform and pdf-archive 0 (1452 passed); archive tier 0; both censuses 0; `cargo test -p
conformance` 0 (471 passed) after `git add -N`; `tools/state.sh --round 1970 docs` 0.
