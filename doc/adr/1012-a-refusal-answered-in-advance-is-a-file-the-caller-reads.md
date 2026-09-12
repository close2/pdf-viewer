# 1012 — A refusal answered in advance is a file the caller reads

Status: accepted. Session 992.
Context: `crates/pdf-transform/src/archive/{toml,config}.rs`, `crates/pdf-transform/src/archive/mod.rs`
(`ArchivePlan`), `crates/pdf-transform/src/bin/quorra-transform.rs` (`--config`, `--remedy-sites`),
`doc/profiles/*.toml`, `doc/rfc/0007`, ADR 0954, `doc/questions/A54`–`A60`,
`doc/pdf-a-mitigations.md` sections 14 and 15. Companion to ADR 1018 (departures).

## What the owner decided, and what this round built

RFC 0007 reframed a refusal as a question the converter asks, answerable in advance in a
configuration file. ADR 0954 accepted the reframe and left five questions to the owner; `A54`–`A60`
(2026-09-12) answered them. This round builds the *format* and the two answers it can carry today,
and stops where the owner drew the line — `A55`: `derive` and the external-tool executor are the
**next** converter round's, and the format is to be built so they slot in.

The format is TOML (`doc/checks/*.toml` and the ledger already are, `doc/rfc/0007` section 3), read
by a subset reader written for it — `archive/toml.rs`, the same argument
`tools/conformance/src/toml_subset.rs` makes for the ledger: a data file this project documents and
reads, a subset small enough to state in one place, and no serialisation dependency taken for it.
It is **not** a TOML parser: it accepts the subset and rejects the rest by line.

## The seam, which is the load-bearing decision

**The configuration is read by the caller and handed to `apply` as data**, exactly as `Policy` and
`Budget` are — so RFC 0002 section 5's claim that `apply` opens no path, reads no clock and spawns
no process stays true, and stays a thing this project tests rather than asserts. `archive/config.rs`
turns the file into a `Configuration`; the CLI reads the file in the binary and folds the result
into the `ArchivePlan`. Nothing in the library touches the filesystem.

## What a configuration contributes, and why it changes no pipeline

`A54`–`A60` and the mitigations catalogue name five remedy kinds — `stop`, `discard`, `preserve`,
`derive`, `supply` (the fifth is catalogue section 0.2's, the operator as the source). Two reach a
real conversion this round, and the choice of which is the honest one:

- **`discard`** at a site that is one of `doc/pdf-a-conversion-limits.md` section 3's seven losses
  becomes the `Authorisations` entry the caller would otherwise type — read off `decision.rs`'s
  `REMEDIES` so it cannot disagree with what the converter authorises. A configuration authorising
  `image-smoothing` and `--authorise image-smoothing` convert a document identically.
- **A departure** (ADR 1018).

Every other remedy — `preserve`, `derive`, `supply`, and `discard` at a site whose lossless rewrite
is unbuilt — is **recognised, validated and enumerated but not applied**. Naming one leaves that
site's requirement refused with the sentence it already carries, which names what the remedy waits
on. This is `CLAUDE.md` principle 1 and the round's own rule: *a promise nothing will keep is worse
than a refusal with a sentence*. The report names such sites (`Configuration::unbuilt`), so an
operator sees their intent was read rather than ignored.

So installing a configuration changes no pipeline unless it authorises a built loss or names a
departure — ADR 0954's requirement that `stop` stay every site's default, kept by construction: an
empty configuration and `refuse-any-loss.toml` (which names no site) both authorise nothing.

## Three things the format carries that the drafts did not, from the catalogue's findings

- **A `default` key** that may only be `stop` (section 14 point 9): without it `refuse-any-loss` is
  byte-identical to no file at all, and an operator who has to *show* an auditor what their pipeline
  was told cannot. The reader accepts `default = "stop"` and refuses any other value.
- **A target qualifier** (`doc/rfc/0007` section 4.6): `[site."x".target."4f"]` wins for its target,
  and a row for another target is inert. This is why one profile works with all six targets.
- **A shape qualifier** (section 14 point 1): `[site."x".shape."array"]` reads and validates, so a
  configuration can be written against the distinction between a blend mode as an array and as a bare
  name. No shape-aware *remedy* exists yet, so a shape-qualified row is inert, like a row for another
  target — the format slots the shape in; the remedy that reads it is a later round's.

## Enumerability is a gate, and errors name both

`quorra-transform archive --remedy-sites --to <target>` prints every refusal site the target binds,
generated from the same table the converter decides from (`config::sites`, off `census`). A site
cannot exist undocumented, and a configuration naming a site no requirement is called, a remedy
outside the five-word vocabulary, an `on-failure` chain (`A57` — one alternative, not a chain), or a
target a qualifier does not name is an **error naming both** rather than a quiet `stop` — the
failure mode ADR 0954 exists to remove.

## The profiles are the acceptance test

`doc/profiles/`'s four profiles were written before the reader existed. They are now re-expressed in
the real format and held to it by `crates/pdf-transform/tests/profiles.rs`, which loads each for all
six targets — so a profile cannot drift from the format again. `refuse-any-loss` and
`only-metadata-loss` and `as-if-printed` fit as they stand; `keep-everything` still marks its
`prefer`, `supply` and per-site `keep` lists `NOT-YET-IN-FORMAT`, and its one three-D `on-failure`
chain became a single alternative (`A57`).

## What is left for the next converter round

`derive` and the shared external-tool request/executor (`A54`, `A56` — no offer in the first
version, a warning at the tool-configuration site); `supply`'s machinery (the operator's value into
the file, recorded as theirs); `preserve` by attachment (waits on the 4f/4e writer) and by appended
pages (waits on session 994's authoring amendment, `A58`); and the shape-aware remedies the shape
qualifier is built for. The format carries a `[tool.…]` block past the reader unread, for the
executor to consume.
