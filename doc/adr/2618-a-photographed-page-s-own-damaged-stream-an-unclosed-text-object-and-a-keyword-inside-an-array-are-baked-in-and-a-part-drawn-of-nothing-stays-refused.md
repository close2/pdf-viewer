# 2618 — A photographed page's own damaged stream, an unclosed text object and a keyword inside an array are baked in, and a part of `/Contents` drawn of nothing stays refused

Status: implemented. Session 1892, the second archive slot of batch 130; ADR 2619 is its checker's half; Q768 unused.
Reads the gate ADR 2516 decision 5 drew under the owner's answer A694 item 3, report kind by
report kind, for the pages `as-if-printed` photographs.

## Context

ADR 2516 decision 5 keeps the test that a photographed page is one the window drew whole, and
bakes in what A694 item 3 calls bad input: "if the input is bad we will never be able to get
"correct" input. we will just bake something into the pdf representing the state we currently show
the pdf". It names the faults of the page's own stream it admits (an operator short of its operands,
a path with no current point, a matrix with no inverse, an undefined resource name, a stream damaged
part-way, a soft mask the clause does not describe) and keeps refused what records this reader's gap
on good input. ADR 0816's fence is the reason a photograph is taken at all: the converter edits no
producer's content stream, and a photograph invents no mark the window did not draw from the file.

`fonts/photographed.rs` built "a stream damaged part-way" as `DamagedContentStream` alone, the
variant for a stream reached by resource name, and missed `Content { issue: Damaged }`, the same
damage in the page's own `/Contents` (Table 31). Two sentences the interpreter reports as
`Unsupported::Operator`, "BT without ET" and "<keyword> inside an array", fell to the arm for an
operator no standard defines and were refused because the fence's operator list does not hold them.
Nineteen documents of the corpora were held at 2b by this, `050734`, `064034` and `504076` among them.

## The decision, kind by kind

1. **`Content { Damaged }`, every `Damage`, is baked in**, beside `DamagedContentStream`. §7.4.1 has
   the reader invoke the stream's filter, and §7.8.2 has it act on the objects that came out in
   order. For `Truncated` and `Corrupt` the bytes drawn are the prefix the producer's compressor
   wrote; for `CheckValue` every byte the data describes; for `ForeignCharacter` the pairs past the
   character, which `pdf_syntax::Damage` says are not certainly the producer's but are what the
   file's bytes decode to. The window adds no mark the decoded bytes do not state, so the photograph
   is the file as far as it reaches, which is A694 item 3's bad input.
2. **"BT without ET" is baked in.** The stream ended inside a text object (§9.4.1, Table 105). Each
   glyph shown before the end was painted when it was shown (§7.8.2's order), and §9.3.6 makes the
   clip of modes 4 to 7 at the `ET` that ends the text object, so a text object that never ended
   clips nothing. The window's picture is the standard's own reading of the operators as far as
   they go: the recovery is the standard's rule, not a guess.
3. **A keyword inside an array is baked in.** §7.3.6 admits only objects into an array and states
   no recovery; the interpreter steps over the keyword and keeps the array, so the keyword draws
   nothing and every mark drawn is one the stream's objects state. This is the class decision 5
   already admits — an operator short of its operands is equally a recovery the standard does not
   state — and the brief's hypothesis that it stays fenced as a guess did not hold: A694 item 3
   decides bad input, and refusing here would refuse the archive of a file whose every drawn mark
   is its own.
4. **A part of `/Contents` the window drew nothing of stays refused**: `Undecodable` (no byte came
   out, which a filter this reader does not implement also produces), `TooLarge` and `TokenTooLong`
   (a bound of this reader's), `Unreachable` (an object this reader could not reach, which may be one its
   parser could not read), and `NotAStream` (content the producer named and the
   file does not hold, which a photograph would carry with no named loss; A727's `page-not-held` is
   that construction for a page, and one for a stream would be its own decision).
5. **The classification is pdf-model's.** `Unsupported::stream_syntax_fault` names the two
   sentences as `StreamSyntaxFault` from the one constant each that the interpreter writes them
   with, so the gate does not match prose. §7.2.3 makes white space separate syntactic constructs,
   so neither sentence can be the spelling of an operator a stream wrote.

Every other kind is as ADR 2516 left it, and the narrow gate (`as-shown`, 2u, 2a) refuses all of
these as before.

## Consequences

The pages these kinds held reach the next gate; a document that still refuses names the requirement
that holds it. Two documents the gate frees were at first written as
conforming by this tree's checker and rejected by veraPDF on pages that were not photographed: an
operator veraPDF reads where the survey read a salvaged number, and a font program whose data stops
short. ADR 2619 is that checker's half: `509284` now photographs the five pages whose runs §7.3.3
spells as no number and passes veraPDF at both targets, and `435321` refuses on its cut program.
With both, corpora `as-if-printed` converts 411 → 419 at 2b and 416 → 427 at 4, against HEAD's.
The census against HEAD's is in the session's record. The photograph is still the
window's drawing at 300 dpi, with the page's text named in the report and `xmpMM:History`.
