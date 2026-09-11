# Habits: measuring

Status: **standing** — method, not code.
Read by: a round that measures anything — a benchmark, a profile, an A/B, or a price it is about
to write down. `doc/performance.md` is what is already known, `doc/verify.md` the instruments
that are not §2 gates, and `tools/state.sh` prints what may not be quoted from a document.

`doc/habits.md` is the index of the six, and it states what a habit is and what each keeps.

- **A datum's readers are not bounded by the crate that carries it, and one of the readers a census
  names may not be one.** `data/standard-fonts/` was recorded as having three readers inside
  `pdf-font`. It has two — the third was a file *extension*, `.pfb` on ten programs that are bare
  CFF — and two more outside that crate, in `viewer-ui`. **The grep that gets this right is over
  the path, not over the module**: everything that names the directory, in every crate, plus
  everything that names the constant it is compiled into. A reader list assembled from one crate's
  imports is a list of that crate's readers (ADRs 0971, 0980). This is the sharpened form of the
  finding that a census built for one reader of a datum proves nothing about another — `data/cmaps/`
  had two readers and one census, and the parser nobody had counted dropped mappings in silence.

**A price is a claim, and it decays like the others.** This project already holds that a ledger row's
*reason* decays, that a claim about the standard's silence decays, and that a note's *count* decays.
The estimate a round writes when it measures a piece of work and **declines** it belongs on that list,
and the six-hundred-and-fifties found two inside one week, in opposite directions. ADR 0474 priced an
exact edge coverage at *nine `scan::fill` calls where there is one* and left the item open for that
reason; it was **one call**, because `tiny-skia` already contained the nine pieces, and the built form
is faster. ADR 0469 priced a shading's transfer function as an existing walk redone one crate along;
that design would have been **wrong by 64 levels of 255**, because a simplifier three hundred sessions
older had already dropped the curve the walk assumed.

Both prices were written by rounds that had measured carefully and were right about everything else.
What moved underneath them was a *third* thing neither was looking at. So: **a round that takes an
item priced by an earlier round re-derives the price before believing it**, exactly as it re-derives
a count or a population — and the cheapest re-derivation is asking what the libraries and the layers
*already contain*, because that is the part a pricing round tends not to enumerate.

**Four shapes of wrong price are known now, and the last two are the ones to watch for.** ADR 0474's was
too high because a library already held the pieces; ADR 0469's named the wrong *place*, and would have
shipped a 64-level error; and ADR 0489's was **one price quoted for two cases** — a group's shape and
its alpha are one number wherever its opacity is 1.0 throughout, which is decidable while the content
stream runs, so the shape channel it priced was the cost of the *other* case only. A price that names
no condition is a price for the hardest case, charged to every case.

**ADR 0570 is the third shape again with a tell worth having.** §11.7.5.2's per-region transfer
function was priced as a per-pixel channel and a pass in all three backends, and half of it needed no
pixel: the clause offers exactly two candidates at any point, the topmost object's function and the
page's default, so *may this object's function ever be chosen* is answerable from the object alone
and only *what is the composite here* needs a point. **The tell is that the clause's own sentence
separates the two and the price did not** — its subject is "the last (topmost) elementary graphics
object enclosing that point", and the qualifier that makes half of it cheap, "but only if the object
is fully opaque", attaches to the object rather than to the point. So when re-deriving a price, read
the clause for the question it is *two* of, and check that the price has as many numbers as the
clause has subjects.

**And a *negative* claim decays when the population grows**, which is not the same decay as any of the
others, because nothing in the tree changes. The six-hundred-and-fifty-fifth found the ledger saying
"no corpus document writes one" of a shading pattern's `/ExtGState`: true when written, and measured
over corpora with no crawl in them. `doc/pdf.js` holds 38 Type 2 patterns and **zero** state one; the
crawl holds 1504 and **42** do. A sentence of the form *no document does X* carries its population
inside it whether or not it says so, so **the round that adds a population owes a re-derivation of
every negative measured over the old one** — and `doc/todo/03` now holds whole-population figures
precisely so those are answerable rather than inherited.

**A fourth shape: a price that names the wrong *mechanism*, and is therefore right about the cost and
wrong about the fix.** `doc/todo/41` priced "a refusal whose encoded bytes outgrow the budget" as a
thing to hold, and the seven-hundred-and-twelfth session measured it at **25 000×** — real, reachable,
and still not a memo's problem: what makes that document expensive is that reaching the refusal
inflates a gibibyte, so windowing the chain removes the cost on *every* read where a memo removes it
on every read after the first, and needs no entry, no charge and no eviction argument (ADR 0586).
**The tell is that all three ways to take the item as posed gave something up** — the ceiling, the
cache, or soundness — and when every construction of a fix is bad, the fix is in the wrong layer.

**An A/B that raises a constant can be stopped by a *second* constant, and the picture then lies
about which one moved.** The nine-hundred-and-forty-fifth session priced a mesh's tessellation by
rendering every corpus mesh page at `PATCH_STEPS` 10 and at 60 — and the finer arm came back at a
mean 6.81 of 255 with a person and a dog missing their limbs, which reads as *ten steps is far too
coarse* and is the opposite of the truth. What the finer arm had done was run the same document into
`MAX_TRIANGLES`, eleven lines further down the same file, whose units are *this program's triangles*
and whose headroom therefore falls as the square of the fineness. With that bound lifted, the same
A/B puts the page at 0.0029. **So before believing an A/B on a constant, ask what else is counted in
a unit that constant is a factor of** — and the tell is that the arm you expected to be *better* is
the one that looks broken. ADR 0961; trap 1 is what caught it, because the number alone said
only that the two arms differed.

**And a price measured on the file can miss a multiplier that only the *caller* has.** ADR 0585's
population was right — most images run a filter in front of their codec — and the number that
decided the work was found by reading `pdf_model::content::image::draw` and counting: one `Do` asked
`Document::image_stream` **four times**, three of them reports that decoded the chain before asking
which codec it was. No profile names that as redundancy and no census of the corpus can see it, because
it is a property of this tree. So: **when pricing a memo, count the calls before measuring the misses**
— the redundant call is the cheaper defect to find and often the larger one, and on a thousand-page
sweep removing it beat the memo outright (−2.35% against −2.06%).


- **A negative answer from a tool is a claim about that tool, not about the world.** `which
  cargo-fuzz` reports nothing here because `~/.cargo/bin` is not on `PATH`, and two consecutive
  rounds wrote "`cargo-fuzz` is not installed on this machine" into an ADR and a todo — leaving a
  fuzz target unwritten — while the binary had been on disk since 26 July and `doc/environment.md`
  said how to invoke it. The check that would have cost two more seconds is
  `ls ~/.cargo/bin`. Session 428, ADR 0264.
- **`grep` over a corpus needs `-a`, and without it a census of PDFs reports zero.** These files
  are binary, and grep's own binary detection suppresses the match — `grep -rl FieldMDP
  doc/pdf.js/test/pdfs` prints nothing while `grep -arl` prints `xfa_filled_imm1344e.pdf`, with
  `-l` and a recursive walk and no other difference. A ledger row spent thirty-one rounds saying
  the corpus stated no §12.8.2.4 transform on the strength of the first form, and the document it
  missed is the corpus's **one certification signature** — the file that row's own neighbours cite
  by name. This is the counting half of the rule above it: a negative answer from a tool is a claim
  about the tool. **The independent check is this tree's own reader**, and it is the one that found
  it; where both are available, run both, because the reader is the instrument under test and the
  grep is not. Session 568, ADR 0403.
- **A negative claim about a corpus names a population, and half of ours did not.** The round after
  the one above re-ran every "no corpus document does X" sentence in the tree — about 165 of them —
  and **five were false**. Two were stale counts (§14.7.2's `/IDTree` was "none" in three places
  while the crate's own reader said twelve; §14.10.2's `/SpiderInfo` was "none" in one ledger row
  and "5 documents" in another row of the *same file*). Two were neither stale nor miscounted but
  **scoped**: §12.4.3's articles and §12.3.5's collections are absent from pdf.js and present in the
  four submodules under `doc/corpora/`, which every other absence claim in this tree is measured
  over. So the sentence was true of one population and written about another, and no re-reading of
  it could say which. **Write which corpus.** Session 570, ADR 0405.
- **A grep with `-a` is still a byte search, and a PDF hides its names from one.** ADR 0403's fix
  makes `grep` see the file; it does not make it see a name inside a §7.5.7 object stream or a
  content stream, which is where a modern producer puts most of them. `/SetOCGState` is the clean
  demonstration: two corpus documents state it and `grep -a` finds **neither**. So a census over
  names walks the *objects* the cross-reference table names — `examples/witness_census` prints the
  raw, object and stream counts side by side for exactly this reason, and four of the twelve
  `/IDTree` witnesses are visible only in the middle column. "Run both" means the reader and the
  grep where a reader exists, and the object walk and the grep where one does not; never the grep
  alone. Session 570, ADR 0405.
- **Ask the linker what a binary can execute.** `nm <binary> | grep -c <symbol>` answers "could
  this program ever run that code, under any input" with no fuzzing, no coverage build and no
  argument — it found that eleven of thirteen fuzz targets did not contain the function that
  crashed, which is a stronger statement than any of them could have made about *reaching* it.
- **This machine has two classes of core, and an unpinned duration is a draw from a lottery, not
  a measurement.** `doc/environment.md` names the processor and holds the one-line command that
  says which core is which; what follows for a measurement is here. A **fixed, serial, in-memory** piece of this
  tree's own work — a small document opened from bytes already in memory, its first page
  interpreted — measured **0.75 ms in some processes and 1.50 ms in others, on a machine whose
  load average was under two**. That is not noise an average removes; it is bimodal, and a band
  wide enough to hold both ends is wide enough to hold a regression. Every wall-clock figure this
  project has ever taken, `doc/performance.md`'s launch timelines included, was drawn that way
  without anybody knowing. **So a duration that has to be compared with another duration is taken
  pinned** — `taskset -c` on a list *derived* from `cpuinfo_max_freq` rather than written down,
  because a list of CPU numbers is a claim about a machine — and **the minimum of several fresh
  processes**, since contention adds time and never removes it. Pinned and minimised, the launch
  gate's spread fell from 100–400% to 0.6–22%. Session 922, ADR 0884.
- **A wall-clock gate needs something that says whether the machine is worth believing, and it
  belongs in the gate.** The same probe as above, in a child of its own, with a band: outside it,
  the gate prints every figure and judges none, saying why. Checked against the defect rather than
  assumed (trap 13) — eight busy threads on the gate's own cores put twelve of twenty-eight
  figures outside their bands, and the probe said so first. The corollary is the part that makes
  it a gate rather than a decline: **find the figures with no clock in them and judge those
  always.** Bytes read and peak resident came back identical in forty-four runs, and they are what
  gates the claims that matter most. Session 922, ADR 0884.
- **Wall-clock benchmarks lie under load; count instructions instead.** One change measured as a
  24% regression and an 8.5% improvement twenty minutes apart. **A/B in one sitting**, and measure
  the baseline on this machine rather than trusting a number in this file.
- **One sitting is not enough when the arms are minutes apart and the answer is a duration — take a
  ratio *inside* each run.** Some things cannot be counted with callgrind: a window's response to a
  gesture is two threads, an event loop and a display server, and instructions do not answer it. The
  seven-hundred-and-forty-ninth session measured one such thing over four alternating runs while the
  load average went from 3.7 to 72, and in **seconds** the change read as a small regression — the
  second *before* run came to rest in a median of 0.89 s and the first *after* run in 0.92. The
  frames those runs were made of cost 0.39 s and 0.59 s, so the comparison was of the machine. The
  same runs expressed as *how many frames of work the window waited*, each divided by its own run's
  frame cost, separate completely: every before cycle above 2.18 and every after cycle below 1.68.
  **So when the quantity is a latency, find the unit the run itself provides** — a frame, a refresh,
  a page — and divide by it; a ratio between two durations of the same run has no machine in it.
  ADR 0657 section 7.
- **A baseline checkout carries less of the machine than the round's own worktree does, and the
  difference reads as the round's change.** `tools/worktree.sh` links six gitignored things into a
  round's tree; a `git worktree add` at the base commit, taken by hand for a before/after
  comparison, links none of them — and the tree also holds `doc/*.pdf` and `doc/corpora/*`, which
  the script does not name and which no `git` command will restore because they are gitignored or
  submodules. The eight-hundred-and-thirty-second session's `--bin pointers` run moved **113
  pointers** between *live* and *not carried* on the specification PDFs alone, with the round's
  change nowhere near them; the shape it takes is a defect class shrinking, which is the reading
  nobody questions. **Diff the two checkouts' walked file sets before believing the sweep** — a
  dozen lines of `os.scandir` — or link `doc/*.pdf`, `doc/corpora/*` and everything
  `tools/worktree.sh` lists, and rerun. ADR 0760.
- **Take the *before* half with a patch file, never with `git stash`.** `git diff > round.patch`,
  `git checkout HEAD -- <files>`, measure, `git apply round.patch` — three commands that touch
  nothing outside the worktree. Why the stash is forbidden here, and how to recover one that has
  already gone wrong, is `doc/environment.md`'s; this entry is the measuring half and the two used
  to carry the incident twice, dated to two different sessions.
- **Pin the pool before counting a serial change in a program that has one.** Callgrind counts
  every thread, so a work-stealing pool's *spin* is in the total and it is not deterministic.
  `open_one` on two corpus pages read **+0.154%** and **+0.010%** for a change that removes an
  allocation and can cost nothing; the diff was `crossbeam_deque::Stealer::steal` +1.24 M and
  +5.11 M, and with `RAYON_NUM_THREADS=1` both pages read −0.003%. This is the converse of
  `doc/performance.md`'s older rule — *quote the clock for a parallel change and the counter for a
  serial one* — and the converse is the direction that produces a phantom regression rather than a
  phantom win. Session 500, ADR 0335.
- **Attribute a regression by removing the suspect, not by reading the profile.** The profile shows
  the *shape* of the extra work, not its cause; one stubbed field said 96 of 110 M.
- **A crash met on one document is attributed to whichever of its properties you already have a word
  for.** Session 916 found a confined worker killed on `bug1815476.pdf` while four committed fixtures
  survived, and wrote down the difference it could name: encryption. It was a font — the document
  names a face it does not embed, and `pdf_font::substitute` walks `/usr/share/fonts` (ADR 0870).
  Nothing about that reading was careless: two populations of four and one differ in every way, and
  the property the finder picks is the one they were working on at the time. **The instruments are a
  minute's work and they name the cause rather than a correlate** — the kernel's own audit line in
  `dmesg` gives the system call number (`sig=31 … syscall=257`), and `strace -ff -e trace=openat` on
  a process you start yourself gives the *path*, which is what settles it; `yama/ptrace_scope` at 1
  restricts `ptrace` to descendants and does not stop that. And the durable answer is the population:
  a sweep over enough documents for the classes to disagree said the same defect kills more of the
  *control* class than of the encrypted one. Sessions 916 and 917, ADRs 0876 and 0877.
- **Two spellings of one loop are two programs, and the difference can exceed the optimisation you
  came for.** A fused number parse that removed a whole pass over every token measured as a **+1.1%
  regression** written as `for &byte in body { … _ => break } read += 1;` and as **−5.4%** written as
  `while let Some(&byte) = body.get(read)` — same function, same arithmetic, byte-identical answers,
  **750 M instructions apart**, 6.5% of the page, and 454 M of the difference sat in code callgrind
  could attribute to no source line. A slice iterator that must also report where it stopped is two
  cursors the compiler has to keep in step. The suspect was eliminated by *removing* it — capping the
  slice at seventeen bytes made it worse, so the slice length was not the cause — rather than by
  reading the disassembly. **So an optimisation that measures badly has two hypotheses, not one**:
  the idea is wrong, or the spelling is. ADR 0370 met the same thing from the other side, where
  replacing an index *with* an iterator measured +2.64%. Session 589, ADR 0424.
- **A gate's own printed timings are only as good as what else is in its process.** The oracle's
  `processor time` and `slowest pages` rows read a factor of two high for thirty-nine rounds,
  because a second test in the same binary was walking the same corpus under `rayon` beside it, and
  the report is built from per-page `Instant` spans. One page read 93.2 s where the same work alone
  is 5.4–6.7 s, and a todo quoted the inflated row as evidence for the regression it was a symptom
  of. Nothing could see it, because a self-timing gate has no second reading to disagree with.
  Session 447, ADR 0282.
- **Check that the cheap probe moves the right way before bisecting on it.** Cheap and *sensitive
  to the thing you are looking for* are different properties. `doc/todo/43` named the corpus gate
  as the probe for a slowdown because it "takes seconds"; it is **faster** at the bad end than at
  the good one, so a round following the instruction would have concluded there was nothing to
  find. One sample at each end of the window, before the first bisect step, costs a minute.
- **When a page's error has a suspiciously round size, do the arithmetic.** Seven pixels of
  gradient where there should be an edge is 1800 ÷ 256.
- **Profile before believing an explanation, even one whose arithmetic matches.** A 48-second page
  was attributed to clip masks with `3576 × 485 kB = 1.7 GB`, exactly the memory held and silent
  about the time: callgrind put the masks under 4% and the gradient at 78.9%.
- **A suspiciously clean measurement is a reason to check the instrument.** Four callgrind numbers
  flat to four significant figures meant the benchmark was panicking and callgrind was faithfully
  counting the panic. **Second instance, session 161**: our ink measured at exactly half the three
  C renderers' on ten pages running, 2.00 to three significant figures, which is not what hinting
  does. Our renders and `hayro`'s carry an alpha channel and the three C ones do not, and
  `magick -colorspace Gray` was averaging alpha in as a fourth channel — so **the tell was that
  the two renderers agreeing with us were the two whose output *format* matched ours**. Ask what
  the agreeing group has in common besides the answer.
- **A lesson recorded where it was learned and not where it is *used* has not been recorded.**
  The paragraph above was written in session 161, in this file and in `CONTRADICTED_GLYPH_EDGES`.
  The recipe in `doc/todo/00-ambiguous-bucket.md` — the file a session opens when it goes hunting
  — still carried the broken command, and sessions 197 and 199 followed it and drew the same
  wrong conclusion twice, with the same 2.00 ratio in front of them. Both ADRs carry the
  correction now and the recipe is fixed. **When a habit lands, ask which document a person will
  be holding when they need it.** ADR 0163.
- **Look at the heatmap's shape before opening anything else.** Twelve of the oracle's fourteen
  unexplained pages were diagnosed in two sessions without a debugger: a heatmap that is the
  whole silhouette says colour, one that is glyph outlines says grid-fitting, and the ink table
  then says which. Both are three minutes per page against a list that had not moved in twenty
  sessions.
- **Measure the instrument before deciding you are slow.** Eleven sessions treated the oracle's 85
  seconds as the price of having an oracle; 95% was three programs re-answering a question.
- **Measure before optimising, and delete what does not measure.** A `FontRef` cache changed a dense
  page by less than noise and was removed with the reason recorded; the same session's real win was
  hoisting a string allocation, 1.37 ms → 18 µs.
- **An eager lookup on a cold path is a hot-path cost when the path runs per object.** Reading
  `/AcroForm` per constructed appearance was 2.7× the whole feature's cost.
- **A cost written down beside one call is not a cost anybody adds up.** `Pages::index_of`'s doc
  comment says it is a search that cannot skip a subtree and names the two callers it was written
  for; a third arrived, called it *in a loop over 988 outline items*, and inherited the comment's
  blessing without its argument — 344 ms of every page turn. **Ask of any function documented as
  expensive: who calls it in a loop.** One `grep`, and it found a second (`named_page`). ADR 0124.
- **A failed frame must not be reported as a drawn one.** `viewer-ui` answered
  `Rendered::Presented` when its GPU path refused a page, so the core recorded the page as shown,
  never asked again, and the window kept the *previous* page under a title bar naming the new one
  — a page a person cannot view and no reason given. It now answers `Rendered::Failed`, draws it
  on the CPU backend instead (which is what `CLAUDE.md` keeps that backend for), and says which.
  **And a refusal is recorded as an answer**: the scheduler must not re-ask a question whose
  answer cannot change, or the two spin. ADR 0125.
- **A performance defect on a path no gate walks is found by a person using the program.** The
  corpus interprets page one, the oracle renders pages it is handed by index, and neither turns a
  page. The largest document this project owns — ISO 32000-2, 1023 pages, committed in `doc/` —
  was in no gate at all until session 141 made it two tests.
- **Look at what a safe idiom compiles to in a loop that runs per pixel.** `.round()` on a clamped
  float is a library call — 205 M instructions on one page, 10.7%.
- **The exact fix is often available and is usually better than the approximate one.** A memo keyed
  on the input tuple beat an interpolated lookup grid: 3249 M → 1075 M, and simpler.
- **A change made for correctness that is also an order of magnitude faster means the old code was
  doing work that was worse than useless.** One mesh raster replaced 4096 flat pieces: 35.47 G →
  3.08 G, and closer to the references.
- **When the first design of a fix is the obviously safe one, still measure it.** Refusing to cache
  timeouts is unarguable in principle and left two pages accounting for 46 of 57 seconds.
- **A ratio measured on four pages is a fact about four pages.** ADR 0137 counted 1.01–1.13 strips
  touched per command and concluded duplication was not the problem, which is true of those four;
  `issue12841_reduced.pdf` is *two* commands each covering the page, so sixteen strips replay both
  sixteen times. Computing the same ratio per page is one function and is what made the split safe
  to ship. **Ask of any measured constant whether it is a property of the thing or of the sample.**
  ADR 0139.
- **A function only an example calls is a function nobody has measured.** `command_extents` rebuilt
  every command's clip chain from the leaf: 606 ms on one page, six times its whole rasterisation,
  correct and unnoticed for two sessions. **Before moving code onto a path a person waits on, time
  it there.**
- **A priced item names a loop, and the loop it names may not be the one the file takes.** The
  handover priced "colour-managing an image in parallel" for thirty sessions on
  `issue19971.pdf`'s photograph. `image::unpack` is the per-sample conversion, it is obviously
  the loop, and a JPEG does not enter it: `zune-jpeg` writes components into the raster and
  `convert_channels` converts that in place afterwards. Parallelising the obvious one measured as
  noise. **Before optimising a named function, check on the named file that it runs** — one
  `callgrind_annotate` would have said so before the change rather than after. ADR 0147.
- **Ask what a parallel unit's answer depends on before asking how to divide it.** A colour
  conversion is a function of one pixel's samples, so a band boundary changes which conversions
  are *repeated* and never which answer is given, and the split is byte-exact at any band size.
  A rasterisation is not a function of one row's geometry — a curve clipped by a strip's edge is
  re-parameterised — which is the whole of ADR 0138. The two look like the same problem and are
  not.
- **A serial pass over every pixel is what bounds a parallel render**, and it hides inside a
  function whose cost nobody attributed: `impose_on_medium` was 7.8 ms of a 17 ms page, all of it
  eight integer divisions per *transparent* pixel — and §11.4.7's isolated page group makes most of
  a page transparent. Amdahl's law names where to look after any successful division. ADR 0139.

- **A check deferred for cost belongs wherever the cost is already being paid, and nothing tells
  you when that place appears.** Table 45's `/CheckSum` was read and not verified for eighty-three
  sessions, on a reason that was true — "checking would mean inflating every attachment" — and
  that expired the moment one path decoded one stream. The clause names where it belongs:
  "the checksum of the bytes of the **uncompressed** embedded file". **After a session that
  makes something decoded, decompressed or laid out for the first time, re-read the entries whose
  reason for being unread was that nobody had it yet.** ADR 0145, and it is ADR 0108's regular
  expression looking for a different kind of blocker: not "needs §X" but "would cost too much
  here".

- **The three sweeps found a fourth shape in the hundred-and-ninety-first, and it is the
  strongest one yet: a row whose "this program has no ___" was about a *verb*.** §12.8.6 said
  a usage-rights signature grants "features of a PDF processor that are not available by
  default" and that "this program has no feature behind such a gate"; §12.8.2.3 said the same.
  Both were true when written and both stopped being true in the hundred-and-thirty-fifth and
  -sixth sessions, when this program learned to fill in a field and save the file — which are
  exactly the rights Table 258 grants and exactly the changes Table 257's `/P` restricts. And
  the requirement was not a new one: §12.8.2.2.1 has always carried, in a parenthesis, "(These
  changes to the document shall also be prevented if the signature dictionary is referred from
  the DocMDP entry in the permissions dictionary.)" A `shall`, addressed to a processor that
  modifies, unread for fifty-six sessions after this one became one. `ViewState::set_field` now
  refuses at `/P` 1 and permits at 2 and 3, and §12.8.2.3's `should` — remove a UR signature
  the modification exceeds — is named as owed. **After a session that gives the program a verb,
  the rows to re-read are the ones whose reason is about what the program *is*, not only about
  what a clause needs.**

- **Sweep for the reason's *shape*, not for its clause.** Sessions 118 and 122 grepped the
  ledger's notes for "while §X does not exist" and for entries claimed unread. The
  hundred-and-seventy-fourth grepped for a third shape — "this program has no ___", "no panel",
  "which this is not" — over `partial`, `reported` **and** `inapplicable` rows, and found
  §12.6.3 saying "[n]othing raises an event … this crate has no events", which stopped being
  true in the hundred-and-thirty-second when `Command::Pointer` landed. Forty-one sessions. The
  three sweeps are twenty lines of Python apiece and each has paid on its first run.

- **An `inapplicable` row whose reason is "this program has no ___" is a row waiting for a
  session that gives the program one.** §14.3.3 was `inapplicable` because "a viewer with a
  document-properties panel would read it; this one has no panel", and the panel arrived seven
  sessions before anybody re-read the row. That is the second instance after §12.7.4.2's field
  names, and the trigger is ADR 0122's: **after a session that adds a capability, sweep the rows
  whose reason begins "this program has no"** — `inapplicable` as well as `partial`, which the
  earlier sweeps did not cover.

- **Read the whole sentence a feature is built from, and count what the other half is worth
  before deciding.** §12.3.3 says a click makes a processor "jump to a destination **or trigger
  an action**"; the hundred-and-sixty-sixth session built the jump and shipped a `Command`
  variant shaped exactly like half a sentence. Two sessions later the count — 281 corpus outline
  items with an `/A`, 32 of them not a go-to — said the other half was one refactor away, and the
  variant became a path nobody takes and was removed. **A command shaped like half a clause is a
  command that will be replaced**, and the habit is ADR 0110's one level up: where a rule lists
  what it applies to, count them against the code *before* designing the interface.

- **A gate that cannot see a surface is a gate that cannot see a surface.** The corpus
  interprets page one, the oracle rasterises pages it is handed, the text gate reads words and
  the date gate reads strings — not one of them opens a viewer, so every line of chrome this
  project draws is unwatched by all four. `viewer-ui/tests/panel.rs` answers it the only way that
  discriminates: rasterise the panel's own display list with `render-cpu` and *count ink*, then
  delete the glyph drawing and check the count goes to zero. A test that asserted the display
  list held the right number of commands would have passed with every glyph missing.

**A *composition* decays faster than the total it adds up to, because a total is what a later round
re-takes.** `doc/todo/42` §1 carried "76.6 M instructions, of which inflating the two cross-reference
streams is 18 M and nothing can remove it" for four hundred and eighty rounds. The total was
re-measured four rounds before ADR 0677 and the breakdown was not — so what a round choosing work
would have read was the *ranking* of a tree that no longer existed, and it was inverted: two
consecutive rounds then took double-digit percentages out of the three quarters that sentence
dismissed in a subordinate clause (ADRs 0667, 0677), each larger than the part it called
irreducible. **Whoever re-runs a floor re-runs the attribution under it**, and a round quoting a
share rather than a total should say which run printed it.

**And on a parallel path, a share of the *total* is not a share of the *critical path* — they can
differ by the number of threads.** ADR 0687 retook the composition of page 101's rasterisation, six
hundred rounds after it was last taken, and found a serial planner at 9.13% of the instruction
count. The page is granted eleven strips whose slowest holds 10.6% of the estimated cost, so the
drawing's contribution to the critical path was about the same 25 M the planner was: **9% of the
sum was half of the wait.** Callgrind counts every thread, which is what makes an instruction
profile of a parallel program honest about work and silent about latency — and `CLAUDE.md` ranks
latency first. So a round profiling anything divided asks *which side of the division is this on*
before ranking it, and the two numbers that answer it are the profile's and the planner's own worst
strip.

**A hot callee is invisible under a fat link, and one attribute is the whole instrument.**
`[profile.release]` inlines aggressively across crates, so callgrind reports a callee's cost against
its *caller's* name and files the inlined library code under `uint_macros.rs`, `slice/index.rs` and
`take.rs` — which reads like a caller that is diffusely expensive rather than one function that is
not. A temporary `#[inline(never)]` on the suspect and one rebuild turn it into a row of its own:
that is how ADR 0677 found a quarter of `Document::open` inside a function that reads three integers
out of a seven-byte record. Take the attribute off before measuring the result, because it costs
about twelve instructions a call and this population is a hundred thousand of them.

**And one measurement that was wrong because of *when* it was taken.** §9.10.2's last-resort
permission applied to simple fonts was measured, found to cost `pr4922.pdf` its whole readback, and
dropped — two rounds before the round that removed the interaction. Re-measured after it, the same
code is free and lifts two documents off the floor. **A measurement is a measurement of the tree as
it stands**, and a rule refused on one round's evidence is worth re-measuring after the round that
changes what it touches.
