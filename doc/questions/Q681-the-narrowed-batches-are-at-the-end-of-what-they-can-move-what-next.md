# Q681 — The narrowed batches are reaching the end of what they can move: what next?

Asked by round 1815, the review of 2026-10-27 into whether the batches are making progress (review
1815, ADR 2464). Nothing below is built. Until this is answered, the briefs follow ADR 2464's rules
R1–R5, and the slots R1 leaves over go to the frontier's largest object.

**What the review found.** Batches 120 and 121 followed the narrowing in every slot. What those
slots can still move is small:

- **The ledger.** Of the twelve rows `partial` at batch 108, nine are still `partial`. Of the seven
  leaf families still `partial` or `reported`, five are held outside this tree, two of them by
  answers of yours. Only §13.2.6.2/§13.2.6.3 and K.2 can be finished here.
- **PDF/A.** `pdf.js as-if-printed` at part 4 moved −3 and then up to +4 over seven archive slots.
  Before the narrowing it gained about seven documents a slot. Most of the remaining refusals are
  not a remedy's to carry.

## 1. Is any of these blocks to be reopened?

Each is held by an earlier decision, with what reopening it would move:

| block | held by | what reopening it moves |
|---|---|---|
| `image-smoothing`, a profile's `stop` | the profiles' design (`doc/todo/72` section 4 item 5) | 43 refusal instances over the four census lines |
| the `as-shown` font losses, a `stop` | the profile's own stop | 94 instances over four sentences |
| the fence rows | ADR 0816 | 41, 29, 23 and 17 instances |
| part 4's `DeviceCMYK` beside an RGB intent | ADR 2130 (the operator supplies the press profile) | 29 |
| CIE Jab (§7.4.9) | A348, declining CIE 131 | one row family |
| the public-key handler (§7.6.5.1–3, §7.6.6) | A66 "Wait for a real trigger", A168 | four rows and a derived one |

*Recommendation*: **keep the fence and the two `stop`s.** Each is a refusal that tells the reader
what a conversion would lose, and that is principle 1's "no silent" rule at work. **Reconsider the
press profile only if you want part 4's 29**, because a shipped default would be this program's
choice of press, made on the operator's behalf. Keep A348 and A66 as they stand. The point of the
table is that **these blocks, not the rounds, now set the pace.**

## 2. With the frontier this small, what should the other slots do?

Three options:

- (a) Keep the narrowing, and accept batches of four or five slots.
- (b) Return two slots a batch to the robustness lanes (the corpus and the oracle). `CLAUDE.md`'s
  "Work is chosen from both, never one" names those lanes as the instrument for the denominator the
  narrowing has left unworked since batch 120.
- (c) Name another target.

*Recommendation*: **(b)**. Keep the ledger slot and as many archive slots as ADR 2464's frontier
line holds objects, and give two slots to the oracle's worst pages.

## 3. May the orchestrator push a batch branch so that CI's `window` job runs before the merge?

§12.2's WinUI half is evidenced by CI alone. It has been briefed in five batches because each fix
waits for the next CI run after a merge and your push. Rounds never push, and the orchestrator
pushes nothing today. `ci.yml` already accepts `workflow_dispatch` so that a branch can go
through its jobs before it is merged (ADR 0450). What is missing is a branch on GitHub for it to run on.

*Recommendation*: **allow the orchestrator to push `ci/batch-<first>` (never `main`) and dispatch the
`window` job on it.**
Its result would be read before the merge. If you would rather keep every push yours, ADR 2464 rule
R2 already stops a slot being briefed without a run to read.
