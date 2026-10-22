# 2222 — Direct ICC solids retain source and current context

Status: accepted with measured correctness cost; final integration pending. Session 1694.

## Decision

An ordinary direct ICC colour selected before `ri`, `RI` or `UseBlackPtComp` must use the parameters at painting. The old state retained a converted colour but only retained RGB source operands. Changing intent after selecting an ICC source therefore left its paint stale. Section 11.7.5.3 requires current elementary-paint parameters; group-to-parent conversion still uses its separate outer `Do` state.

Replace the two RGB-only source fields with a narrow source enum: inline RGB, or an immutable reference-counted ICC profile and one, three or four components. Selection retains the actual `read_in` space after operand-count recovery, rather than reconstructing a source from the declared colour space. Initial colour selection and device operators use the same constructor. Empty and invalid operands preserve existing recovery and diagnostics. Other colour families keep their previous fallback conversion; this is not an implementation of every colour-space dependency.

The existing two bounded pre-alpha memos distinguish source family, exact component bits and ICC profile identity, then compare the existing complete conversion context. Owning the source and conversion keeps identity-bearing objects alive through save/restore. Selection seeds the memo with its already-converted colour. No dirty flag, new cache or global invalidation is introduced. Current alpha and actual-paint reporting remain outside the memo.

Pattern binding remains first priority and retains its distinct invoking-mark parameters. Generic ICC pattern tints continue their direct conversion path. Pattern cell setup clears unrelated ordinary sources. Forms and saves clone the retained resolved source without re-resolving a shadowed resource name. The existing RGB tint memo route remains shared and bounded.

## Evidence and limits

Root-owned independent tests were committed before this change: the fixed baseline passed two and failed eleven, with stale colours rather than parse failures. The candidate passes all thirteen after the independent group-alpha oracle correction explained in ADR 2220. The correction preserves its tolerance and models intermediate 8-bit storage; both inherited and cold in-group source selection produce the same opaque, half-alpha and overlapping pixels.

Supplementary rendering-intent controls exercise each component of three- and four-channel profiles, malformed/empty operand recovery and inherited-versus-cold group paint. Existing mixed-paint and pattern-tint controls remain intact. Scoped warnings-denied all-target clippy, formatting and 34 tests passed. An isolated unmodified export passes all thirteen independent tests; each deliberate omission of source retention, ICC profile identity or complete conversion-context matching fails its targeted assertion (exit 101). The calibration driver exits 0 after 157 s, peak 0.65 GiB, and restores the exact source. Paired whole-interpretation costs are recorded in ADR 2223; no performance-neutral claim or final integration acceptance is made.

The full model-and-colour suite passes 2301 tests with 21 skipped: exit 0, 62.615 s tests, 173 s hold, 394 s elapsed, peak 3.90 GiB. Combined integration remains separately owed.

## Cost decision

Root accepts the scoped correctness tradeoff provisionally, subject to combined corpus and launch gates. Same-output ordinary component/profile restore costs about 20–21% more in these synthetic cases, or 1.400/1.872 ms per 20,000 marks (about 70/94 ns per mark). Restoring an earlier source can require a fresh conversion after the other source occupied the bounded slot. This cost is retained explicitly; there is no performance-neutral claim and no broader cache expansion.

The intent/BPC populations require work the stale baseline omitted: representative pixels change from [139,135,152] to Saturation [0,167,255] or BPC [116,111,133]. Their additional 2.133/2.401 ms per 20,000 marks is not an equal-output comparison. All other representative samples agree across both artifacts. Generic ICC tints remain direct, and their paired median ratios span 0.98159–1.00770; mixed-use ratios span 0.99992–1.01268. RGB controls are 1.01111/1.01928. Short repeated-solid samples remain noisy (paired range 0.72358–1.43466); no sample was discarded or chosen to improve the verdict.
