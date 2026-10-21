# 1675 — Poison is not absence, and black generation belongs to the paint

Session 1675, slot 5 of batch one hundred and four; clause round, 2026-10-21. ADRs 2186, 2187.

**Built.** `rgb_route`, `named` and `presses_cached` return `NoPress` on poison. Page and nested
blending-space readers report their fallback; soft masks retain the fault, parallel interpretation
refuses an unknown press count, and census/CLI callers cannot print a forged zero. Thread-poison
fixtures include a subprocess for the process-wide cache, leaving sibling tests uncontaminated.

**Paint.** §11.7.5.3's report now follows an actual DeviceRGB paint under the current pair into a
profile-characterised DeviceCMYK target. A shared conversion predicate travels with cached image
and shading colours; selecting parameters or a pattern alone stays silent. Paired fixtures exercise
fill, stroke, image, direct shading, shading pattern, cache hits and q/Q.

**A pixel changed.** The image conversion had dropped the pair. Passing it through makes RGB
(0.2,0.4,0.6), BG(k)=1−k, UCR(k)=k/2 draw [52,62,79] instead of [51,102,154]. The test checks
that derived pixel. The kept `black_generation_order` example also exposes a separate stale solid
colour: `rg` before `gs` still draws the old conversion. This defect is recorded, not fixed here.

**Readings.** §11.7.5.3 and aggregates §11.7/§11.7.5 become `partial` for that ordering defect;
§12.2 becomes `partial` because the accepted WinUI host owes chrome/placement records. Deliberate
profile and other-host departures remain separate. §7.4.9 stays `partial`: A348 already bought
PIMA 7667, so e-sRGB/e-sYCC need extraction and implementation; CIEJab's text remains unheld.
The §12.7.4.1 claim that Table 199 scripts are excluded is replaced by the existing runner path.
Launch/menu wording is coordinated with the host round. ADR 2187 states the next tests for each debt.

**Gates.** Scoped pdf-colour/pdf-model/safedocs fmt and clippy `-D warnings` exit 0. Four focused
black-generation tests pass, including the actual image pixel; nextest 2268 passed, 21 skipped.
Quotations exit 0 (ledger diverging 3); conformance exits 101 at the concurrent Session split
teaching pointer (`session/editing.rs`); its spec/frontier/records checks pass. The kept example exits 0.
Combined six arms, held-list rustdoc and final conformance belong to batch integration.
Initial image test failed before pair propagation; initial conformance rejected a misplaced clause
attribution and a T.801 table caption treated as a PDF table; both are corrected, not waived.

**Proposed habit.** Test a state change on both sides of the operation that consumes it; keep a cache's
diagnostic with its converted value. A report-only witness cannot prove the pixel received the state.
