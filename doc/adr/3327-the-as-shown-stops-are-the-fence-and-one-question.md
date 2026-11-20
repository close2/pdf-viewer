# 3327 — The `as-shown` stops are the content-stream fence, and one of them is a question

Session 2156. Status: accepted. Context: `doc/questions/A694` item 1, `doc/questions/A50`, ADR 0816.

`doc/questions/A694` item 1 keeps R08's and R03's stops under `as-shown`; the brief asked which of them
an elegant remedy frees without a photograph. Read reason by reason over section 9's census:

- R03's 42 at 6.2.2 (an operator no standard defines), its 3 at bound names that are not UTF-8 and its
  odd-length hexadecimal string are bytes of the producer's content stream; every remedy is an edit
  there, the fence ADR 0816 draws, and A50's closed list admits none of them.
- R08's rows are fonts a reader cannot draw without this viewer's layout or a photograph (no
  descriptor, an Adobe-Identity font no `ToUnicode` names, a code the encoding names no glyph for); a
  face embedded for them would be a mapping of this reader's choosing, which A694 item 1 answered.
- `graphics/inline-image-interpolation-is-off` (two pdf.js documents, their only stop) differs from
  R02's `/Interpolate`, which both profiles already discard with `image-smoothing` named, only in where
  the key is written. That is the owner's to admit: `doc/questions/Q978`.

Nothing is built for `as-shown` here.
