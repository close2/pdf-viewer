# 1952 — A configuration's name is the operator's, and an abbreviated filter is named in full

Session 1558, the archive slot of batch eighty-nine, on `doc/todo/72` section 2's ranking.
Status: **accepted**; sections 1 and 2 **built**, section 3 **built for the identity** and
decided for the rest. Context:
ADRs 0816, 1188, 1234, 1286, 1942; `doc/questions/A48`; `doc/pdf-a-mitigations.md` section 12.

## 1. `optional-content/configuration-names` is a `supply`

ISO 19005-2 section 6.9 (and ISO 19005-4 section 6.10) requires every optional content
configuration to state a `/Name` unique within the file, which its NOTE 1 recommends be chosen to
identify the content of the variant. ISO 32000-2 §8.11.4.3's Table 99 makes the entry an optional
label "suitable for presentation in a user interface". All 45 corpus documents failing the site
(`examples/archive_unbuilt`, 1 450 pdf.js and corpora documents) fail it by stating no name;
none by stating one twice.

**No name is in the file to be found, so the converter does not write one on its own authority.**
`A48`'s line — "state an interpretation the standard defines; never fill in an absence" — rules
out a table default (`Answer::Stated`) here, where the catalogue's entry had argued for one. What
it leaves is the catalogue's own second sentence: the operator states the wording. So the site
joins `SUPPLIABLE`, with one key, `name-template`, which must hold `{n}` exactly once; `{n}` is
the first number from 1 that no configuration's name already is, compared as text the way the
validator compares. The three places ADR 1234's `/AS` removal reaches are the three a name is
written into, after the `/Order` completion and the `/AS` removal so the edits compose. The report
carries one supplied-fact line per name and `xmpMM:History` records the supply, as every `supply`
does. **Refused still**: two configurations stating one name (renaming either rewrites text the
producer did write), and an unnamed configuration written inside a `/Configs` array that is an
object of its own. `as-if-printed`, `as-shown` and `keep-everything` answer it with
`"Configuration {n}"`; the default, as everywhere, is `stop`.

## 2. `file-structure/stream-filters-are-standard`: the abbreviations are spelled out

The 26 failing streams of the census name `A85` (9), `XXXDecode` (7), `BrotliDecode` (4), `AHx` (3),
`Fl` (2) and `DCTecode` (1). The first, fourth and fifth are §8.9.7's Table 92 abbreviations,
which that subclause says "are valid only in inline images"; `pdf_syntax::filter` decodes each as
the filter it abbreviates in any stream. So the page was drawn through that filter, and the rewrite
(`Rewrite::FilterNamesSpelledOut`, `Answer::Stated`) writes the full name and leaves the bytes
untouched — re-encoding is unnecessary where the encoding already is the standard one. The
interpretation stated is Table 92's own. **A name that is no abbreviation is refused** by
`sites::abbreviated_filters` with a sentence: this tree does not decode it, so what the stream
holds is unknown. Brotli is not an ISO 32000-2 filter; deciding it would be a decoder's work.

## 3. `graphics/no-transfer-function-in-a-graphics-state`, per profile

§10.5 is drawn (`doc/todo/13`), so removing `/TR` moves the colours this reader shows wherever the
function is not the identity. Three answers: (a) **every `/TR` in the file the name `Identity`** — 26 of the
census's 27 documents — removes with nothing drawn moving, at any configuration
(`Rewrite::IdentityTransferFunctionRemoved`, `Answer::Stated`: Table 53 leaves the initial
function device dependent, and it is the identity on this reader's device); **built**, and a
shortened finding list or a state written inside another object stays refused; (b) a real function under **`as-if-printed`**: §8.4.1's Table 53 makes it compensation
"for nonlinear response in a particular output device", which a press replaces with its own, so `discard`
with the function kept as data in `xmpMM:History`; (c) under **`as-shown`**: the window drew the
function, so baking it in is the only lossless answer, and it rewrites every colour operand, image
sample and shading of every mark under that state — content, behind ADR 0816's fence — so the
profile stops there. Owed: (b), whose one corpus witness is `issue6931_reduced.pdf`, and `/TR2`.
