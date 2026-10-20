# 2126 — The files a kind opens are compacted, and each claim in them checked against the tree

Session 1645, batch one hundred's slot 7, a docs round. Status: **accepted** and **applied**.
Asked by the project owner on 2026-10-20: "go through the md files, which are read by every
subagent. Can we compact them? Is all information still valid?" ADR 2110 did the files every round
reads; this is the second half, the files `tools/round.sh` hands a kind of round. Context: ADRs 0281
(a counted fact is a command), 1023 (a correction is applied, not appended), 0187 (another standard
is cited, never quoted), 2110 (the method), 2127 (the gate and the small finds this round made).

## 1. Method, and what was measured

`doc/state-of-play.md`, `doc/verify.md`, `doc/ui-boundary.md`, `doc/third-party-data.md`,
`doc/stack.md` and the `doc/todo/` files `tools/round.sh` names (01, 30, 56, 57) were read whole by
ten readers in parallel — four over `state-of-play.md` by paragraph range, two over `verify.md`, one
per remaining file or pair — each checking every path, symbol, command, ADR, trap and count against
the tree, under ADR 2110's rules: a false claim corrected where it stands, a repeat kept once and
pointed to, a printed figure made the command that prints it, retired history deleted, every
capability, clause citation, ADR carrying an argument and owner's quotation kept. One paragraph per
write, re-read immediately before it, because siblings edit `state-of-play.md` in one-sentence hunks.
After the pass, every ADR number the old texts cited was looked for in the new texts and in the
other documents; one, ADR 1751, was cited nowhere any more, and its capability was restored (§2).

| file | words before | after | change |
|---|---|---|---|
| `doc/state-of-play.md` | 20 376 | 20 266 | −0.5 % |
| `doc/verify.md` | 21 782 | 19 818 | −9.0 % |
| `doc/ui-boundary.md` | 16 485 | 6 670 | −59.5 % |
| `doc/third-party-data.md` | 13 577 | 12 254 | −9.7 % |
| `doc/stack.md` | 2 202 | 2 109 | −4.2 % |
| `doc/todo/01` | 7 256 | 7 123 | −1.8 % |
| `doc/todo/30` | 3 915 | 3 589 | −8.3 % |
| `doc/todo/56` | 15 254 | 10 938 | −28.3 % |
| `doc/todo/57` | 3 490 | 1 719 | −50.7 % |
| the set | 104 337 | 84 486 | −19.0 % |

**The hypothesis held for the other files and not for the first.** `state-of-play.md` carries almost
no printed counts and almost no restated ADR titles: it is a capability list in which nearly every
clause is a capability with its clause and its ADR, and the rules keep those, so its words barely
moved and its corrections were what the pass bought. The savings were where a file had grown as a
log: `ui-boundary.md`'s status block was some 530 lines of what each session added (98 spelled
ordinals, now 0), `todo/57`'s "What is done" was eight paragraphs of built work, `todo/56` argued
questions the owner has since answered. A file that is long because it is a list of what the program
does is not compacted by this method; one that is long because it remembers how it got there is.

## 2. Claims found false, each corrected in place

**`doc/state-of-play.md`** (16, and a definition added): the owner's current target "stated on 2026-10-10, is
performance" — the target since 2026-10-14 is code improvement, with PDF/A remedies beside it, and the
performance target keeps todo 69; a password asked "in all three" windows — four; `render-raster` over
"the document renderer this project commissioned" — over this tree's `raster/`; "(ADR 1585,
`doc/questions/Q270`)" — answered, A270; "Page one goes to the graphics device" unconditioned — by
default (ADR 1831, A393); "refuse the own-backdrop construction" — a non-isolated knockout group;
"the eleven Tables Annex O.3 and O.4 print" — the eleven parameters; the C ABI "the one host without a
way to raise the flag" — `quorra_window` raises it (ADR 2088); GTK "Tier 1 … a `gdk::MemoryTexture`"
— both toolkit windows draw on the device and read back (ADRs 2070, 2071); `viewer_host::native_window`
"for all three windows" — the two toolkit windows and the C window (ADR 2105); the .NET layer missing
(ADRs 2054, 2073); "`/NOTICE`" — the tree's `NOTICE` file; the key table cited to ADR 1466 — ADR 0526,
1466 being the wheel; the drive "takes them", three — and `quorra-confined`; `pdffs` — the binary is
`quorrafs`; "every Table 22 bit is named (one as consumed by nothing)" — every `Bit` variant is
consumed and bit 10, retired, has none; a named destination's page "read from the view state (ADR
1724)" — carried out as its page and its Table 149 view (ADR 1751, which superseded that row). Added:
what "the three windows" means.

**`doc/verify.md`** (25): `cargo fuzz` "fails with no such subcommand" — it runs; the `x509` seeding
route under `crates/pdf-model/src/` — `crates/pdf-signature/src/`; `xmp` "the tree's only XML"; the
`fragment` and `confined_wire` "only" inputs no document carries; the drive's "three windows" and its
usage and verdict lines; "each in `quick`" — `remedies` is not; `tools/state.sh traps`'s
"four-fifths"; two catalogue lines unlike their examples' headers; pdf.js "pinned at v6.1.200";
"fourteen" specification PDFs (twice); an oracle line "271 s ours, 90 s" nothing prints; the
*Measuring* pointer into `doc/habits.md`; `a_zoom_gesture_out` under `viewer-ui` — `viewer-device`'s
library tests; `MAX_OPERATIONS`, which no longer exists (ADR 1809); the lane switch in `viewer-ui` —
`viewer-device`'s `lane.rs`; "two un-ignored tests" — four; "(749's rule)" — ADR 0749 is not that;
`@/tmp/paths` against the scratch rule (trap 192); a sentence ending mid-clause; an unbounded `xargs
-P 8` (trap 116); a figure "131 of 255" now the example's print; and the fuzz wrapper "does not run
under `tools/bounded.sh`", which ADR 2127 made false and this round rewrote.

**`doc/ui-boundary.md`** (22): the `Command`, `Event` and `Query` lists, read as complete and missing
39 variants, now name their source files; `Edit` "four" — eight; `Restrict` "two levels" — a scope and
four; `Event::Extracted`'s fields; `unsafe_position.rs` "exactly two" crates — six names; the
dependency lists of `viewer-core`, `viewer-ffi`, `viewer-accessibility` and `viewer-host`;
`viewer-gtk`'s "whole public interface"; the toolkit windows "draw no page through a graphics device"
and `quorra` "its one consumer" (ADRs 2070, 2071); rule 2 cited to this file where it is ADR 0378's;
the chrome's "four tabs" — six; a test-case count; `Query::FreeTextAt` on a file's annotation "a
decision nobody has made" — built (ADR 0304), and "Near, and far"'s two left items, both built (ADRs
0304, 0329); `viewer-render` and `viewer-gpu`, which do not exist; the ADR 0247 quotation credited to
`doc/todo/30`; three quotations not verbatim (rule 2, §7.9.6 elided, a doc comment) made exact or
paraphrase; RFC 5280 quoted — paraphrased (ADR 0187).

**`doc/third-party-data.md` and `doc/stack.md`** (15): the codecs "from upstream's repository" — the
owner's fork `close2/hayro`; `hayro-ccitt` "decoded fax until then" — census evidence only;
`accesskit_unix` 0.24.0 as the tree's — it pins 0.22.1; `const-oid` "six of ten" — nine; BSL in the
licence list — not in `deny.toml`; "`crates/` holds no `.pdf`" — three built fixtures; "exactly two"
crates lifting the denial; the toolkits "excluded from the cross-target checks" — in none; `p256` and
`p384` "declined" — taken (ADR 0532); format-corpus "sparse, three of five" — the recipe only; "No gate
requires it" — the tests that skip are named; `Q209` — answered, A209; two JPEG module paths;
`rasterrocket`'s ignore file; the ICC.1 texts' download form.

**The `doc/todo/` files** (22): todo 57's DCT encoder and confinement tranche "the two the owner
decides" — A01 and A02 answered both on 2026-09-06 and neither is built; "all five writers" through
qpdf — five of the eight; "24 threads" — four under the lock (ADR 1766); `pdf-vfs`'s generators; a
dangling "item 3". Todo 30: "two crates" lift `deny(unsafe_code)` — three; the device path "owed" —
built (ADRs 2052, 2070, 2071), the dmabuf texture and `QRhiWidget` left; `viewer-ffi` "cannot stop a
draw" — narrowed. Todo 01: §12.10 and §12.10.2 "until Q271" — answered (ADR 1918); §12.8.2.4 "named
rather than built" — implemented; `/tmp/emit.md`; a hand `git worktree` recipe — `tools/batch.sh
export --as base`. Todo 56: §12.6.4.17 and six other rows "out-of-scope"/`partial` — `implemented`;
the exclusion "still stands" — ratified (A286, ADR 1904); Table 200 "refused" — run (ADR 1614); Table
197 "four of ten" — ten (ADR 1750); `pdf-sandbox`'s descriptor limit "zero" — 8, the script worker's 0;
`perform_all`'s file; the script worker missing from the code list; Adobe's reference "404" — 200 on
2026-10-20; ISO 21757-1 quoted verbatim three times — paraphrased (ADR 0187); `/tmp/paths-56`.

## 3. Deletions, by file and first words

**`doc/state-of-play.md`**: "`doc/HANDOVER.md`'s reading table is the pointer to this file"; "and every
sentence below is a capability rather than a plan"; ", which is where they did not"; "which is the
first thing in this tree that clicks"; "where `viewer-ui` stopped reading `stdin`"; "and one vocabulary
for two directions is a trap with a tick"; "decided by the project owner and written into" and "GPU
bring-up is therefore *on* the critical path by choice" (CLAUDE.md's); "the points have been the host's
since the clause was built"; "which is every value of that entry obeyed rather than reported"; "which is
`CLAUDE.md` section 2 reaching a panel" and "which is why a row is fetched when it is about to be drawn";
"**Not one *pdf.js* document states a thread** … 115 beads" (now `tests/articles.rs`); "had no surface
for their licences at all"; "three tables that disagreed about the arrow keys"; "and page one is drawn
while it is read, as in `quorra`" (repeated later, ADR 1867); "which the owner ratified in
`doc/questions/A393`"; "a 512-pixel reduction … eight pages kept" and "a list of 65 536 commands" (now
the constants' names); "names all seven"; "`doc/todo/30` made freezing conditional"; "and is why the
sentence is worded about consumers"; "`doc/todo/30`'s order made GTK4 first because"; "by the hundred";
"rather than counted here"; "and two rounds read "eleven queries" off it"; "and a non-baseline image
that states its own colour space is transcoded" (the conversion paragraph's); "for a clause every
requirement of which is executed except one `shall`" (PLAN §5a's); "so a ten-thousand-page `/Kids`
array costs it a second"; "redrawn". Moved, not deleted: the overprint mode's `render-raster` sentences,
the suite's gate sentence, the restriction paragraph out of the directory paragraph, the three workers'
descriptors into the worker's bullet.

**`doc/verify.md`**: "this list said 1369 tests where the gate printed 1371"; "Twenty-three
conformance-ledger rows and one module header"; "which was a recipe for seven hundred sessions"; "the
fifteenth hand rebuild measured 775 of the 838"; "a seventy-five-arm `match` scores nothing"; "for five
pushes while everything here was silent"; "The darwin one was added in the same round"; "since the
three-hundred-and-eighty-fourth"; "Both of these pass; added in the four-hundred-and-eleventh"; "in the
nine-hundred-and-thirty-seventh session all three were red"; "four … 209 passing ones"; "all 19 sandbox
tests"; "since session 995"; "It was here rather than there because the read walk"; "28 deaths";
"twenty seconds"; "and the one no gate had ever run over the corpus"; "took its refusals from 36 to
12"; the census figures each example prints (clip_chain "3490 `sh` operators", content_budget "3.76" and
"483.84 MiB", integer_entry "the fifteen names", "About twelve seconds", "at 24 threads", "5.6 GiB",
variable_text "622 widgets", presentation "978 opened", associated_file "Over the 974", absence_audit
"the 1251 PDFs", cell_header "over 1251 files", group_shape "162 groups", non_isolated "about 17
minutes", group_blit "73 047", filtered_edge "by 0 and quorra by 131", sampled_lane "88.31 %", signature
"67 460 files" and "(session 689)" and "found being declined six times", luminosity "87 groups",
colour_transform "158 images", press_cost "17 to 46 ms", spec_annotation "12 545 annotations" and "**The
number that used to bound it is gone**", spec-errata "(**151**)" and "**This comment said 79**",
"fourteen PDFs in 6.4 s"); "its counts having held across rounds"; "**107 of them when the census was
built in session 954**"; "77 of those rows"; "twenty-one figures"; "in three seconds"; "nine fresh
processes"; "on 2026-09-06 … 3.74 GHz"; "Found by the seven-hundred-and-sixty-third session"; "~45 min";
"Since ADR 0656" (and four like it, now citations); "Sessions 425 and 426 wrote" and "Prefix the run"
(environment.md's); "Those two lines used to be stated here as well"; "A fresh worktree had neither
directory at all until"; "is some ninety thousand files and 126 GB" and the per-shape seeding recipes
(each `fuzz/seeds.sh` arm's); forms_data's "`python3 fuzz/seed_forms_data.py …`" and "the one on this
disk held no FDF file"; serialize's hand recipe and "**This line is here because its absence was
invisible.**"; page's "which nothing reached until the four-hundred-and-twenty-eighth", "And a second
seeder since the five-hundred-and-ninety-second", "26 whole one-page documents", "1882 seeds",
"the five-hundred-and-ninety-third spent that merge", "100 with a `/Function`", "10–30 execs/s", "the
one libFuzzer called 15 s", "six forks sharing 24 cores"; the drive's "**Release binaries first**", "It
prints `step, window, …`", "Its clicks are **asked of the window**", "About nine minutes for the four"
(the script header's); AT-SPI's "this line said it "is already true here"", "A walker that printed only
names would have shown", "— 242 `Form` elements", "And since ADR 0425" (and three like it), "and is
where ADR 0623's "three of nine" came from"; confined_wire's "**It reads `MAGIC` out of `protocol.rs`
since the seven-hundred-and-thirty-sixth**", "**A question this script does not ask now stops it**,
since the eight-hundred-and-sixteenth", "**The length was a million until the
eight-hundred-and-twenty-first**"; display_list's "the second target whose input is a process", its
hand recipe and "841 MB"; cms's "which since the eight-hundred-and-twenty-fifth", "**Point it at the
whole disk rather than at `doc/pdf.js` alone**, which is what this line used to say", "Clean at 1 000
000 in the three-hundred-and-seventy-seventh"; revocation's "neither of which anything in this tree read
before"; x509's "Since the six-hundred-and-eighty-ninth", "which since the eight-hundred-and-twenty-first",
"added in the six-hundred-and-eighty-ninth session", "Clean at 1 000 000 in the
three-hundred-and-ninety-second"; ccitt's "it was the one target here without a line"; "the twenty-one
functions"; "among the sixteen"; "44 edges to fresh seeds' 594"; "runs about twice as fast"; "peaked at
2.15 GiB over 3 340 s"; "~7 s"; "~4 s over all fourteen documents"; the oracle's "(46 pages in session
21)", "~95 s and writes 319 MB", "~30 s when that was written", "99.7% hit rate", "session 166 saw it at
85.7%", "*271 s ours, 90 s*", "(3.2 → 5.0 s)", "the 974 first pages", "in 0.2 s"; "pinned at v6.1.200"
and "the 974 PDFs" (now the commands); "**What it said in the seven-hundredth session**"; "**And since
the eight-hundred-and-sixteenth it prints**" and "eight of the fifteen targets".

**`doc/ui-boundary.md`**: the status block from "Status: **built, shaken out, and frozen** — seven
consumers" through every "**And the four-hundred-and-fourteenth added …**" to "**And the
one-thousand-two-hundred-and-first …**", with "**The thousand-and-seventy-sixth added one field**",
"**And Table 172's `/RC` added one field**", "**And the same round gave**" (twice), "**And it added
nothing at all for §12.9's rubber band**", "**And `Command::Archive` and `Event::Archiving`**", "**And a
script's timers**", "**And a script's keys**" — each now one line of four lists (messages added, shapes
changed, capabilities that added nothing, the mechanisms preferred to a message), its clause, test and
ADR kept; "N consumers failed to compile", "`QUORRA_EVENT_KIND_COUNT` moved 16 → 19", the wire's kind
numbers and "gained its hundred-and-twelfth entry point" (now a `grep` of `quorra.h` and `tools/state.sh
hosts`); "This was `doc/HANDOVER.md`'s section 0 until"; "**Everything a viewer still owes was blocked on
one missing interface.**"; "#### Why it was the headline" and "Five owed items were the same item" (all
five done); "was amended in the hundred-and-thirtieth session"; "and since the
seven-hundred-and-seventy-fifth"; "the defects it found when it was written were all a host's own";
"**What the third host cost.** … 112 `#[unsafe(no_mangle)]`"; the variant lists (now the source files);
"**`Layout` is the fourth policy value**", "**`Present` is the third**", "**`Restrict` is the one policy
value**" (one sentence); "a line ADR 0734 recorded as a tension"; "147 of the corpus's first-page
widgets"; "974 corpus documents digested"; "`doc/todo/37` was the audit that found it"; "**The vocabulary
is complete**, and ten sessions of building on it"; "**`Command::Report` was added in the
one-thousand-and-twenty-seventh**"; "So what is left of §0 is **hosts**" (todo 30–32); "**The panels
this file called "the largest single thing this project owes"**"; "**The one thing it needed that did
not exist is worth naming**"; the `viewer-render` and `viewer-gpu` bullets; "**Search landed in the
four-hundred-and-fourteenth**"; "'the whole of it' was true when ADR 0346 wrote it"; "which is how eleven
queries came to reach no symbol"; "and which the edit log joined in the hundred-and-thirty-fifth
session"; "1665 pages"; "**This paragraph used to add**"; "**ADR 0244's ≈3.2 GB/s was a first-frame
number**"; "sampled back out of the window as `srgb(187,227,248)`"; "The read-back is 0.34–1.18 ms"
(ADR 2049's); "68 corpus documents understate"; "the six corpus documents"; "+1.69%" (ADR 0118's); "It
dissolved exactly as predicted"; "decided in the seven-hundred-and-twenty-fourth session"; "**And it is
wired in since the seven-hundred-and-thirty-sixth**"; "eleven questions"; "about 2% of a raster at the
median" (ADR 0607's); "a sparse page's open falls from 8.7 ms" (ADR 0640's); "Form-field editing landed
in the hundred-and-thirty-fifth session".

**`doc/third-party-data.md`**: "ADR 0133 cites "`HANDOVER.md` §1" and means this."; "as of the
eight-hundred-and-eighty-seventh session … MIT from the hundred-and-thirtieth"; "`deny.toml`'s
allow-list dropped MPL at the first move"; "What the move costs the graph is nothing"; "One obligation is
new"; "in the order this section used to prescribe"; "804 KB", "all 239", "3.9 MB blob"; "15 documents
left the incomplete list"; "the 40 fonts naming an `Identity` ordering"; "the two-hundred-and-fifty-sixth
session opened the pictures"; every "taken in the three-hundred-and-seventy-sixth" to "since the
four-hundred-and-eleventh" with its package count ("**61 packages**", "brings **31**", "bring **28**",
"213 lines", "**41 packages**", "**23 packages**", "Every one of the 23", "a first for this record", "a
second first", "a third first"); "which this row read as one until the five-hundred-and-fifty-eighth";
"before this session or after it"; "examined and declined in that session … The
four-hundred-and-sixty-seventh read every sidecar"; the format-corpus table's file counts and sizes; "It
used to be left on that *and* on value"; "73 MB checked out" (now `du -sh`); "and it has not yet been
reached"; "fetched for the first time in the eight-hundred-and-fifty-fifth session"; "beside the
three-hundred-and-ninety-second's", "200 lines in tree", "`Cargo.lock` did not move"; the holding
sentence repeated per text ("`/doc/*.pdf` excludes it, `python3 tools/spec-md.py …`" in each of eleven
sections, "free to obtain is not free to redistribute", "**The licence position is ADR 0187's,
unchanged**", "the same discipline `doc/pdfa/`'s reprints … already impose" twice, "`/doc/*.pdf` and
`/doc/md` were already ignored") — now one paragraph after "Provenance is a principle-4 question"; "Until
now this tree carried only ISO 32000-2"; "three agents had recorded"; "That is a stance the project
already took"; "Two agents had declined those rules"; "about 5 300 words … exactly zero times" (now a
`grep -c`); "since then"; every page and word count ("334 pages", "126 pages, about 54 000 words" and
five more, "145 pages", "1584 pages", "180 pages", "186 pages", "6.4 MB"); "in the
nine-hundred-and-fiftieth session" (twice), "in the same session", "in the
nine-hundred-and-forty-sixth session"; "after all, which was an open question"; "fetched on 2026-09-22
into the gitignored `doc/`"; "273 witnesses", "274 property names across 14 schemas"; "the tables are
treated with the caution"; "Fixed where it acts", "A citation was wrong and is corrected"; "That is the
position the ISO 16684-1 and ISO 15076-1 previews above are held under"; "The route this replaces did
not work"; "twenty-eight items", "It is what session 941 acted on", "as it did in sessions 939 and
940"; "the situation has since improved rather than changed"; "122 XML fragments"; "**2 908
documents**" (now a `find`); "fetched the same day"; "for the first tranche"; "Nothing was taken and
nothing was read" (kept as one line); "after the ITU server had answered `500`"; "All 195 run-length
codes"; "after the catalogue's direct PDF link had answered `500`"; "`cargo deny check` passes with
`pdf-font` naming it" (now `doc/stack.md`).

**`doc/stack.md`**: "which decoded fax until then"; "It adds 57 packages to the lock" (now `cargo
tree`); "The ten modules were `pdf-model`'s until the thousandth session"; "four of the corpus's ten
signature values", "refuses exactly those four"; "measured 2026-08-23, and ADR 1063"; "**23 new locked
packages**"; "Two more curves were takeable the week their line went stable"; "the one lock change is
`hybrid-array`"; "a dozen object identifiers"; "It also gave `cms::Digest` a second reading for six of
its ten"; "The one dependency that was owed is spent, and cost twice"; "What this file got wrong was the
package count"; "The lesson generalises past this row" (the rule kept).

**`doc/todo/57`**: "Status: **open**, and no longer on a branch: `round-867` is merged"; "§7.5.7's
producer half is written by `optimize`'s object streams"; under "What is done", the eight paragraphs
"RFC 0002 §14's first landing", "**The amendment, the serializer, and `split`**", "**`merge`, and one
clause per reconciliation**", "**`pages`, and what a rotated raster cannot be asked**", "**§14.7's
structure tree, carried by all three verbs**", "**`optimize`, and §7.5.7's producer half**", "**What a
piece still says about its pages**", "**Annex F (ADR 1293)**" (each now one line with its ADRs, the
capability in `state-of-play.md`); "is the short list it is, because the rest of it is carried"; "Over
the 905 rotated pages the worst mean falls" (the `pages_corpus` walk prints it); "five levels of 255",
"0.5000 at its worst"; "## 2. The RFC 0003 hand-off"'s "The owner's sequencing, 2026-09-03", "**Taken**
(ADRs 0840, 0841", "That is the whole cost of the hand-off" (one paragraph, taken); "`pdf_transform::
Operation` lives in `pdf_model::restriction`" (to the Clauses line); "Taken when the verbs settle"
(overtaken by A02); "this section used to be five paragraphs arguing for it"; "That is ADR 0835 §5's
prediction"; "`issue19517.pdf` costs poppler 23.9 s"; "**`optimize` is in the readback**", "The
rewritten file is the one output whose whole *point*".

**`doc/todo/30`**: "under the bound too wherever the toolkits' rule holds" (one sentence with its ADRs);
the panels bullet's "`quorra` setting each run in its own face", "`quorra-qt` the scales through a
document", "all three lay a right-to-left paragraph's tabs leftward", "read from Pango's layout",
"a script that changes nothing drawn", "`quorra`'s drawn list takes Up and Down" (one sentence, every
ADR kept); "First the device renderer leaves `quorra`'s binary for a library" (built).

**`doc/todo/01`**: "**§12.10.2** — every projected map in the census"; "§12.10" from the headings
bullet; "It was given to two proposals in turn"; "— the run that found `Query::Find`"; "**§12.8.2.4's
transform, named rather than built**" (built); the hand recipe "`git worktree add --detach`".

**`doc/todo/56`**: the status's members-by-ADR list (the capability list's); "The premise this file was
commissioned to test"; "Priority: 56 … the exclusion in `CLAUDE.md` amended"; "**Both questions this
file left are answered.**"; §2.4's "This is not the obstacle it looks like"; §3's "This file's commission
anticipated", "the single sharpest reason this file's commission asked", "The word counts say the same
thing … 95 times" (now a `grep`), "This is the part worth the words", "every one of them was being
resolved against ISO 32000-2", "this round priced it", "§9's step 4 asked a round to acquire the
standard", "which is the pinning rule below proving its worth", "without the repository ever being
named", "has had for four hundred rounds"; §4.1 whole, with "86 clause-13-multimedia", its row table and
the two quoted ledger notes (now the state and a ledger `grep`); §4.2's three count tables ("276", "18 /
57", "34.5%"); §5's "Today this program draws the raw value", "Note that these are `shall`-executed on
open", "99.6%", "253 `.unwrap()`" (now a `grep` over the pinned crate); §6's "When that sentence was
written this tree had no sandbox", "*Against*: a third worker is a new part", "But this is a design
decision and belongs in an RFC", the "What it would be allowed to reach" list; §7's reasons 1–4 (one
sentence), "**Formatting only**" (prose), "That step is judgeable", "Put together:", "So §7's ordering
stands", "**And if the owner declines**"; §8's exclusion as it stood, "What has changed since it was
written", the quoted Table 218 sentence, "**The dependency, measured**", the proposed amendment, "The
owner decides." (now "Ratified", A286, ADR 1904); "79 documents"; "A Tier 1 member not yet bridged"; §9's
steps 1–5 and "~~Acquire ISO 21757-1:2020~~" (its three obligations kept as rules).

## 4. What stays as it was

`doc/adr/`, `doc/history/`, `doc/reviews/` and the owner's `A`/`Q` files. Every command line of
`doc/verify.md` but the drive's usage line, and the 198 example stems it names. Every capability of
`state-of-play.md`; every ADR the old texts cited is cited in the new or in another document. No
sibling's record or ADR. Q540 was not needed.
