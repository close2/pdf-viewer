# 0949 — The licence the standard states, and the condition it carries

Session 949. Status: **accepted**. The third slice of `quorra-transform archive`
(`doc/questions/A22`), taking section 10.1 of `doc/pdf-a-conversion-limits.md`: the `DeviceN`
`/DefaultCMYK` a document with no CMYK profile needs, and the `xmpMM:History` entry
`doc/questions/A48` makes it conditional on.

## What was blocking, measured before anything was written

ADR 0948 shipped the output intent and the identification schema. Over the veraPDF corpus that
left the `DeviceCMYK` sentence of section 6.2.4.3 as the **sole** remaining blocker on
thirty-two documents — nineteen under PDF/A-4 and thirteen under PDF/A-2b — sole in the strict
sense: for every one of them the converter's report named exactly one refused requirement and it
was that one. An sRGB output intent does not license `DeviceCMYK`, and this program ships no CMYK
profile because nobody may redistribute one (section 10.1's finding).

**Thirteen of the thirty-two are now converted; nineteen are not, and the difference is the
standard's rather than this converter's.** That is the first thing this slice established and it
is worth stating plainly, because the round was briefed with the thirty-two together.

## 1. Part 2 has a second licence and part 4 does not

ISO 19005-2 section 6.2.4.3 admits a device independent `DefaultCMYK` **or a `DeviceN`-based
one**, and its NOTE 2 says why the second is admissible: such a space is subject to section
6.2.4.4 and is thereby device independent. ISO 19005-4 section 6.2.4.3 states the same sentence
with the `DeviceN` half absent — a device independent default, the current blending space, or a
CMYK destination profile, and nothing else.

A device independent four-component space is an `ICCBased` profile with `N` 4, which is the CMYK
profile the document did not have. So under part 4 there is no construction to write: the answer
is `--output-intent-profile` and the sentence a user gets says so. Under part 2 there is, and it
is the standard's own:

- **The space** is §8.6.6.5's `DeviceN` over `[/Cyan /Magenta /Yellow /Black]` — exactly the four
  names that clause reserves to the subtractive process colourants of a CMYK device, which is
  also why it needs no `Colorants` entry: section 6.2.4.4 requires one for a *spot* colour and
  none of the four is one.
- **The tint transform** is §7.10.5's PostScript calculator function stating §10.4.2.5's
  conversion. The clause's formula is `red = 1.0 - min(1.0, cyan + black)` and the same for green
  from magenta and blue from yellow; §7.10.5.2's operator set has no `min`, so the clamp is
  written `dup 1 gt { pop 1 } if`, which is that value in the operators the clause does define.
  Nothing here is invented: `tests/archive.rs` parses the written stream back through
  `pdf_model::function` and holds it to the clause's arithmetic at five points.
- **The alternate space** is the ICC sRGB profile the output intent would have named. §10.4.2.5
  produces `DeviceRGB`, and what makes those three numbers mean sRGB is the same assertion the
  output intent makes — so the two constructions agree by sharing one profile object rather than
  by coincidence. Where the profile in hand is of another family the construction is **refused**,
  because writing the transform's result into a space of another family would be asserting an
  arithmetic no clause states. One corpus document is refused for exactly that (its file already
  holds a GRAY destination profile), and it is the thirteenth of the thirteen.

## 2. Where the entry goes, and why it is a structural walk

`TechNote 0010`'s A028 resolves that parts 2 and 3 are read as if a default colour space had
itself to be defined in the resources dictionary **explicitly associated** with the content
stream, and A003 says which four dictionaries that names: a page's, a tiling pattern's, a form
`XObject`'s and a Type 3 font's. `pdf_archive` already reads it that way — `DefaultSpace` carries
`explicit` beside `in_force` for this reason — so the converter has to write the entry into every
one of them, not into the catalog and not into one page.

Two decisions inside that:

- **The sites are found by walking the document's structure, not by reading the validator's
  findings.** A `Findings` list is capped, so a document failing in more places than the cap would
  be one this rewrite silently half-finished. The walk starts at every page, descends through
  `/Resources` into `XObject`, `Pattern` and `Font` entries and through `/Annots` into `/AP`, and
  is itself bounded (`MAX_RESOURCE_SITES`) with the output's own verdict as the net when it stops
  early.
- **Every such dictionary, not only the ones whose stream paints in `DeviceCMYK`.** A
  `/DefaultCMYK` says how the four numbers of a `DeviceCMYK` value are to be read, and a file in
  which the same `0 0 0 1 k` meant one thing on one page and another on the next would be worse
  than the one that came in. The entry changes nothing where nothing selects the space: §8.6.5.6
  remaps when a device space is *used*.

A page that states no `Resources` entry of its own has no explicitly associated dictionary at all
under A028, so the requirement is asking for the entry it does not have. It is given the one
§7.7.3.3 already puts in force — **the same reference** where it inherits one by reference, a copy
where the entry above it is direct — so nothing a name on that page resolves to changes. And an
entry the producer already wrote is **never** replaced: a `/DefaultCMYK` in the file is the
producer saying how their `DeviceCMYK` is to be read.

## 3. The permission and its condition are one thing

`doc/questions/A48` allows this construction on two conditions: reported per document, and
recorded in `xmpMM:History` naming the clause. ADR 0927 states the shared condition of all four
of the owner's permissions — *what was written is reported* — and the sharp form this slice gives
it is:

> a document whose XMP packet will not take the history entry does not get the construction
> either.

`Prepared::of` builds the default, then the packet; if the packet edit fails, the default is
withdrawn with a reason that says so. A converter that kept the permission while dropping its
condition would have helped itself to a licence nobody granted, and making that a compile-time
data-flow rather than a rule somebody remembers is the only way it stays true.

The report carries both halves: the `Decision::Stated` sentence saying what a reader is now told
`DeviceCMYK` means, including §10.4.2.1's own word for the price — "crude approximations" — and a
`Conversion::recorded` field naming what went into the history.

## 4. `xmpMM:History` is appended to, never replaced

`pdf_model::xmp` gains `record`, beside `restate`, and the same span surgery: one pass of the
tokenizer, an insertion, every other byte the producer's. Three cases and they are three different
answers:

- a packet stating one `xmpMM:History` whose value is one `rdf:Seq` — the event goes at the end of
  that sequence, after every entry already in it;
- a packet stating none — one is created holding this single event;
- anything else — **refused**, with the packet untouched, because a history is the record of what
  happened to the file before this program saw it and there is no reading of "record the action"
  under which throwing that away is the right answer.

The event states all three of the fields ISO 19005-2 section 6.6.6 names. Part 4's section 6.7.5
requires two of them and demotes `parameters` to a recommendation (ADR 0931 on why part 2's row is
outside validation and part 4's is a check), so writing all three answers both parts and neither
part's reader has to know which converter wrote it.

**The `when` field is the one place this verb reads a clock**, and it is why
`the_same_input_twice_produces_identical_bytes` now says which fixture it is about. A recorded
action's time is a fact about when the action happened; there is nothing else it could be, and the
alternative — a converter that recorded a time it had made up — is the failure this whole ADR is
against. `xmp::instant` writes it as ISO 16684-1's date form in UTC, whole seconds, through
Hinnant's civil-date arithmetic with the epoch shifted to a March so that no month length depends
on the leap rule.

## 5. One decision table row that is not one rewrite

`Answer::CmykUnderPartTwo` is the first row of `REMEDIES` whose answer is not a rewrite, because
the sentence it answers offers two licences and which of them applies is a fact about the profile
in hand. `Answer::rewrites` therefore returns two slots rather than one, so that `Prepared` builds
whichever `decide` turns out to choose — it cannot know before the constructions exist.

The ordering is the ranking §10.4.2.1 states and section 10.1 repeats: **a supplied CMYK profile
takes the output intent and no default is written at all.** The approximation is what is left when
the ICC route is closed, never a shortcut past it.

## What this does not do

- **PDF/A-4 is unchanged**, for the reason in section 1. A converter that wrote the `DeviceN`
  default for a part-4 target would be writing a construction that part does not license, and
  stage 3 would refuse the file anyway — correctly.
- **A document whose own output intent holds a GRAY or CMYK profile and which still fails the
  `DeviceCMYK` row is refused**, because the alternate space has to be RGB. Embedding a *second*
  sRGB profile beside the one the file already holds would answer it, and that is a decision about
  what a converter may add to somebody's file rather than a line of code: it needs its own
  argument, and it needs the report to name two profiles rather than one. One corpus document
  waits on it.
- **`graphics/transparency-group-colour-spaces-obey-the-colour-rules` is still the largest
  post-conversion failure** — twelve documents under each target reach stage 3 and are refused
  there. That is ADR 0948's second section working as designed and it is the next colour question,
  not this one.
