# Q54 — Does `apply` run the external tool, or return a request for the caller to run?

Source: `doc/rfc/0007` §7 question 1, raised by session 954 designing the remedy configuration.
Status: **open** — answered when `A54-does-apply-run-the-tool-or-ask-the-caller-to.md` exists
beside this file.

## Why it needs the owner

RFC 0002 §5's second rule is that `apply` has **no filesystem, no clock, no environment**, and §9
turns that into a determinism claim this project *tests* rather than asserts: the same inputs
produce the same bytes with no flag needed. An external tool invocation breaks both rules.

Two shapes are available and the difference is architectural rather than stylistic.

**`apply` runs the tool.** Simpler, streams naturally, and every consumer — the CLI, the KIO
worker, the FUSE filesystem, a future viewer menu — gets remedies for free. It also puts process
spawning inside a library whose whole stated boundary is that it does neither that nor file
access, and every consumer inherits that whether it wanted it or not.

**`apply` returns a request and the caller runs it.** The library stays pure, the binary is the
only thing that spawns, and a consumer that does not want to run other people's programs simply
does not. It costs a round trip per invocation, complicates streaming, and turns `apply` from a
function into something closer to a coroutine — which is a real cost in a codebase that has
deliberately kept it a function.

## What the tree does meanwhile

Nothing: no tool is invoked, and every refusal is terminal, which is the behaviour ADR 0947 built
and ADR 0954 amends in principle without changing code.

## Recommendation

The second. The purity of `apply` is load-bearing for more than tidiness — RFC 0003's confined
worker and RFC 0002's determinism gate both rest on it — and a boundary that is expensive to cross
is exactly what should sit in front of running an arbitrary program on an untrusted document.
