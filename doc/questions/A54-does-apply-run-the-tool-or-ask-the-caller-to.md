Status: complete
Given: 2026-09-12, in conversation — transcribed by the round
Owes: the request type and the shared executor in every shipped consumer (a converter round)

> I am not conviced about you recommendation for Q54. If I want an input pdf converted to pdf/a
> and there is configuration how this can / should be achieved, I think a normal user would expect
> it just to happen. Or is the caller our own converter program? Maybe I am not really
> understanding the problem.

Reading: after the round explained that the caller is this project's own converter program in
both shapes — one command, remedies just happen, nothing makes a person run tools by hand — the
owner chose the option named *request + shared executor*: `apply` returns a data value (program,
args, input, expected type, bounds) and the caller executes it. Every consumer we ship — the CLI,
the KIO worker, the FUSE filesystem — executes requests through one shared executor, so "it just
happens" is the default in each of them, and the only code in the tree that spawns a process is
that one file. The confined-worker plan (RFC 0002 §5) and the determinism gate (its §9, and
RFC 0007 §4.4's recorded-tool-output case) keep the shape they were built on: a request and its
result are data, so a test replays a recorded output and the conversion is deterministic by
construction.
