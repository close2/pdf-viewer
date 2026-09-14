# 1055 — A parameter of a branch we do not convert on

Date: 2026-09-14. ADR 1069. Clause 11's `partial` rows, §11.7.5.3 first. Touched `content/ext_gstate.rs`,
`pattern.rs`, `transparency.rs`, `content.rs`, `report.rs`, `viewer-core/src/report.rs`, two test files, a new
census and the ledger. **Two rows moved to `implemented`: §11.7.5.3 and §10.4.2.4.**

**What was owed turned out to be a fork, not a debt.** Both rows held on §11.7.5.3's bullet — "the functions
used shall be the current black-generation and undercolour-removal functions in effect in the graphics state
at the time of the painting operation" — without asking what algorithm those functions are parameters *of*.
§10.4.2.4 says: invocations inside its own formula, and Table 52 types the graphics state parameter the same
way. §10.4.2.1 ranks that formula below §10.3's branch, where this tree's conversion into a press is (ADRs
0009, 0263, 0796) and which has no black-generation step for a stated function to replace. So the bullets
*select* among functions a conversion uses; reading them as a requirement to use one would make every ICC
enabled processor non-conforming on every such page, which runs against §10.4.2.1's own ranking — ICC enabled processors "should always follow" 10.3, and the algorithm is what a less-capable one "may choose". The third
bullet was inapplicable all along — it conditions on the device's native space being `DeviceCMYK`, and this
one is a screen. **`doc/todo/23` had this reading in the four-hundred-and-twenty-seventh session** and used it
only to justify `rgb_to_ink`; the refusal on the other side of that fork outlived its own argument by six
hundred sessions.

**The refusal was pure loss.** `black_generation_stated` withheld §11.4.7's page-group pair, §11.6.6's
group-scoped press and §11.5.3's four-component mask half and drew the page on the device — which converts
`DeviceRGB` to sRGB and evaluates the stated functions exactly as much as compositing in ink does. It gave up
the space §11.4.7 requires and bought nothing. Those four conditions are gone; `Unsupported::BlackGeneration`
is what is left, raised once per interpretation by `note_black_generation_departure` on the clause's own two
halves — a function is stated **and** this interpretation converted into four components.

**The condition narrowed too, and that half is Table 57's** (trap 11): `/BG2` and `/UCR2` admit "the name
Default, denoting the black-generation function that was in effect at the start of the page", and §10.4.2.4
makes a stated function a function dictionary, so a name is not one; Table 57's precedence decides which of
each pair is in force. `examples/black_generation_census` measures the two conditions five-fold apart — over
`doc/pdf.js` and the three web corpora, 8 of 963 and 910 of 39 127 documents state one of the four, against
**1 and 189** stating a function. Found on the way: §10.4.2.4's row claimed the mask identity holds "whatever
the black-generation and undercolour-removal functions returned" above an algebra that had substituted the
nominal `k` for both. What cancels is `B = U`. Corrected.

**Gates.** Conformance 267/267; tier 1 otherwise green but for siblings' in-flight files. `raster_golden`
**held 974, moved 0**, which the census predicts; the corpus gate's incomplete list is unchanged and the
oracle passes, 62 pages held by name. The new test fails on either plant — the refusal restored, or the
condition widened back to any non-null entry — and the rewritten one fails when `group_press` is allowed a
knockout group, its lever in place of `/BG2`.
