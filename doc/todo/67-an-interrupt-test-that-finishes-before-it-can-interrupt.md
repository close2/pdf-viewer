# 67 — An interrupt test whose deliberately slow draw finishes before there is anything to interrupt

Status: **open**, found twice in the round after 1156 — by round 1155 on base 9f145bff in a
throwaway control worktree, and by round 1156 on this machine roughly nine runs in ten. A 10-band
defect numbered past its band because the band is full.
Priority: **medium** — it fails tier 1's own `cargo test -p viewer-confined` on this machine and
so bites any round that runs the suite here, while CI's slower runner still passes it.
Code: `crates/viewer-confined/tests/confined.rs`, the test
`a_host_drawing_marks_that_will_not_finish_interrupts_its_own_draw` and its `HOST_UNFINISHED`
constant. ADR 0650 is the evidence the test holds up.

## What is wrong

The test draws ten thousand page-covering fills and asserts the draw is still **unfinished** after
`HOST_UNFINISHED` so that the interrupt has something to interrupt. Its own comment priced that
draw at 27.6 s when it was written in session 745; the machine now finishes it inside the two
seconds, so the assertion at the receive timeout fires with *the host's draw finished, so this test
interrupts nothing*. Nothing in the interrupt path is known to be wrong — the test has stopped being
able to say either way, which is a hole in ADR 0650's evidence rather than a regression.

## What a round owes

Measure first (trap 8): time the draw on this machine and on the CI runner's class of machine.
Then make the draw one that cannot finish — a bigger amplification document, or a draw the test
holds open by construction (a mark that waits on a signal the test owns) — rather than a re-tuned
constant, which is the shape that decays again. Whichever, the record states the measurement and
the test's comment states the new price. Both rounds that found it declined to move the constant
because that is a decision for a round that owns the gate; this file is where it is owned.
