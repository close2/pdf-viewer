# ADR 0505 — The entry a halftone dictionary carries that is not a halftone

Status: accepted, 2026-08-23. Session 677. Closes the debt §10.5's ledger row named as what kept it
`partial`, and which round 660 recorded when it finished the pattern half (ADR 0487). Amends the
ledger rows for §10.5, §10.6, §10.6.5, §10.6.5.1 to §10.6.5.6 and §11.7.5.2. Changes nothing ADR
0479 decided about *where* §10.5 is applied to a shading's colours, and nothing ADR 0204 decided
about halftone **screens**.

## The decision

**Table 57's `/HT` is read, for one entry of it and no other.** §10.5's second bullet puts a
transfer function inside a halftone dictionary and says it wins; a halftone dictionary's screen
stays as unread as it was.

**The two sources are kept apart on the graphics state and composed per component**, rather than
folded at the `gs` that sets either.

**§10.6's rows for the halftone *dictionaries* go from `inapplicable` to `implemented`.** They said the
dictionary was read by nobody, and after this round one entry of it is — while the screen beside that entry is
ignored under a permission §10.6.5.1 states outright, which `doc/PLAN.md` makes `implemented` rather than
`inapplicable` on §10.7.2's flatness precedent: "a permission *exercised*, which is `implemented` where there is
code to name". `partial` was the first answer and the fourteenth sweep refused it, correctly — a `partial` row
must name something owed, and nothing is.

## Why a screen owes this at all

`CLAUDE.md` records that §10.6's halftones are inapplicable **on the standard's own condition**, and
that condition is real. The mistake this ADR corrects is one level down: it was read as making the
whole *dictionary* inapplicable, and a halftone dictionary carries two unrelated things.

§10.1's list of rendering steps separates them itself, and the two bullets are four lines apart:

> - For any object for which transfer functions are in effect, apply those transfer functions; see
>   10.5, "Transfer functions" for details.

> - If the raster output device supports PDF-defined halftoning, apply halftoning according to 10.6,
>   "Halftones".

One is conditional on the device and the other is not. §10.6.1 then states what a device needing no
screen still owes, in the sentence that excuses the screen:

> Some output devices can reproduce continuous-tone colours directly. Halftoning is not required for
> such devices; after gamma correction by the transfer functions, the colour components shall be
> transmitted directly to the device.

And §10.5's own second bullet is a `shall` with no device in it:

> The current halftone parameter in the graphics state may specify transfer functions as optional
> entries in halftone dictionaries (see 10.6.5, "Halftone dictionaries"). This is the only way to
> set transfer functions for nonprimary colour components or for any component in devices whose
> native colour space uses components other than the ones listed previously. A transfer function
> specified in a halftone dictionary shall override the corresponding one specified by the current
> transfer function parameter in the graphics state.

So the dictionary is the **carrier** and the entry is §10.5's. The screen beside it — a frequency,
an angle, a spot function, a threshold array — is §10.6's and is still not performed.

**The middle sentence of that bullet is the one that reads like an escape and is not.** "This is the
only way to set transfer functions for nonprimary colour components" says what the mechanism is
*uniquely* for; it does not say it is *only* for that. The sentence after it is unqualified, and
Tables 128 to 131 repeat it entry by entry — "A transfer function, which overrides the current
transfer function in the graphics state for the same component", and in Tables 130 and 131 "which
**shall** override".

## What the clause decides, entry by entry

`/HT` is "dictionary, stream, or name" and the three shapes are three different answers.

**The name.** Table 57 gives it exactly one: `/Default`, "denoting the halftone that was in effect
at the start of the page". Table 52 says what that is — "a PDF reader shall initialise this to a
suitable device dependent value" — so it is *this device's* halftone, and a device's halftone is not
one of the "halftone dictionaries" §10.5's bullet reads a function out of. `/HT /Default` therefore
takes an earlier `/HT`'s override **off** and reveals whatever `/TR` states. Any other name is a
halftone nobody defined and leaves the parameter alone, which is what `/TR`'s unknown names already
do one clause up.

**A Type 5 dictionary** is the one read per component. Its keys "shall be name objects representing
the names of individual colourants or colour components"; §10.6.5.6 names this device's — "Red ,
Green , and Blue for DeviceRGB " — and Table 132 makes `Default` the halftone "for any colourant or
colour component that does not have an entry of its own".

**Any other dictionary or stream** governs all three components. §10.6.5.6 used to say exactly that,
in a sentence contrasting the two ways a component dictionary can be used — and **erratum #311, state
Review/Completed, strikes the whole paragraph it is in**, which `tools/spec-errata emit` printed for
this clause and `doc/md/` still carries. So the reason is the one that survives the strike: Table 52
makes the parameter "[a] halftone screen for gray and colour rendering", one per graphics state, and
§10.6.5.6's own opening says what Type 5 exists for — "[s]ome devices, particularly colour printers,
require separate halftones for each individual colourant. … Halftone dictionaries of Type 5 allow
individual halftones to be specified for an arbitrary number of colourants or colour components."
A halftone that is not Type 5 therefore governs the rendering rather than a colourant, and "the same
component" in Tables 128 to 131 is every one of them.

**This is the errata rule earning its keep**: `doc/todo/02` §4 says a round implementing a clause
runs `emit` on that document *before* it writes, and `check` afterwards alone would not have caught
it — the struck run reaches `check`'s floor but its extracted words are run together
("graphicsstate", "halftonedictionary"), so nothing matched. The two `doc/errata-read.md` rows for
these clauses said "untouched — §10.6 is inapplicable on the standard's own condition", which was
true only while nobody read the clause.

**`HalftoneName` needs no case of its own**, because the clause supplies the branch a device with no
halftones of its own takes: "[i]f there is no HalftoneName entry, **or if the requested halftone
name does not exist on the device**, the halftone's parameters may be defined by the other entries
in the dictionary".

## Why the two sources are kept apart

Table 52 makes `halftone` and `transfer` **two** graphics state parameters, each with its own
initial value, and either can be set without the other. So `TransferState` carries both and composes
them, rather than resolving at the `gs`:

- `/TR /Identity` clears the first bullet and must leave the second;
- `/HT /Default` clears the second and must leave the first;
- a Type 5 halftone may speak for one component and say nothing about the other two, and §10.5 is
  explicit that this is a per-component question — "[e]ach colour component shall have its own
  separate transfer function; there shall not be interaction between components."

That is why `Transfer`'s channels became optional and why `Component` has three answers rather than
two. **`Identity` is an override**: a halftone naming it replaces the `/TR` in force for that
component with the identity, which is not the same as a halftone that says nothing and leaves the
`/TR` running. It is one keyword apart in the file and a whole function apart on the page — and it
is the shape that actually exists, which is the next section.

`effective` is derived and never assigned; composing it at the `gs` rather than at each mark is the
cheap direction, since a page has one `gs` per state and many marks under it. Every existing caller
reads `in_force()` or `shared()` and is unchanged in what it receives.

## The population, and the instrument that was wrong about it

`examples/transfer_function_census` gained the `/HT` question, and it asks it **twice**: once by the
resource walk it already had — every page's `/ExtGState` and every form `XObject`'s — and once by a
scan of every object the cross-reference table names, descending through each object's own
dictionaries and arrays so that a halftone written inline in an `/ExtGState` is not missed for want
of an object number.

**The two disagree, and the walk is the one that is wrong.** Over the SafeDocs crawl the walk finds
`/HT` in fewer documents than the scan does: an `/ExtGState` reached only from a pattern's resources
or an annotation's appearance is invisible to a page-and-form walk. That is the false zero this
tree has produced twice before, caught this time because the count was taken by two instruments
instead of one. Run the census rather than reading a number here.

What both agree on is the shape: of the whole crawl, three documents carry a `TransferFunction` at
all, **every occurrence in either corpus is the name `Identity`**, and not one of the three also
states a `/TR`. So no corpus page is drawn differently by this change, and the fixtures are the
witness — trap 8 — each one mutated against the tree that does not read `/HT`, which fails all four.

**Zero drawn wrong is not the reason to implement it and was not the reason to defer it.** The
sentence is a `shall` in a clause `CLAUDE.md` puts in scope, the code is eighty lines in the module
that already reads Table 57, and the alternative — a `partial` row naming a clause nobody executes —
is what this project calls owing in silence. It is a different case from §11.7.5.2's per-region
model, which is deferred because *inventing a second raster channel* for a population of zero would
be speculative; here the design is read off the clause and costs nothing to state.

## What this does not decide

- **A halftone screen is still not performed**, and §10.6, §10.6.1 to §10.6.4 stay `inapplicable` on
  §10.1's condition and §10.6.1's sentence. Nothing here samples a spot function or a threshold
  array.
- **`/HTO`, Table 57's halftone origin, is still read by nobody.** It is a screen's phase, and a
  device with no screen has no phase to place.
- **§11.7.5.2's per-region model is untouched.** Its report now fires for a transfer function stated
  by either route, which is what the clause means by "the halftone and transfer function to be used
  at any given point", and its row says so.
- **`/BG`, `/BG2`, `/UCR` and `/UCR2` are unchanged**: §10.4's black generation and undercolour
  removal, read for a flag and never evaluated.
