# 1153 — One document, one destination profile, and the second one shipped

Status: accepted. Session 1157.
Context: `data/icc/GRACoL2006_Coated1v2.icc`, `data/icc/PROVENANCE.md`, `NOTICE` section 4,
`doc/third-party-data.md`, `crates/pdf-transform/src/archive/prepare.rs` (`CMYK`, `shipped_for`,
`prepare_intent`, `ProfileSource`), `crates/pdf-transform/src/archive/decision.rs`
(`WRONG_FAMILY`, the page-level comment), `crates/pdf-transform/src/bin/quorra-transform.rs`,
`crates/pdf-transform/tests/archive.rs`, `crates/pdf-colour/src/icc.rs`,
`crates/pdf-archive/src/table/graphics.rs`.
Builds: `doc/questions/A18`'s shape — ship the standard profile, with a flag to override it, and
report that adding an output intent reinterprets the existing marks — for the other colour family.
No new remedy, no new authorisation, no rendering change.
Clauses: ISO 19005-2 sections 6.2.3, 6.2.4.2 and 6.2.4.3; ISO 19005-4 sections 6.2.3, 6.2.4.2
and 6.2.4.3; ISO 32000-2 §8.6.5.5, §8.6.4.4, §14.11.5 Table 401, §10.4.2.5.

## 1. The owner's decision, and what was left to argue

Asked whether to ship `GRACoL2006_Coated1v2.icc` from the ICC profile registry as the converter's
default CMYK output-intent profile, beside `data/icc/sRGB2014.icc`, in the shape `A18` set for
sRGB, the owner answered on 2026-09-21: **"perfect. use it."**

That settles *which file*. What it does not settle, and what this ADR is for, is **when** the
file is used — because a document is allowed one destination profile and a page drawing in both
device spaces cannot be licensed by one.

## 2. Why this profile and not another

`data/icc/PROVENANCE.md` carries the reading in full; the argument in one paragraph is that the
same clause decides it as decided the sRGB one. ISO 19005-2 section 6.2.4.2 names four ICC texts
and asks a destination profile to conform to one of them. This file's header states 2.0.0, which
is read against ICC.1:1998-09 by ADR 0950 section 6's reading — part 2 asks for conformance to a
document, not for a profile to *state* which — and ICC.1:1998-09 clause 6.3.3.2's Table 29
requires of a colour output profile ten tags, all ten of which are present, as is ICC.1:2001-12
Table 28's eleventh. The IDEAlliance 2013 profiles are licence-clean and version 4.2.0, which is
the number the sRGB argument already rejects; `PSOcoated_v3` is the right version and its licence
forbids redistribution without ECI's written permission; three others carry a copyright notice
with no grant at all. This one's `cprt` tag is the grant, and it names embedding and sharing
outright, with one prohibition — not altered — which a hash is how anyone checks.

## 3. One document, one destination profile

ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3 each require that where an
`OutputIntents` array holds more than one entry, every entry stating a `DestOutputProfile` state
the **same** indirect object. Section 6.2.4.3 of each then licenses a device colour space only
through a destination profile of *that space's own family*: an RGB profile for `DeviceRGB`, a
CMYK one for `DeviceCMYK`, and any of them for `DeviceGray`.

So a page painting in both `DeviceRGB` and `DeviceCMYK` **cannot be licensed by output intents at
all**, whichever profile is embedded. The question is not which profile is better; it is which of
the two rows gets answered.

**The rule taken: the CMYK profile where the document's unlicensed device colour is CMYK and none
of it is RGB, and the RGB profile otherwise.** `prepare::shipped_for` reads that off the
validator's own report — the four rows of section 6.2.4.3, two per part — rather than walking the
document a second time.

The tie is settled by which family has a **second** licence. Only CMYK has one: ISO 19005-2
section 6.2.4.3 admits a `DeviceN`-based `/DefaultCMYK`, its NOTE 2 explaining that section
6.2.4.4 makes such a space device independent, and `prepare_default_cmyk` already builds one out
of §10.4.2.5's own transform. So under part 2 this order licenses a mixed page *completely* — the
one profile goes to RGB, the `DeviceN` default answers CMYK — and the reverse order would leave
the RGB row failed with nothing to answer it. Part 4 states no such second licence: its sentence
asks for a *device independent* `DefaultCMYK` where part 2's admits a `DeviceN`-based one. A mixed
page under part 4 therefore keeps one refusal whichever way the choice goes, and taking the same
order in both parts is what makes this one rule rather than two.

**What the refusal says is the point of taking it** (trap 5). `WRONG_FAMILY` already names the
flag that answers it and states the difference between the parts; a mixed page under part 4 is
refused by name with that sentence and no file is written. A wrong conversion would be a file
whose `DeviceCMYK` means whatever an RGB profile says, which is nothing.

**The three cases the clause disposes of, each differently.** *Different documents* is not one
case: each conversion decides for its own document, so a CMYK document and an RGB document each
get what they need. *One page* is the case above. *A form XObject* is the same case as one page:
§8.10.1 draws its content stream as part of the page that invokes it, section 6.2.4.3's licence is
about the space being *used*, and `pdf_archive`'s survey already reports such a use at the page it
is drawn on — so a form XObject painting in the other family makes its page a mixed page, with no
separate treatment and none owed.

## 4. Why part 4's page-level output intents are not the answer

ISO 19005-4 section 6.2.3 admits a PDF/A output intent in a page dictionary's own
`OutputIntents` array, and makes a page-level one the *current* intent for that page. It is
therefore a real route for a document whose RGB and CMYK live on **different** pages, and the
survey knows which page uses which family, so the objection is not that the file cannot say.

It is declined, and the clause is why. The same paragraph makes page-level intents available only
"if the document does not contain a document-level PDF/A OutputIntent", and then requires an
`OutputIntents` array on **every** page whose contents are not fully specified in
device-independent colour. So taking the route means writing no document-level intent at all and
an array on every device-dependent page — a different shape of output for a document that may
have hundreds of pages — and it *still* leaves a refusal for any single page that draws in both
families, which is the case this converter actually meets. The document-level intent answers the
part 4 page-level row outright (`graphics/a-device-dependent-page-carries-an-output-intent`
already reads it that way), so nothing is failing that this would fix. Revisit it by argument if a
corpus measurement ever says a per-page split is the common shape; it is not one today.

## 5. What was measured

`crates/pdf-transform/tests/archive_corpus.rs`, at the base commit and again after, under the
heavy-walk lock. The site that moved is PDF/A-4's
`graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent`: **19 documents
refused before, 8 after**. PDF/A-4 as a whole goes from 177 converted and 182 refused to 188 and
171; at the default authorisations, 150 converted and 209 refused to 161 and 198.

Two cross-checks worth having rather than assuming:

- **PDF/A-4's RGB row is 16 both times.** No document lost its RGB licence to the new profile,
  which is the rule stating itself — the CMYK profile is taken only where no RGB row failed.
- **Nothing moved at any part 2 target.** ADR 1105 measured that supplying an RGB profile
  converted none of that target's twelve RGB witnesses, because each already held a destination
  profile of another family and section 6.2.3 forbids a second. The part 2 CMYK row is quiet here
  for a *different* reason: it was already answered, by the `DeviceN` `/DefaultCMYK`. So the
  shipped CMYK profile converts no document that was refused under part 2; what it changes there
  is the *quality* of the answer — a CMYK-only part 2 document now takes the destination profile
  instead of §10.4.2.5's transform, which §10.4.2.1 itself calls a crude approximation, and writes
  no `xmpMM:History` entry because nothing was interpreted.

The remaining 8 part 4 refusals are the two shapes the clause leaves: a page drawing in both
families, and a document already holding a destination profile of another family.

## 6. What the report says

`A18`'s second sentence, checked rather than assumed. `Conversion::profile` carries the source,
the profile's `desc` tag, its colour space and its `cprt` tag, and the rendered report prints all
four — so a person who embeds the shipped CMYK profile is shown its name, that it is CMYK, and
IDEAlliance's terms verbatim from the tag, which is `doc/pdf-a-conversion-limits.md` section
10.1's rule applied to a profile this program supplies rather than only to one an operator hands
it. The JSON form carries the same four fields. `OUTPUT_INTENT_REINTERPRETS` is unchanged and is
the second half: a PDF/A output intent is what a conforming reader colour-manages device colours
through, so every `DeviceCMYK` value in the file now means what the destination profile says it
means.

`ProfileSource::Shipped` now describes itself as "a profile this program ships" rather than naming
sRGB: which one it is, the report says by printing the profile's own tags beside the word.

## 7. What this is not

Not a rendering change. The viewer converts `DeviceCMYK` by §10.4.2.5 exactly as before, no
display path is touched, and `raster_golden` does not move. What changed is which bytes a
*conversion* writes into an output intent stream.
