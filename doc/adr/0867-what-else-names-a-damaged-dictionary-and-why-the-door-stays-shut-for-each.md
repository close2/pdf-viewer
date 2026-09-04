# ADR 0867 — What else names a damaged dictionary, and why the door stays shut for all 327 of them

Status: accepted. Session 912.
Clauses: ISO 32000-2 §7.7.3.2 and §7.7.3.4 (the page tree and its inheritance), §7.8.3 and Table 34
(resource dictionaries), §7.10.2 (sampled functions), §9.6.5.1 with Table 112 and §9.6.5.2 (encodings and their
fallbacks), §9.7.3 (`/CIDSystemInfo`), §9.8 with Table 122 (font descriptors), §12.3.3 (outline
items), §14.7.4.3 with Table 358 (`/Pg`), §7.3.8.2 (a stream's extent).
Code: none — this ADR is the reason a change was *not* made, and the instrument that made the
question askable.
Measurement: `crates/pdf-model/examples/damaged_dictionary_consumers.rs`.
Beside ADR 0866, which is the one key that does come through.

## Why this is a decision and not a note

ADR 0784 gave §7.3.7's damaged-dictionary door one consumer and said nothing about the rest, so for
fifty-two sessions "the door stays shut" was a *silence* rather than a decision — the shape
`doc/todo/02` section 1 warns about, where a refusal outlives its reason because nobody wrote the
reason down. ADR 0866 opens it for one key on a test. This ADR is the same test applied to
everything else the corpora actually name, so that the next round to want a second key finds an
argument rather than an absence.

## The instrument, and the question nobody had asked

`Pages`' recovery reaches a damaged object by the object's **own** `/Type /Page` declaration,
because the tree that would have named it is what has failed. That is the hardest case, and it hid
an easier one: **a damaged object is often named by a reference out of an object that parses
whole.** §7.3.10 makes that reference the file's own statement of what the object is for, made in
bytes the damage did not reach — so it is an identity a consumer could rest on, and the *key* it is
stated under names the clause that would have to answer ADR 0866's first condition.

`examples/damaged_dictionary_consumers` counts exactly that: for each document, the damaged
dictionaries its bytes hold, and each reference to one out of an object `Document::get` reads whole
where `get` on the target answers §7.3.10's null.

## The population

Over **90 535 documents** — `corpus-cache/tika-issue-tracker`, `corpus-cache/openpreserve`,
`doc/pdf.js/test/pdfs`, the four `doc/corpora/` submodules, and all 65 944 of
`corpus-cache/safedocs/cc-main-2021-31`, named because a claim of absence is denominated or it is
nothing (ADR 0758):

| | trackers, `openpreserve`, `doc/pdf.js`, `doc/corpora` | `cc-main-2021-31` |
|---|---|---|
| documents read / opened | 24 591 / 24 407 | 65 944 / 65 720 |
| documents holding a damaged dictionary | 287 | 24 |
| damaged dictionaries in all | 885 | 68 |
| documents where a whole object names one | 97 | 2 |

**328 such references over 58 distinct keys.** The shape is round 908's finding again, one level
up: an issue tracker states this and the open web barely does — 885 damaged dictionaries in 24 591
files against 68 in 65 944 — because a crawled page is a file a producer shipped and a web server
served, and a tracker attachment is a file somebody filed *because* a program choked on it.

The keys, by count:

`/Pg` 59, `/Parent` 43, `/FontDescriptor` 33, `/Pages` 20, `/CIDSystemInfo` 15, `/Helv` 14,
`/Encoding` 12, `/Contents` 10, `/G` 7, `/Outlines` 7, `/R15` 7, `/R17` 6, `/Function` 5,
`/Helvetica_00` 5, `/P` 5, `/Prev` 5, `/First` 4, `/Shading` 4, `/AcroForm` 3,
`/DestOutputProfile` 3, `/Last` 3, `/Next` 3, `/PTEX.InfoDict` 3, `/R25` 3, `/R29` 3, `/Resources` 3,
`/SMask` 3, `/A` 2, `/Activation` 2, `/FontFile2` 2, `/Kids` 2, `/Length` 2, `/R10` 2, `/R11` 2,
`/R31` 2, `/ToUnicode` 2, and twenty-two keys once each including **`/CharProcs`**, `/Root`,
`/Metadata`, `/EmbeddedFiles`, `/FontFile3`, `/Colorants`, `/ViewerPreferences`, and the four
garbled ones `/Firct`, `/Pzrent`, `/Functioi`, `/unction` — which are the same defect one object
over, a key whose own bytes were altered.

## The test, applied

ADR 0866's first condition is the one that decides every row: **does the consumer's clause state
that an absent entry draws nothing?** Not *may this reader cope* — every one of these can be coped
with — but *does the standard already say what the residue is*, so that taking a subset substitutes
nothing.

- **The page tree — `/Parent` 43, `/Pages` 20, `/Kids` 2, `/Root` 1, `/Pg` 59.** No. §7.7.3.4 makes
  `/Resources`, `/MediaBox`, `/CropBox` and `/Rotate` inheritable, so an entry the damage took is
  read as an ancestor's or, failing that, as ADR 0389's chosen sheet — a value this reader picked.
  That is exactly the case ADR 0784 already covers, and it covers it *by the object's own
  declaration* with a report about the substitution, which is a stronger guard than a reference.
  `/Pg` is Table 358's structure-element page and resolves to the same objects; §14.7 asks nothing a
  page tree does not.
- **Font descriptors — `/FontDescriptor` 33, `/FontFile2` 2, `/FontFile3` 1.** No, twice over.
  Table 122's entries are the metrics a processor builds a **substitute** from when there is no
  program (§9.8.1 and §9.6.2.2's fourteen), so a missing `/Flags`, `/ItalicAngle` or `/MissingWidth`
  changes what is drawn rather than removing it. And `/FontFile2` and `/FontFile3` name streams,
  which is the next bullet and also ADR 0836's refusal.
- **Streams — `/Contents` 10, `/Length` 2, `/Metadata` 1, `/ToUnicode` 2, `/DestOutputProfile` 3,
  `/SMask` 3.** Structurally out of reach before the clause is even asked: `Document::damaged_dictionary`
  "parses no stream data", because a damaged dictionary never reaches its `stream` keyword, and
  §7.3.8.2's `/Length` is one of the entries the damage is likeliest to have taken. There is nothing
  behind the door for these.
- **Encodings — `/Encoding` 12.** No, and it is the sharpest refusal on the list. §9.6.5.2 makes an
  `/Encoding` entry an *override* of a Type 1 font's own mapping, and Table 112's `/BaseEncoding`
  cell states the fallback where it says less — "[i]f this entry is absent, the Differences entry
  shall describe differences from a default base encoding", which for an embedded program is the
  program's built-in one.
  So a `/Differences` array the damage cut short does not lose glyphs — it hands the codes past the
  cut to a *different* name and draws another glyph in the producer's place. ADR 0106's archetype.
- **Character collections — `/CIDSystemInfo` 15.** No. §9.7.3 makes `/Registry`, `/Ordering` and
  `/Supplement` the name of a collection, and §9.7.5.2's CMap is chosen by it; a partial one names a
  different collection, which selects different glyphs.
- **Functions and shadings — `/Function` 5, `/unction` 1, `/Functioi` 1, `/Shading` 4.** No. Table 38
  makes `/Domain` required and clamping, and §7.10.2 adds `/Size`, `/BitsPerSample` and `/Encode`;
  a missing `/Domain` does not remove a value, it changes every value the function returns.
- **Outlines — `/Outlines` 7, `/First` 4, `/Last` 3, `/Next` 3, `/Prev` 5.** No, for a reason worth
  keeping because it is not the usual one. An outline item's residue *is* close to an omission —
  §12.3.3 makes `/Title` required and an item without one has nothing to show — but the chain
  entries are the tree itself, so a prefix that lost `/Next` loses **every sibling after it**, not
  its own row. What the damage costs is other objects' rows, which is neither an omission the clause
  defines nor a thing this consumer could name.
- **Resource dictionaries — `/Resources` 3, and the named-resource keys `/Helv` 14, `/G` 7,
  `/R15` 7, `/R17` 6, `/Helvetica_00` 5, `/R25` 3, `/R29` 3, `/R10` 2, `/R11` 2, `/R31` 2,
  `/Arial-BoldMT` 1, `/F2` 1, `/im6664` 1, `/R8` 1, `/R;1` 1.** Not one question but one per Table 34
  entry, and §7.8.3 itself states no outcome for a name it does not carry — each operator's clause
  does. So a round wanting this owes the same three conditions **per resource type**, and the
  named-resource rows are not a `/Resources` population at all: each is a font, an `XObject`, a
  pattern or a shading whose own clause is one of the bullets above.
- **The long tail — `/AcroForm` 3, `/A` 2, `/P` 5, `/V` 1, `/F` 1, `/Activation` 2,
  `/Deactivation` 1, `/Assets` 1, `/EmbeddedFiles` 1, `/URLS` 1, `/Params` 1, `/Colorants` 1,
  `/ViewerPreferences` 1, `/PTEX.InfoDict` 3.** Nothing here draws a mark on a page by itself, and
  `/PTEX.InfoDict` is not a key this standard defines at all. Left, with no round having a witness.

## What this decides

**The door stays shut for every key but `/CharProcs`**, and the reason is on the record per family
rather than by omission. A future round that wants a second key answers ADR 0866's three conditions
for it — beginning with a sentence of *its* clause that states what an absent entry means — and
re-runs this census, which is what makes the population a number rather than an impression.

**And a rule the instrument earned**: a claim about which objects a recovery could reach is a claim
about *references*, not about object numbers, and nothing had counted them. The census is cheap
(34 s over 65 944 documents, 1.61 GiB peak) and is the thing to run before the next such argument.
