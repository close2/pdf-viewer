# 2760 — A page not drawn whole is refused by what the window did not draw

Session 1974, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/fonts/photographed.rs`:
`is_a_fault_of_the_documents_own_stream`, `not_drawn_whole` and its three sentences).
Context: ADRs 2394, 2516 (decision 5), 2618; ADR 2759 is the reader's half; R03 and R08 of
`doc/pdfa-blocking-issues.md`; `doc/questions/A694` items 1 and 3.

## 1. The gate

ADR 2618 bakes into a photograph what the window drew of a page whose own stream is malformed, and
keeps refused what records this reader's gap. ADR 2759 has the reader name two more faults of the
stream's own bytes as their own kinds, `ColourOperands` (§8.6.8) and `MalformedInlineImage` (§8.9.7),
and three more sentences `StreamSyntaxFault` names. The gate admits the two kinds beside
`OperandShortfall` under the wide gate (`as-if-printed`, `state-every-loss`), and the sentences reach
it through `stream_syntax_fault` unchanged. The narrow gate (`as-shown`, 2u, 2a) refuses them all, as
it refuses every fault of the stream (A694 item 1). No profile's choice moves: each photographed page
carries ADR 2394's `text-as-image`, the page's text named in the report and `xmpMM:History`.

## 2. The refusal names its cause

One sentence covered every report the gate keeps, so a census line could not say whether a document
waited on this reader or on its own bytes. It now says which of three it is, from the first report
that holds the page: **a bound this reader sets** (`LimitReached`, `ListBytes`, a `/Contents` part
past `TooLarge` or `TokenTooLong`); **an image or a shading this reader could not build from the
file's bytes** (`Image`, `Shading`, `ShadingBackground`); **a part of `/Contents` the window drew
nothing of** (`Undecodable`, `NotAStream`, `Unreachable`, ADR 2618 decision 4). Everything else —
§9.3.8's knockout, the transparency constructions this reader composites otherwise, an operator the
fence does not name — keeps the sentence it had.

## 3. What each document does now (the seven the census named)

| document | before | now |
|---|---|---|
| `504076` | `sh` on `/` | **freed** at 2b and 4 (`as-if-printed`), at 4 (`state-every-loss`; 2b stops at `metadata/properties-use-known-schemas`, as before) |
| `492484` | `true`, `EI`, an inline image with no `/W` | **freed** at 4 under both; 2b stops at `implementation-limits/values-written-in-content-streams` |
| `150277` | `sc` with 27 operands; then an infinite coordinate on page 57 | the pages to photograph pass the pixels one conversion holds (`PAGE_TOO_LARGE`) |
| `219789` | `Do` with no name, `sc` with 2, 6 and 8 operands | the same pixel bound |
| `427669` | `MAX_OPERANDS` on page 17 | kept: a bound this reader sets |
| `507676` | inline images with no pairs | kept: `R73`'s Flate data and two inline images' data do not decode, and one inline image's samples stop short |
| `poppler-90-0-fuzzed` | `W` the stream ended on | photographed; the output fails `graphics/only-operators-the-base-standard-defines` on a stream the conversion's `/A85` restatement makes decodable |

`427669` and `507676` are the file's own damage — every stream named fails `zlib` as well — and stay
refused by name: a bound cannot tell the file's fault from this reader's, and an image whose data does
not decode cannot either (ADR 2618 decision 4's argument). Widening the gate to them is not this
round's to decide.
