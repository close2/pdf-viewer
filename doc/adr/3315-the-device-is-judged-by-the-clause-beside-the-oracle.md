# 3315 — The device is judged by the clause, beside the oracle

Session 2153. Status: **implemented.** Context: RFC 0016 section 5; ADR 3207; the owner's A948
(recommendation 3); ADR 3314 (`quorra-draw`).

## Decision

`portable-conformance/adapters/quorra-gpu` answers the protocol for this program on the graphics
device: `render` through `quorra-draw` (zero-based page in, one-based `--page` out), `text` and
`query` through `portable-adapter`, exactly as `adapters/quorra` does, because extracted text and
document structure are not drawn and so have no device of their own. Its requests that are not
drawn therefore answer as the CPU adapter's do, and only its `pixels` checks judge anything new.

**A disagreement between the two adapters is read against the clause, not against each other.** The
CPU backend is the oracle for the *cross-backend comparison* (CLAUDE.md principle 2), but in this
suite neither backend is a reference: a `pixels` check's expected value is the clause's arithmetic
(ADR 3206), and the backend that misses it is wrong whichever one it is. A device failure is a
finding against the device, named in the round's record and the run's table; a check both miss is
a finding against both.

The pdf.js adapter waits (A948): it needs a JavaScript runtime this machine does not have.
