# 2690 — A spot colourant defined only in its DeviceN space is given that space's Separation

Session 1936, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/spot_separation.rs`: `Derived`,
`restricted` and the refusal sentences; `archive/sites.rs`: `colorant_entries`,
`pages_inheriting`, `place_in_devicen`; `archive/rewrite/values.rs`: `write_colorants`;
`archive/prepare.rs`: the spare numbers and `Owed::written`).
Context: ADR 2490 (R12: "its Separation derived from the producer's DeviceN space, the tint
transform restricted to that colourant", loss none for a PostScript calculator function, both
shipped profiles), ADR 0965 (why a sampled restriction is an approximation), ADR 0973 (the file's
own Separation array is the entry where it states one), ADR 0816.
Clauses: ISO 32000-2 §8.6.6.4, §8.6.6.5, §7.10.1, §7.10.5; ISO 19005-2 section 6.2.4.4,
ISO 19005-4 section 6.2.4.4.

## The premise, checked

R12's seven pdf.js documents: `colorspace_atan`, `_cos` and `_sin` (`DeviceN [/X /Y /Z]` over
`DeviceRGB`, Type 4), `issue17065` (`[/L /A /B]` over `CalRGB`, Type 4, the space written twice,
once in the page tree node's own resources and once in a shading pattern), `issue9940`
(`[/IBM /None /None /None]` over `CalRGB`, Type 4, an empty attributes dictionary written as an
object of its own), `postscript_type4_many_outputs` (`[/c0 … /c8]` over `DeviceCMYK`, Type 4) and
`devicen` (`[/Red /Green /Blue]` over `ICCBased`, **Type 0**). None is `NChannel`. The census
names R12 as each one's only refusal under both shipped profiles at both parts.

## The decision

§8.6.6.5 says what the entry is: "the alternate colour space and tint transformation function of a
Separation colour space describe the appearance of that colourant alone, whereas those of a DeviceN
colour space describe only the appearance of its colourants in combination." That colourant alone
is the combination with every other tint zero. So the `Separation` written names the colourant,
copies the space's alternate as the file wrote it, and has as its transform a new Type 4 stream:
the producer's program with operands pushed after its opening brace — the fixed values below the
input's position, `k+1 -1 roll`, the fixed values above it. Each fixed value is zero clipped to
that input's `Domain` (§7.10.1 clips before the function runs), written as a real because
§7.10.5's operators tell integers from reals; the `Domain` is the producer's for that input and
the `Range` the producer's own. The function is therefore the producer's, evaluated exactly where
the producer's space is evaluated. A one-component space's own transform is the entry as it stands.

- **No loss**, so it stays `Mechanical`: every profile, `no-data-loss` included, takes it, and the
  entry is informative for a plain `DeviceN` ("may be useful to some PDF processors"). Rendered
  with and without the entries, every output page of the six documents is pixel-identical.
- **One array per colourant across the file**, the same object wherever it is written, as section
  6.2.4.4's second rule requires; two spaces that would derive different ones are refused.
- An attributes or `Colorants` dictionary written as its own object is copied into the space
  rather than written into, since other spaces may share it (§8.6.6.5 NOTE 7). A page that
  inherits a node's direct resources is given the node's entries in the copy `PageResources`
  writes for it.

## Refused by name

A Type 0 transform (`devicen.pdf`): its restriction is a resampling, kept refused until its loss is
named (ADR 2490), and no owner word is needed to keep it so. A Type 2 or Type 3 transform over more
than one component, which has one input and so is malformed. An `NChannel` space: §8.6.6.5
evaluates its components individually through these entries, so writing one would change what the
page draws from the producer's combination to a per-component evaluation; `/Process` components
are named only under `NChannel`, so a plain `DeviceN` fixes every other input at zero, its tints
being subtractive. A program that does not open with its brace, and an unreadable `Domain`.
