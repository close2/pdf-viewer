# Confine the interpreter and the rasteriser

Status: the image codecs are confined; nothing else is.
Priority: 34
Clauses: —, this is `CLAUDE.md` principle 3
Code: `crates/pdf-sandbox`, and whatever transport §0 grows

Spike D confines the image codecs — JBIG2, JPEG 2000 and CCITT run in `pdf-sandbox-worker` under
seccomp-BPF and Landlock, with no filesystem and no network (ADR 0014). Principle 3 asks for the
same of the interpreter and the rasteriser, which are the larger attack surface by far.

**§0 turned this from a design question into a transport change**, and that is the whole reason
it is worth recording here. The open question used to be "the protocol would have to carry a
display list rather than an image, which is a real design question". If the boundary is
`Command`/`Event` with `Raster` payloads — which it is — the question dissolves: the confined
process owns document, interpretation and rasterisation, and the host receives pixels and events.
One protocol instead of two, and simpler than shipping display lists across it.

Two things to keep in view when it is taken:

- **`viewer-core` is already free of threads, I/O and clocks** by rule, so nothing in it has to
  change to live in a confined process.
- The sandbox is a **flag** and the default is the safe one. `--no-sandbox` trades panic
  containment and a memory ceiling, not memory safety, and there is deliberately no path that
  falls back to in-process work when the worker fails to start.
