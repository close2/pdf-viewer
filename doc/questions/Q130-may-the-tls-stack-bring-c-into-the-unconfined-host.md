# Q130 — May the TLS stack a form is sent with bring C into the unconfined host?

Source: round 1227, building the client `A98` ratified (ADR 1291 section 4).
Status: **open** — answered when `A130-may-the-tls-stack-bring-c-into-the-unconfined-host.md`
exists beside this file.

## Why it needs the owner

`CLAUDE.md` principle 3: "Any C dependency (notably JBIG2 / JPEG2000, both historically severe
attack surfaces) must be isolated in the sandboxed process and justified in writing." `A98` chose
`ureq` over `rustls`, and `rustls` cannot run without a cryptographic provider. Every stable one is
C: `ring` 0.17 (C and assembly taken from BoringSSL, built by `cc`) or `aws-lc-rs` (AWS-LC, larger,
C too). The pure-Rust `rustls-rustcrypto` has no stable release. So the ratified shape puts C into
`viewer-host`, which is the *unconfined* side, over bytes a server chose — the letter of the rule
says that is not allowed, and a round does not relax a principle.

## What the tree does meanwhile

It uses `ring`, in `viewer-host` alone, reached only when a person lets a form be sent (the default
level is `ask`). ADR 1291 section 4 writes down the cost and the argument: the host already links
GTK and Qt, which are C and C++; the rule's examples are codecs over *document* bytes, and the
confined worker still has neither C nor a network; `ring` is among the most fuzzed and reviewed
code of its kind. `openssl`, `openssl-sys` and `native-tls` are banned in `deny.toml`.

## The options

1. **Accept `ring` in the host, written into principle 3 as a named exception** — TLS for a
   transmission a person allowed — until a pure-Rust provider is stable, and revisit then.
2. **Move TLS into a confined helper process** that holds only a socket, so the C runs under
   seccomp and Landlock like a codec would. Costs a second broker protocol and a process per
   submission; the request is small and rare, so latency does not argue against it.
3. **Send over `http` only** and refuse `https` until a pure-Rust provider is stable. No C at all,
   and a form sent in the clear, which is worse for the person than any of the above.

## Recommendation

**Option 1.** The rule exists to keep hostile document bytes away from C, and it still does:
nothing a PDF contains reaches `ring` except the URL the person approved. Option 2 is sound and
costs a protocol for a rare act; it becomes the right answer if a codec-style exploit class ever
appears in the provider. Option 3 is not worth offering a person.
