# Habits: measuring

Status: **standing** — method, not code.
Read by: a round that measures anything — a benchmark, a profile, an A/B, or a price it is about
to write down. `doc/performance.md` is what is already known, `doc/verify.md` the instruments
that are not §2 gates, and `tools/state.sh` prints what may not be quoted from a document.

`doc/habits.md` is the index of the six. **A habit cited by number — `habit 107` — is this file's
`## 107.` heading**: every numbered habit is appended here whatever its kind, so the numbered tail
below holds habits about splits, hosts and ledgers as well as measurement.

- **A datum's readers are not bounded by the crate that carries it, and one of the readers a census
  names may not be one.** `data/standard-fonts/` was recorded as having three readers inside
  `pdf-font`. It had two — the third was a file *extension*, `.pfb` on ten programs that are bare
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
over corpora with no crawl in them. `doc/pdf.js` held 38 Type 2 patterns and **zero** stated one; the
crawl held 1504 and **42** did. A sentence of the form *no document does X* carries its population
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
  names walks the *objects* the cross-reference table names — `pdf-model`'s `witness_census` example prints the
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
  gate's spread fell from 100–400% to 0.6–22%; what a person gets is the unpinned figure, which habit 48
  takes beside it. Session 922, ADR 0884.
- **A wall-clock gate needs something that says whether the machine is worth believing, and it
  belongs in the gate.** The same probe as above, in a child of its own, with a band: outside it,
  the gate prints every figure and judges none, saying why. Checked against the defect rather than
  assumed (trap 13) — eight busy threads on the gate's own cores put twelve of twenty-eight
  figures outside their bands, and the probe said so first. The corollary is the part that makes
  it a gate rather than a decline: **find the figures with no clock in them and judge those
  always.** Bytes read and peak resident came back identical in forty-four runs, and they are what
  gates the claims that matter most. Session 922, ADR 0884.
- **Wall-clock benchmarks lie under load; count instructions while exploring, and take a verdict on a
  duration in one `--clock` hold.** One change measured as a 24% regression and an 8.5% improvement
  twenty minutes apart. **A/B in one sitting**, and measure the baseline on this machine rather than
  trusting a number in this file. Where the count and the clock disagree the clock decides (habits 68,
  87; `doc/todo/02` section 0 item 3).
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
  difference reads as the round's change.** `tools/worktree.sh` links the gitignored data (its
  `linked=` list), `doc/*.pdf` and `doc/corpora/*` into a round's tree; a `git worktree add` at the
  base commit, taken by hand for a before/after comparison, links none of them, and no `git` command
  restores them because they are gitignored or submodules. The eight-hundred-and-thirty-second session's `--bin pointers` run moved **113
  pointers** between *live* and *not carried* on the specification PDFs alone, with the round's
  change nowhere near them; the shape it takes is a defect class shrinking, which is the reading
  nobody questions. **Diff the two checkouts' walked file sets before believing the sweep** — a
  dozen lines of `os.scandir` — or link everything `tools/worktree.sh` links, and rerun. ADR 0760.
- **Take the *before* half with a patch file, never with `git stash`.** `git diff > round.patch`,
  `git checkout HEAD -- <files>`, measure, `git apply round.patch` — three commands that touch
  nothing outside the worktree. Why the stash is forbidden here, and how to recover one that has
  already gone wrong, is `doc/environment.md`'s; this entry is the measuring half.
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
  page. The largest document this project owns — ISO 32000-2, 1023 pages, kept (gitignored) in `doc/` —
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

## 36. A neighbour can take half of a figure without ever queueing for a processor, and `/proc/self` is the wrong thread to ask

**This was trap 36 and it is a habit**, moved here in the one-thousand-and-twenty-fifth session on
`doc/reviews/1018-what-retiring-a-trap-would-cost.md`'s recommendation. The argument for moving it
is the position that springs it: *taking a timing figure on a machine other rounds are using*,
which on this machine is every timing figure there is — so its two citations measured not rarity
but a lesson sitting in a file the round that needed it did not open. It is method rather than a
defect in a program, which is the line `CLAUDE.md` draws between a trap and a habit. **The number
36 is kept**, because ADRs cite it and an ADR is not edited to follow a file that moved underneath
it (ADR 0232 §2). Nothing of it is changed. Traps 34 and 35 are the same family and stay where they
are: whether they follow is a separate question with its own evidence.

Two mistakes about the same instrument, both made in the nine-hundred-and-thirty-eighth session,
and the second one silently.

**A wall-clock figure on a shared machine is three quantities, not two.** The obvious two are the
work and the time spent waiting for a processor somebody else had — and the second is not what
inflates a short figure. Measured: eight spinning processes pinned to **exactly** the eight CPUs a
gate pins its children to raised a one-millisecond figure by **43%** and the gate's fixed-work probe
by **74%**, while the kernel's own wait counter read **exactly zero in all twenty samples**. A short,
freshly woken task is what the scheduler runs first, so a process of a millisecond is essentially
never preempted; what the neighbour does instead is sit *inside* the core — an SMT sibling, a shared
cache, a boost clock four busy cores do not reach. **A figure can lose half its speed to a
neighbour with nothing to subtract and nothing to measure but a fixed-work probe.**

Where the wait *does* appear is the excursion: over fifteen consecutive samples, fourteen read 0.97
to 1.08 ms with a wait of zero and one read 3.947 with a wait of 2.825. That is a factor of four
explained exactly, and it is worth subtracting — `sched_info.run_delay` is accumulated in
nanoseconds at every wakeup and is unambiguously somebody else's. **The lesson is which is which:**
the wait explains the *tail*, the sharing explains the *level*, and a gate that treats either as
the other will widen the wrong thing.

**And ask the right thread.** `/proc/self/schedstat` is the **thread group leader**, and libtest —
like most harnesses — runs a test on a thread of its own, so the leader sleeps in a join for the
whole measurement and every counter reads zero however busy the machine is. The first version of
this instrument reported `runq 0.000` on every sample under saturating load and looked like a
finding. `/proc/thread-self/schedstat` is the calling thread. The same applies to anything else
under `/proc/self/` that is per-task rather than per-process — `stat`, `stack`, `wchan`,
`sched` — and it is invisible because the file exists, parses, and answers.

ADR 0916.

**And the failure this instrument was actually heading for is not a wrong figure — it is no
figure.** `launch_path.rs` is the only gate in this tree that can see a startup regression, and
three rounds in one week did not run its clock figures at all, each on a private judgement that the
load average was too high to believe them. The judgement was sound and it left no trace: a figure
nobody takes is a figure nobody can argue with, and a regression landing in such a week lands in
silence. **So the rule about a shared machine has a second half.** The first is that a wall-clock
figure taken beside a neighbour is worth less than it looks. The second is that *declining to
measure* is a worse answer than measuring and saying what the machine was — worse precisely because
it looks like caution, which is trap 39's shape in the round rather than in the instrument.

What the gate does now is the shape to copy. It reads the load average before and after each figure
and compares it against a **derived** ceiling — the machine's physical core count, from `sysfs`'s
own topology, which is the point above which a freshly woken child can no longer be given a core to
itself and so is exactly the contention measured above. Above it the figure is taken again a few
times, because a one-minute average lags a figure of milliseconds and a neighbour's build may
already have finished; and the loop **stops at the first attempt taken under the ceiling** rather
than pooling the attempts, because a band has a floor as well as a ceiling and the minimum of three
times as many draws would fail out of the bottom of one. Where no attempt is quiet, the smallest is
printed with its load and not judged. **Every figure comes out with its population and the load
beside it**, judged or not, so that a round reads the machine's fitness off the output instead of
deciding it alone. ADR 1226.

- **A number from an instrument that has never fired is not a finding — it is an uncalibrated
  reading.** Session 1025 woke twenty dormant conformance sweeps and gave nine of them an *act*
  verdict with a count attached. Three sessions were then spent acting on those counts, and the
  counts were mostly the instruments' own rules: `permitted`'s 49 were 47 false (it classifies a row
  by every entry its note *mentions*, not the entry the note says is *owed*); `blockers`' 30 expired
  refusals were **all** false (`while §` and `until §` are conjunctions, and the sweep read them as
  dependency claims); `counts`' 5 double-counts were 4 false; `unread`'s 114 keys are almost entirely
  this ledger's own conventions — a correction carries the claim it retires, a calibration describes
  a planted defect, and neither is a statement about today's tree.

  The cause is not carelessness in any round. **An instrument nobody runs is also an instrument
  nobody calibrates**, so a sweep's first real run reports its own rule as loudly as it reports the
  tree, and the two are indistinguishable in a count. The three sessions were not wasted — every one
  of those sweeps is calibrated now, each with the shape it was blind to written into its own
  documentation — but the work they did was *instrument repair*, and it was briefed as ledger work.

  So: before a round is sent to act on a sweep's number, someone reads **ten of its hits** against
  the standard and reports what fraction are the rule rather than the tree. Ten reads is under an
  hour and it is the difference between a slot that closes rows and a slot that discovers a grep is
  too crude. The real defects this class of work did find — §12.3.5.2's unnamed conditional `shall`,
  §8.11.4.3 filing `/Configs` under Table 99 when Table 98 states it, §7.5.7's `/Extends` — were each
  found by reading a clause, never by a count going down.

## 37. A refusal can stand in front of another refusal, so the site is measured again after one is lifted

Lifting the structure-tree refusal on the converter's appended page exposed a second refusal on the
very document the round existed for — a carriage return the face search demanded a glyph for — and
only re-running the same census showed it: the "after" was eight of nine until the second cause was
read (ADR 1163). A round that removes a refusal measures the site again with the command it measured
before, rather than inferring the gain from the refusal it removed.

## 38. A census over a display list owes a column for a verdict with nothing under it

The overprint census printed "verdict and no mark" beside its counts and that column caught three
defects in one run — two in the census (a command kind never visited, a fold that dropped its own
field) and one in the tree (a flag set where a mode is computed rather than where a mark is emitted,
refusing 177 pages for a mark not on them). Trap 13 plants a defect when a sweep comes back clean;
this is its counterpart for a walk that comes back full — a traversal that cannot report a miss is
one nobody can believe (ADR 1178).

## 39. A clause that ships an EXAMPLE is tested by running the example through the code

§8.11.4.4's usage adjustment had been an AND with the group's prior state since the mechanism was
built, so a usage dictionary could turn a layer off and never on; every `shall` had been read and
the defect survived a rewrite. The clause's own EXAMPLE — object 4 starts off and receives an ON
recommendation when printing — is the sentence that found it, and `doc/md/` carries every example
while no gate reads one. A round implementing such a clause encodes the example as a fixture
(ADR 1173).

## 40. Measure what a flag closes across the classes the clause distinguishes, not across what the machine has

ADR 1209 measured `--font` with the machine's DejaVu and Droid, both `glyf` faces, and concluded it
closed none of the corpus font refusals; two were closable that day by naming a bare CFF, which
§9.9's Table 124 makes a different class of input rather than a different file. The measurement was
of the tester's font directory, not of the capability (ADR 1221).

## 41. An instrument models the caches the shipped path keeps between gestures, or it prices the wrong configuration

Interpreting a page with a fresh `FontCache` read 12.45 ms where the same page after its
predecessor, against the cache `viewer-core` keeps per document, reads 1.37 — a factor of nine in
the exact stage the round was ranking, and the mistake looks like a correct measurement of a real
function (ADR 1260). A gesture's price includes what the last gesture left behind: ask which caches
the host owns across the gesture and fill them the way the host fills them. The same ADR's other
half: a page turn and a zoom step are drawn by different rasterisers, and a budget for one taken on
the other is a configuration nobody runs.

## 42. When the faster arm of an A/B also changes which library function runs, the profile must show the one you asked for

`zune-jpeg` picks its colour conversion when it reads the headers; `set_options` afterwards changes
only the output buffer's shape, so asking the same decoder for RGBA after `decode_headers()` filled a
four-lane raster with a three-lane conversion — the instrument reported −15.3% and the pixels were
wrong. The tell was in the profile: `decode_jpeg` had vanished and the three-lane conversion was
still there to the instruction (ADR 1271). A library that takes an option after it has decided
something answers half of it, faster, and a wall clock cannot tell the difference.

## 43. A never-taken branch in a per-mark function is not free; ask the question where the answer is known

Two spellings of one report's condition in `draw_mark` cost 0.12% each; the same test at
`draw_image`, the one kind of mark it is about, cost 0.002% (ADR 1266). The measurement that
discriminated was not which spelling but which function.

## 44. A fix an earlier ADR named is a hypothesis, re-derived before it is built

ADR 1254 measured its 4.66 levels correctly and named the fix as the cube's input curves; the stage
with the unbounded slope was at the output end, and building only what the ADR named would have
left the defect in the grid (ADR 1267). The brief says this of `doc/adr_revisit/` notes; the same is
true of a named fix inside an accepted ADR.

## 45. A callgrind total that moves while every function keeps its count is layout, not work

Round 1222's plane generalisation moved a 3000-mark page by 0.41% with every Rust function at the
same instruction count in both binaries; the whole delta was glibc's `memmove` at an unchanged call
count and a different per-call cost, and an untouched page moved by the same mechanism (ADR 1281).
Diff per function before quoting a total, and use a page that runs none of the change as the control.

## 46. Idle work a render thread cannot interrupt is measured apart from the job that waits behind it

The 2× sharp pass began the moment the thread was idle and could not be stopped, so a zoom step's
render queued behind it and the queue time was charged to the next prediction as if it were drawing
(ADR 1289). Stamp when a job is taken up, not when it was asked; and let uninterruptible idle work
begin only after the view has been still for as long as that work is predicted to take.

## 47. Reordering an `#[inline]` guard's fast path is re-measured per function on the device page

Swapping one inline test for a discriminant test plus a niche `matches!` cost +0.10%, all inside
`show_text`, with nothing in the source looking hot (ADR 1311); the fast test went back to the
overprint parameter and the mode check into the cold half. The per-function diff is what found it.

## 48. A parallel path is judged on this machine's two core classes, not only pinned

A job handed to the pool may land on a Zen 5c core and take about 1.5 times as long as on the
interpreter's own thread, so a decode that is faster pinned can be slower in the viewer: the
single-image page's median went from 68.9 ms to 98.7 ms unpinned on an identical code path
(ADR 1321), which is why the first image a walk finds is left to the interpreter. Measure pinned for
the minimum and unpinned for what a person gets.

## 49. A census's stderr goes to `/dev/null` before a walk

A census binary picked up a sibling's temporary `eprintln!` and wrote 1.4 GB in two minutes
(round 1240). Redirect a walk's stderr, or `strings`-check the binary, before the lock is taken.

## 50. A changed pixel is held to the clause's geometry, never to the oracle's

When a change re-expresses a mark, compute the clause's shape analytically and compare each moved
pixel to it: the redacted arc was within one step of §8.4.3.3's semicircle while render-cpu's own
stroker was eight off, so a byte comparison with the oracle would have called the correct output a
regression (ADR 1324). And a plant must hit the step that differs — premultiplying a stencil at upload
changes nothing because its alpha is 0 or 255; only the shader's multiply failed the test.

## 51. A census's residue is fixed by its named causes and checked file by file, not by a second walk

Round 1246's one walk split the 951 unseparated pages into two causes; both were built, and the
named files were then opened by hand — one walk, not two, on a machine where a walk is the cost
(ADR 1329).

## 52. A cost floor is stated by measuring an "off" arm of the expensive step

With the overlap walk switched off, two join-dense pages drew 19% and 24% cheaper than before the
walk existed, so the walk's own cost and its scaffolding's could be told apart, and 99.4% of the
clusters it walks on one page genuinely leave the set (ADR 1359). A floor measured that way is a
number; argued from the profile alone it is a guess.

## 54. A fan-out's floor is judged by timing its jobs by kind, not by counting segments

After the stroke constructions a stroke's segment cost 25 times a glyph's, so a 4096-segment floor
silently kept whole pages on the walk's thread; the per-job probe on one pinned core gave the weight
(ADR 1395). And the scaffolding an older ADR built to make a construction exact is measured as its own
"off" arm — ADR 1375's tiling was the largest cost left in raster's fill (ADR 1397).

## 55. A drain, a lever and a lane are each judged with the others switched off

Two causes of a forced drain were one wall: the rectangle lever alone and the repeated-key lever
alone each moved nothing, and only both together took the text page's turn from 9.9 to 6.8 ms — a
`#[track_caller]` print on `drain_queue` and `charge` names each drain's site (ADR 1409). And before
"lane A differs from lane B" is accepted as a cause, switch each lane off in turn at one thread: that
turned "the hybrid differs" into "the quantised phase differs" (ADR 1407). A font-catalogue walk is
measured cold by evicting `/usr/share/fonts` with `posix_fadvise(POSIX_FADV_DONTNEED)` first — about a
second against a tenth warm on this machine (ADR 1406).

## 56. Two diverging arms are diffed as event logs before a hypothesis is formed

The brief for round 1291 offered three causes for 176 thread-dependent pages; logging every atlas
insert, probe answer and repack per arm with the page boundaries, stripping per-device ids and
diffing the two sequences found the first differing line in one read, and it disproved all three
(ADR 1419). And a callgrind summary picks each column by function NAME, never by rank:
`callgrind_annotate` sorts by cost, so a function missing from one arm shifts every column after it
and two of round 1292's early tables were mislabelled that way (ADR 1421).

## 57. A phase's clock is timed step by step before a lever is chosen for it

`doc/performance.md`'s "transfer" column on a photograph read 53 ms, and 96% of it was raster's
host-side area reduction on one thread; the upload itself was 2 ms at 10 GB/s (ADR 1433). And
before a sweep or a grid replaces a box-prefiltered scan, count the scan's visits against the pairs
that meet and the pairs actually asked: 31 M cheap box tests cost less than the sorting that would
have avoided them, and the expensive questions were already limited to pairs that meet (ADR 1431).

## 58. A ranking's candidates are listed on this machine before its order is chosen

ADR 1430 ranked a chrome word's face by style first; listing the candidates (`fc-list` plus a
`cmap` count) showed that order would hand a bold Chinese label to a face of 17 Han characters
over one of 41 071, so the one search the page and chrome now share ranks repertoire first (ADR
1441). And a batch's first build is timed by sampling every `cargo`'s children at one moment:
that showed one lock per profile — `dev` builds queue behind each other while `release` and `gates`
builds run beside and compile the same dependencies again (ADR 1451).

## 59. The page is profiled before the brief's lever is built

The largest photograph without restart intervals cost 4 823 M instructions, under 6% of them in the
codec the brief named: its soft mask's `/Matte` inversion ran per sample, and a table of 65 536
entries per component took the turn from 180 to 65 ms (ADR 1457). The lever the brief named —
cutting the entropy data into bands — was built byte-identical and measured as a loss on every
page with other decodes in flight, and was not kept.

## 60. A memo's hit rate and the cost of a hit are read before a bigger table is built

Plans' ICC memo already answered 99.97% of pixels; the cost was the hit itself, 82 instructions per
pixel, and copying the previous pixel's output where the input repeats removed most of it (ADR
1469). And a corpus `grep -r` misses `doc/corpora/*`, which are symlinks — `grep -R`; a first count
of public-key witnesses missed four that way (round 1319).

## 61. The stage that holds the memory is found before the lever is designed

The brief named the tiled frame for a 7.9 GiB peak; `VmHWM` reset between stages
(`echo 5 > /proc/self/clear_refs`) showed the frame at 0.99 GB and the structural-similarity
comparison at 7.45 GB, nine `f32` planes of the whole page at once — banded, the corpus walk's
peak fell from 11.3 to 5.2 GiB (ADR 1481). And a signal that "the pool would otherwise idle"
includes the pool's size: callgrind runs at one thread, where every decode is alone and the cut is
pure overhead. A speed-up is called byte-identical only after per-page digests from exported HEAD
and change trees, each in its own target directory, agree (ADR 1479).

## 62. A disputed pixel's closed form is computed, to more digits than a level, before a construction or an ordering is called wrong

Admitting a cached clip region moved three pixels of `bug1721218` by one level; their exact areas
were 65.500037, 135.500024 and 59.500014 levels — each within 1.5·10⁻⁷ of a rounding boundary that
`f32` sums cannot decide, and two tiles over the same pixel disagreed with each other. The fix was
precision (a residue filled in `f64` from the same points), not a choice between constructions,
and admission then moved nothing (ADR 1491).

### 53. The disputed pixel's closed-form area is computed before either construction is called wrong

A square drawn as eight pieces showed one more level at its corner than the square drawn whole; the
corner pixel's exact coverage is 0.885 of a level, so the pieces were right and the whole square's
fixed-point converter truncates — ADR 0476's accepted gap — while the real defect sat in the next
column (ADR 1374). And a floor is measured step by step: switching the walk's scaffolding off one
stage at a time is what showed no cheaper exact detector exists (ADR 1373).

## 63. The inputs per pixel are counted before a per-pixel lever is built

The brief named four levers for the exact meet; counting showed 226 partial edges beside each
asked pixel against 20 through it, which pointed at a fifth — runs of short edges crossing a row
as one edge — that halved the cost (ADR 1503); a merging lever needs a locality cap, since uncapped
it lost. And an A/B across commits compares the two binaries with `md5sum` even when each commit
has its own export: a bisect's builds came out identical because the export was rebuilt in place
(round 1335). The pool's occupancy is read before a cut is shared into it: eight frames on eight
CPUs were 66% busy, and dividing them further lost on every page (ADR 1505).

## 64. A text rule is A/B'd by token agreement with a reference, and a slow unit's time by its operator count

Word counts against `pdftotext` said a per-font word gap moved 12 pages away and 7 towards, where
token F1 said 13 better and 1 worse — `pdftotext` joins leader dots, and a count cannot see which
words match (ADR 1515). And a fuzz slow unit's time is divided by its operator count before its
per-item cost is named: both "fontless show" units were `MAX_OPERATIONS`-bound Type 3 cycles, and
the brief's 2.7 µs per show was a per-operator cost (ADR 1521).

## 65. Repeats are counted per render, not per gesture, before a per-frame cache is built

ADR 1503 counted 59% of `bug1721218`'s meets as repeating a pixel; a zoom step draws that page's
group as two renders, chromatic then black, and the black repeats every meet of the chromatic —
inside one render only 14% repeat, so the cache the brief aimed at a frame would have reached a
fraction. The meet kept for the NEXT render, keyed word for word and never by id, hit on every
black-render meet (ADR 1517). Count where the repeats actually are before choosing the cache's
lifetime.

## 66. A recorded "not a lever" is re-measured when the driver stack changes

ADR 0179 read the launch instance's backend set as no lever; a thousand sessions later, on this
driver stack, an instance without GL brings the device up in 17 ms instead of 27 (ADR 1532). And a
launch gate that prints "device up" and "document joined" at one instant hides the work done after
the join; printing when the render request arrives made page one's interpretation behind the
device a number, which is what ADR 1531 then moved onto the document thread.

## 67. A library's licence is read against `deny.toml` before anything else about it is measured

Two engines in RFC 0008 and the one pure-Rust audio stack in RFC 0009 (`symphonia`, MPL-2.0) fell on
the licence line before any other judgment; a timing table built before the registry's licence
string is compared with `deny.toml`'s list is wasted work. Ask the registry first, then measure.

## 68. Two levers are ordered by the clock before one is built, and the spawner's environment is read before a blocker is called unsolved

ADR 1541 ranked the small clip regions (0.230 G instructions) above the pass recording (0.115 G);
by the clock the order was reversed — a probe skipping every region fill saved 4.5 ms, and
`prepare_run`'s 7 138 buffer and bind-group calls took 9–10 ms per render (ADR 1555). An instruction
count ranks work on one thread; a driver call's cost is on the clock. And before a confinement's
blocker is called unsolved, grep the spawner for what it already sets in the child's environment:
`MALLOC_ARENA_MAX=1` had been set by `confined_transport::Host::start` for every worker, which is
what let the confined worker build a pool as wide as the machine (ADR 1554).

## 69. A clock that moves on both arms is the machine's, and a section reads another's finding from its record

`request_device` read 4–10 ms on both arms of an A/B whose callgrind difference was 0.9 ms; `free`
showed 1 GiB free under 43 GiB of page cache and strace put the excess in the kernel's GPU-memory
ioctls (ADR 1569) — read `free` and strace the ioctls before believing a clock that moved on both
arms alike. And a state section may not write (ADR 1487), so where one section needs a census's
result it reads the newest record that carries it and names that record — `main-checkout`'s
re-seed line reads the `**Stale today:**` sentence of the newest history record (round 1370).

## 70. An artefact is found by its content, and a saving on helper threads is ordered by the clock

A brief named three fuzz slow units by description ("the 5 125-glyph `post` table") and the
same-size triplet that looked like them was the wrong one; round 1374 found the right three by
scanning every unit for a format 2.0 `post` table with 5 125 glyphs. Find an artefact by its
content before measuring it. And work already on helper threads is mostly hidden from the clock:
the exact areas fell 42% in instructions and bought 1.5–2.5 ms, while deciding convexity on the
walk thread cost 7.7 ms to save 4.7 (ADR 1582) — order a saving by the clock on the thread that
waits, not by instructions wherever they run.

## 71. A recursion bound is chosen after the stack cost per level is measured at two or three stack sizes

Boa 0.22's parser has no depth limit. Before `Budget::nesting` was set, round 1383 measured
26–32 KiB of stack per nesting level in dev and release alike: an 8 MiB thread parses 256 levels
and overflows at 320, 16 MiB parses 400, 2 MiB parses 64 and not 128 (ADR 1602). The fuzz target's
256 sat at the edge of the worker's 8 MiB thread, which the measurement showed and a guess would
not have; the bound is 128.

## 72. A turn is two runs less one, and a stream-dictionary census is a byte grep

A callgrind turn on a page that fills process-wide caches (the press and ink tables) is measured as
a run of two interpretations less a run of one; a single run counted the ink table's 1.19 G as if
every turn paid it (ADR 1632, `turn_interpret`). And a census of a key population that lives in
stream dictionaries — `/Order`, `/Filter`, `/DecodeParms` — is a byte grep, because a stream
dictionary is never inside an object stream: 126 393 Type 0 functions over the whole crawl in 40 s
(ADR 1636).

## 73. A gate's harness is read before a row is proposed for a cost

`turn_path` interprets a page without `viewer-core`, so no row of it can see the field table a
repaint builds, and a form page proposed for a row there would have measured a cost the harness
never pays (round 1417, ADR 1670). Before a row is proposed for a cost, the harness's own module
comment says what it runs; a cost outside it needs another instrument, not a row.

## 74. A wait on a lock's holder is a wait on its pid, never a `pgrep -f` of a string the waiter's own line holds

The rule line forbids a `pgrep -f` whose pattern appears in the grepping command, and the lock's
holder file names the pid for exactly this: `kill -0 $(cut -d' ' -f1 <lock>.holder)` ends when the
hold ends, where a pattern wait matches itself and never does (round 1425; trap 124's incident is
the same shape with a `pkill -P`). It is not only the lock: round 1460 waited for its own
`tools/batch.sh check` with `until ! pgrep -f 'tools/batch.sh check' …; do sleep 5; done`, the
loop's shell carried the pattern on its own command line, and the loop ran 614 s and was stopped by
hand, 382 s of it after the check it waited for had ended. A job you started is waited on by the pid
or the output file you gave it, never by a pattern its waiter also spells.

## 75. A change around a thread join is profiled on the page whose walk goes on after the join

Round 1428's first version kept a drain's lent threads until no exact meet was left, which held
`bug1721218_reduced.pdf`'s walk at the join and slowed its pinned turn from about 140 to 147 ms while
the Type 3 page it targeted improved (ADR 1692). A change that moves where work happens around a
join is measured on a page whose walk continues past that join as well as on the page it is for; the
targeted page alone shows only the gain.

## 76. A build is not a walk, and a walk declared large is not queued before its tree is built

Round 1428's `cargo build --release`, left at the wrapper's default large kind, queued 2 239.5 s
behind the arms export for a hold of 1.1 s, and `cargo test -p conformance`, which the rule line says
is not a walk, waited 1 820.8 s under the lock in the same batch (round 1435, ADR 1706). A build
runs outside the lock or as a small walk, and a walk declared large is asked only once its tree is
built, so the lane it waits for is spent on the walk and not on the compile.

## 77. The drain sequence is printed before anything is designed around "the work after the drain"

A brief placed `bug1721218_reduced.pdf`'s 232 exact meets "in the 11 ms of walk after the drain"; a
probe that printed each drain's jobs, weight, thread count and the backtrace of what forced it showed
them in a second drain's commit, forced at a layer boundary and run on the walk's thread (round 1434,
ADR 1704). The probe took two minutes; the design it would have saved took an hour. Where a frame's
work is split by drains, the sequence is printed first and the design follows what it says.

## 78. A tick that carries whole milliseconds keeps the fraction it drops

GTK's script-timer clock reset to `now` on every tick and told the core 299 of 300 ms, then 0 ms
for ever — 184 173 ticks of nothing, seen only by the drive (round 1433, ADR 1702). A ticker that
hands out whole units keeps the remainder and rounds its waits up; a test that ticks a known span
and counts the ticks is the one that would have caught it before the drive did.

## 79. A long campaign's targets share one hold, started when the other lane is free

Round 1442 ran five fuzz targets side by side in one hold of the second lane for an hour, where
five holds in sequence would have been three and a half hours of a lane; even so, four siblings'
small walks queued 860 to 1 134 s behind it while the arms held the first lane (ADR 1716). A campaign
that will hold a lane for long is one hold for every target it owns, and it is asked when the other
lane is free, so the batch's short walks keep a lane of their own.

## 80. A status line is waited for by its prefix, never by equality with the whole line

`tools/batch.sh arms` ends its README with `done <date>, the lock held <n> s …`; a wait written as
`[ "$(tail -1 README)" = done ]` never matched it and round 1466 waited about 480 s after the export
had finished. A wait on a status file reads `grep -q '^done'` or the file's own stated prefix, and it
is bounded, so a line whose tail changes shape does not hold a round for the shape it remembered.

## 81. A walk queued for either lane is queued early, and its wait is reported

A request that may take either lane is granted the first lane to free, not its place in a queue:
round 1470's `--tree 12` walk asked at 23:01 and was granted at about 00:13, while a `raster_golden`
asked at 23:56 took a lane first. A round that needs a gate's result in order asks for it as early as
its edits allow, and its record says how long the grant took, so the lock's cost is counted where it
fell rather than inferred from the round's wall.

## 82. A paired corpus walk's wall is read as CPU over lanes before it measures a worker-side lever

`read_corpus` asks a mount and then computes the same page itself, so its wall is about (the test's
CPU plus the workers' CPU) divided by its four lanes: 267–274 s plus 295–319 s against 147–163 s of
wall, and a pool six times wider moved the walk nowhere while the wait a face's user sees halved
(round 1484, ADR 1804). Before such a walk is the figure for a worker-side speed lever, read
`utime` and `cutime` from `/proc/<pid>/stat` to see the ceiling the lever can reach, and measure the
lever on the question a user asks.

## 83. A profiled page is checked for drawing on one thread before a lever is built

The owner's A1 drawing was profiled instruction by instruction, three wastes were cut and the page
fell from 15.37 s to 11.24 s; its largest lever was none of them — the page rasterises on one thread
because `pdf_render::strips` marks a stroke's whole height uncuttable and the page frame's `re S`
spans every row, so wall time equalled user time at four threads (round 1488, ADR 1813). Before a
lever is built from an instruction profile, compare the walk's wall with its user time: an
instruction count cannot show idle processors.

## 84. A census of marks by device extent maps the hull before it grows it by the width

`Stroke::device_width` answers a path-space width; a census that grew a mark's mapped box by that
figure called 0.2 px dots marks of 2–4 px and would have sent a batch after the wrong population
(round 1490, ADR 1817). The hull is grown by the stroke's width in path space and then mapped, or
the width is mapped first; and a census whose population decides a lever is checked on a handful of
marks by hand before the lever is priced.

## 85. A census of added files filters git's name list; it never trusts a pathspec glob

`git diff --diff-filter=A <base> HEAD --name-only -- 'crates/*/src/**/*.rs'` printed three of the
seven modules added since the last re-read, because git's `**` is not the shell's and matches
without `:(glob)` only where the shell's would not (round 1501, ADR 1839); the brief's figure of five
came from the same shape. A census of added files takes the whole `--name-only` list and filters it
with `grep -E '/src/.*\.rs$'`, and a figure in a brief is re-derived by the round at its own HEAD.

## 86. A classification called "the population" is tallied on the witness by what it declines

An axis-only cut of the thin-segment classification looked complete and admitted 3 699 730 of the
witness's 4 400 060 strokes; the 667 904 it declined were diagonal hatching dots and still 40 % of
the drawing's instructions (round 1496, ADR 1829). Before a classification is called the population
a lever was priced on, count on the witness what it declines and why, and read the declined share
against the profile, not the admitted count against the total.

## 87. A cut in a per-token or per-operator path is kept on the clock, not on the instruction count

Making the content reader's salvage test a cold call inlined the token wrapper into the operator
loop and cut 1.09 G instructions off the witness's read — and made the read 0.41 s slower, +19 %,
in one clock hold with the arms interleaved (round 1498, ADR 1833). A cut in a path run once per
token or per operator is A/B'd on the clock, in one hold, before it is kept; callgrind's count ranks
candidates and decides nothing.

## 88. A lane's seed is re-measured in the batch that cuts the lane's cost

`quorra` seeds a lane's cost per command before it has drawn a page (ADR 1831): the device at
1 788 ns and the processor at 667 ns. After ADRs 1829 and 1835 the device measures about 250 and the
processor 178–256, and the device is never measured on a page the seeds gave the processor — so a
stale seed silently decides which lane is ever tried (round 1504, ADR 1845). A batch that cuts a
lane's cost by several times re-seeds the lane in the same batch, and the record says the figure.

## 89. A census's lever is named after its refusals are tallied by reason

Round 1500's record said the witness's 2 169 forbidden rows were strokes under a level of width and
clips, and the brief built a slot on it; the per-reason tally showed the backend refused 61 curved
strokes and nothing else, clips held no row, and thin strokes were not the cause (round 1505,
ADR 1847). Before a lever is built from a census, the census prints each refusal's reason and the
rows that reason holds on its own; a lever named from the total is a guess.

## 90. A merge's clock gate runs with nothing of the merge's own beside it

Batch eighty-one's merge read thirteen figures under their floors — the batch's wins — moved the
floors, and re-ran the two clock gates while the merge's own `tools/batch.sh check` built the fuzz
workspace beside them and a one-off round built a whole workspace in its own clone; the re-run read
four launch figures and three turn figures over their *ceilings* at a load of five to ten, the busy
end of trap 110, and told nothing about the tree. A clock gate is re-run alone: the merge's static
checks go before or after it, never beside, and a round working outside the batch runs its heavy
commands through `tools/bounded.sh --lock` so they queue behind the hold instead of under it. The
re-run waits for the load the band file names, and the log says what the load was.

## 91. A stage noisier than the end-to-end figure is measured as itself before a change is called lost

Round 1517 drew the confined worker's page turn beside its read and withdrew it once, because the
end-to-end turn sat inside the read's own spread (3.14–4.08 s in both arms) and showed nothing. A
probe that stamped the worker's stages into the frame it sends showed the draw after the read had
gone from 504–657 ms to 53–83 ms, and the drawer went back in (ADR 1871). When a change moves one
stage and the end-to-end figure is noisier than that stage, the stage is measured as itself — a
stamp, a split clock — before the change is judged, and the record says which figure judged it.

## 92. A clock hold's output is written to a file whole

Round 1524 cut its first clock hold's output through `tail` and had to take a second hold for the
figures the cut removed. A hold is the batch's scarcest resource (every waiting clock run stops the
siblings' grants, ADR 1863): its whole output goes to a file under `scratchpad/r<n>/`, and the
report reads from the file.

## 93. A mechanical split is proved by the multiset of its lines and by removing every widening

Three rounds of batch eighty-five split a file of six to nine thousand lines into a module tree and
proved each the same way (ADRs 1906, 1908, 1910, 1912, 1914): the before and after files compared as
multisets of non-blank lines, where the only differences allowed are the added `impl` headers and
braces, imports, visibility and rustfmt's re-wrapping — a lost or duplicated line shows there and
nowhere else, because the compiler does not report it; and every `pub(super)` the split added
removed one at a time with a compile after each, so that a widening nothing reads is found (round
1536 found one). The drive, the golden set and the arms then prove the behaviour; the two checks
prove the text.

## 94. A reference renderer's upgrade is A/B'd from the package cache before a gate move is read as ours

mupdf 1.28.4 → 1.28.5 turned one oracle page ambiguous and two foreign-reader pages different on
2026-10-13, with this tree's raster unchanged. Round 1538 extracted 1.28.4 from `/var/cache/pacman/pkg`
and ran both against the same files and the same zlib: the new version fills the rows of a
`/Length 0` stream grey after a "premature end" it alone reports, and reads a font stream short at a
token no reader can take as a number (ADR 1913). Two runs settled what a day of reading our code
would not have. When `pacman.log` shows a reference upgraded, the A/B comes first.

## 95. A sweep over a count from a grep is planned after the sorter has read the lines

Batch eighty-five's first slot was briefed to rewrite the 245 lines `grep -rE "hundred-and-|session"`
finds; `tools/comment-history.py` sorted them in a minute into 0 history, 21 names, 109 legitimate uses
and 115 in code, and three gates had held the history at zero since ADR 1767 (round 1534). A grep is a
reading list, not a debt; the brief that names a count from it names the sorter's figure instead.

## 96. A split that keeps the old file as the root re-points the ledger rows that name it

ADRs 1912 and 1914 split `image.rs` and `render-cpu/src/lib.rs` into module trees and kept each old
file as the tree's root, so every ledger row naming the file went on passing the path-exists check
while 87 of them pointed at code that had moved (round 1543 re-pointed 108). A split ADR lists the
rows naming the file it splits and moves each `code` path to the module that holds the row's item,
read item by item, in the same round — the checker cannot see a path that still exists.

## 97. A figure quoted from a question is a population, not this tree's gain

Q271 counted 158 projected maps with degree-shaped registration points, and the brief priced A271's
departure at 158 gained. Re-run through the program's own reading at HEAD, 151 were refused by their
point shape and 7 by their projection method or WKT first — 151 is the gain (round 1541, ADR 1918). A
departure is priced by the census the program runs, not the census the question ran.

## 98. An object a confined worker asks the host to act on is checked against what the host told it

`bookmarkRoot.execute()` asks the viewer to activate an outline item. The view state holds the request
only if the object is in the outline the realm was told, and refuses any other with a sentence,
because a confined worker's answer is data and not a command (round 1549, ADR 1934). ADR 1922's
refusing object never needed the check because it carried nothing; the moment an object carries a
reference the host acts on, the reference is checked against the host's own list.

## 99. An owner's "Owes" line copied from an RFC carries the RFC's date

A254's "Owes" copied RFC 0009 §12's list, written 2026-10-05; the poster it named had been built by
ADR 1561 on 2026-10-06, two days before the answer was given, and round 1548 found that only by
reading the §13.4 row's history before building. Before a slot builds an item an answer owes, the
row's status and the ADRs it cites are read at HEAD; an item already built is a sentence in the
record, not a day's work.

## 100. A narrowed exclusion is re-read into every row that rested on it, in the same round

When `CLAUDE.md`'s clause 13 entry was narrowed to the player alone (A254, ADR 1946), the one row the
slot named moved and six more had to: three `out-of-scope` rows became owed, two became
`implemented` for the appearance they already drew, and the headings above them cascaded to
`partial`. An exclusion's wording is what every `out-of-scope` note cites, so changing it changes
their status whether or not anybody reads them. The round that narrows an entry greps the ledger for
the exclusion's name and re-reads each row it finds against the new wording before it reports.

## 101. A sub-millisecond clock figure outside its band is judged by the instruction count beside it

Batch eighty-eight's merge chain had one red gate: a tiny document's cold open — 0.511 ms against a
band of 0.320 to 0.500 — and two reruns put a different tiny document outside each time while the
large documents sat at the fast edge of theirs. The same gate prints, for every document, how many
instructions the open executes under callgrind, and that figure is deterministic: the batch and
HEAD differed by two thousand instructions in a million, the batch lower, and read the same bytes
in the same calls. The clock had measured the disk's round trips on a sub-millisecond open, not the
program. Before a sub-millisecond clock figure is bisected, the instruction count and the read calls
printed beside it are compared against HEAD; where they are equal the figure is the machine's, and
the record says so with both numbers. HEAD is measured in a tree of its own
(`git worktree add --detach`, every submodule and untracked document linked in, the workers built
beside the test) rather than read from a log, because the machine on the day is the variable.

## 102. A worked example's printed figure is checked against its own formula before it sets a tolerance

Guidance Note 7-2 section 4.2.3 prints an intermediate coordinate 6 mm away from what its own
formula gives, with either rotation convention. Round 1561 found this because its test's tolerance
was one unit of the example's last printed digit, which the formula could not meet, and the test
now says why its tolerance is what it is. A worked example is a document's claim about a
computation; before its last digit becomes a test's bound, the computation is run from the
document's inputs and the two are compared, and a disagreement is written into the test's reason.

## 103. A new arm of a host's event match is one line that hands the event to a method

Two rounds of batch eighty-nine each added an arm to the GTK and Qt windows' `react` matches, and
together they pushed both past clippy's hundred-line limit, which the second round lifted with an
`expect`. A match over an exhaustive event enum is where every feature arrives, so its arms grow
with the features; an arm that carries its body makes the match the longest function in the
window. An arm is `Event::X(it) => self.x(it)`, and the body is the method's, where it can be read
on its own and tested on its own.

## 104. A command that can outlive the prompt cache is waited for in calls of four minutes

Batch ninety's census of 48 rounds found 85 % of everything the rounds wrote to the prompt cache
written by the 4 % of requests that followed a gap over five minutes, and the gaps were the rounds'
own wait-loops — `timeout 590 bash -c 'until grep -q "^bounded: exit" …'` — written to the Bash
tool's ten-minute ceiling, every one of which outlived the cache. The rule, `tools/wait.sh`'s two calls under a
270 000 ms harness timeout, is the fourth line of `doc/environment.md`'s rule block; ADR 1976 has the
census and the price, ADR 1977 the rule.

## 105. A residue an ADR leaves has its premise checked in the file's bytes before it is built on

ADR 1948 §4 said two documents copied in page order because an annotation the tree claims by object
reference reached the readback without its id. Round 1567 built that placement and then found,
with `mutool show`, that neither document holds an object reference: one has an untagged caption,
the other no `/MarkInfo` at all, and both are the ADR's own "text nothing claims", kept in page
order by decision. The placement was right to build and two other pages gained by it, but the
hypothesis "58 → 60" was never possible. A residue is a claim about files; the first hour of a round
that takes one up opens the files and reads the objects the claim names.

## 106. A dialogue a drive step answers puts the keyboard on its list

Round 1565's archive chooser is a card in `quorra`, a drop-down over a list in GTK and a combo box
over a list in Qt. The drive answers each the same way — Down, then Enter — and that is possible only
because every one of the three puts the keyboard focus on its list when it opens. A dialogue whose
focus lands elsewhere makes the drive's step toolkit-specific, and a step that differs by toolkit is
a step that goes untested in one of them.

## 107. A crate-scoped check runs bare; the lock is for walks

Round 1571 queued `cargo nextest -p pdf-archive` and a clippy of two crates under `tools/bounded.sh
--lock` and waited thirteen to twenty-five minutes behind the batch's walks for checks that ran in
nine seconds. The lock exists so that two walks of the corpus do not share the machine's memory
(ADR 1684); a crate's own tests and lints are a build's size, which `doc/environment.md`'s rule block
already lets run bare. Tier 1 scoped to a round's crates is run bare, and a walk, a hold or an export
takes the lock.

## 108. A round that moves what `doc/teaching/` names redraws the diagram, not only the name

`tools/conformance/tests/teaching.rs` holds every path, `path::name` and ADR the teaching documents
write to one that exists, so a moved function fails the test by name (ADR 2003). The sentence and the
diagram around the name it cannot judge: a stage whose function moved into a sibling module may still
read as if it lived where it did. Each diagram ends with a maintenance note naming the files and ADRs
it rests on; a round that touches one of them reads the note and redraws the diagram in the same
round — `cargo test -p conformance --test teaching` is the command, run before the report — and the
test's failure is the reminder, never the whole of the work.

## 109. A split that walks up from a method's body stops at the `impl` line

Round 1584's first extraction of a method out of a 4 636-line `impl Viewer` walked upward over the
method's doc comments and attributes to find its start, and did not stop at `impl Viewer {`: the
opening of the block went into the first member's text and the result had an unclosed delimiter. A
method's start is bounded by the line that opens its block as much as by the member above it; the
walk checks for `impl` and `}` before it takes a line.

## 110. A census's `--since` is checked against the batch's own span first

Round 1581's brief gave the cache census a `--since` of 12:40Z for a batch whose merge commit is
stamped 09:18Z, because the orchestrator wrote the time from memory rather than from `date -u`; the
census would have counted nothing, and until the round made it refuse an empty count it printed
zeros that read as "no loss". Before a census over a time range runs, the range is read against the
batch's own stamps — `/home/AI/heavy-walk.log`, the merge commit's `%cI` — and a brief's timestamp is
taken from the clock, never estimated.


## 111. Whether an optional conversion's clause applies is read off where the tree does the conversion

§8.6.5.7 binds "when this type of implicit conversion is done" — a condition on the processor's
act, not on the colour space's dictionary. Round 1580's brief hypothesised from `/Alternate`; the
answer was in `ColourSpace::to_cmyk`, the one place this reader passes an `ICCBased` colour's values
through to a press of the same profile (ADR 1996), and nothing in the space's entries could have said
so. A clause conditioned on what the processor does is read against the code path that does it,
found by its name in the tree, before any attribute of the input is consulted.


## 112. A round's clock hold stops the round's own background drive first

Round 1582 had a drive of the four windows running in the background when its `--clock` hold was
granted, and the first line of its hold script stopped that drive before any figure was taken. A
hold takes both lock lanes so that no *other* round's walk shares the machine; a round's own drive
is outside the lock and is the one load the lock cannot see. The hold script's first statement names
and stops it, and the record says so beside the hold's load figure.

## 113. Where a figure came from is read off the adapter line the instrument prints

Round 1578 was told to take its device figures on the owner's display because the orchestrator took
the headless runs for Xvfb's software adapter. They were not: `zoom_frame` and the turn gate open the
machine's own adapter headless, and every figure was the Radeon's through RADV, which the instrument
had printed on its first line. A sentence about where a measurement was taken is copied from that
line, and `doc/environment.md`'s rule follows from it — what is measurable headless is measured
headless, and the owner's loop is for what a display alone can show.

## 114. A script that rewrites a file by line number inserts nothing mid-pass, and slices by line

Round 1591's visibility fixer took line numbers from one scan and inserted `use` lines earlier in the
same pass, so every later number pointed one line off and `pub(super)` landed on the wrong items; its
import script sliced a module's header and body by character offset and truncated two files before
the arms caught it. A rewrite over a file is one pass that collects, then one pass that writes from
the bottom up — or it re-reads the file between passes — and it cuts at line boundaries, never at
offsets, so that the diff it leaves is the diff it meant.

## 115. A guard whose effect a later step redoes is tested where it acts

Round 1590's route check in `decide` could be planted out without any end-to-end test noticing,
because `take_routed` at the end of each command re-converted the page and the window still arrived.
A guard that a later stage would make good anyway is invisible from the outside; its test sits at the
store it guards, where the planted-out guard changes what the store holds. Trap 13's plant is run at
the layer that the guard changes, not at the window.

## 116. The census's next site is read over the veraPDF corpus unfiltered

`archive_unbuilt`'s `run` filter keeps only the rows at least one document fails, so over pdf.js and
the corpora "the next `not-built-yet` site" can be absent without saying so — round 1585 found no
such site there at 2b and ranked over `doc/veraPDF-corpus` instead, where nine and three documents
reached the two it built. A brief that asks for "the next site" names the corpus the ranking is
taken over, and a round re-ranks over the veraPDF corpus before concluding that none is left.


## 117. A scratch measurement over the specifications' pages is capped or declared `--long`

Round 1589's first census looped over every page of the specifications under `doc/` and paid the
whole structure walk on each; it held the second lock lane for 1 074 s and a sibling's clock hold
waited 682 s of that. A measurement a round writes for itself is sized before it is queued — the
first five pages of each document answered the same question in a tenth of the time — and one that
must walk everything says `--long`, so the lock knows what it was given.

## 118. A scope added to the interpreter sits where every run of a group's content passes

Round 1588 held a group's blending space as a scope so that a device colour inside it could be
redefined (§11.7.2); its first placement was on the press-only path, and a group that gives up its
press reruns its content in the parent's space, where the colour's meaning has to hold as well. The
scope moved to `group_commands`, which every run passes — the press run, the black half and the rerun
alike — and the tests that plant it out are written for the rerun too. A new piece of interpreter
state is placed by listing the runs a group's content can take, not the one the round is thinking of.

## 119. A split in a crate no arm binary links is proved by the dependency graph, not by the arms

Round 1598 split `pdf-transform`'s redaction walk and did not walk the six arms: `cargo tree -i
pdf-transform` against the arm binaries' crates (`render-raster`, `pdf-sandbox`) finds no path, so
the binaries the arms run cannot differ from HEAD's and a forty-minute walk of the lock would have
answered nothing. "Nothing drawn moved" is a claim about the arm binaries; where the changed crate is
outside their graph the proof is the graph, written into the record with the command that printed it,
and the arms are for a change the binaries contain.

## 120. A validator's `over` right after an edition is re-read is first a question about the corpus's edition

After ADR 1991 fixed the validator to the 2005 XMP edition, `t3-archive-val` failed one veraPDF pass
file. Round 1592 found the file states `xmp:Rating` as `1.0` where the 2005 edition types the property
as a closed choice of Integer, which admits no decimal point; veraPDF reads a later edition's Real.
The validator was right, and the ruling is the corpus's against the standard (ADR 2020). The tell was
a sibling pass file differing only in the value's spelling. When an `over` appears right after an
edition changed hands, the first question is which edition the corpus file was written against, and
the answer is read from the edition's own type table before any validator line is touched.

## 121. A wall-clock difference callgrind cannot see is first run under `setarch -R`

Round 1593's encode walk ran 146–217 ms where HEAD's ran 84–108, with callgrind's instructions,
simulated cache misses, branch mispredictions and page faults equal between the two builds; the
difference was real but bimodal, and only with address-space randomisation off (`setarch -R`) was it
the same every run. Before any theory is built on a wall figure the deterministic counters do not
explain, both builds are run under `setarch -R` so that the difference is reproducible, and the
record says whether it survived.

## 122. An ADR a brief cites as the source of a limit is opened before anything is built on it

Batch ninety-five's brief, and A287's own reading, cited ADR 0604 for the limit on a GTK 4 widget's
surface; round 1606 opened it and found a decision about `--ignore-restrictions`. The limit is ADR
0244's Decision 2. A citation copied from memory into a brief is a claim that decays like any other,
and a round that would build on it reads the ADR's title and decision first, then names the right one
in its own record so the next brief does not inherit the slip.


## 123. A census's count of a witness's objects is read against the file's bytes before it is built on

`archive_unbuilt` said the halftone witness held types 6, 10 and 16; the file holds one halftone, and
the other two were graphics-state entries with halftone keys that nothing names, which this validator
counted and veraPDF did not. Round 1599 read the bytes, narrowed the validator to what an `HT` entry
or a type 5 halftone names, and built the one remedy the file needed (ADR 2034). A count the census
prints is a claim about the file; the witness is opened and veraPDF's own failed-check count read
beside it before the count becomes a contract.


## 124. A sentence that forbids "X or Y" is read against the standard's own list of operations

Table 333 forbids interpreting a `Private` element "or export[ing it] to other document formats", and
round 1603's brief took the clipboard copy for the export. §14.8.1's NOTE names "[e]xtraction of text
and graphics for pasting into other applications" and "[c]onversion to other common file formats" as
two operations, so the copy is neither the export nor the interpretation, and only the reading of
types inside the subtree is bound (ADR 2042). Before deciding which half of a disjunction binds an
operation, the standard's own vocabulary for that operation is found, and the decision quotes it.

## 125. A header's sentence about what an accessor returns is read against the accessor's `match`

`quorra.h` said `quorra_event_bytes` gives a submission's body; the accessor's `match` had no arm for
`Submit`, and the sentence had been false since the kind was added. Round 1612 found it only because it
was building the command that needed the body (ADR 2060). A sentence in a header or a doc comment that
claims coverage ("gives", "returns", "for every kind") is a claim about one `match`; the round that
builds on it opens the `match` first, and a test that calls the accessor for the kind in question is
what keeps the sentence true.

## 126. A doc comment moved out of the module that imported its link target is re-qualified in the same edit

A rustdoc link written as [`Engine::Interpreter`] resolves through the `use` lines of the module it
sits in; when a split moves the comment into a module that does not import `Engine`, the link breaks
and the `names` sweep is what finds it, one gate later. Round 1614 met this once in `pdf-font`'s split
(ADR 2064 §3). The split script qualifies every bare path in a moved doc comment — or carries the `use`
with it — in the same pass, so the sweep has nothing to find.

## 127. Code moved out of a binary is grepped first for the guarantees its location was giving

ADR 0378's second rule — nothing that judges a picture ever sees one — was held by the stand-in policy
living in `quorra`'s binary, which no crate can link to; ADR 2049 planned the renderer's move without
seeing that the rule depended on the address, and the rule's test fired on five moved files (round
1608, ADR 2053). Before planning a move out of a binary or a private module, grep the code and the
ADRs for what its location was providing — "a private module of a binary", "cannot link", "no caller
outside" — and decide for each whether the new home holds it by structure or by a test.

## 128. A walk change on a line-heavy page is priced on the walk's thread, apart from the makers

Round 1610's first build noted the kept runs line by line on the encode walk's own thread; a
one-threaded callgrind run put the note at 154–229 M of about 3.5 G instructions, and the fit frame paid
15–20 ms of encode for it (ADR 2056). As a duration the cost was invisible, because one-threaded counts
fold the parallel line makers' share into the same total and a thirty-five-instruction note per line
is a few percent of that. Price the walk's thread by itself — its instructions with the makers' excluded,
or its wall under the lock with the makers pinned — and read a per-element cost against the element
count before calling it small.

## 129. "Build on the fork's `main`" is checked against the pin before a commit is made there

Round 1619 was told to put the T.88 patches on `close2/hayro`'s `main`; that branch turned out to be
thirty-one commits behind the commit this tree pins, with its own JPEG 2000 work on top, so a branch there
would have carried a sibling codec backwards past the release the tree depends on (ADR 2074). Before
building on any branch of a fork, ask `git merge-base --is-ancestor <pinned commit> <branch>`; where the
answer is no, build on the pin itself — the base each patch header already names — and say so.

## 130. A teaching walk's example strings are produced by running the code

Round 1621 wrote the third walk with every readback, selection and clipboard string taken from a run —
the readback example, and a C program against the ABI for the drag and both copies — rather than worked
out by hand from the clauses (ADR 2078); the second way that touches no crate is `rustc --extern`
against the batch target's rlibs, choosing the hashes that compile together. A string worked out by hand
is a claim about the code that nothing checks, and the walk's reader cannot tell it from a measurement;
a string from a run is wrong only where the code is, which is the kind of wrong a walk is allowed to be.

## 131. A decision a window makes around the shared key table moves into `viewer-host` before a fourth host copies it

The sentence a markup key owes when nothing is selected — "select some text first", with §12.5.6.10's
reason — is written in `viewer-gtk`, `viewer-qt`, `quorra` and, since round 1618, `viewer-ffi`'s key
layer (ADR 2072), beside the one `viewer_host::meaning` the four share. The key table was moved into
`viewer-host` so that the windows would not each decide a press; a decision made around it in one
window is the same kind of thing, and the third copy is the signal to move it, the fourth the cost of
not having. Before adding a host, grep the windows for what they decide after `meaning` returns and move
it first.

## 132. A split that moves code one module down rewrites every `super::`, not only the `use` block

Moving a body into a child module changes what `super` means for every path in it: round 1622's first
pass rewrote the `use super::` block and missed the `super::`-prefixed paths inside expressions — the ones
preceded by `(` — and the compiler found them (ADRs 2080, 2081). A split script treats `super::` as a
token to rewrite wherever it stands, or rewrites to `crate::`-rooted paths that mean the same at any
depth; `cargo check` after the move is the gate, and a path the compiler accepts because it happens to
resolve to a different item of the same name is the case the script's assertion is for.

## 133. A site identifier is checked against `--remedy-sites` before a census walk is spent on it

Round 1615's first `--who` census named a FileAttachment site by a guessed identifier, printed no witness
and cost a second 220-second walk (ADR 2067). The site identifiers are printed by `tools/state.sh
--remedy-sites` in a few seconds; a census that takes a guessed one is a measurement of nothing, and
its empty answer looks like a corpus with no such document. Read the identifier off the list, paste it,
then walk.

## 134. A census's first-match reason is checked by freeing that reason alone

Round 1616's cut census labelled 21 020 refused marks by the first class each matched — "zero-length
subpath", "curved", "extent past range" — and the labels said three causes where there was one: with the
range condition alone changed, every row became legal (ADR 2068). A first-match label is the order of the
checks, not the cause; before building for the labelled reasons, free each candidate reason by itself and
count what the census then admits — the reason whose freeing admits everything is the one to build for.

## 135. rustdoc's column names the link's text, so a script that edits there walks back to the bracket

rustdoc reports an unresolved or private intra-doc link at the column of the link's *text* — inside the
brackets and the backticks — not at the opening `[`. Round 1629's script, which escaped a hundred and
thirty-six links by the reported column, had to walk back from that column to the bracket and assert it
found one before editing (ADR 1938's status line); an edit made at the column itself splits the text and
leaves the bracket, and rustdoc then reports a different link at a different column, which looks like
progress. Assert the bracket, assert the line is a comment line, and count the edits against the count of
errors.

## 136. An RFC's shape for an existing ABI is read against the ABI's founding ADR's refused alternatives first

RFC 0010 §3.2 proposed a table of callbacks for `quorra_window`; ADR 0247 §2 had refused callbacks for
this ABI years of rounds earlier, for re-entrancy, and round 1626 met the refusal's reason in the first
build — `say` called from inside `drain` while the window is borrowed, and a C# exception crossing
`UnmanagedCallersOnly` ending the process (ADR 2088). Before building a shape an RFC proposes for an
interface that already exists, open the ADR that founded the interface and read its *refused
alternatives*: a shape refused there is refused for a reason that is still in the code, and the RFC is
amended by a section rather than the ADR overridden in silence.

## 137. After a move, visibility is widened from the compiler's errors and then narrowed back one item at a time

A split that moves items into child modules leaves some of them reachable only with a `pub(super)` or a
`pub(in crate::…)`, and the easy answer — widen whatever the compiler names and stop — leaves every
widening in place whether it is still needed. Round 1630 widened from the errors and then removed each
`pub(super)` by itself, keeping only the ones whose removal failed the build: 47 kept and 3 narrowed in
the colour split, none to narrow in the syntax split (ADRs 2096, 2097). A fifteen-line script does the
narrowing; the proof it leaves is that every visibility word in the result is one the compiler asked for.

## 138. A neighbour's mid-edit that breaks the shared build is stepped around with an export of one's own files

In a batch of eight rounds in one worktree, a sibling's split in progress can leave the tree uncompilable
for an hour; round 1623 met a `pdf-colour` split that way and ran its tier 1 in `tools/batch.sh export
<session> --as <name>` — HEAD plus its own eight files — whose warm target directory checked in about
twenty seconds, then confirmed the worktree copies identical with `cmp` (ADRs 2082, 2083). Waiting for
the neighbour costs the wall budget and a cache; the export costs a build directory the batch already
owns. The gate figures reported are then the export's, and the record says so.

## 139. A toolkit's gate on a shared decision becomes a typed parameter, not a step left in the host

When round 1628 moved the key decisions into `viewer_host::decide`, GTK still had one step of its own — a
placed control may have consumed the Enter, and the page's button must not also be pressed. Left as a
step in the GTK host, that would have kept one copy of the decision alive beside the shared function; as
the `Keyboard::{Page, Control}` parameter it is a fact the host states and the shared function decides on
(ADR 2092). A host's exception to a shared decision is a parameter of the decision, so that the next host
cannot forget it and the rule has one text.

## 140. An invariant that is a sum is also tested on one example whose content is known

`TextLine`'s rule — its text is exactly its characters' text joined — was checked by the census as a
byte-count equality, and the equality held while the line read "Der Drucc": a replacement had been
added once per code it covered and the counts still summed (round 1625, ADR 2086). A sum is satisfied by
any rearrangement of the same bytes and by any pair of errors that cancel; one example with its expected
string written out — the walk's own "Der Druc" — is what a sum cannot replace. Where a gate checks an
invariant by counting, add one fixture whose content the test states in full.

## 141. A transcript's usage is read on its input side; its output count is where the stream opened

A Claude Code transcript writes a request's `usage` as the stream began, so its cache and input figures
are the request's and its `output_tokens` is not: round 1636 found a request that wrote a 37 kB file
recorded with 16 output tokens, and batches 96–98's 3 952 round requests summing to 0.05 M. A census
that priced output from those figures would call generation free. `tools/cache-census.py` prices the
input side only and takes a re-issued wait's output as a stated constant; a figure about what an agent
generated comes from somewhere that counted it, never from a sum over a transcript's `output_tokens`.

## 142. When a dependency gains a bound, every test whose only input the bound makes cheap is listed and re-aimed

Round 1633 moved the JBIG2 codec to the fork that carries ADR 1447's loop bounds; the seven fuzz-found
symbol dictionaries that had been the confined worker's only slow input were now refused in milliseconds,
and the worker's kill — the thing those tests existed to exercise — was reached by nothing (ADR 2102). A
pass-either-way check would have hidden it. When a dependency's new bound makes an input cheap, list the
tests that used that input for its cost, give each one an input that is still honestly expensive, and
require the codec's own sentence rather than any outcome, so a lock that stops naming the fork fails the
test instead of passing at a deadline.

## 143. A drive's `wrong` on a loaded machine is re-run before it is attributed

Round 1634's drive of the GTK window scored two steps `wrong` — controls not yet drawn in the screenshot
— on a machine carrying seven sibling rounds; the batch-start binary scored the same steps right, and a
re-run of the round's own build scored them right too (ADRs 2104, 2105). A `wrong` verdict from a
photograph is a race between the window and the camera as much as a judgement of the change. Before a
`wrong` is attributed to a change, re-run the step once on the same binary and once on the batch-start
build; attribute it only when the change's binary is the one that stays wrong.

## 144. A substitution declined because it "settles what the processor decides" is checked against what the file can reach

ADRs 0965 and 0982 declined to name an `ICCBased` space `DeviceCMYK` because §8.6.7's overprint mode
distinguishes the two; round 1631 read §8.6.7 to its condition — the mode "shall have an effect only when
the overprint parameter is true" — and a whole-file scan for `OP`/`op` true, no content walk, decides
whether that difference can reach any mark in this file (ADR 2098). A decision the processor makes
differently for the two spellings matters only on a page where the condition holds; before declining a
substitution for the processor's sake, find the condition in the clause and ask the file whether it is
ever met.

## 145. A `--keep-going` claim is checked at one job

Round 1641 gave the docs gate `--keep-going` so that a failing crate no longer hides the errors of the
crates after it, and planted an error in two crates to prove it; with cargo's full job count both errors
showed without the flag as well, because every doc job had already started before the first failed. Only
at `-j 1` does the missing flag hide the second crate, and only that control makes the planted test say
anything (ADR 1938's status line). A claim about what a build flag prevents is tested under the
scheduling that would otherwise mask it: one job, so the order is the one the claim is about.

## 146. After a compaction, every ADR the old text cited is looked for in the new

A compaction deletes sentences, and a sentence that cited an ADR may have been the only place a built
capability was named: round 1645's pass over the files a kind opens left ADR 1751 — a named destination
carried out as its page and its view — cited nowhere, and the check that looks for every old citation in
the new texts and the other documents was what found it and restored the sentence (ADR 2126). The
check is mechanical: the set of ADR numbers in the texts before, minus the set in the whole tree's prose
after; each number left is a capability the compaction lost, to be put back in one sentence or shown to
be named elsewhere.

## 147. A census of refusals names the output's second verdict

A conversion the converter prepares and writes can still be refused when its output is re-checked
against the target, and that refusal carries no `Refused` decision of its own; a census that reads only
decisions shows such a document refused with no reason. Round 1639 found thirteen at level B and
fourteen at part 4 listed that way — the whole of the colour-family group it then built a remedy for
(ADR 2114). A census of refusals lists the second verdict's refusals as their own kind, with the
requirement the output failed, or it is not a census of refusals.

## 148. A construction on simulation of simplicity is checked in its own perturbed world

Round 1643's union-boundary walk decides ties by simulation of simplicity — every coordinate carries an
infinitesimal perturbation so that no three points are collinear and no two crossings coincide — and its
first check against a finite-offset brute-force ray reported thousands of mismatches wherever pieces'
edges were collinear, which is exactly where the perturbed world and the real one disagree by design;
the two real defects hid behind them (ADR 2122). A construction that perturbs is checked by a probe that
perturbs the same way: an exact ray to a point on the edge, compared contour by contour against a dumped
mark, so that a mismatch is a defect and not the tie-breaking rule seen from outside.

## 149. A launch delay is read as blocked time and instructions before it is read as work

Round 1644 was sent after thirty milliseconds Qt's window seemed to pay for the device lane, and found
nine: the window's thread executed the same instructions to its first resize on both lanes to a hundredth
of a percent, and the difference was time blocked on the dynamic loader's lock while the device thread
loaded the Vulkan driver and its LLVM beside it (ADR 2124). Two threads that both load shared objects
serialise on that lock, and the wait shows as neither CPU time nor anything a trace names. Before a launch
delay is read as work to remove, sample each thread's `schedstat` to the milestone and count its
instructions under callgrind; what is blocked is not a lever on the code that waits.

## 150. A project file a round cannot build is parsed as XML before it is handed to CI

Round 1650 wrote a WinUI 3 project on a machine with no Windows and no `dotnet`, so nothing of it could
be compiled before CI; a scratch check that parsed every `.csproj`, `.xaml` and manifest as XML found a
`--` inside an XML comment in the project file, which XML forbids and MSBuild would have refused, and
named two planted faults as well (ADR 2136). When the only compiler is a CI runner a push away, the
parse the round can run — XML well-formedness, balanced brackets in the code-behind — is run first; it
costs seconds and turns the first CI run from a syntax check into the finding it is meant to be.

## 151. A recipe that names a fixture directory derives it from the tree, and a test counts what it yields

`fuzz/seeds.sh`'s certificate arm named `crates/pdf-model/src/*.rs` as where the certificate fixtures
were; the twelve certificates moved to `pdf-signature` and the arm seeded none for weeks, printing
"0 out of a fixture" with nothing failing (round 1653, ADR 2142). A path to fixtures is a claim about the
tree that decays when a split moves them — trap 25's shape in a script rather than a test — so the recipe
walks the tree for what it needs (every source file under `crates/`) rather than naming a directory, and a
test counts the fixtures the recipe yields against the constants the tree holds, so the day the count
drops is the day it fails.

## 152. A split script finds an item's start at the previous blank line, not by walking over `///` and `#[`

Round 1654's split walked back from a method's `fn` line over the lines starting `///` or `#[` to find
where the item began; a method whose doc comment was followed by a multi-line `#[expect(...)]` lost the
attribute's continuation lines, which start with neither, so the doc and the attribute's head stayed with
the previous method and clippy's empty-line-after-attribute error was what showed it (ADR 2144). An item
begins after the previous blank line (or the previous item's closing brace), and a split script that
moves whole items finds the boundary there; it never classifies the lines of an attribute one by one.

## 153. A dependency called "C" is checked in its own `lib.rs` and feature table before the confinement argument is planned

Batch 101's brief sent round 1651 to confine `mozjpeg-rs` because it "wraps the C library `mozjpeg`";
the crate's own source carries `forbid(unsafe_code)`, has no build script, and reaches C only behind an
opt-in feature nothing enables — it is imazen's pure-Rust port — so principle 3's written justification for
a C dependency was not owed, and the confinement rests on ADR 0014's reasons and the SIMD `unsafe` in
five of its packages instead (ADR 2138). A dependency's language is read off the crate — its lint
attributes, its `build.rs`, its feature table on crates.io — before an argument is built on it; a name
that sounds like a binding is a hypothesis.

## 154. A brief's named instrument is checked to reach the subsystem it is meant to measure

Batch 101's brief told round 1649 to measure the idle build with `window_against_whole`; that example
draws on the processor and never reaches the device, so it could not have seen a change to what the
device keeps between frames, and the round measured with `zoom_frame` instead (ADR 2134). An instrument is
a claim about which code it exercises; before a before/after is planned on one, `cargo tree` or a grep of
its crate graph says whether the subsystem under change is in it. The same shape as trap 227 for gates:
the name of the measurement is not the measurement.

## 155. A dependency added to a confined worker is read with `cargo tree --workspace -e features` before it is called done

Round 1651 added `mozjpeg-rs` to the sandbox worker and its own gates were green; the shipped worker is
built in the same Cargo run as every other program, and `bytemuck`, which the encoder's dependencies
link, arrived with five features that `color`, `peniko` and `winit` turn on — more code and more
`unsafe` inside the confined process than its manifest asks for, which the worker-features gate refused
at the merge (ADR 2138 §6). Feature unification reaches a binary through every package it links, not only
its decoders: before a new package in a confined worker is called done, `cargo tree --workspace -e
features,normal -i <package>` for each package it brings says what the shipping build will turn on.

## 156. A departure is re-read in every walk with a bound and every writer the row names

Round 1659 re-read the twenty `departed` rows and found its two defects not in the code each note argued
for but in a second implementation beside it: the merge's field walk, which stopped at sixty-four links
and left the fields past it out of the §12.7.4.2 check in silence, and a save that dropped an import's
template pages without a word (ADR 2154). A departure's note defends one place; the row's `code` array
names several, and a bound or a writer in any of them can cut short what the clause requires. Re-reading a
departure means reading each bound's overflow and each writer's omission in every file the row names, not
only the sentence the note defends.

## 157. A cold-cache figure checks that the drop worked before it is judged

`dd oflag=nocache` — `posix_fadvise(DONTNEED)` — exits 0 on a `tmpfs` file and on a file another process
holds mapped, and nothing in a clock figure then says the "cold" open read warm pages; round 1661 gave
the launch gate `fincore`'s count of what stayed resident after each drop and declines both cold figures
when it is not zero, ahead of the machine guards (ADR 2158). A figure that claims a cold cache carries
the evidence of the eviction with it, or it is the warm figure under another name.

## 158. A drive's "before" binaries are the batch's own warm-up build of HEAD, copied before a sibling rebuilds

Round 1660 needed HEAD's windows for its before-and-after drive and built them in an export, which waited
3 375 s behind a sibling's gates on the lock's lane; the batch had built HEAD's release binaries at its
open, into the shared target, before any round changed a line (ADRs 2156, 2157). A round that will drive
"before" copies `quorra`, `quorra-gtk`, `quorra-qt` and the workers out of the batch target into its
scratch in its first minutes, while they are still HEAD's — one copy, no build, no queue — and its
"after" is the only export it owes.

## 159. A lever on `rasterize` is measured in one hold with the launch and turn gates, from the start

Round 1657 moved the read-back page's medium passes to the device and measured the toolkit launch gate
before and after, as its brief named; the same hold showed `quorra`'s page turn through `launch_path`
falling from 4.1 to 2.5 ms, below three bands' floors, because the page turn draws through the same call
(ADR 2150). A change to `rasterize` or to anything under it reaches every window and every gate that
times a frame, not only the one the brief names; the round's one `--clock` hold includes `t2-launch_path`
and `t2-turn_path` with the toolkit gate from the start, so the bands that move are re-taken by the round
rather than found by the merge.

## 160. "The C# cannot be compiled here" is tested before it is believed

Three rounds wrote the WinUI 3 window on the premise that only CI's Windows runner could compile it;
round 1666 installed a .NET SDK into its scratch and compiled both C# projects on Linux against the real
Windows App SDK references with warnings as errors, the XAML stubbed and the packaging tooling off — and
the first compile found two errors the first CI run would have stopped on (ADR 2168). The SDK and the
check script are kept at `/home/AI/dotnet-check/` (`cs/check.sh`); a round that writes C# runs it before
it reports, and leaves to CI only what Linux cannot see — the XAML compiler, the packaging, the MSVC build
of the ABI's library.

## 161. A drive runs on the export's copy of its script, never the worktree's

Bash reads a script as it runs: a sibling's edit to `tools/drive-windows.sh` during round 1668's drive
made bash run the steps twice and fail one with `local: can only be used in a function`, and one window's
step read `wrong` for the edit, not the change (ADRs 2172, 2173). In a shared worktree the drive — and
every long script a round runs — is started from the round's export (HEAD's copy of the script, or the
round's own) and not from the worktree's path; the same rule the merge learned for `tools/batch.sh` when a
resumed round rewrote it mid-chain.

## 162. Where a sibling edits the same crate, the multiset proof is taken from the round's own hunks applied to HEAD

Round 1669 rewrote doc comments across `viewer-core` while slot 5 edited the same crate's code; a
multiset of the worktree's files would have mixed both rounds' hunks and proved nothing about either.
The round classified its own hunks — every one a `///` or `//!` line — applied them to HEAD's copies, and
compared the multiset of non-comment lines there: unchanged in twenty-seven files, with a planted type
change named (ADR 1938's status line). In a shared worktree the proof that nothing moved is a statement
about the round's hunks, not about the file, so it is taken from the hunks.

## 163. A census that decides a statement-or-loss row runs before and after under both shipped profiles

Round 1663 made the machine's face a loss the printing profile discards and the `as-shown` profile stops
at; under `as-if-printed` alone the thirty-six documents simply moved from refused to converted, and only
the `as-shown` column showed what the change meant for a reader who wants the page as the window drew it —
a construction refusal becoming a stop at a named loss (ADR 2162). A row that can be a statement under
one profile and a loss under another is measured under both before and after, or the census answers one
reader's question and calls it the converter's.

## 164. A cost lever read off a profile's split is priced at its ceiling against the bar before it is built

ADR 2122 §5 read the folding measure's profile — 45 % pair comparisons, 43 % crossings that bound nothing
— and named discarding those crossings as the lever to bring the measure under a fifth; round 1664 built
the discard to its end and found the pair comparisons alone cost 80 M instructions a draw against a whole
budget of 68 M, which the profile's split had said before anything was written (ADR 2164). A lever is a
claim that removing one share brings the total under a bar; the arithmetic that checks it — the shares that
cannot be removed, summed, against the bar — is done first, and a lever that fails it is not built.

## 165. A drive step for a new settings word runs in `quorra-confined` too

Round 1667 added `--launched-documents` and drove it through `quorra`, `quorra-gtk` and `quorra-qt`, all
green; the confined window's `CARRIED` list refuses every word it does not know, so the same step read
`wrong` four times there until the word was carried (ADR 2170). A new settings word is driven in all four
windows before it is reported, and the confined window first, since it is the one that refuses by
construction rather than by accident.

## 166. A review script that divides by a count first prints the commits it read nothing from

Round 1704 ran `doc/reviews/1456/slots.py` over batches 104–108 and it crashed dividing by zero: five
batch commits wrote "Round durations … not available", and the script had never met a batch body without
a figure. The crash *was* the finding — eight of thirty-two durations stated, no tool-use counts — but a
script that stops at the first empty body hides how many there are. A review instrument that averages,
sums or divides over a span prints first the commits or records it could read nothing from, by name, and
only then the figure over the rest (`doc/reviews/1704/slots.py`; ADR 2242).

## 167. A split that moves `#[cfg(test)] mod tests;` into files of its own runs the `flags` gate

`tools/conformance/src/flags.rs` read every user-facing string for a flag the binary does not accept and
skipped only the inline `#[cfg(test)] mod tests { … }`; round 1711's split of the confined wire put the
tests into `protocol/tests/*.rs` behind `#[cfg(test)] mod tests;`, and a fixture string naming
`--trust-anchors` read as a message of the program's (ADR 2256). The sweep now skips the files such a
declaration names; a split that moves tests out of line runs `cargo test -p conformance --test flags`
before it reports, and reads the sweep to see that it still treats the moved files as test code.

## 168. An export whose sources change while its tests run builds into a private `--target-dir`

Cargo's lock protects a build, not a test binary that is already running: in batch one hundred and eight a
mutation export built on the shared target directory replaced nextest's executable in the middle of its
suite, and twenty tests failed on a binary that was no longer the one under test (ADR 2233, the
"Test infrastructure" section). A round whose export's sources will change while a suite runs — a
mutation, a plant, a bisect — gives the export a target directory of its own by `--target-dir` on the
command line, never by exporting `CARGO_TARGET_DIR` (ADR 0344), and names the tested binary's hash in
its report.

## 169. An Xvfb started with `-displayfd` names its display on that descriptor, and trap 181's check is made on that number's lock

Batch one hundred and eight's first display wrapper started Xvfb with `-displayfd` and then waited for the
lock file of a display number it had chosen itself; `-displayfd` picks the number and writes it to the
descriptor, the lock it creates is that number's, and the wrapper waited on a file that never came,
stopping before the viewer launched (record 1703). The display is read off the descriptor first, and
the ownership check trap 181 asks for — the lock absent before, the wrapper's own pid in it after — is
made on that number.

## 170. A test selection that runs zero tests is a failure, not a pass

GTK's first binary-target invocation in round 1690 selected zero tests and exited 0; the corrected
library selection ran four (ADR 2214). A gate or a round's own check that filters tests by name or by
target prints the count it ran and treats zero as the selection's failure — `cargo nextest run` prints
`0 tests run` and exits 0 unless `--no-tests=fail` is given, so the flag or the count is part of the
command.

## 171. Before a band is held to a whole-process memory figure, its file-backed and anonymous shares are read apart

The launch gate banded `open_peak_mib`, the open child's whole `VmHWM`, and was red at four merges running;
round 1705 read the child's `smaps_rollup` and found seven parts in eight of that high-water file-backed —
the test binary's own text and the C library's — moving 7 to 11 MiB with the page cache while the
anonymous share was identical to the kibibyte between the base and HEAD. A megabyte held live across the
open moved the old figure from 8.8 to 8.8 inside its 5..9 band and the anonymous figure from 0.805 to
1.812 (ADR 2244). A band on memory names the share it holds — anonymous for what the program asked for,
file-backed never — and the plant that proves the instrument sees a megabyte is run before the band is.

## 172. A gate that reads one named source file is taught the directory in the round that splits the file

Three tests of `viewer-ffi` read `src/abi.rs` by name — the unsafe positions, the header against the
library, every query reaching the ABI — and round 1719's split of that file into eighteen children would
have left each reading an 88-line root and passing on nothing (ADR 2272). A split lists, before it moves a
line, every gate and every `tools/state.sh` section that names the file (`grep -rn '<file>' tools crates
doc/todo/02-every-round.md`), and the same round makes each read the root and its children joined, with
the counts it held before — 355 entry points — printed unchanged after.

## 173. A conformance test that reads `git ls-files` runs in the worktree, and the report says which tests ran where

An export made by `tools/batch.sh export` sits under the ignored `scratchpad/`, so `bounded`,
`state_sections`, `submodules` and `workspaces` — the tests that read git's file population — cannot pass
there, and rounds 1706, 1713 and 1719 each lost a run finding that out (ADR 2260). When a sibling's
mid-edit forces a round's gates into its export (habit 138), the tests that read git run in the worktree,
the rest in the export, and the record's `**Gates.**` paragraph names which ran where.

## 174. A rustdoc quotation is attributed to the nearest `§` before it, so the quoted clause's own sign stands directly before the quote

`every_quotation_is_the_standards_own_words` finds the clause a blockquote belongs to by the nearest
`§` above it; round 1718 put two verbatim quotations under the right clause with a neighbouring clause's
`§` between, and both failed as not that clause's words (record 1718). The quoted clause's `§` goes on
the line that introduces the quote and no other clause sign sits between them; and an ISO 19005 clause is
written "section 6.2.11.2", never with `§`, which the checker reserves for ISO 32000-2.

## 175. A corpus census whose per-document lines are read back runs one test at a time

`tools/state.sh archive` runs `archive_corpus`'s tests in parallel, and their stdout interleaves; round
1714's first per-document diff of HEAD against its export was garbled by two tests' lines crossing (ADR
2262). A census whose lines are going to be read document by document — a diff against HEAD, a refusal
keyed by file — runs with `--test-threads=1`, or writes one file per test; the parallel run is for the
verdict alone.

## 176. A control for a colour inherited into a group makes the group composite

An opaque single fill alone in a `DeviceCMYK` group gives the group's space up and draws in the parent's,
so round 1715's first control for a `g` inherited into a CMYK group was green on HEAD and on the fix
alike and proved nothing; painted at half alpha over a colour the group restores from, it went red on
HEAD with the expected pixel worked from ADR 0263's four corners (ADR 2264). A control that claims a
group's colour space decides a pixel first makes that group composite — a partial alpha, a blend, or a
second mark under it — and shows the expected pixel differs from the parent-space one before it is
believed.

## 177. A vocabulary item a census reports unreached is first looked for in the events

`tools/state.sh hosts` printed "Query: 35 of 36 reach the ABI" for two batches, and the one it named,
`LogicalSelection`, needed no entry point: `quorra_copy` raises `Event::Copied`, whose `logical` field is
the same answer, read back by `quorra_selection_copy_text` (ADR 0519; round 1725, ADR 2284). A census of
a vocabulary against the entry points that ask it counts the answers an event carries too, by a checked
list of its own, so that an item answered without being asked is not reported as a gap and not given a
second way to be asked.

## 178. When a loss becomes machine-dependent, the fixtures are grepped for the faces this machine has

Five archive tests passed on HEAD only because the unrestated embedding route silently dropped the
`machine-face` loss: their fixtures named `/LiberationSans` and `/Tahoma-Bold`, faces installed on the
test machine, so the shipped face was never needed and no loss was ever stated (round 1722, ADR 2279). A
change that makes a loss or a refusal depend on what the machine has installed — a font, a profile, a
codec — greps the fixtures for the names `fc-list` (or the equivalent census) prints, and either names the
loss in the test or chooses a fixture no machine supplies; a test green for want of a statement is a
statement.

## 179. The arms are ranked for defects with the glyph quantum off

Ranked by their worst lane's mean error, HEAD's arms put three pages at the head — `copy_paste_ligatures`,
`issue16316`, `endchar` — each a single line of type cropped to its own box, reading 0.6 to 1.1 of 255;
with ADR 0498's glyph quantum off they read 0.002 to 0.07, and the first page a defect explained was
fifth (round 1726, ADR 2286). The quantum is a trade ADR 0498 made and not a defect, and it dominates any
page small enough that its glyph edges are most of its pixels; a round that ranks the arms to find a
defect ranks them with the quantum off (the `--no-glyph-quantum` arm, or the oracle's own switch), and
reads the gpu lane's 4x glyph pages as raster's sample grid (ADR 0076) before anything else.

## 180. Before a brief routes a backend's finding into a report type, `cargo tree -i` says whether the report can see it

Batch one hundred and eleven's brief routed a folding-union decline into `pdf_model::Unsupported`, the
page's unsupported-features report; that report is worded before anything is drawn and lives in a crate
above `render-cpu` (`pdf-model` depends on it, not the reverse), so the planned route did not exist and
round 1723 typed the decline where the draw returns instead, with the hosts still to carry it (ADR 2280).
A brief or a round that plans to carry a finding from one crate into another's report runs `cargo tree -i
<producer>` first and reads whether the report's crate is above or below it; if above, the finding travels
in the draw's return value and the report is the host's.

## 181. A fixture's colour is one the specification permits before its expected pixel is derived

Round 1729's first e-sYCC fixture carried two colours outside PIMA 7667 section 4.3.4's luminance bound,
and the expected pixels worked by hand from the encoding's equations were right for values the encoding
never admits; the fixture was replaced by four permitted colours rather than the expected values adjusted
(ADR 2292). A fixture whose expected value is derived from a specification's formula first checks that
its input lies inside the domain the specification states for that formula — a gamut, a luminance bound,
a depth — because a formula evaluated outside its domain produces a number and not an expectation.

## 182. An export's sources are corrected while its walk is still queued, not by re-queuing

A census or an arms walk of an export can wait an hour or more for its turn on the first lane, and the
build it runs inside its hold reads the export's sources at that moment: round 1734 corrected its
export's sources while the walk was queued and the hold's build took them, where re-queuing would have
cost the whole wait again (ADR 2302). A round that finds a fault in its export while a walk of that export
is queued fixes the sources in place and leaves the queue position alone — trap 233's rule still holds, the
export is not *re-exported* — and reads the hold's build log to see the correction was taken.

## 183. A drive step about the processor's own drawing launches every window with `--cpu`

Round 1728's step for a union decline saw nothing in the toolkit windows at first: under Xvfb with a render
node the device lane takes page one, so the processor never drew the planted mark and the title had nothing
to say (ADR 2290). A drive step that exercises what only the processor does — a decline, a fallback, an
oracle route — launches each window with `--cpu` (the confined window's equivalent included) and says so
in its name, and its control run without the flag shows the step reading "not offered" rather than a
false pass.

