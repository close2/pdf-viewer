# 2303 — The CJK pack is found beside the program, read by its broker, and held to digests it carries

Session 1734, the archive slot of batch one hundred and twelve. Status: implemented in
`quorra-transform` and `archive::fonts::pack`. Context: ADR 2302, ADR 2140 (the confined worker),
`doc/questions/A569`, `A98`, `Q596`; `doc/cjk-pack.md`.

## The premise that did not hold

The brief placed the pack "where the machine keeps this program's data (the same place the base
faces live, `doc/stack.md` says where)". No such place exists: the base faces are compiled in
(`pdf_font::standard`, `include_bytes!`), and `doc/stack.md` names no data directory. A569's own
reading says where instead — "the tens of megabytes sit beside the exe".

## Decision

1. **Where.** The directory `cjk-pack` beside the `quorra-transform` executable, or the one
   `--cjk-pack <dir>` names. Nothing else is searched — not a font directory, not a cache — and
   nothing is fetched: A98's client is for a transmission a person allowed, and A569 makes the
   pack a download a person chooses to make. A collection's font without its file is refused with a
   sentence saying the pack is not installed and where it goes. **The directory beside the
   executable is decided before any worker exists** (`with_the_pack_beside`, which adds
   `--cjk-pack` to an `archive` command line that named none): asking where one's own executable
   is is a system call outside the confined worker's filter, and the first build that asked it
   there was stopped with `SIGSYS` by `confined::every_verb_writes_what_the_in_process_route_writes`.
2. **Read by the caller.** `apply` opens no path (RFC 0002 section 5), so the plan carries a
   `CjkPack` built over the caller's `PackFiles`. The binary reads the five files through its
   `World`, so a confined run reads them by the same broker as every other input, and the fence
   names the five paths (`route::fences`). **The cost**: an `archive` run whose pack is installed
   reads about 27 MB whole, because the confined worker cannot reach the disk later; a run with no
   pack directory reads nothing. The viewer never reads it.
3. **Held to digests the program carries.** `Collection::digest` is each program's SHA-256 as
   `tools/cjk-pack.py` writes it, and a program is hashed on the first font that asks for its
   collection; other bytes are refused as not this pack. The build is reproducible — two builds
   from the same inputs gave the same five digests — which is what lets a binary carry them.
   Versioning is the digests: a rebuilt pack is a new pair of binary and download, and how that
   pair is published is `doc/questions/Q596`'s question to the owner.

## What this does not decide

How the download is published, named and versioned beside a release — Q596. Until the owner
answers, the pack exists as `tools/cjk-pack.py`'s output, and the census measures it through
`QUORRA_CJK_PACK`.
