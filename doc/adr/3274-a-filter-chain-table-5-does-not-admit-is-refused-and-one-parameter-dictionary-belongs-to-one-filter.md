# 3274 — A filter chain Table 5 does not admit is refused, and one parameter dictionary belongs to one filter

Status: accepted. Session 2143. Context: ISO 32000-2 §7.3.8.2 Table 5, §7.4.1, Table 6, §7.5.8.2; audit 2142 findings
B01 and D01; ADR 3273; trap 5.

## The defect

`Document::filter_chain` kept only the `/Filter` elements that resolved to names, and a `/Filter` neither a name nor an
array was an empty chain: `[/ASCIIHexDecode 7]` decoded as the hex stage alone and `/Filter 7` handed the raw bytes back,
both as whole decodes, and the renumbered chain gave every later stage the parameters of the one before it.
`decode_parms` (and `xref.rs`'s own `parms_for`) gave a single `/DecodeParms` dictionary to every stage of a longer
chain, silently, and with B05 that ran a predictor after every stage.

## The clause

Table 5: `/Filter` is "\[t]he name, or an array of zero, one or several names"; `/DecodeParms` is a dictionary "or an
array of such dictionaries, used by the filters specified by Filter, respectively", and "\[i]f there are multiple filters
and any of the filters has parameters set to nondefault values, DecodeParms shall be an array". So a single dictionary
over a longer chain is malformed, and the standard leaves its reading to the reader.

## Decision

- One reading, `pdf_syntax::document::chain_of`, used by the whole route, the window, the image route, the inline-image
  extent, the writer's recompression, the crypt-filter lookup and the cross-reference stream (which resolves nothing,
  §7.5.8.2). It answers `MalformedChain`, carried as `StreamRefusal::Chain`, for a `/Filter` neither name nor array, a
  `/Filter` element not a name, a `/DecodeParms` neither dictionary, array nor `null`, and an array element neither
  dictionary nor `null`.
- **A single dictionary is the dictionary of the one filter that can take it.** For a one-filter chain that is the
  filter; for a longer one it is the one filter Table 6's Parameters column says yes to, and no other stage is given it;
  where two or more take parameters the file has not said which, and the chain is refused (`SharedParms`). This keeps
  `[/ASCII85Decode /FlateDecode]` with `<< /Predictor 12 … >>` decoding as its producer meant, which is the only reading
  under which the dictionary parameterises anything, and refuses the case where any reading is a guess.
- An array shorter than the chain leaves the stages past its end their defaults, as `null` does; this is unchanged and
  is a choice, recorded here.

## Consequences

The audit test for B01 passes, both assertions. The writer already refused a shared dictionary over a longer chain
(`parms_objects`), so the two now agree.
