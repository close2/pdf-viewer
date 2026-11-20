# 3290 — A deflate stream without RFC 1950's framing is drawn and reported as `Damage::Unframed`

Session 2147. Status: accepted. Context: ISO 32000-2 §7.4.4.1; ADR 0343 (damage keeps its bytes), ADR 0836
(`CheckValue`), ADR 2438 (`ForeignCharacter`, the first damage in a chain), ADR 3273 (a parameter refused by
name); trap 5.

## The defect

`FlateDecode` falls back to raw deflate when RFC 1950's two-byte header fails its check, and the fallback
reported a whole decode. §7.4.4.1 states the format — the Flate method "is fully defined in Internet RFC 1950 ,
and Internet RFC 1951 " — and bare deflate is not RFC 1950's stream: no header, no Adler-32.

## Decision

- `Damage::Unframed`: decoded as raw deflate to its final block (or to a flush, ADR 0744). Both routes say it:
  `flate` over a whole buffer and the pump's `Inflate::ended`. A raw decode that stops short keeps
  `Truncated` or `Corrupt`, the graver report. `stops_short` is false.
- **The bytes are drawn.** ADR 3273 refused a parameter because no byte of its output is the file's; here every
  byte the deflate data describes came out, which is the robustness half of the question, so the shape is
  damage with the data kept, `Decoded::damage`. Content streams take the existing report
  (`ContentIssue::Damaged`, `DamagedContentStream`) and are drawn as before.
- `Damage::questions_the_bytes` is false for `Unframed` alone. Every consumer that refused damaged bytes
  because they were damaged asks it instead: font programs (`pdf-font`), ICC profiles (`pdf-colour` and
  `pdf-transform`'s archive), embedded media and SWF bodies, stream re-encoding (`serialize`,
  `rewrite/streams`, `optimize_images`), a default space's content stream; an object stream's last member
  keeps its stated end. A stream drawn before the report existed is drawn after it.
- `Damage::in_a_chain`: the earliest damage is the chain's (ADR 2438) except `Unframed`, which changed no
  byte and gives way to any later stage's damage, so a prefix is never reported as a whole decode.

## Cost

Documents whose streams lack the header gain a report line per such content stream; what they draw does not
change. The census (`damaged_stream_census`) counts the new variant in its own column.
