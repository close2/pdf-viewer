# ADR 0146 — The catalog row that was wrong about eighteen entries

Status: accepted, 2026-08-02. Session 170. A sweep, the largest stale claim this ledger has held,
and the two entries the sweep made worth implementing.

## The sweep

Two regular expressions over `doc/conformance/ledger.toml`, the pair sessions 118, 122 and 159
each ran once: one for a note whose *reason* names a blocker that may have expired, one that pulls
every `/Key` out of a sentence claiming something is unread and greps the tree for it. Twenty
minutes, and the handover's three false-positive shapes all appeared again — a note quoting its
own retired wording, a key named in a sentence about something else, a key that is a string in an
unrelated module.

And §7.7.2, which is not a false positive.

## The claim

Table 29 is the document catalog. The row listed what the tree reads — `/Pages`,
`/OCProperties`, `/AcroForm`, `/OutputIntents`, `/Version` — and then:

> Not read, and every one of them is a *viewer* feature rather than a rendering one:
> `/PageLabels`, `/Names`, `/Dests`, `/ViewerPreferences`, `/PageLayout`, `/PageMode`,
> `/Outlines`, `/Threads`, `/OpenAction`, `/AA`, `/URI`, `/StructTreeRoot`, `/MarkInfo`,
> `/Lang`, `/SpiderInfo`, `/Metadata`, `/PieceInfo`, `/Perms`, `/Legal`, `/Requirements`,
> `/Collection`, `/NeedsRendering`, `/DSS`, `/AF` and `/DPartRoot`.

**Eighteen of the twenty-five are read**, most of them by the session that built their clause:
`/PageLabels` and `/Names` in the forty-eighth, `/Dests` in the forty-ninth, `/Outlines` in the
fiftieth, `/Threads` in the ninetieth, `/StructTreeRoot` in the seventy-eighth,
`/ViewerPreferences` in the eighty-first, and so on down to `/DSS` in the ninety-ninth. The row
closed with

> `CLAUDE.md` puts page labels, outlines and destinations in scope as things that *display*; none
> is started

— written before all three shipped, and outliving them by a hundred and twenty sessions.

**The failure is structural and this file has named it twice already.** A clause family's row is
not maintained by the sessions that implement its *members*: §12.4.2's session had no reason to
edit §7.7.2, because the two clauses do not cite each other. It is the same shape as §12.3's
parent row ("[n]ot one of them is implemented", forty sessions), and as the ninety-fifth
session's six understating rows in one family. **The defence that keeps working is reading the
family rather than the row, and the cheap instrument is a grep for the key the note says is
absent.**

## What the sweep made worth building

`/PageMode` is not a viewer feature in the sense the row meant. It names a **panel**:

> UseOutlines Document outline visible … UseOC Optional content group panel visible …
> UseAttachments Attachments panel visible

Until the hundred-and-sixty-sixth and -seventh sessions this program had no panel, so three of
the entry's six values named nothing it could do. They name something now, and reading the entry
became worth a session — which is the habit ADR 0122 records from the other side: *after a
session that adds a capability, re-read the rows whose reason begins "this program has no".*

`pdf_model::viewer_preferences::Opening` is Table 29's `/PageMode` and `/PageLayout`, and
`Query::Opening` hands them over. `viewer-ui` opens the sidebar on the tab the document asks for,
and says once what it cannot do: `UseThumbs` wants §12.3.4's panel, which is not drawn, and
`FullScreen` wants chrome that does not exist here.

Measured over 961 corpus catalogs: 38 state a `/PageMode` — 22 `UseNone`, 11 `UseOutlines`, 4
`UseOC`, and one the undefined `/Use0` — and 43 a `/PageLayout`, 24 `SinglePage` and 19
`OneColumn`. So 15 documents now open with a panel they asked for, 24 get exactly the layout they
asked for in silence, and 19 get one sentence.

## One enum, two tables, and the rule that keeps them apart

Table 147's `/NonFullScreenPageMode` shares Table 29's vocabulary and **not** its value set:
§7.7.2 adds `FullScreen` and `UseAttachments`, and §12.2 lists neither — the first because it is
that entry's own condition ("meaningful only if the value of the `PageMode` entry in the catalog
dictionary … is `FullScreen`"), the second because the table simply does not name it.

One enum with the union, and the *reading site* refuses the two extra names where the clause does
not offer them. That is a rule about which entry was read rather than about what a name means, so
it belongs there rather than in the type. A reader that accepted them both would let a document
say "when you leave full screen, go full screen".

`Opening` is deliberately a separate struct from `ViewerPreferences` rather than two more fields
on it. One struct holding two tables is how a row comes to claim what its neighbour implements,
which is the defect this ADR is mostly about.

## What is genuinely unread, with a reason each

Seven entries, and the row now names them: `/SpiderInfo` (§14.10's web capture, 5 documents),
`/PieceInfo` (§14.5's private producer data, 5), `/NeedsRendering` (Annex K's XFA, deprecated in
PDF 2.0 and on `CLAUDE.md`'s exclusion list, 3), `/DPartRoot` (§14.12 reaches document parts from
a jump's own reference), the catalog's `/AA` (§12.6.3's document-level triggers, 3) and `/AF`
(§14.13 reads it wherever an object states one, 6), and `/Metadata` (§14.3.2's XMP, 319
documents — and what §12.2's `/DisplayDocTitle` has been waiting for).
