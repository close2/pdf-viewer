# 0821 — A merge is a reconciliation per clause, and a refusal by name

Session 888. Status: **accepted**. The ninth decision record of RFC 0002's implementation, on the
long-lived branch `round-867`, and the suite's second verb on ADR 0817's serializer.

## Context

Session 886 landed `split` and deliberately did not land `merge`, on the ground that the hard part
is not the machinery. That reading is confirmed: cross-file renumbering was already
`pdf_syntax::serialize::Assembly`'s — it takes a list of sources and answers the output's numbering
for an object out of any of them — so concatenating pages cost almost nothing, and this record is
about the other half. RFC 0002 §6.2 names it in one sentence: the merge-time document-level
reconciliations are a long tail of individually small decisions, each of which must be a documented
choice rather than an accident.

A document is not a bag of pages. It states optional-content groups and one default configuration
for them, an interactive form with a namespace of field names, name trees whose keys "shall not
overlap", one outline, one page-label tree, one output intent. Several documents cannot simply both
have those, and a verb that concatenated pages and guessed at the rest would be the placeholder
principle 1 forbids. So the decision below is one reconciliation per clause, each derived from a
sentence and each with its refusal path.

## Decision

### 1. §14.11.5 has two homes for an output intent, and the page's is the one in force

The clause's own first sentence puts `/OutputIntents` "in the document catalog dictionary (see
7.7.2, "Document catalog dictionary") or a Page dictionary (see 7.7.3.3, "Page objects")", and two
paragraphs later it decides between them:

> The data in an output intent dictionary shall be for informational purposes only, and PDF
> processors are free to disregard it. If a PDF processor chooses to respect output intents, then
> when processing a page that has an associated (page-level) output intent, that page-level output
> intent shall be used.

**This tree does choose to respect them** — `content::colour::output_intent_space` reads a
`/DestOutputProfile` and lets it decide what a device colour means — so the `shall` bound it, and
until this session only the catalog was read. That is the round's one reading finding, and it was
found by needing an answer to a *writer's* question rather than by auditing the clause.

The change is provably inert on everything this project measures: a scan of all 974 corpus
documents found **no** page-level array and 17 catalog ones. What it unblocks is the merge. Where
one contributing document states an array and it is the only contributor, the array goes on the
merged catalog, which is `split`'s answer. Where several documents contribute, **each stating
document's array is written onto its own carried pages** and the merged catalog states none —
because a catalog array is a statement about *every* page of the document, and a page out of a
source that stated none would be drawn under a colour meaning it never had. The clause's second
home is exactly the construction that need, and it is one this reader can now read back.

### 2. §12.7.4.2's last sentence is a refusal, and the alternative is recorded as not taken

> In addition, actual field dictionaries with the same fully qualified field name shall have the
> same field type ( FT ), value ( V ), and default value ( DV ).

So a fully qualified name two sources share is **permitted** where the three agree — the clause's
own case of one field with several representations — and **cannot be written** where any differs.
The first is carried with a warning naming the field; the second is `Refusal::FieldCollision`,
naming every colliding name and the clause, at RFC §4.4's exit status 4.

The construction that would resolve a disagreement exists and is deliberately not taken: §12.7.4.2's
own hierarchy would let a merge put each source's roots under a synthesised non-terminal field, so
that every name gained a per-source prefix. That renames every field in the output, and a field's
name is what §12.7.6.2's submit-form action exports and what a data file matches on — a change to
what the document *means* that is invisible on the page. RFC §6.2 asked for "an honest warning
naming the fields" as tranche-one behaviour and for silent coexistence to be refused; a refusal that
names the clause and the fields is the stronger half of that, and the rename waits for a flag
somebody asks for.

**A collision inside *one* source is carried with a warning instead.** The clause binds the document
that holds both fields; that document already held them, carrying what the producer wrote is RFC
§11.1's whole premise, and refusing would decline to merge a file every reader opens. The corpus
walk found the witness on its first run (`issue15096.pdf`), and the first implementation refused it.

### 3. §7.9.6's keys are renamed, and `/Dests`' references are chased

> The keys contained within the various nodes' Names entries shall not overlap; each Names entry
> shall contain a single contiguous range of all the keys in the tree.

A key two sources share cannot appear twice, so the later source's becomes the first free `key (2)`,
`key (3)` — deterministically, because RFC §9's first layer applies to a rename as much as to an
offset — and every rename is reported. Each category is its own namespace, and the merged tree is
one root node holding one sorted `/Names` array, which the clause permits ("[i]f the root node has a
Names entry, it shall be the only node in the tree") and whose order is `<[u8] as Ord>` exactly.

**`/Dests` is the one whose references are chased, and the reason is that this tree's own reader
treats its two homes as one.** §12.3.2.4 gives a named destination the catalog's `/Dests` dictionary
keyed by name objects and the name tree keyed by strings, and `Destination::named` asks both by the
same bytes; so the merge uses **one** namespace across both homes and every source, emits each
source's entries back into the home it used, and rewrites the destination wherever the standard says
one is stated by name: an annotation's or an outline item's `/Dest` (§12.3.2.3, §12.3.3) and a
`/GoTo` action's `/D` (§12.6.4.2). That is why such objects cross **replaced** rather than copied,
and why the renames are computed before the closure walk rather than after it — an object cannot be
rewritten once it has been taken by reference.

For every other category the rename is reported and the references are not chased, because this
program does not know what states them. That is a named limit rather than a silence, which is the
line this whole verb is drawn along.

### 4. §12.3.3's outlines are spliced, not parented under a synthesised item

Table 150 makes the items "a linked list, chained together through their Prev and Next entries and
accessed through the First and Last entries", and Table 151 makes a top-level item's `/Parent` "the
outline dictionary itself". So each source's top-level items cross with `/Parent` renamed to the
merged dictionary and `/Prev`/`/Next` naming their neighbours in input order; everything below the
top level crosses untouched, so each source's outline keeps its own shape.

RFC §6.2 proposed "one top-level item per source, a documented choice". **It is not taken**, and
Table 151 is the reason: `/Title` is "( Required )" and this program has no title for a source — the
seam holds no paths, and a document need state no `/Title` — so the proposal costs an invented string
on every merge. Splicing invents nothing.

An `/Outlines` whose `/First` is absent states no items, since the entry is "( Required if there are
any open or closed outline entries )"; 29 of the 974 corpus documents have exactly that, and not
carrying it loses nothing.

### 5. §8.11's default configurations become one, on the clause's own parenthesis

§8.11.4.2 makes `/OCGs` "[a]n array of indirect references to all the optional content groups in the
document" with "[e]very optional content group shall be included in this array", so the union is the
only conforming answer; and it makes `/D` one required default configuration, so two have to become
one. §8.11.4.3, Table 99's `/BaseState` row, is what makes that derivable:

> If BaseState is present in the document's default configuration dictionary, its value shall be ON

A conforming source therefore starts every group ON and names its exceptions in `/OFF`. The merged
`/D` omits `/BaseState` — its default is `ON` — and its `/OFF` is the union of what each source
turned off. A source stating `/BaseState /OFF` has said what the clause forbids and is read rather
than refused: every group it did not name in `/ON` goes into the merged `/OFF`, which is that
source's own initial state written the way a merged configuration can state it, warned about by
name. `/Order`, `/RBGroups`, `/Locked` and `/AS` are lists and concatenate; `/Configs` is a list of
whole alternate configurations and concatenates too; `/ListMode`, `/Intent`, `/Name` and `/Creator`
are single-valued and the first source that states one wins.

**Two groups sharing a `/Name` is not a collision.** Table 96 makes it "[t]he name of the optional
content group, suitable for presentation in an interactive PDF processor's user interface" — a label,
not an identifier — and §8.11.3.2 makes content name its groups by object through the resource
dictionary's `/Properties`. Nothing is renamed; a duplicate label is warned about.

### 6. §12.8.1: a signature crosses without its `/V`

> A byte range digest shall be computed over a range of bytes in the PDF file, that shall be
> indicated by the ByteRange entry in the signature dictionary.

> The digest shall be recomputed and compared with the one stored in the document. Differences
> between the two indicates that modifications have been made since the document was signed and thus
> the signature shall be considered invalid.

A merged file is not the file any signature was computed over, and its `/ByteRange` offsets name
unrelated bytes in it. So a signature **field** is carried and the signature dictionary its `/V`
named is not: the output states no signature rather than one it knows cannot verify, warned by name
per field. `/SigFlags` bit 1 stays set, correctly — the document does still contain a signature
field. NOTE 1's incremental-update construction, which preserves a signature, is precisely what a
merge is not.

That settles §12.8.2.2's singular by construction as well: "[a] document can contain only one
signature field that contains a DocMDP transform method", and merging two certified documents would
produce two — but no `/V` crosses, so no transform method survives at all.

### 7. §12.4.2's labels are one entry per page

A number tree keyed by page index cannot be concatenated: the merged indices are new and a source's
selection may reorder or subset its pages, so no range of a source's survives. Where any source
states `/PageLabels`, the merged tree holds one entry per output page reproducing the label that page
had, which the clause makes exact — "[t]here is no default numbering style; if no S entry is present,
page labels shall consist solely of a label prefix with no numeric portion" — and "[t]he tree shall
include a value for page index 0" is met because every page has one.

**One documented choice with an edge**: a page out of a source that states no `/PageLabels` had no
label, and the standard does not say what such a page is called. It gets `<< /S /D /St n >>` with its
own one-based position — the decimal number a reader shows for an unlabelled page — so that it keeps
its identification instead of falling into the preceding source's labelling range, which is the one
answer that is certainly wrong.

### 8. §12.8.2.2 is asked of every source, so the policy moved from per-plan to per-document

Table 22's flags and this clause's certification are each *one document's* assertion over its reader.
`pdf_transform::apply` therefore opens every source the plan names and decides **once per opened
document** rather than once per plan; a merge of a document that permits assembly with one that does
not is refused on the second's word, by name. The verdict itself is unchanged, and `Operation::Assemble`
is the right one: Table 22's bit 11 says "[a]ssemble the document (insert, rotate, or delete pages …)"
and a merge inserts each input's pages, while `restriction::certification_permits` already recorded
why every level of Table 257 permits it — every sentence of that table is about a change that "shall
invalidate the signature" *of this document*, and a merge leaves every source's bytes where they were
and writes a different file beside them.

### 9. What the merged catalog does not carry, and why `/Info` is on that list

The structure tree, `/MarkInfo`, `/Metadata`, `/Threads`, `/SpiderInfo`, `/Collection`, `/Perms`,
`/Legal`, `/Requirements`, `/DPartRoot` and `/OpenAction` are not carried and every one a source
states is named in a warning — `split`'s argument, that a construct nobody thought about is a
construct nobody is told about.

**`/Info` is deliberately among them.** §14.3.3's entries are claims about *the document* — its
title, its author, its producer, when it was created — and the merged document was made by no
source's producer at no source's creation time, so carrying one source's would write a false claim
into the file and synthesising one would be authoring metadata this program does not author. qpdf
reaches the same behaviour through `--empty`; here it is the default and the reason is the clause's
rather than the tool's.

### 10. A merged document with no root field states no `/AcroForm`

Table 224 makes `/Fields` "( Required ) An array of references to the document's root fields", and
§12.7.3 makes the interactive form dictionary what "[t]he contents and properties of a document's
interactive form shall be defined by". An `/AcroForm` whose `/Fields` is empty therefore states no
interactive form, and the merge writes none.

**This one was found by the instrument rather than by the reading**, and it is the sharpest argument
for building the walk in the same round as the verb. The fixed second document of `merge_corpus`
states an `/AcroForm` with an empty `/Fields`; carrying it put an interactive form dictionary into a
merged document that had no field at all, and this tree's own reader then drew a *different* source's
annotation differently. An entry that changes what another source's page marks is exactly what these
reconciliations exist to prevent, and the raster oracle is what said so.

## Consequences

- **`Plan::sources` exists and `apply` opens several documents.** The seam's shape is unchanged —
  still a plan of data, sources of bytes, sinks by name — and the confined-worker split RFC §8 names
  is still a transport change.
- **Three new refusals**: `Refusal::FieldCollision` (§12.7.4.2, exit 4), `Refusal::PageTwice`
  (Table 31 gives a page one `/Parent`, exit 2) and `Refusal::Assembly` for the sentences that say
  the output could not be built at all.
- **The corpus walk earned its round twice over.** Its first run found two defects that no reading
  had: §8.11.4.3's `/OFF`, `/Order` and `/RBGroups` read as direct arrays when `issue18823.pdf`
  states them as references, so the merged configuration turned nothing off and the page drew with
  layers the document had hidden; and the empty `/AcroForm` above. Both are the shape ADR 0817's
  round recorded — found by the instrument, right for a reason the clause states.
- **A latent flake in `tests/split.rs` was surfaced and fixed**, and is now trap 30: `MemorySinks`
  hands its outputs back in the order they were *opened*, which under rayon is thread order, and
  three assertions indexed the vector by position.
- **What is still owed** is in `doc/todo/57`: `pages`, `optimize` and §7.5.7's producer half,
  `split --at-bookmarks`, the structure-tree fragments no verb carries, a per-input password for
  `merge`, and a corpus-wide *foreign* readback of what these writers produce.
