# ADR 0140 — The `CMap`s a processor is said to have

Status: accepted, 2026-08-02. Session 156. The third and last item of the handover's
third-party-data list, and the second time this project has met a `shall` by carrying data rather
than by writing code — ADR 0133 was the first.

## The clause, and what it says about where the data is

ISO 32000-2 §9.7.5.2 lists some seventy `CMap` names in Table 116 and states the obligation
without hedging:

> A PDF processor shall support Adobe-CNS1-7, Adobe-GB1-5, Adobe-Japan1-7 and Adobe-KR-9 character
> collections.

and it is equally plain that the mappings are not in the standard:

> The CMap programs that define the predefined CMaps are available through a variety of online
> sources.

Until this session a font naming one was refused and reported — thirteen fonts across the pdf.js
corpus. That was honest, it was trap 5 working as intended, and it was not the clause. **A
requirement whose answer is data is not met by reporting that you have no data.**

## What was decided, and what was already decided

The licence question was answered in the hundred-and-thirtieth session and re-verified in the
hundred-and-forty-eighth: Adobe's `CMap` files are BSD-3-Clause, and the trap is the *other* half
of `poppler-data` — `cidToUnicode`, `nameToUnicode`, `unicodeMap` are Glyph & Cog's under
GPL-2-or-3 and none of them is here. What this session added is the work, in the shape the
handover specified.

`data/cmaps/` holds all 239 files Adobe publishes for the six character collections, taken from
Arch's `poppler-data 0.4.12-2`, with `COPYING.adobe` beside them verbatim and a digest per file.
**All of them rather than Table 116's list**, for one reason that is not laziness: the `usecmap`
chains inside them are then transitively closed by construction, rather than by a pruning rule
somebody would have to keep right as the files change.

`crates/pdf-font/build.rs` deflates each file on its own into a 3.9 MB blob and emits an index of
`(name, offset, packed, plain)`. `pdf_font::predefined::cmap` binary-searches the index, inflates
one entry, follows its `usecmap` by name, and memoises the result. **Nothing is decompressed at
startup**, which is `CLAUDE.md`'s rule for compiled-in data: `callgrind_interpret` on page 101 of
ISO 32000-2 is 2 133.2 M against session 153's 2 137.7 M — a repeat, and the right answer, because
that page names no predefined `CMap`.

**Adobe's bytes go in unconverted.** `CMap::parse` already reads this syntax, because §9.7.5.4's
embedded `CMap` streams are written in it; the `-UCS2` files state `/CMapType 2` and use
`beginbfchar`, which `ToUnicode::parse` already reads because that is §9.10.3's form. So there is
no converter, no second format to keep in step, and no opportunity to mistranslate a mapping while
compacting one. The compacted alternative — `hayro`'s 250 KB brotli blob — buys 3.6 MB of binary
and costs all three.

## §9.10.2's third method came with it, and it is where the gates moved

The clause's third route to a Unicode value was `partial` for the same reason and became reachable
the moment the data arrived:

> c. Construct a second CMap name by concatenating the registry and ordering obtained in step (b)
> in the format registry -ordering -UCS2 (for example, Adobe -Japan1 -UCS2).

`pdf_font::predefined::cid_to_unicode` reads that file; `LoadedFont::text` consults it in the
clause's own position, after `/ToUnicode` and before the permission §9.10.2 grants where its
methods fail. It applies to an **embedded** composite font as much as to a substituted one,
because the collection says what a CID means whether or not the program defining it is present.

The two methods are keyed differently — `/ToUnicode` by character code, the collection's table by
CID — which is why `Meaning` is an enum rather than one table. Folding the second into the first
would mean enumerating every code a `CMap` defines, and a UTF-32 codespace is not a finite thing
to do at load time.

## What the gates say

| gate | before | after |
|---|---|---|
| corpus documents drawing incompletely | 91 | **76** |
| oracle pages we call complete | 1665 | **1681** |
| oracle pages agreeing | 836 | **845** |
| oracle pages contradicted | 70 | 72 |
| text readback | 97.9% | **98.2%** |
| documents below the 0.90 text floor | 42 | **36** |

**The two new contradicted pages are a silence ending, not a defect**, and the distinction is one
this project has had to make before. `noembed-eucjp.pdf` and `noembed-sjis.pdf` are one line of
あいうえお in a non-embedded Japanese font. They used to report a `CMap` this tree did not have, so
the oracle did not judge them; now they draw, and they draw the same five kana in the same places
as all three references — in a different face, at worst tile 18.98 against a bound of 5.00. That
is `CONTRADICTED_SUBSTITUTED_FONT`'s existing argument, and ADR 0133's five net pages are the
precedent. Reaching for a report to make a contradiction go away is what trap 5 forbids.

## What this does not fix

The 27 documents whose composite fonts name an `Identity` ordering. Their codes are indices into a
font nobody supplied, and no table can say what an index into an absent font means — the handover
said so before the data arrived and it is still true after.

## The habits

- **A capability recorded as blocked on a decision outlives the decision.** This row said
  vendoring was "a licensing decision rather than a coding one" for a hundred and fifty sessions.
  The decision was taken in the hundred-and-thirtieth and written into the handover; the *row*
  never heard, because nothing fires when a stated blocker expires. That is ADR 0108's regular
  expression finding its fourth instance, and the first where the blocker was this project's own.
- **A test that pins a refusal is a test that must be rewritten when the refusal ends**, and it
  will not fail helpfully: `a_predefined_cmap_is_refused_by_name` failed with "a predefined CMap
  this tree has no data for must be refused", which reads like a regression. It is now
  `a_predefined_cmap_is_resolved_by_name` and asserts the thing that says the `CMap` was really
  consulted — that a *two-byte* code comes back, which an unconsulted `CMap` could not produce.
- **A vendored set needs a check the licence cannot give you.** `cargo deny` reads Cargo metadata
  and cannot see 12 MB of data. `notices.rs` checks that each of the 239 files still carries
  Adobe's notice and disclaimer inline and still hashes to `SHA256SUMS`; the count in `NOTICE` is
  asserted against the count on disk, because 239 file names in a notice is a page nobody reads
  and a number that has drifted is a lie anybody can check.
