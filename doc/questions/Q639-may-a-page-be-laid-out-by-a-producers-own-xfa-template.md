# Q639 — May a page be laid out by a producer's own XFA template?

Asked by round 1773 (RFC 0011, section 7), the first of the XFA rounds `A633` set.

## What is asked, and why it is asked now

`A633` began XFA at one round per batch and said the first round "must ask for that amendment by
name rather than assume it". A **dynamic** XFA form (XFA 3.3 chapter 9, page 333: a subform whose
occurrences are decided by the data, or a `subformSet`) has no pages until a processor lays them
out from its template, and Table 29's `/NeedsRendering` asks for that "when the document is first
opened". That is a layout engine, and `CLAUDE.md`'s *Authoring content from nothing* says "no layout
engine, no text-setting". The static half of XFA — calculations, validations, formatting and events
over the AcroForm §K.2 makes the form — composes nothing and needs no amendment; RFC 0011 section 5
builds it first.

## The amendment, worded in the shape of the four before it

> **A page laid out by a producer's own XFA template.** Where a document's interactive form
> carries an XFA resource whose template is not XFAF — Table 29's `/NeedsRendering` is `true`, or
> the template states an `occur`, a `subformSet` or other grammar XFA 3.3 excludes from XFAF —
> the pages that template generates, laid out by XFA 3.3's own layout and binding rules from the
> template and the document's own data, are on the near side. Every mark on such a page is one
> the template names; what this program decides is only what XFA 3.3's algorithm decides. Nothing
> may be placed on it that the template and its data do not state, and a page so laid out is said
> to be generated, in the window's title or report, as an appended page is.

It passes ADR 1120's provenance test (every mark is the producer's) and **fails ADR 1123's second
part**: no clause of ISO 32000-2 fixes the positions — XFA 3.3, the text §K.2 names as the
resource's format, does. Accepting it means accepting that *a normative reference's layout
algorithm may stand where a clause's position would*. Its costs (RFC 0011 section 6): layout on the
launch path, a generated page with no producer's bytes for the oracle, and typesetting from metrics.

## Recommendation

**Accept it later and separately**: once the static half is in and a measurement round has priced
laying out page one of the corpora's `/NeedsRendering` documents. Asked now so that no round
assumes it. A second, smaller question rides with it (RFC 0011 section 12, question 2): whether
`CLAUDE.md`'s XFA entry is reworded to bring the static half into scope while the dynamic half
stays excluded.

## What the tree does meanwhile

A `/NeedsRendering` document shows the page its producer wrote for processors that decline, and
every window says it is an XFA form and what is shown instead (ADR 1932). Annex K's rows stay
`out-of-scope`. The static half proceeds at one round per batch, beginning with the `datasets`
packet a save leaves stale (ADR 2380).
