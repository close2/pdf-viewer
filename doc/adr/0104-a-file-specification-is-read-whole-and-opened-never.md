# ADR 0104 — A file specification is read whole and opened never

Status: accepted, 2026-08-01.

## Context

Seven of the ledger's 48 `reported` rows were §7.11's, and their notes all pointed at one
sentence:

> A file specification shall refer to a file external to the PDF file or to a file embedded
> within the referring PDF file … The file is considered external to the PDF file in either
> case.

`CLAUDE.md` principle 3 gives the renderer no filesystem and no network, so *following* a file
specification is refused by architecture. The rows read that as the end of the matter, and it is
not: **refusing to open a file and being unable to name it are different things**, and this tree
was doing the second while claiming the first.

Three separate places read a specification, ad hoc, each a few lines: `attachment.rs` twice and
`action.rs` twice more. Each did `/UF` then `/F`, and each decoded both with `text_string`.

## The rule the code was breaking

§7.11.2.1 ends with a sentence the ledger row had already quoted — before anybody implemented
the clause — and which the code contradicted:

> The component substrings shall be stored as bytes and shall be passed to the operating system
> without interpretation or conversion of any sort.

Table 43 makes `/UF` a **text string** (§7.9.2.2) and `/F` a **byte string**. They are not two
spellings of one value: `/F` is whatever bytes the producing platform used for the name, and
decoding it as `PDFDocEncoding` corrupts a file name in every locale where that is not what the
bytes are. The row's own note said so — "a reader that decoded these bytes as text would corrupt
a file name in every locale where they are not UTF-8" — and the three call sites did it anyway.

That is a smaller instance of the shape ADR 0101 found: **a warning written into a ledger note
before the code exists is a warning nobody reads when the code arrives.**

## Decision

`crates/pdf-model/src/file_spec.rs`: §7.11 read whole, with a filesystem nowhere in it.

- **§7.11.1's two forms.** `FileSpec::parse` takes a string or a dictionary and answers `None`
  for anything else.
- **Table 43.** `/FS`, `/F`, `/UF`, the three deprecated platform keys, `/ID`, `/V`, `/Desc`, and
  the *presence* of `/EF`. `/EF`, `/RF`, `/AFRelationship` and `/CI` stay with the modules that
  own what they point at — the clause's own division, not a convenience.
- **§7.11.2.1's components, as bytes.** Split on unescaped SOLIDUS, `\/` un-escaped, empty
  components kept because "[a]ny of the components may be empty". `FileSpec::display_name` is
  now the *one* place a byte string becomes text, and its doc comment says that is a display
  decision.
- **§7.11.2.2's resolution.** The clause states its answers rather than only its rules, so both
  of its EXAMPLEs are tests. A `..` with nothing to cancel is **kept** — a documented choice,
  because the clause cancels one only "when the component immediately preceding . . is not
  another . .", and dropping it would silently move the target to the root.
- **§7.11.2.2's restriction on relative URL specifications**, which is *a security rule wearing
  a syntax rule's clothes*: "[t]he scheme, network location/login, fragment identifier, query
  information, and parameter sections shall not be allowed". A relative specification that
  smuggles in an authority resolves against a different host from the document's. Nothing here
  fetches a URL; what the check defends is the day something does.
- **§7.11.5's URL form**, which `components` refuses to split — a URL is not a path, and
  applying the separator rule to one invents components out of its scheme.

The three ad-hoc readers now call it. `action.rs`'s `file_specification` is one line.

## What it is not

It produces no path for an operating system, and there is deliberately no function that would.
Reading a specification is not fetching one, which is the same position §12.6.4.8's URI action
takes: resolve, print, decline (ADR 0070).

## Consequences

`reported` falls **48 → 41**, `implemented` rises 358 → 364. Nine unit tests, six of them the
clause's own examples and answers. No gate moved — 840 agreeing, 65 contradicted, 90 incomplete
— which is the honest shape of this work: nothing on a page changes when a viewer stops
mis-decoding a file name it was never going to open.

The durable half is the type. **`FileSpec::bytes` is `Vec<u8>` and cannot be handed to anything
expecting text without the conversion being visible**, which is what stops the fourth call site
from repeating the first three.
