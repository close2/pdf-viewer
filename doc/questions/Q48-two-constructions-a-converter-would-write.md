# Q48 — Two constructions proposed on 2026-09-07, neither yet ratified

Source: `doc/pdf-a-conversion-limits.md` §10.1 and §2.2. Both were proposed by the assistant while
answering the owner's questions, and neither has been decided.
Status: **open** — answered when `A48-two-constructions-a-converter-would-write.md` exists beside this file.

## Why it needs the owner

Each writes into the output a construct the producer did not write. Neither invents a *mark*, so
neither is obviously outside ADR 0816's fence — and neither is obviously inside it, which is why
they are here rather than in the code.

**1. A DeviceN `/DefaultCMYK` where no CMYK profile is available.** ISO 19005-2 §6.2.4.3 NOTE 2
makes a DeviceN-based `DefaultCMYK` device independent, and ISO 32000-2 §10.4.2.5 states the
CMYK→RGB transform outright, so the tint transform is the standard's rather than ours. It is
non-destructive: the content stream keeps the producer's CMYK numbers and the default only says
how to read them. Its cost is stated by the standard in the same place — §10.4.2.1 calls these
algorithms crude approximations — and it is visible on a photograph separated for press.
Without it, any document containing `DeviceCMYK` is unconvertible unless the user supplies a press
profile.

**2. An empty glyph where a substituted font would otherwise reference `.notdef`.** Both parts
forbid a text-showing operator referencing `.notdef` (ISO 19005-2 §6.2.11.8, ISO 19005-4
§6.2.10.9). Where the converter is building the substitute program anyway it chooses the mapping,
so it can map such a code to a real but empty glyph: the page looks exactly as it did and no
`.notdef` is referenced. The clause's NOTE says the prohibition exists because `.notdef` carries
no semantic value, and an empty glyph carries none either — where mapping the code to a plausible
letter would give it a false one.

## What the tree does meanwhile

Nothing built. `doc/pdf-a-conversion-limits.md` documents both as proposals.

## Recommendation

Allow both, report both per document, and record both in `xmpMM:History` naming the clause used —
§10.4.2.5 for the first, the substitution for the second. The line they share is the one worth
ratifying explicitly: **state an interpretation the standard defines; never fill in an absence.**
