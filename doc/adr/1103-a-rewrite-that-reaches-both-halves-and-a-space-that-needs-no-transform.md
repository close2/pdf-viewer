# ADR 1103 — a rewrite that reaches both halves, and a space whose components need no transform

## Status

Accepted.

## Context

Two residues, each named by a sentence in a ledger row and each about a route the code declined to
take. They share a shape: the clause states enough to decide one case whole, and the refusal was
written around the case it does not.

**§11.6.6 and §11.7.2.** A transparency group whose `/CS` names four components is drawn as a
*pair* of element lists — the chromatic half and the black half, one content stream interpreted
twice, resolved against one another per pixel before the group is painted onto its parent
(`pdf_render::GroupBlending::FourComponents`). §11.4.6's knockout rewrites that element list, and
`Interpreter::group_press` refused a knockout group the pair outright: the rewrite ran after both
runs, and editing one half would leave the other describing a different construction.

**§8.6.6.5.** A `DeviceN` space with an attributes dictionary whose `/Subtype` is `NChannel` asks
for its components to be taken apart:

> For NChannel colour spaces, the components shall be evaluated individually; that is, only the
> ones not present on the output device shall use the alternate colour space of that component.

On a display no component is present, so every one of them takes "the alternate colour space of
that component". Combining what comes back is what the clause never states — NOTE 3 hands that to
the processor, and Table 72's `/Solidities`, `/PrintingOrder` and `/DotGain` parameterise such an
algorithm without defining one, over inks on paper, under a sentence saying "PDF processors need
not use this information". So the whole-space tint transform stood for every `DeviceN` space, which
is what §8.6.6.4 and §8.6.6.5 require of a processor with none of the named colourants.

## Decision

### 1. Apply §11.4.6's rewrite to both halves of a press pair, or to neither

`knockout_construction` takes the pair and returns it. Each of its three rewrites goes through
`commit_knockout`, which runs the same function on the black half, requires the two results to
still `paired`, and leaves the construction untouched where either fails — under which the group
keeps the report §11.4.6 already had. `Interpreter::group_press` drops the knockout condition.

**Two arguments make this safe rather than optimistic.** The rewrite is a function of the element
list alone, and the halves differ only in what each colour resolved to — `black_half` has already
checked that their structures agree — so the branch taken is the same for both; and where it is
not, `paired` catches it and nothing is committed. **The initial backdrop needs no conversion of
its own**, which is what would have made this a backend question: a group reaching `group_press` is
isolated, and §11.4.6 states what such a group composites each element against — "[a]n isolated
knockout group composites the element with a transparent backdrop."

No backend changes. `render-cpu`'s `composite_in_own_space` already encodes both lists under the
group's own `Compose`, which is `Compose::Knockout` when the group carries `knockout: true`;
`render-gpu` and `render-raster` refuse a four-component group by name, as before.

### 2. Read §8.6.6.5's process dictionary where it answers for every component

`ColourSpace::nchannel_process` reads Table 71's `/ColorSpace` and `/Components` and answers with
the process space and, per component of it, which tint supplies it (`Tints::Process`). The tint
transform becomes `Tints::Transform`, so nothing else about the `Separation` variant moves: the
arms that convert a colour still evaluate one thing and convert the result in `alternate`, which is
"the alternate colour space of that component" under either reading.

Four conditions, each a sentence of the clause, and every one of them refuses rather than guesses:

- `/Subtype` is `NChannel`, because Table 70 says "[a] value of DeviceN for the Subtype entry, or
  no value, shall mean that only the previous features shall be supported";
- `/Components` names exactly the components of `/ColorSpace`, which is the correspondence Table 71
  requires and the only thing that makes the array an index;
- every name in the space's `names` array is one of `/Components` — "[a]ny component not specified
  in the process dictionary shall be considered to be a spot colourant", and a spot colourant is
  the case above that needs a blending nobody states;
- every component of `/ColorSpace` is supplied by a name, **unless** the process space is a CMYK
  one, which is the family §8.6.6.5 gives the permission to ("[f]or a CMYK colour space, a subset
  of the components may be present, and they may appear in any order in the names array") and the
  one whose absent value §8.6.4.4 defines ("0.0 shall denote the complete absence of a process
  colourant"). The test is the *space*, not the entry: §8.6.5.6's `/DefaultCMYK` and §14.11.5's
  output intent substitute a four-channel profile for the name `/DeviceCMYK`, and Table 71 lets a
  process dictionary name that profile outright, so reading the entry would answer differently for
  one profile depending on which way the file reached it. §8.6.5.5's Table 67 admits four ICC data
  colour spaces and `CMYK` is the only one of four components, so the channel count separates them.
  For any other process space an omission is refused rather than filled in, and the reserved names
  `Cyan`, `Magenta`, `Yellow` and `Black` that the clause lets a file leave out of `/Components`
  are likewise declined: such a space keeps its tint transform, which is conforming.

The route is asked **before** the `alternateSpace` and `tintTransform` parameters are read, which
is the clause's own order — "PDF processors need not use the alternateSpace and tintTransform
parameters, and may instead use custom blending algorithms, along with other information provided
in the attributes dictionary if present" — so a space whose tint transform this tree cannot read no
longer loses the fill.

## Consequences

**The population, from `pdf-model --example nchannel_census` over 90 340 documents** (`doc/pdf.js`,
`doc/corpora`, and the crawled `safedocs`, `tika` and `openpreserve` caches): 7983 `DeviceN` spaces
in 2417 documents, of which 3764 in 1720 documents carry an `NChannel` subtype. **3580 of those, in
1650 documents, have no spot colourant**, and only two of them name every component of their process
space — 3313 omit one under a `/DeviceCMYK` process space and 265 under a process space named
otherwise, which is why the CMYK permission had to be read as being about the space rather than
about the entry. 184 spaces in 116 documents have a spot component and keep the tint transform.
**No document anywhere states a `/MixingHints` dictionary**, which is the measurement behind the
paragraph below: the entries that would parameterise the unstated blending are not in the world
either.

**The spot case is refused in the code and in the ledger rather than at run time**, and that is a
decision with a cost. A report would fire on a departure, and there is none to name: §8.6.6.5's
earlier paragraph makes the tint transform a `shall` for a device with none of the named colourants
— "PDF processors shall be able to approximate the colourants if they are not available on the
current output device, such as a display" — and NOTE 3's "rather than being required to use a
specified tint transformation function" says what the alternative is a permission *from*. A report
saying so would fire on every such document for a choice the standard sanctions, which is trap 11's
shape. What the refusal costs is written here and in the row instead: the blending it would need is
unstated, and Table 72 says what would parameterise it if somebody wrote one.

**Both ledger rows stay `partial` and narrow.** §11.6.6 and §11.7.2 keep the four components this
tree cannot sample — a `/CS` or a `/DefaultCMYK` naming four components that are not an ICC
profile, and a press past `colour::MAX_PRESSES` — which `PagePress::Beyond` names in the report.
§8.6.6.5 keeps `/Colorants` and the per-component evaluation for a space that has a spot colourant.

**One test changed its lever.**
`transparency_groups.rs::the_blending_space_is_the_one_in_force_rather_than_the_one_declared` used
the knockout refusal as the one construction that still made a four-component group undrawable. It
now uses a `/DefaultCMYK` naming a four-component space this tree cannot sample, which is closer to
that test's own subject: the group declares `/DeviceCMYK` and what is in force is §8.6.5.6's entry.
