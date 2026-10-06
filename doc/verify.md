# Verify it — the instruments in this tree, and when to run them

Status: **standing** — the catalogue. `doc/todo/02-every-round.md` §2 is the round's own gate
sequence and **owns those commands**; this file owns everything a round runs when it has a reason
to, which is most of what is here.

Read by: whoever needs `cargo deny`, a fuzz target, a callgrind counter, a cross-target check, a
census example or the AT-SPI recipe. `doc/HANDOVER.md`'s reading table is the pointer to this file.

**The gate sequence is not repeated here, deliberately.** Two documents stating the same commands
is how they drift apart, and they had: this list said 1369 tests where the gate printed 1371, and
omitted `render-raster`'s corpus gate altogether. `doc/todo/02` §2 is the one copy.

**This catalogue does not name every example, and how many it misses is counted rather than
claimed**: `tools/state.sh instruments` lists each example under a package's `examples/` whose
name appears nowhere below, with the first line of its own header, which says what it measures.
An example a round writes gets its line here in the same round.

**Nothing here runs in a fresh clone until the specifications are unpacked**, which is one command
and is in `doc/environment.md`.

```sh
cargo run -p conformance --bin ledger      # counts the statuses, writes nothing
cargo run -p conformance --bin ledger -- --write  # regenerates rows, keeps every status
tools/governing-quotations.py              # or `tools/state.sh governing`
  # **The other half of `--bin quotations`.** That one reads every quotation in this project's
  # prose against the specification Markdown in `doc/md/` and reports the ones that match a
  # specification and then diverge — so a quotation of the *other* document this tree quotes
  # verbatim, `CLAUDE.md` itself, matches nothing, and matching nothing is indistinguishable
  # from not being a quotation at all. Twenty-three conformance-ledger rows and one module
  # header in `pdf-syntax` quoted a sentence `CLAUDE.md` retired on 2026-09-03 for ninety-two
  # sessions on that account, and the sentence named §7.5.7's object-stream packing as out of
  # scope while the module next door generated object streams (ADR 0989). It **reports and does
  # not fail**, for `--bin quotations`' own reason: attribution here is a proximity rule, so the
  # residue includes correct prose — a note quoting its own earlier wording, a document saying
  # what `CLAUDE.md` *used to* state. `doc/adr/`, `doc/history/` and `doc/rfc/` are printed
  # apart, because a record quoting the sentence it retired is quoting it correctly. Seconds.
cargo run --release -p pdfref --bin undrawn -- <the oracle gate's log>
  # **`doc/todo/00` step 7, which was a recipe for seven hundred sessions.** Our ink minus the
  # lightest reference's, over the artefacts the oracle gate already left on disk — the one defect
  # a distance cannot name, because a page drawing less than everybody is not necessarily far from
  # anybody. It reads its population *and* its exclusions off that log rather than assembling
  # either, which is the whole point: the fifteenth hand rebuild measured 775 of the 838 pages it
  # listed where the gate had printed 839, having silently dropped the entire `doc/corpora/pdfbox`
  # population, because the gate prints `pdfbox/cweb.pdf page 10` and writes
  # `pdfbox/cweb/p10/…` — the corpus label is a directory, not part of the name (ADR 0985, and
  # trap 25's mirror). It prints the denominator and exits non-zero on a page it cannot measure
  # rather than dropping the row; it **reports** the alarm and does not ratchet it, because the
  # groups live in the gate and a note is a person's (trap 39). ~3 min, renders nothing.
RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -- --force-warn clippy::cognitive_complexity
  # **The lint `clippy.toml` configured and no crate ever enabled.** `clippy::cognitive_complexity`
  # is a nursery lint and the workspace names it nowhere, so its threshold was a setting for an
  # instrument that had never run once. This is the line that runs it. `--force-warn` rather than
  # `-W` is what keeps it a warning under the `-D warnings` the gate sets, so the whole workspace
  # is judged instead of the run stopping at the first crate that trips it.
  #
  # **Read the metric before reading the count**, because it is not the one the name suggests: it
  # counts `if` expressions and subtracts returns, so a `match` of any width and a nesting of any
  # depth both score nothing. Both were measured against planted functions under this tree's own
  # `clippy.toml` before the threshold was deleted (trap 13) — a seventy-five-arm `match` scores
  # nothing, sixty sequential `if`s score sixty-one. That is why `clippy.toml` no longer carries a
  # threshold: the lint cannot see the shape this tree's long functions have, and a number nobody
  # can act on is worse than no number. ADR 1024. Minutes, and it renders nothing.
cargo deny check                           # from the workspace root: fuzz/ is its own workspace
# The two platforms without a confinement, checked the way CI checks them. **`RUSTFLAGS` is not
# optional**: the workspace's lints are `warn` so that a local build stays usable and CI turns them
# into errors, so a cross-target check without it is a different build from the one that gates a
# push. Three dead constants off Linux got through exactly that gap (ADR 0194).
RUSTFLAGS="-D warnings" cargo check --target x86_64-pc-windows-msvc -p pdf-sandbox -p pdf-render -p viewer-confined --all-targets
RUSTFLAGS="-D warnings" cargo check --target aarch64-apple-darwin  -p pdf-sandbox -p pdf-render -p viewer-confined --all-targets
RUSTFLAGS="-D warnings" cargo check --target x86_64-pc-windows-msvc -p viewer-ui --all-targets
RUSTFLAGS="-D warnings" cargo check --target aarch64-apple-darwin  -p viewer-ui --all-targets
  # **This is the line that stands in for CI's `platforms` job**, and it is worth knowing what it
  # reaches: `-p viewer-ui` drags `viewer-accessibility` in, whose `Bridge::new` takes a waker that
  # only the Linux build has an adapter to give — and one unused parameter off Linux failed *both*
  # platform jobs for five pushes while everything here was silent, because these two lines are
  # nobody's core gate and nothing asked for them. The darwin one was added in the same round
  # (ADR 0450); the Windows one alone would have caught it, and two say which platform.
  # `--all-targets` rather than `--bins` since the three-hundred-and-eighty-fourth: `DEFAULT_BACKEND`
  # is a `#[cfg(windows)]` constant and the test that states it is DX12 lives in the binary's own
  # `mod tests`, which `--bins` does not build. `-p viewer-ui` has no `criterion` in its dev tree,
  # so this one does cross-compile where `--workspace --all-targets` does not.
  # `--workspace --all-targets` does *not* cross-compile here: `criterion` pulls `alloca`, whose
  # build script needs a C toolchain for the target and this machine has neither MSVC nor macOS's.
  # The CI runners do, which is why the `platforms` job builds the binaries and these two check
  # what a benchmark does not reach.
RUSTFLAGS="-D warnings" cargo check --target x86_64-pc-windows-msvc -p viewer-ffi --all-targets
RUSTFLAGS="-D warnings" cargo check --target aarch64-apple-darwin  -p viewer-ffi --all-targets
  # **The C ABI cross-compiles and the two toolkit hosts do not**, which is worth a line rather than
  # a shrug: a C ABI over a pure-Rust core binds no platform, and that is most of the point of
  # having one. Both of these pass; added in the four-hundred-and-eleventh (ADR 0247).
  # **The two native hosts are deliberately in none of these**, and asking for either says why:
  # `glib-sys`'s build script needs a cross-compiling `pkg-config` wrapper, and `viewer-qt`'s
  # `cc-rs` wants `lib.exe`. Both targets would also need the toolkit's own development files,
  # which is a platform package manager's job rather than a Rust target's. A host binds a platform
  # and is checked on it — the same rule that makes `viewer-accessibility` Linux-only in its own
  # manifest (ADRs 0214, 0244, 0246). What that costs on *this* machine is the other half of the
  # same rule: `cargo clippy --workspace` and `cargo test --workspace` build both hosts, so GTK 4's
  # and Qt 6's development files have to be installed to run the gates at all. CI installs
  # `libgtk-4-dev`, `qt6-base-dev` and `qt6-base-dev-tools` for exactly that reason.
  # **And the toolkit is not the only thing about a machine that decides whether `viewer-qt`
  # links**: `qt-build-utils` picks the first of `lld`, `ld.gold`, `mold` it can run, this machine
  # has `lld` and GitHub's runner has not, and the two choose different archive semantics. That
  # cost `test` a job while `check` stayed green, because `clippy` links no binaries. To run what
  # the runner runs, build with a `PATH` of symlinks to `/usr/bin`'s entries *minus* `lld`,
  # `ld.lld`, `lld-link` and `wasm-ld` — `env PATH=$dir cargo build -p viewer-qt --all-targets`.
  # ADR 0463; `crates/viewer-qt/build.rs` says what makes it link under either.
# **Three of CI's jobs are here and nowhere else, and in the nine-hundred-and-thirty-seventh
# session all three were red on `main` while `doc/todo/02` §2 ran green twice.** That is not a
# contradiction; it is what this file is for. The last CI run to pass on `main` was a hundred
# sessions earlier, so three unrelated defects had accumulated where no local gate looks:
#   * **`deny`** — `error[yanked]: detected yanked crate (try cargo update -p wnaf)`. A yanked
#     crate is nobody's code change: the version in `Cargo.lock` is withdrawn upstream *after* it
#     was locked, so this job goes red on a tree that has not been touched. `cargo update -p <it>`
#     is the whole fix where a patch release exists, and it is worth running `cargo deny check`
#     rather than assuming, because the failure names the crate and the remedy.
#   * **The Windows check** — `error: field 'program' is never read` in `confined-transport`, a
#     field the only reader of which is inside a `#[cfg(unix)] impl`. Third instance of ADR 0194's
#     shape and second of ADR 0450's, and the lines above are what see it. `#[cfg(unix)]` on the
#     field is the fix, never `#[expect(dead_code)]`: the field *is* read on Linux, so an
#     expectation would be unfulfilled there and fail the other way (trap 7's rule has a direction).
#   * **`cargo +nightly miri test -p pdf-render -p pdf-syntax --lib`** — `error: unsupported
#     operation: mkdir not available when isolation is enabled`. **One unsupported operation aborts
#     the whole run**, so four file-system tests in `pdf-syntax::file` took 209 passing ones down
#     with them and the job reported nothing about aliasing at all. Miri is here for undefined
#     behaviour in the parsers; a test whose subject is the file system has nothing for it to check,
#     so it carries `#[cfg_attr(miri, ignore = …)]` — the idiom `filter.rs` already uses for
#     zlib-rs — and `file.rs`'s `scratch()` says so once for the four that share it. **A round that
#     adds a test which opens, creates or removes a file owes that attribute**, and nothing local
#     will tell it so.
# And the Windows *read path* runs here, which is the only way to test it from Linux: the two
# implementations are chosen by `#[cfg(unix)]` / `#[cfg(not(unix))]`, so rewriting those two
# attributes compiles the thread-and-channel one on this machine. ADR 0194 has the recipe; all 19
# sandbox tests and the whole corpus gate pass through it.
# **`viewer-confined`'s `awkward_classes` sweep is `doc/todo/02` §2's since session 995** (ADR
# 1015), with its `--bins` line: the other confined program over the same population as `pdf-vfs`'s
# read walk (ADR 0879), a document of each of `corpus-classes`'s ten classes from every corpus root
# on this disk, opened as a descriptor and drawn through `pdf-view-worker`, three page turns
# apiece; what fails it is a **death**. It was here rather than there because the read walk gates
# the same class of defect over more questions — but through a different program, which is why it
# moved. What stays here is its calibration: with `no_machine_fonts()` taken out of
# `viewer_confined::worker::confine` it reports 28 deaths in six of the ten classes, which is how
# it was shown to be able to fail (trap 13).
cargo bench -p pdf-model
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_interpret            # stops at the display list
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_rasterise [file.pdf] [page]
cargo run --release -p pdf-model --example glyph_reuse -- [file.pdf] [page] [scale]  # ADR 0131
cargo run --release -p pdf-model --example strip_spans -- [file.pdf] [page] [scale]  # ADRs 0137, 0139
cargo run --release -p pdf-model --example render_at -- [file.pdf] [page] [scale] [out.png]
  # our own render at any resolution, which is how §3a's step 5b tells a scan-conversion
  # difference from a difference in the shapes themselves
cargo run --release -p render-raster --example zoom_ladder -- [file.pdf] [page] [out-dir]
  # the two backends compared up a ladder of magnifications and back down, through one device,
  # switching coverage lanes where `viewer-ui` does. `doc/QUORRA_FEEDBACK.md` §11
cargo run --release -p render-raster --example zoom_frame -- <file.pdf> [page] [scale] [factor]
  # what a zoom step costs phase by phase on a device that has already drawn the page — the second
  # frame, against warm caches, which is the one a person waits for. `ZOOM_FRAME_COVERAGE=compute`
  # measures the lane the window actually draws a moved view on (ADR 0767);
  # `ZOOM_FRAME_SEQUENCE=1,1.25,1.5,1.25,1` generalises the pair to a session (ADR 0424)
cargo run --release -p render-raster --example frame_budget -- [file.pdf] [page] [label]
  # where one refresh's 8.333 ms goes on the shipped path — interpretation, scene, encode, transfer,
  # the device's passes — for a page turn, a warm repaint and a zoom step, each on the lane the
  # window gives it; `FRAME_BUDGET_ROUNDS`, `FRAME_BUDGET_WINDOW=WxH` (ADR 1260)
cargo test --release -p render-raster --test turn_path -- --ignored --nocapture
  # that table's turn and step rows held to `doc/checks/turn-path.toml`'s bands, by the same method
  # (`tests/support/frame_cost.rs`): three pinned children of five rounds a page, each with the
  # launch gate's calibration probe, judged under `release` only; twenty seconds. ADR 1513. A child
  # is judged only on a quiet device too: amdgpu's `gpu_busy_percent`, averaged over half a second
  # before the child and after it, at or under the check file's `device_busy_percent`; above it the
  # row prints "not judged (device N% busy against T%)" and the run exits 0, as it does over the
  # load ceiling; a device with no such counter is printed unread (ADR 1537)
cargo test --release -p render-raster --test turn_path -- --ignored --nocapture a_planted_busy_device_is_seen_as_busy
  # the device check's calibration: this binary keeps the adapter busy with a compute loop for six
  # seconds and the counter must read over the threshold while it runs, behind the heavy-walk lock
  # because it occupies the device a sibling may be measuring on. ADR 1537
cargo run --release -p render-raster --example ink_ladder -- <stem>...
  # each backend's total ink on a pdf.js page at four resolutions, against §10.7.4's area rule:
  # a total that walks toward the other backend's as the scale rises is a per-boundary cost, a flat
  # one is geometry. The ladder that exposed the thin overlaps a heuristic missed (trap 54, ADR
  # 1341); a stem with no document is a panic, not a skipped row
cargo test -p raster-gpu --lib stroke_set
  # the same ladder as a gate inside raster: every fixture stroked both ways, and the two masks
  # agree to a sixteenth of a pixel at 1×, 2×, 4×, 8× against §8.4.3.2's distance set, whose area
  # is derived in closed form or by a sampling quoted beside the fixture (ADR 1361, trap 58)
GESTURE_PDF=<file.pdf> cargo test --release -p viewer-ui --bin quorra -- --ignored a_zoom_gesture_out --nocapture
  # a zoom gesture on the real render thread and adapter under a simulated 120 Hz clock: how many
  # of each notch's refreshes put up a frame of its own and how many a stand-in;
  # `GESTURE_NOTCHES`, `GESTURE_EVERY`, `GESTURE_SUPERSAMPLE` (ADR 1289). Needs a device
PDFVIEWER_RASTER_COVERAGE=gpu PDFVIEWER_RASTER_SCALE=4 \
  cargo test --profile gates -p render-raster --test corpus -- --ignored --nocapture
  # §2's quorra gate pointed at the *other* coverage lane — the one `viewer-ui` switches to past
  # ten times magnification, and the one no gate had ever run over the corpus. Either knob turns
  # the ratchets off and the run says so; a value that is none of `cpu`, `gpu` and `compute` is a panic
  # rather than a silent default. `PDFVIEWER_RASTER_SCALE=4` is the interesting pairing, because
  # the lane exists for magnification: ADR 0283 took its refusals from 36 to 12 there.
FIRST_FRAME_COVERAGE=gpu cargo run --release -p render-raster --example first_frame -- [page] [scale]
  # what the first frame costs that the tenth does not, on either lane (ADRs 0179, 0283)
cargo run --release -p viewer-host --example outline_census -- [file.pdf]
  # how many rows §12.3.3's outline becomes and how many the document's own `/Count` signs ask to
  # be open, which is the number ADR 0244 quotes for the GTK host's tree
cargo run --release -p viewer-ui --example chrome_ladder -- [file.pdf] [page] [out-dir]
  # the window's *whole frame* offscreen — page and chrome in one scene, which no gate does —
  # with a device per rung beside one device, which is what separates state from magnification
cargo run --release -p pdf-model --example clip_chain_census -- [file.pdf] [page]
  # what a page's clip chains and shading fills cost a rasteriser to *build*: how many chain steps
  # it performs today against how many distinct clip nodes there are (the two are equal only where
  # nothing is shared), the mask bytes both ways against `MASK_BUDGET`, and how many of a shading
  # fill's pixels its clip can admit. Written for `doc/todo/40`, and what it answered was
  # `doc/todo/40`'s neighbour: 3490 `sh` operators shading 10.4 M pixels a render to keep 85 608
  # (ADR 0236). A profile cannot see either number — it counts commands, not what a command covers.
  # **It prints three arms and a fourth count since ADR 0656**, which is what turned that item from
  # an open question into a priced choice: `today`, `exact` — reuse restricted to the prefixes whose
  # band equals their child's, the ones reusable byte for byte — and `full`, the whole proposal with
  # ADR 0219's departure in it; plus how many chain steps state a rectangle that admits every pixel
  # of the band, which is the saving that needs no departure at all and is now taken. Scanned mask
  # *rows* rather than operations, because a fill over a 792-row band and one over four are not one
  # unit
cargo run --release -p pdf-model --example content_budget_census -- doc doc/pdf.js doc/corpora
  # what a page's content costs in the three quantities `doc/todo/10`'s bounds name: operators,
  # lexer tokens, and decoded bytes. It counts both of the first two in one pass, which is what
  # makes it an A/B rather than two measurements — `MAX_OPERATIONS` said operators and counted
  # tokens, and the ratio is 3.76 corpus-wide, about 2 for text and about 7 for Bézier artwork
  # (ADR 0306). Also the largest single decoded stream and the largest page /Contents total, which
  # are the two numbers `Limits::max_stream_len` is set against: 483.84 MiB over 5 047 187 streams
  # of 65 967 crawled documents. Every argument is walked recursively, so `corpus-cache` is one
cargo run --release -p pdf-model --example integer_entry_census -- @<paths>  # --list-keys, --witnesses N
  # where in the world a **real** stands at an entry ISO 32000-2 types as an **integer**, which is
  # §7.3.3's writer-side error and the population `doc/questions/Q31` and `Q39` are about. The keys
  # are derived from the Arlington model rather than listed (trap 25) — every name typed `integer`
  # or `bitmask` in some table and `number` in none — and the fifteen names typed both ways are
  # counted in a table of their own with witnesses instead of being judged. Two halves, because an
  # inline image is not an object and both documents in the world that write a Table 87 dimension
  # as a real write it inside a `BI`; the second half reads its own tokens rather than asking
  # `pdf_model::inline_image` (trap 8). `--list-keys` prints the population itself, so a zero can
  # be told from nothing having looked; `--witnesses N` prints every document a key's real appears
  # in, which is what a round measuring *reach* needs. About twelve seconds per eight thousand
  # documents; the whole disk at 24 threads crosses an 8 GiB `RLIMIT_DATA`, so it shards, and one
  # 5.6 GiB attachment in `batch5/poppler` is not walkable inside a round's memory budget at all.
  # ADR 0912
cargo run --release -p pdf-syntax --example linearised_census -- doc/corpora doc/pdf.js/test/pdfs
  # what §6.3.2.1's declined `should` is worth: how many documents state `/Linearized` inside
  # §F.3.3's 1024-byte window, how many of those state an `L` that is not the file's own length and
  # are therefore what Table F.1 calls not linearised at all, and what this reader touches before
  # page one — bytes, read calls, cross-reference sections and object offsets — against Table F.1's
  # `E`, the end of the first page. `LINEARISED_CENSUS_VERBOSE=1` prints a line per document;
  # `LINEARISED_CENSUS_COLD=<directory>` makes each figure the quickest of seven opens of a copy
  # whose pages were dropped first, so that directory must be one whose pages *can* be dropped
  # (`/tmp` here is a `tmpfs` and cannot). Give it disjoint roots: it recurses, so naming `doc`
  # beside `doc/corpora` counts everything twice. No display, no device and no sandbox, so it runs
  # at any load. ADR 1225
cargo run --release -p pdf-model --example ccitt_decoder_census -- <directory>... 2>/dev/null
  # whether `pdf-ccitt` decodes every stream object whose filter chain ends in §7.4.6's
  # `CCITTFaxDecode` to the scan lines `hayro-ccitt` decodes, bit for bit, and how long each takes;
  # the measurement ADR 1349's move onto this tree's own decoder rests on, with the streams stating
  # Table 11's `/DamagedRowsBeforeError` above zero counted beside. Inline images are not reached.
  # A corpus walk: behind the lock. ADR 1349
cargo run --release -p pdf-model --example spot_depth -- @paths.txt   # --pages N, --separate
  # how many spot colourants each page names, as a distribution with its largest page and every
  # page at or above the bound the tree carries on §10.8.3's planes — the standard states none, so
  # the bound is what documents do (trap 38, ADR 1311). `--separate` interprets each such page and
  # counts the pages the separation is made for against the pages it gives up on, each with its
  # report's reason (ADR 1329); a page given up with no report is printed as a defect
cargo run --release -p pdf-model --example image_decode_census -- doc/pdf.js/test/pdfs
  # how much of page one's interpretation is image decoding, and how much of that a parallel decode
  # could take off the critical path: `decode` one after another against `longest`, beside `interp`.
  # The measurement ADR 1321's parallel decode rests on
cargo run --release -p pdf-model --example rebuild_census -- corpus-cache doc/pdf.js doc/corpora
  # what a *rebuilt* cross-reference table loses to §7.5.7's object streams: how many documents
  # reach `xref::rebuild` at all, how many of those carry object streams the scan can see, and
  # where the table then puts each number the streams' own headers name — at an offset, inside a
  # stream, or nowhere. It takes directories as well as files and reads the `N` pairs itself, so
  # the count is the documents' rather than the recovery's (trap 8), which is what lets one run
  # print both arms of a before-and-after. It is also where the recovery's budget comes from: the
  # widest object-stream expansion among the rebuilt documents on this disk (ADR 0395)
cargo run --release -p pdf-model --example vertical_form_census            # curated; also --pdfjs, --crawl
  # the two populations §9.7.5.1's NOTE has, printed side by side, which is trap 13's second shape:
  # **the clause's** — a `Type0` stating writing mode 1, embedding no program, in a collection Table
  # 116 publishes a vertical `CMap` for — read out of the files' own dictionaries and the same on
  # every machine; and **the program's**, how many codes those documents then draw upright because
  # the substituted face states no `vert` form, which is this catalogue's (§9.5 NOTE 5). It walks
  # every dictionary *nested* inside an object as well as the objects the table names, because a
  # `Type0` need not be an indirect object — `issue11555.pdf` writes one inline in its page's
  # `/Resources`, and the walk without the recursion found no font in it at all (ADR 0764, trap 25).
  # `PDFVIEWER_TRACE_VERTICAL_FORM=1` names each code with its font and character
cargo run --release -p pdf-model --example hollow_glyph_census            # curated; also --pdfjs, --crawl
  # how many `CIDFontType2` dictionaries embed a `TrueType` program whose `loca` says every glyph
  # is empty, and how many of those reach their glyphs through a `/CIDToGIDMap` stream — the
  # intersection ADR 0350's hand-built fixture was justified by a negative about. It reads the
  # `loca` by hand rather than through `skrifa`, so it measures the corpus and not the reader, and
  # it walks every dictionary *nested* inside an object as well as the objects the table names,
  # because a font need not be an indirect object — `issue16553.pdf` writes one inline and the walk
  # without the recursion could not see it (ADR 0765, trap 25)
cargo run --release -p pdf-model --example stroke_adjustment_census   # curated; also --pdfjs, --crawl
  # what §10.7.5 actually reaches, which is not what a dictionary states: strokes painted with the
  # stroke adjustment parameter enabled, how many of those the clause's second requirement already
  # promotes to one device pixel, how many of the rest are a single axis-aligned run — the only
  # shape a grid fit is defined for — and how many of those have edges off the pixel grid, which is
  # the population the clause's *first* requirement would move. `absence_audit`'s §10.7.5 block
  # counts the documents that state `/SA true`; this counts what survives to the display list, and
  # the two differ by an order of magnitude (ADR 0848)
cargo run --release -p pdf-font --example vertical_feature_census
  # which of OpenType's six registered vertical features the faces *on this machine* state, which
  # `GSUB` lookup shapes appear under `vert`/`vrt2`, and how many of Adobe-Japan1's own 251
  # vertical forms each such face supplies. It answers two questions `doc/todo/21` §7 had written
  # down as sentences — whether a second registered feature is worth consulting for what `vert`
  # misses, and whether any face here states a lookup that is not a single substitution — and both
  # are claims about a font catalogue, so both decay the moment a font is installed (ADR 0765)
cargo run --release -p pdf-model --example field_flag_census -- doc/pdf.js/test/pdfs/*.pdf
  # which of §12.7's twenty field flags any real document states (ADR 0197)
cargo run --release -p pdf-model --example variable_text_census -- doc/pdf.js/test/pdfs/*.pdf
  # what §12.7.4.3 actually lays text out for, which the two censuses above cannot say: 622 widgets
  # of a text field or a combo box, 305 with no /AP /N stream, and 73 of §12.5.6.6's free text
  # annotations — with each one's /DA font classified by what its descriptor says about a baseline,
  # and §12.7.5.4's list boxes counted beside them. `font_metric_census` counts the fonts a *page*
  # draws with, which is a different population and was mistaken for this one (ADR 0240). **It also
  # counts the /DA font names §7.3.5's escaping is about and prints each one**, and it takes
  # directories as well as files so that the crawl is one argument rather than an xargs batch per
  # census — which is what the write half of ADR 0453 was measured with
cargo run --release -p pdf-model --example presentation_census -- doc/pdf.js/test/pdfs/*.pdf doc/*.pdf
  # what any real document says about §12.4.4: 978 opened, 1971 pages walked, and **not one**
  # states a /Trans, a /Dur or a /PresSteps — asked of the page tree rather than of the raw bytes,
  # so a /Trans inside an object stream would have counted. `--example presentation_fixture` writes
  # the three-slide document that therefore has to stand in for one (ADR 0230)
cargo run --release -p pdf-model --example witness_census -- --pdfjs Collection Threads IDTree
cargo run --release -p pdf-model --example associated_file_census -- --pdfjs   # also --crawl
  # §14.13.2's two forms of associated file, counted apart, which is the question a name census
  # cannot ask: an `/AF` array's specifications split by whether they carry an `/EF`. Over the 974,
  # 7 documents state 36 arrays naming 44 specifications and **all 44 are embedded**; over
  # `CC-MAIN-2021-31`'s 65 944, 23 documents and 45 specifications, also all embedded. So the
  # external form the clause states is a construction no document in reach uses, and the reader
  # for it and the note that says a file is out of reach are both held by built witnesses (ADR 0918)
cargo run --release -p pdf-model --example absence_audit
  # the pair `doc/todo/01`'s sixteenth sweep runs, and the two halves of one question: **is there
  # really no corpus document that does X?** The first asks a name three ways of each of the 1251
  # PDFs — the file's raw bytes, every object the cross-reference table names *including the ones
  # inside object streams*, and every stream's decoded data — and prints the three counts side by
  # side, so a term a byte search undercounts is visible as a term. `--names` ranks every distinct
  # name in the population, which turns "is there a witness for this entry" into a lookup;
  # `--pdfjs` narrows to the 974, because half of this tree's absence claims were about that
  # population and said "the corpus". The second re-asks seven written claims through the readers
  # that would act on them, since a name being stated is not the structure being stated. Run both:
  # on §14.7.2's /IDTree the object walk finds four documents no grep over the bytes can see,
  # which is ADR 0403's rule with the instruments the other way round (ADR 0405)
cargo run --release -p pdf-model --example cell_header_census -- doc/pdf.js/test/pdfs/*.pdf doc/*.pdf doc/corpora/*/**/*.pdf
  # §14.8.4.8.3's two routes to a table cell's header cells, counted apart: over 1251 files, 21 883
  # cells, 281 stating Table 384's /Headers (2 of them an empty array) and **17 152 of the 17 431
  # cells that end with a header answered by the *search* rather than by an array** — which is what
  # says the algorithm is the feature and the entry is the exception (ADR 0312). It also prints the
  # two counts that decided what was *not* taken: 0 of 6197 TH state /Short, and no document's
  # tables outgrow the grid `TableStack` keeps
cargo run --release -p pdf-model --example group_shape_census -- doc/pdf.js/test/pdfs/*.pdf
  # every `Command::Group` a first page holds, with `alpha_is_shape` beside it — §11.6.4.2's shape
  # question, which decides whether §8.5.4's clip at the group's blit is §10.7.4's intersection or a
  # product. It asks the same question a *second* way, from the command list alone, which is how a
  # backend that never sees `/AIS` has to ask it (quorra's ADR 0074), and prints where the two part.
  # Written to answer one line of a dependency's question and kept because it is how the two proofs
  # are compared: 162 groups over the 964 first pages, 135 carrying shape, 61 of them beyond a
  # command-list proof and none the other way (ADR 0554)
cargo run --profile gates -p pdf-model --example non_isolated_group_census -- @<paths>
  # which pages state §11.4.4's result step — a non-isolated, non-knockout group composited under a
  # blend mode of its own, which is the one construction that costs a second run of the elements
  # (`CpuRasterizer::remove_the_backdrop`, ADR 1107). The predicate is that rasteriser's own
  # conjunction over the display list's group commands rather than a second reading of the clause,
  # so §11.7.4's *synthesised* groups (ADR 1170) are counted beside a file's own `/Group` and
  # counted apart; the first page only, which is the denominator §11.4.4's row states, and
  # `--pages N` for more. A document no object of which carries Table 58's `/BM` is not interpreted
  # at all, which is what makes it affordable over the crawl — about 17 minutes at two threads, and
  # it wants them: at four it peaked over 10 GiB. The two exclusions and the blending-space
  # self-check are printed beside the count, because a predicate's exclusions are part of it
cargo run --release -p pdf-model --example group_blit_census -- doc/pdf.js/test/pdfs/*.pdf
  # what each first page's transparency groups would cost to composite, in blitted pixels, and it
  # is `pdf_render::group_blit_demand` itself rather than a second reading of the same idea — the
  # instrument that sized `MAX_GROUP_BLIT_PIXELS` (ADR 0780). **It interprets and never
  # rasterises**, which is what lets it be run over a population holding `poppler-978-0.pdf`, whose
  # 73 047 page-spanning groups take some 640 s to draw and 2.5 s to interpret. Run over a corpus
  # in several processes with `xargs -0 -n 40 -P 8`; the summaries add
cargo run --release -p render-raster --example filtered_edge_colour
  # what each of the three backends' image filters does to the *colour* of a partly transparent
  # sample — §8.9.6.2's "smooth the edges of the mask, not … the painted colour values", which is
  # the difference between filtering premultiplied and filtering straight. One scene, because it is
  # the only shape that separates them: every image in both cross-backend suites is opaque, and on
  # an opaque raster the two arithmetics agree. The CPU backend and vello depart from the painted
  # colour by 0 and quorra by 131 of 255, which is the shipped rasteriser (ADR 0697,
  # `doc/todo/55`, `doc/QUORRA_FEEDBACK.md` §39)
cargo run --release -p render-raster --example sampled_lane_column -- [--scale N] doc/pdf.js/test/pdfs/*.pdf
  # the population quorra's sampled coverage lane would give back to the processor if it diverted
  # every mark whose width is not a multiple of its sample pitch. Defaults to ten times
  # magnification, because that is where `viewer-ui` takes that lane and a census at page scale
  # measures a lane no frame draws. It answers a cost question with a measurement instead of a
  # guess, which is what it was built for: 88.31 % of the lane's marks, and the clause it was
  # offered for would still not be met (ADR 0556)
tools/state.sh selection
  # ADR 0323's instrument 1, composed half, and the one thing in this tree that **clicks**: every
  # corpus document opened at the boundary, poppler's word boxes mapped into device pixels through
  # `Query::PageGeometry`, dragged across with `Command::Pointer`, and `Query::Selection` asked
  # what came back — beside the two self-inverse properties ADR 0323 puts with it. It is a
  # `doc/todo/02` §2 line as well, because two of its three properties are exact; this entry is
  # here because the *section* is how a round reads its counts, and because the drag fraction is
  # printed rather than ratcheted (ADR 0421). `crates/viewer-core/tests/selection_census.rs`
tools/state.sh accessibility
  # ADR 0323's third instrument, and the only one of the three with nobody to disagree with it: no
  # other implementation puts a comparable tree on AT-SPI, so it is a **ratchet** and says so —
  # and it is a `doc/todo/02` §2 line since ADR 0425, its counts having held across rounds. Page
  # one of every corpus document and of every specification in `doc/`, plus every page of every
  # document that states a structure tree, through `Query::AccessibilityTree` — with an *empty*
  # answer classified by §14.7.5.4 rather than counted, because an empty answer is also what an
  # untagged page honestly gives. It found a defect on its first run (ADR 0342). Every capability
  # count has a floor and every defect class a ceiling; `crates/viewer-core/tests/accessibility_census.rs`
  # is the instrument and its two un-ignored tests keep the classification from rotting between runs
cargo run --release -p pdf-signature --example signature_algorithm_census -- @/tmp/paths
  # Table 260's three algorithm families and the fourth ISO/TS 32002 adds, as documents actually
  # state them, over as large a population as this machine can reach — `find -L corpus-cache
  # doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' > /tmp/paths` is 67 460 files and about a
  # minute. **`-L` is not decoration**: in a parallel worktree `corpus-cache` is a symlink into the
  # main checkout and `find` without it reports zero paths, which is a false zero rather than an
  # empty crawl (session 689). The `@` form exists because
  # a command line holds a fortieth of them. Three identifiers per signature, because a producer can
  # get them out of step: the `SignerInfo`'s `signatureAlgorithm`, its `digestAlgorithm`, and the
  # algorithm of the key in the certificate that `SignerInfo` names. It is what ranked ADR 0314's
  # work and then ADR 0322's — `id-RSASSA-PSS`, which it found being declined six times, is
  # verified since the four-hundred-and-eighty-seventh session — and what would rank the next:
  # it prints the population of ECDSA, and of DSA — which is nought
cargo run --release -p pdf-model --example type4_comment_census -- @/tmp/paths
  # every §7.10.5 program in a population, classified by what the old comment rule did to it:
  # `refused` where a word left in a comment was not an operator, `silent` where the words were
  # compiled into the program with nothing reported, `harmless` where the two agree. Both arms go
  # through the *current* compiler and the old rule is reproduced as a text transform, so nothing
  # of it has to be kept alive. It is what priced ADR 0361 — and what said the defect fell on
  # hand-written files rather than on generated ones, which is a fact about producers that no
  # reading of the clause could have given. Documents are prefiltered on `/FunctionType` in their
  # own bytes, which §7.5.7 makes sound: a function dictionary is a stream's, and no stream may
  # live in an object stream
cargo run --release -p pdf-model --example luminosity_mask_census -- doc/pdf.js/test/pdfs/*.pdf
  # what a §11.5.3 mask group is painted *with*, against what its /CS declares — 87 groups on
  # this corpus, 39 blending in /DeviceCMYK and 36 in /DeviceGray, and not one setting a `k`
  # colour, which is what turned a report's condition into the departure itself (ADR 0217).
  # Since ADR 0797 a three-component CIE-based /CS is printed with its route: a CalRGB or a
  # matrix profile takes the clause's Y as three curves, a table profile keeps the sRGB grey
cargo run --profile gates -p pdf-model --example colour_transform_census -- @<paths>
  # which of Table 13's three cases each `DCTDecode` image is in — an Adobe APP14 marker, the
  # `/DecodeParms` entry, or the clause's default — read off ISO/IEC 10918-1's marker segments
  # rather than searched for in the bytes, and counting apart the images that write
  # `/ColorTransform` as a direct key of the stream dictionary, where §7.4.1 puts no filter's
  # parameters. That distinction is the whole of ADR 1177: the entry decides 158 images over
  # 17 crawled documents and none of the 974, and the witness a departure was taken on is in
  # the other list. §8.9.7's inline images are walked too
cargo run --profile gates -p pdf-model --example overprint_ink_group_census -- @<paths>
  # how far §11.7.4.3's special overprinting blend mode actually reaches, which is what
  # `render-gpu`'s by-name refusal costs (ADR 1158 section 4; `render-raster` draws it, ADR 1295). Every
  # column is the interpreter's own verdict — `DisplayList::overprints()`, the commands whose
  # `blend()` is `BlendMode::Overprint`, the non-Normal group §11.7.4.3 builds around one, and
  # `Unsupported::Overprint` — so a change to the rule moves the number and not the predicate.
  # A document no object of which carries Table 58's `/OP` or `/op` is not interpreted at all,
  # which is what makes it affordable over the crawl. Since ADR 1182 it also counts the *shape*
  # of each mark's kept set — all three channels, none, or a proper subset — because the mode is
  # Porter-Duff destination-over where it keeps a channel and source-over where it does not, so
  # the first two shapes are one operator a scene vocabulary may already have and the third is a
  # per-channel choice. Its verdict-without-a-mark column is its own self-check and is zero
cargo run --release -p pdf-model --example press_census -- <dir>/*.pdf    # one process per archive
  # which press §11.4.7 gives a page and whether `crate::icc` can evaluate the profile behind it —
  # its `A2B` out, and since ADR 0796 the `B2A` §8.6.5.5 requires of a blending-space profile,
  # counted as carried and as evaluated, side by side, so that an encoding this tree does not
  # read is a number rather than a silence. It shares nothing between documents, so its answer is a function of the files and two runs are
  # byte-identical — which is what `tools/safedocs survey` could not say while the press table was
  # a process-wide budget (ADR 0416). `--sample` measures what a grid of a given side departs from
  # evaluating the profile, which is what `PRESS_SIDE` is answerable to (ADR 0272)
cargo run --release -p pdf-model --example press_depth -- [--pages N] <dir>/*.pdf
  # how many *distinct* presses one interpretation names, which is the number
  # `colour::MAX_PRESSES` is compared against. `press_census` counts the presses a whole
  # population names and so cannot answer this: the budget is per interpretation (ADR 0417), so
  # the question is about a page. It interprets pages and reads `Interpretation::presses_named`,
  # prints the distribution and names every page within one of the bound. This is the half of
  # trap 38's question no clause can answer, and ADR 1254 is what it measured
cargo run --release -p pdf-model --example press_cost -- [file.pdf]…
  # what *sampling* a press costs, as the difference between a cold interpretation and a warm one
  # in the same process. It is the benchmark under `colour::SAMPLED`: a press is 17 to 46 ms of
  # profile evaluation against a 14 to 18 ms interpretation of the same page, so the cache behind
  # the per-interpretation budget is what keeps a page turn a page turn (ADR 0417)
cargo run --release -p pdf-model --example interface_font_census -- doc/pdf.js/test/pdfs/*.pdf
  # which characters a *program's own* text needs — §12.3.3's outline titles, §8.11.4.3's layer
  # names, §7.11.4's file names, §14.3.3's `/Info`, §14.3.2's XMP, §12.4.2's page labels and
  # §12.5.6.14's popups — and which of them the compiled-in fourteen state, asked **both** ways:
  # by character code, which is 256 wide, and by character, which is what the face's own `cmap`
  # answers. The gap is the finding (ADR 0326). It also prints what is still a box, by script,
  # which is the demand any further answer to `doc/todo/27` has to cover. Deliberately not routed
  # through `viewer_ui::chrome`, which is the code under test — `viewer-ui --example
  # chrome_coverage` is that question, and trap 8 is why they are two examples
cargo run --release -p pdf-model --example spec_annotation_census -- doc/*.pdf
  # what the fourteen specification PDFs' annotations are, which is what the Markdown conversion
  # under doc/md/ dropped: 12 545 annotations, 11 462 of them in ISO 32000-2, and in three of the
  # documents they are the *errata* — 434 strikeouts over 4038 words (ADR 0252). Also what §14.7
  # gives. **The number that used to bound it is gone**: this line said `Tree::walk` stops at 65 536
  # items and that tree is larger, which stopped being true in the four-hundred-and-twenty-first —
  # `MAX_ELEMENTS` is 2^20 and ISO 32000-2's tree is 129 389 items in 151 ms (ADR 0257). What is
  # still 65 536 is `MAX_CHILDREN`, a bound on one element's children rather than on the tree
cargo run --release -p spec-errata -- census doc/*.pdf   # the same counts by subtype and §12.5.6.2 role
cargo run --release -p spec-errata -- emit   doc/*.pdf > doc/errata.md   # gitignored: it is the spec
cargo run --release -p spec-errata -- check  doc/*.pdf
  # the two questions that follow: how many struck passages doc/md/ still presents as current text
  # (**151**) and which quotations quote a passage struck out of the clause they cite (**27** —
  # blockquote 8, ledger 9, prose 10), with 75 more landing in another clause. Both numbers printed
  # in the four-hundred-and-forty-fifth and unmoved for eleven rounds. **This comment said 79 and
  # "3, all fixed in the four-hundred-and-sixteenth"** — the 79 became 151 in the very next round,
  # when the comparison stopped keeping whitespace (ADR 0253), and the 3 was never the whole
  # population. **Never a gate**: the conformance checker has to keep
  # comparing quotations against a conversion this project did not make, and this parses fourteen
  # PDFs in 6.4 s. ADR 0252
cargo run --release -p pdf-transform --example archive_census
  # what the PDF/A converter has decided about **every** requirement each of the six targets
  # binds, which is `CLAUDE.md`'s coverage question — denominator the specification, where
  # `tests/archive_corpus.rs` answers the robustness one whose denominator is the world. Seven
  # standings per requirement, three of them reasons the converter is never asked (a rule whose
  # subject is a program, one a clarification puts outside validation, one the validator does not
  # check) and three of them answers (a `REMEDIES` row, a rule the serializer satisfies by
  # construction, a refusal with a sentence of its own). The seventh is the product and is
  # printed in full: the requirements in none of `decision.rs`'s three tables, which a document
  # failing one is refused over with a sentence saying only that the gap is this program's.
  # **107 of them when the census was built in session 954, 0 when it ended**, and
  # `crates/pdf-transform/tests/archive_unconsidered.txt` is the ratchet that holds it there in
  # both directions — arriving fails the build, leaving means striking the line. Empty is not
  # finished: 77 of those rows are refusals that name a rewrite nobody has built, and this
  # example is the list to work from (ADR 0955)
cargo run -p pdf-archive --example targets
  # the other half of the same question, one layer down: how many of the requirement table's rows
  # each target binds, how many are checked, and every requirement that binds some PDF/A-4
  # flavours and not others
cargo run --release -p render-gpu --example frame_split -- [file.pdf] [page] [scale]
  # where a GPU frame's time goes: encoding, the whole frame, and the same target drawn from a
  # list of one rectangle. doc/RENDER_LIBRARY.md §6.1
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_open [file.pdf]  # §7.5's xref alone, in instructions rather
  # than in a wall clock that moves by 2× between runs of the same binary. ADR 0180
cargo run --profile gates -p pdf-model --example parallel_sweep -- [file.pdf] [threads] [one|shared|per-thread]
  # what reading every page costs on one thread, on N sharing a `&Document`, and on N each opening
  # their own — every parallel section inside a pool built with exactly N, because `interpret`
  # bands §8.9.5's colour conversion across `rayon::current_num_threads()` of its own. Two sweeps
  # apiece, since the two arrangements differ most on the second, and `VmHWM` from
  # `/proc/self/status` so the memory is the kernel's number rather than ours. ADR 0260
PDFVIEWER_LAUNCH_CLOCKS=1 cargo test --release -p viewer-ui --test launch_path -- --ignored --nocapture
  # **the launch gate's other half, run when a round has the machine to itself.** `doc/todo/02`
  # section 2 runs this same gate with the variable unset and judges the twenty-one figures no
  # machine can move — bytes, read calls, instructions, memory — in three seconds on any machine.
  # The figures that are wall clocks are claims about *a machine*, and this is how they are asked
  # for: nine fresh processes a figure, pinned, each carrying its own calibration and disk probes,
  # every duration less the time its thread spent waiting for a processor. It prints `NOT JUDGED`
  # and exits 0 where the machine is not the one the bands describe, naming the probe that said so
  # — which on 2026-09-06 was the machine's own boost clock, 3.74 GHz against a rated 5.16 on an
  # idle 63 °C machine. `doc/questions/A29` (options 1 and 2), ADRs 0916 and 0917.

cargo run --release -p pdf-model     --example open_cost -- [file.pdf]
  # where the *launch path's* document half goes: §7.5's xref, the page tree, §12.3.3's outline,
  # §12.8's signatures, each on its own. ADR 0179, doc/todo/42
DISPLAY=:77 target/quorra-gtk --trace=launch,frames [file.pdf]   # and where a *native* host's
  # launch goes, which is a different path: `opened` -> `first frame on the screen` are two stamps
  # inside one process, so the difference is not the machine's (749's rule) and a launch A/B needs
  # only that column. Read the **frame** line beside it — `rasterised ... in 3.25ms, waited 61.53ms`
  # is trap 21, a poll waiting for a main loop that is inside its own first frame, and a round that
  # reads only the first number sees a fast rasteriser. `GSK_RENDERER=cairo` is the control that
  # separates the toolkit's frame from ours. Alternate the arms and say the load average; two arms
  # of an A/B **need `.cargo/config.toml` target directories of their own**, because a worktree
  # inherits `/home/AI/.cargo/config.toml`'s and would otherwise measure whichever linked last —
  # `cargo metadata --no-deps | jq -r .target_directory` is the check. ADR 0678
  #
  # **And a scratch arm that did share the directory leaves a residue that outlives it.** A build
  # script is compiled with its own `CARGO_MANIFEST_DIR` baked in, so an arm built from a scratch
  # export puts scripts naming that export into the *shared* directory — and when the export is
  # deleted the next `cargo build --release` in the **main tree** dies with `data/cmaps is
  # readable: No such file or directory`, naming a path no checkout has. It is trap 10b's shape
  # with the staleness pointing at a tree that is gone rather than at one that moved: the round
  # that measured is finished and the round that pays is the next one to build. Found by the
  # seven-hundred-and-sixty-third session, one merge round after the arms were taken.
  #
  # `touch crates/*/build.rs` and rebuild — the scripts recompile with the real manifest directory
  # and nothing else is affected. **Or avoid it: give the scratch arm its own target directory,
  # which the paragraph above already requires for a different reason**, and the residue never
  # exists. One rule, two hazards.
cargo run --release -p render-raster --example bring_up  -- [all|vulkan|gl]
  # and where its device half goes: instance, adapter, device — one measurement per process,
  # because a second instance in one process is measured with the loader already warm
cargo build --release -p hayro-compare --bins && \
  cargo run --release -p hayro-compare --bin hayro-speed -- doc/pdf.js/test/pdfs/*.pdf   # ~45 min
cargo run --release -p hayro-compare --bin hayro-speed -- --per-document ...  # one line per file,
  # which is how a renderer that is a *program* rather than a crate is joined to the table (ADR 0136)
# **`cargo-fuzz` is installed and always has been.** `~/.cargo/bin/cargo-fuzz`, 0.13.2, dated
# 26 July, beside the `nightly` toolchain it needs. It is **not on `PATH`** in this shell, so
# `which cargo-fuzz` reports nothing and `cargo fuzz` fails with "no such subcommand" — which is
# a statement about `PATH` and not about the disk. Sessions 425 and 426 wrote "cargo-fuzz is not
# installed here" from exactly that check, and left a target unwritten on the strength of it
# (ADR 0264). Prefix the run: `PATH=$HOME/.cargo/bin:$PATH cargo +nightly fuzz …`, or use the
# wrapper below, which does it.
#
# **The fuzz crate is its own workspace, and `doc/todo/02` §2 has both of its lines** — a `cargo
# fmt` and a `cargo clippy`, each naming `fuzz/Cargo.toml`, because `--all` and `--workspace` stop
# at the workspace boundary. Those two lines used to be stated here as well, which is two documents
# stating one command; §2 owns them and `tools/conformance/tests/workspaces.rs` checks that every
# workspace in the tree is named by both (ADRs 0739, 0742).
#
# **A fuzz run's exit status says nothing about whether it fuzzed**, which is why the lines below
# have a wrapper. `tools/fuzz.sh <target>` runs *this file's own invocation* for that target — the
# line is the population, so the two cannot drift — and adds the two questions the bare command
# does not answer: it refuses to start a target whose corpus is empty, and it fails a run whose
# final `ft:` is zero. `tools/fuzz.sh --list` prints every target with the seeds it has here, which
# is how a round finds out that one has none before spending an hour on it. ADR 0742.
#
# **And since the eight-hundred-and-sixteenth it prints libFuzzer's *first* figure beside its last.**
# `INITED` is the corpus's own coverage, before a single mutation, so `INITED → DONE` is what the
# documented run length bought **on top of the seeds** — and it turns out that for eight of the
# fifteen targets that is under a hundred features and for one of them it is zero, twice measured.
# A single final figure cannot tell a target that found a thousand features from one that was handed
# them, which is the question a round asking "did this campaign do anything" actually has. A
# fork-mode parent prints no `INITED` and the wrapper says so rather than subtracting against
# nothing. ADR 0747.
#
# **A fuzz target does not run under `tools/bounded.sh`, and the failure looks like a build
# error.** `cargo fuzz` builds with AddressSanitizer, which reserves about 15 TiB of shadow memory
# before `main`; the wrapper's `RLIMIT_DATA` counts exactly those private anonymous mappings, so the
# process dies with `ReserveShadowMemoryRange failed` having printed no coverage line at all — which
# `tools/fuzz.sh` then reports as "nothing here can say whether it fuzzed", correctly and
# indistinguishably from a compile failure (trap 24). Run a fuzz target directly: libFuzzer bounds
# itself with `-rss_limit_mb`, which the invocations below carry where they need one. ADR 1024.
#
# **`fuzz/corpus` and `fuzz/artifacts` are gitignored, so whether a target is seeded is a fact
# about this disk and not about the repository** — no gate can read it out of the tree, which is
# why the wrapper asks the directory. Two consequences a round meets. A **fresh worktree had
# neither directory at all** until `tools/worktree.sh` was taught to link them, so every fuzz run a
# parallel round made started from nothing and said nothing about it. And a **clone has to
# re-seed**, with `fuzz/seeds.sh` — one `case` arm per target, which `tools/conformance/tests/
# fuzz_workspace.rs` holds every target to beside its line here (ADR 1439); the prose under each
# line says why its population is the one it is. A corpus is not recoverable from the history
# because it was never in it.
#
# **A seeded corpus also goes stale with nothing failing** (trap 107), so before a campaign ask:
# `flock /home/AI/heavy-walk.lock fuzz/seeds.sh check <target>` seeds the target afresh into a
# scratch directory beside the build output, prints libFuzzer's `INITED cov` over the disk corpus
# and over the fresh seeds — a `-runs=0` pass each, under the limits this file's line gives — and
# says `STALE` when the fresh seeds lead by more than the margin the script states, with what a
# re-seed would add and where. It reads the disk corpus and never writes it; the re-seed is a
# separate, deliberate command. A target whose fresh seeds exceed its line's memory limit is "not
# judged", which is a fact about the seeder's population rather than about the corpus. ADR 1559.
# Each pass runs the target built without the sanitiser, as a campaign is, so `check` runs under
# `tools/bounded.sh` and compares the figure the campaign will see.
#
# **Every recipe over the documents keeps one seed per shape** (ADRs 1559, 1571): the population
# is some ninety thousand files and 126 GB, and kept whole each recipe wrote tens of thousands of
# seeds and gigabytes. Each seeder states, per target, the shape its target branches on — a CMap's
# codespace ranges, a security handler's revision, an sfnt's tables, a file's structure, a DER
# object's identifiers — reads only the documents whose memory map names what it needs, and keeps
# the smallest seed of each shape; given `--every` it writes the population whole, which is what a
# shape is proved against, and ADR 1571 has what each shape keeps and what it costs.
tools/fuzz.sh lexer                               # or, without the two questions, by hand:
cd fuzz && cargo +nightly fuzz run lexer         -- -runs=50000   # needs nightly
cd fuzz && cargo +nightly fuzz run cmap          -- -runs=50000   # §9.7's CMap parser
cd fuzz && cargo +nightly fuzz run crypt         -- -runs=50000   # §7.6's algorithms
cd fuzz && cargo +nightly fuzz run variable_text -- -runs=50000   # §12.7.4.3's /DA and layout
cd fuzz && cargo +nightly fuzz run forms_data    -- -runs=50000   # §12.7.8's FDF, §7.9.4's dates
  # **Seed it with FDF files**, `python3 fuzz/seed_forms_data.py fuzz/corpus/forms_data`: a corpus
  # of dates alone never opens a document, and the one on this disk held no FDF file at all, so its
  # runs never reached `FormsData::read` (ADR 1423).
cd fuzz && cargo +nightly fuzz run object        -- -runs=50000   # §7.3's object grammar
cd fuzz && cargo +nightly fuzz run document      -- -runs=50000   # §7.5's file structure
cd fuzz && cargo +nightly fuzz run serialize     -- -runs=50000   # §7.5's structure on the way
  # *out*, which is the half of that clause this project became answerable for when RFC 0002's
  # serializer landed: a document opened, every object copied into an assembly, the assembly
  # written as a whole file, and the result opened again by this reader. A crash is a defect; so
  # is a file this reader wrote and cannot open, which is why the target is the round trip.
  # **Seed it from real documents**, for `document`'s reason and more sharply: the target returns
  # at the first `Document::open` failure, so from nothing it never reaches the serializer at all.
  #   find -L doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \
  #     | python3 fuzz/seed_page.py serialize fuzz/corpus/serialize -
  # which keeps the smallest document of each file structure (ADR 1571).
  # **This line is here because its absence was invisible.** The target arrived with the
  # serializer and this file never named it, so `tools/fuzz.sh serialize` refused to run it — and
  # `tools/fuzz.sh --list` printed the refusal in a row and exited 0, which is trap 25's shape
  # with the sweep's own report as the thing nobody read. `--list` now exits non-zero on a target
  # this file does not name, and that is what found this one (ADR 1024).
cd fuzz && cargo +nightly fuzz run page -- -runs=50000 -fork=6 -rss_limit_mb=4096 -timeout=60
  # **clauses 8, 9 and 11** — a whole document through `pdf_model::interpret`, which nothing
  # reached until the four-hundred-and-twenty-eighth: `nm` finds `pdf_model::interpret` in one of
  # the other thirteen binaries and it calls it on a page with no `/Resources` (ADR 0264).
  # **Seed its corpus first**, and from real documents, because libFuzzer will not invent a header,
  # a page tree, a content stream and a resource dictionary that agree with each other:
  #   find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \
  #     | python3 fuzz/seed_page.py page fuzz/corpus/page -
  # **And a second seeder since the five-hundred-and-ninety-second**, for what no real document
  #   states: `python3 fuzz/seed_nested_content.py <dir>` builds 26 whole one-page documents whose
  #   drawing goes through one of §7.8.2's four *nested* content streams — a form XObject, a tiling
  #   pattern's cell, a Type 3 glyph description, an annotation appearance — with each stream's
  #   decoded size straddling the decoded-stream memo's allowance, which is the line ADR 0427's
  #   route decision is drawn on. Trap 8 is the reason it exists: no document on this disk states
  #   one of the four whose decode outgrows four mebibytes, so `seed_page.py` cannot reach the
  #   pumped branch at all. Aim it at a directory of its own rather than at `fuzz/corpus/page`
  #   when what is wanted is the branch rather than the merge — 26 seeds start fuzzing in seconds
  #   where 40 000 spend most of an hour merging, which is the warning two entries down.
  # 1882 seeds under the target's own 256 KiB ceiling, `cmin` to 1535, **28 535 edges** against the
  # best of the other thirteen at 6483. The run prints the current numbers and `doc/todo/02` §2 carries the
  # warning: libFuzzer merges the corpus once per run, and on the seeds `seed_page.py` produces that
  # merge had been most of the wall clock. `cargo fuzz cmin page` writes the reduced set back and
  # the five-hundred-and-ninety-third spent that merge: about a quarter of the files and a seventh
  # of the bytes, same edges and same features, three quarters of an hour to do. The script prints what the seeds *state* — 100 with a
  # `/Function`, 58 with a `/Shading`, 62 with a `/Pattern` — because a corpus that states none
  # seeds nothing about §8.7.4.5.
  # **It is the slow one, and the flags say why.** Interpreting a page under the sanitiser is
  # 10–30 execs/s where the other targets are microseconds, so `-fork=6` is what makes 50 000 runs
  # about an hour instead of most of a day, and `-rss_limit_mb=4096` is the *sanitiser's* ceiling
  # for a 1500-document corpus held in memory rather than any budget this program states. Expect
  # `slow-unit-` artefacts and read them in a **release** binary before believing them: the one
  # libFuzzer called 15 s is 0.8 s in `target/quorra-retrieve`, which is ASan, the debug assertions
  # and six forks sharing 24 cores.
cd fuzz && cargo +nightly fuzz run xmp           -- -runs=50000   # §14.3.2's XMP, the tree's
  # only XML. Seeded by `fuzz/seeds.sh` with one metadata packet per shape the documents decode
  # to, which `fuzz/seed_streams.py` takes out of them with the CMap, font, fax, security-handler
  # and field seeds of five other targets, each by its own shape (ADR 1571)
cd fuzz && cargo +nightly fuzz run sfnt          -- -runs=50000   # §9.6.3's two glyph-table repairs
  # **seed its corpus with real fonts** — every embedded TrueType program the documents hold, by
  # `fuzz/seeds.sh`. Unseeded it never forms a table directory and tests nothing; seeded it
  # produced two crashers in its first minute (ADR 0175)
tools/drive-windows.sh [--window quorra|quorra-gtk|quorra-qt|quorra-confined]... [--out DIR]   # all three windows,
  # driven end to end under Xvfb (ADR 1453). **The drive list**, each step with its observable:
  # open with Table 147's `/DisplayDocTitle` (the title), `/FitWindow` and `/CenterWindow` (the
  # geometry; GTK 4 places no window and says so); §12.3.3's outline clicked on a Chinese and an
  # Arabic title; page turns by Home, Right, End, Left and the press after an arrow; zoom by keys
  # and by Control and the wheel; find, next and Shift+Enter's previous; §12.5.6.14's popup opened
  # and closed; §12.5.6.5's link; §12.5.6.10's highlight over everything selected and §7.5.6's save
  # read back; §12.3.4's pages panel clicked; the restriction levels (`r`) and print (Shift+P);
  # §7.6.4.1's password; a form whose page states `/Tabs /C` with `/Annots` reversed, so column, row
  # and array order all differ — three values typed by Tab, §12.7.5.2.3's check box, §12.7.5.4's
  # choice and §12.7.5.2.2's push button, the save's `/V` and `/AS` read back and the file reopened;
  # an Arabic word typed at `ArabicCIDTrueType.pdf`; "كتب" typed at a page printing "كَتَبَ", found, and
  # "كُتُب" not (ADR 1477); "عرب" typed at a `TJ` in reading order under a mirroring `Tm`, found
  # (ADR 1490); a text field holding "123", whose second character AT-SPI's
  # `GetCharacterExtents` places inside the field (`29-field-extents`, ADR 1501) — asked of the
  # toolkit's field and of the document's node, in the screen's coordinates and then the window's,
  # and in `quorra-gtk`, whose entry refuses, the document's box read against the entry's place
  # (ADR 1516), and in `quorra-qt`, whose field answers, the document's node read beside it and
  # held inside the field too (ADR 1528); Annex O's `fdf` naming the drive's own loopback server,
  # imported at `--submissions=send` and said with nothing sent at `refuse` (`31-fragment-fdf`,
  # ADR 1527); §14.7's tree on a
  # private AT-SPI bus; and `quorra-confined` on a page of
  # stars its worker sends as marks and its device refuses, the refusal in the title (ADR 1478).
  # **Release binaries first** (the script names the command), and `pikepdf` for the fixtures it
  # writes. It prints `step, window, works|wrong|not offered|manual, what was seen` into
  # `results.tsv` and photographs each step into `shots/<window>/<step>.png` — and every other
  # top-level window beside it, because with no window manager GTK's and Qt's popups and dialogs are
  # not on the root's picture. **No step rests on a title where the window says more** (ADR 1478):
  # the popup by the pixels of its `/C` colour open and closed, `quorra`'s restrictions card by the
  # level its second row sets, print by GTK's dialogue or the other two's "over 3 page(s)", the
  # reopened form by AT-SPI's reading of the fields — the toolkits' widgets, and `quorra`'s own form
  # nodes, whose text runs and selected items carry the value (ADR 1489). Its clicks
  # are **asked of the window**: the drive runs on one private AT-SPI bus, and the outline rows, the
  # pages tab and row, the check box and the choice are found by role and name and clicked at the
  # centre of `Component.GetExtents`, with the find bar's presence a step of its own. `quorra-gtk`
  # and `quorra-qt` answer for all of them; `quorra` publishes bounds (accesskit) for the document's
  # nodes only, so its form controls are asked and its own panels' rows are not. What no window
  # answers falls back to coordinates measured on the fixtures at 1400×1100, and `coordinates.tsv`
  # says which each click was. A step that clicks nothing says `wrong` with the title it saw. About
  # nine minutes for the four.
# §14.7's tree on a real accessibility bus, which is the only way to check the AccessKit bridge
# end to end from here. A session bus, at-spi's own bus and registry, Xvfb, and `busctl` walking
# `org.a11y.atspi.Accessible` from the registry root — a real client rather than this program's
# own types. ADR 0214 has the script; the shape is:
#   dbus-run-session -- bash -c '/usr/lib/at-spi-bus-launcher --launch-immediately & sleep 3
#     ADDR=$(busctl --user --json=short call org.a11y.Bus /org/a11y/bus org.a11y.Bus GetAddress …)
#     DISPLAY=:99 AT_SPI_BUS_ADDRESS=$ADDR /usr/lib/at-spi2-registryd & sleep 2
#     DISPLAY=:99 quorra doc/PDF20_AN001-BPC.pdf & sleep 6
#     busctl --address=$ADDR call org.a11y.atspi.Registry /org/a11y/atspi/accessible/root \
#       org.a11y.atspi.Accessible GetChildren'
# **`org.a11y.Status IsEnabled` is *not* true inside a fresh `dbus-run-session`** — this line said
# it "is already true here", which is a fact about the desktop session and not about the bus the
# recipe builds. Set it, on the session bus, before the viewer starts:
#   busctl --user set-property org.a11y.Bus /org/a11y/bus org.a11y.Status IsEnabled b true
# Without it every adapter stays inactive by design and the application's whole subtree comes back
# empty, with nothing saying why. **An accessible's `Name` is a D-Bus property, not a method**, so a
# walker that calls `GetName` reads every node as `''` and looks exactly like a bridge that lost its
# labels — `get-property … org.a11y.atspi.Accessible Name`. **The registry needs a `DISPLAY` of its own**:
# without one it prints *AT-SPI: Cannot open default display*, exits, and every later call fails
# with `ServiceUnknown`, which looks nothing like the cause. **And the adapter implements no
# `GetRoleName`**, so a client asks `GetRole` and gets AT-SPI's integer — read the names out of
# `atspi-common`'s own enum in declaration order rather than numbering them by hand (ADR 0300).
# `org.a11y.atspi.Component.GetExtents` at each node is what says *where* an element is, and a node
# with no bounds implements no `Component` at all — the call errors rather than answering a zero
# rectangle, which is what "this element has no place" looks like from a client (ADR 0301).
# **Read `Description` beside `Name`**: it is where everything the platform's roles cannot carry
# arrives — a `TH` scoped to both axes, and §14.8.4.8.3's header cells for every cell. A walker that
# printed only names would have shown the four-hundred-and-seventy-seventh round's whole change as
# nothing at all. `bug2014080.pdf` exercises the clause's search and `pdfjs_wikipedia.pdf` Table
# 384's stated array, which is one document each for the two routes (ADR 0312). **And the bus is
# what found the defect that round's tests could not**: a `TH` whose words are in a `P` inside it
# has an empty `Name`, so the header sentence named nothing — every cell in that document.
# **`GetState` answers `au`, an array of 32-bit words** — decode it as 64-bit and every state comes
# back naming a different one, which looks like a plausible answer rather than an error (ADR 0338).
# It is where §12.7.5.2's toggling buttons arrive: `Toggled::True` becomes AT-SPI's `checked`.
# **`annotation-button-widget.pdf` is the witness for §14.8.4.7.2's controls, and it labels its own
# answers**: each of its nine `Form` elements sits beside a paragraph reading "Check box, checked",
# "Radio button, unselected" and so on, so the walk is checked against the document rather than
# against this program. `prefilled_f1040.pdf` is the same feature at scale — 242 `Form` elements.
# **And since ADR 0425 the client may *ask for things* rather than only read**, which is the only
# way to check an action end to end: `org.a11y.atspi.Action.GetActions` names one action, `click`,
# on every node whose content is an annotation; `DoAction(0)` on it ticks the check box and
# `GetState` says so on the next read — the three answers on that document are ticked, unticked and
# refused-because-read-only, which is Table 227 obeyed out loud. `Component.ScrollTo` answers true
# and moves nothing where the element is already on the screen, which is the designed answer;
# `Text.SetCaretOffset` on the page node answers true. **And since ADR 0445 the count of page nodes is
# itself a check**: a document Table 29 arranges in a column publishes one `DocumentFrame` per page on
# the screen, each with its own `Component.GetExtents` — `doc/PDF20_AN001-BPC.pdf` states `/OneColumn`
# itself, so it needs no key press to show two, which matters because a bare `Xvfb` has no window
# manager and `xdotool key` reaches nothing without one. On ISO 32000-2's cover, `DoAction` on the two
# `Link` elements opens both URIs. **Read the *viewer's* stdout beside the bus**: `--trace=access`
# prints one line per request carried out, and a request this host cannot place is printed by name
# instead — which is the half of trap 5 the actions did not change.
# **And since ADR 0623 the recipe applies to all three windows** — `./target/quorra`,
# `./target/quorra-gtk` and `./target/quorra-qt` each publish §14.7's tree, so the same walk
# run three times is what says they agree. Two things a native host adds to the reading. **The
# desktop lists two applications per process**, both named for the binary: `accesskit_unix` embeds a
# root of its own beside the toolkit's, so a walker that took the first application it found would
# be reading GTK's widgets or Qt's and reporting them as this program's structure — find the one
# whose window holds a `DocumentFrame`. And **`--trace=access` is the other half of the
# instrument**: it prints what was published, what a client asked for, and which field a click gave
# a value to or was refused for.
# **And since ADR 0630 a click on §12.7's two toggling kinds is carried out in all three windows.**
# `annotation-button-widget.pdf` is the document to walk it on: nine nodes declaring `click`, of
# which six give a value and three are refused on Table 227 — the same six and the same three in
# `quorra`, `quorra-gtk` and `quorra-qt`. **Read `GetState` back after *each*
# `DoAction` rather than after all nine**: a batch measures the net of a walk in which a radio
# set's second click undoes its first, which is a different question and is where ADR 0623's "three
# of nine" came from. And on a native host the tree is not the whole answer — the control a person
# sees is a `GtkCheckButton` or a `QCheckBox` written back from `Query::Fields`, so photograph the
# window either side of the walk (trap 1: with that write-back removed the bus still says six of
# nine and the pixels do not move at all).
# **Orca is not installed on this machine**, so
# what a person on a desktop still has to do is run one and listen.
cd fuzz && cargo +nightly fuzz run fragment      -- -runs=50000   # Annex O's fragment identifier,
  # and the only untrusted input here that no document carries: it arrives with the request
cd fuzz && cargo +nightly fuzz run confined_wire -- -runs=4000000 -rss_limit_mb=1024
  # the confined viewer's four decoders (ADR 0223). The one target whose input is not a document
  # but a *process*: `pdf-view-worker` runs hostile files behind seccomp and writes its answers to
  # a host that is not confined. **Seed its corpus first**, with `fuzz/seed_confined_wire.py` —
  # a second implementation of the frame layer, which spawns the release worker, asks it every
  # question the transport carries, and keeps every payload either side wrote. Unseeded it never
  # gets past a one-byte discriminant into an outline's tree or a thumbnail's samples.
  # **It reads `MAGIC` out of `protocol.rs` since the seven-hundred-and-thirty-sixth**, because a
  # copy of it went one behind and this seeder therefore refused to run — silently, for as long as
  # nobody re-seeded — which is the corpus for this target being empty. Pinning the greeting is
  # right; copying the constant was not:
  #   cargo build --release -p viewer-confined --bins
  #   python3 fuzz/seed_confined_wire.py target/pdf-view-worker fuzz/corpus/confined_wire \
  #     doc/PDF20_AN002-AF.pdf doc/PDF-Declarations.pdf doc/ISO_32000-2_sponsored_EC3.pdf \
  #     doc/PDF20_AN001-BPC.pdf doc/pdf.js/test/pdfs/issue15716.pdf
  # **A question this script does not ask now stops it**, since the eight-hundred-and-sixteenth,
  # and that is the whole of what this file has to say about its coverage. It used to say the
  # seeder covered 25 of 29 questions and named the four missing; by the time anyone acted on that
  # sentence there were **seven** missing and 32 carried, because three more had arrived in the
  # meantime and a count written down is a count that goes stale in silence. The script now reads
  # `query_kind` and refuses to run against a discriminant it has no entry for, naming it — so the
  # answer to "is it complete" is the exit status of a run rather than a line here. The payload
  # *shapes* stay hand-written, which is what makes it a second implementation. ADR 0747
  # **The length was a million until the eight-hundred-and-twenty-first, and it is the only one in
  # this file that round changed.** Not because the target was "still climbing" — measured out to
  # eight million it is still climbing there too, and *still climbing* is a property of a
  # logarithmic curve rather than a reason. Because **this target buys more coverage per second
  # than any other here**: at a million runs it adds coverage in tens of seconds on a quiet
  # machine where `display_list` adds a fraction of that in ten minutes, so its budget was small
  # relative to what its executions cost. Four million is where the return per doubling halves,
  # and it is under three minutes. ADR 0751 has the curve


cd fuzz && cargo +nightly fuzz run display_list  -- -max_total_time=600 -rss_limit_mb=4096
  # **A corpus of encoded lists goes stale when the encoding moves**, and nothing but its `INITED`
  # coverage says so: re-seed it with the recipe below whenever that figure falls far under what a
  # few fresh seeds reach (ADR 1423).
  # ADR 0607's *other* payload, and the second target whose input is a process rather than a
  # document: a window on the confinement receives display lists, so the unconfined host parses a
  # whole page of geometry that the confined side chose. Four shared tables, a clip table whose
  # entries name each other, a nested command tree, four shading geometries and a soft mask holding
  # commands of its own — ADR 0626. Beyond "nothing panics" it asserts three things, so that
  # deleting a check in the decoder fails this target: every identifier a decoded list holds points
  # at something, every decoded image's samples fill its stated dimensions, and **anything this
  # decoder accepts this encoder can write, reading back the same list**. That last one is what
  # catches the two halves of a codec drifting.
  # **Seed its corpus first**, and the seeder is an *example* rather than a Python script, because
  # producing a display list means running the interpreter:
  #   cargo build --release -p viewer-confined --example list_over_the_wire
  #   <built>/examples/list_over_the_wire --seeds fuzz/corpus/display_list doc/pdf.js/test/pdfs/*.pdf
  # It writes one seed per page under `--seed-max`, 256 KiB by default and for the reason
  # `doc/todo/02` records of `page`: unbounded, the same corpus is 841 MB of seeds and almost all of
  # it is four scanned documents' pixels, which state nothing about this format and are paid for in
  # every merge. Bounded, it is a few hundred real pages in a few megabytes. Unseeded the target
  # reaches an empty list and little else, since a table count is eight bytes
cd fuzz && cargo +nightly fuzz run cms          -- -runs=50000   # §12.8.3.3's signature value:
  # `pdf_signature::der`'s X.690 reader and `pdf_signature::cms`'s RFC 5652 SignedData, the tree's only
  # ASN.1, and the reader every signed document goes through before `x509` sees a certificate.
  # **Seed its corpus** from every CMS object this tree already holds, which since the
  # eight-hundred-and-twenty-fifth is what `fuzz/seed_cms.py` collects, by three routes at once:
  #   find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \
  #     | python3 fuzz/seed_cms.py fuzz/corpus/cms -
  # The `-` is the list on standard input rather than `xargs`, and `-L` because a worktree's
  # corpora are symbolic links; `seed_x509.py`'s block below says why both, and `fuzz/seed_der.py`
  # is the X.690 walk the two share.
  # **Point it at the whole disk rather than at `doc/pdf.js` alone**, which is what this line used
  # to say — "the eleven `/Contents` blobs the nine signed corpus documents hold" — while
  # `grep -alr /ByteRange corpus-cache doc/corpora doc/pdf.js/test/pdfs | wc -l` prints the
  # population it is entitled to. That is ADR 0751's defect one target down, in the same words,
  # and ADR 0754 is the round that fixed it here. The script's summary line says how many arrived
  # by each route and `tools/fuzz.sh --list` how many the corpus holds; no count is written here.
  # The three routes, because a PDF holds a CMS object in three unrelated places: §12.8.3.3.1's
  # signature value in `/Contents`, kept as the file's own bytes so that a producer's
  # indefinite-length BER survives — the shape a from-scratch input never forms; the RFC 3161
  # timestamp tokens a CAdES signature carries *inside* itself as `SignerInfo` attributes
  # (§12.8.3.4.3), which no scan of the file can see because the file states them in hexadecimal
  # inside another CMS object; and §12.8.4.4's Table 262 `/TS`, "[a] stream containing the
  # DER-encoded timestamp", found by RFC 5652's opening bytes in the file and in its inflated
  # streams. There is **no fourth route out of this tree's own fixtures**, and that is worth
  # knowing rather than assuming: `seed_x509.py` has one because `crates/pdf-model/src/*.rs`
  # state their certificates as hexadecimal, and `cms.rs`'s `fixtures` module *builds* its
  # signature values in Rust at test time instead — so the shapes it constructs for the signature
  # formats §12.8.3 defines reach no corpus, and what the corpus has of them is whatever the
  # documents have. Clean at 1 000 000 in the three-hundred-and-seventy-seventh (ADR 0215) and
  # again in the three-hundred-and-ninety-second, after its `SignerInfo` gained a signature and an
  # identifier
cd fuzz && cargo +nightly fuzz run revocation   -- -runs=1000000  # §12.8.4's revocation material:
  # `pdf_signature::revocation`'s two readers, RFC 5280 section 5.1's `CertificateList` and RFC 6960
  # section 4.2.1's `OCSPResponse`, both of which arrive as streams out of a stranger's file
  # (§12.8.4.3, Table 261) and neither of which anything in this tree read before the
  # thousand-and-fifty-third session. ADR 1067 committed to this target with the code.
  # **Seed its corpus** with `fuzz/seed_revocation.py`, `seed_x509.py`'s sibling, by two routes:
  #   find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \
  #     | python3 fuzz/seed_revocation.py fuzz/corpus/revocation -
  # The `-` is the list on standard input rather than `xargs`, and `-L` because a worktree's
  # corpora are symbolic links; `seed_x509.py`'s block below says why both, and `fuzz/seed_der.py`
  # is the X.690 walk all three share.
  # **Point it at the whole disk, and expect `doc/pdf.js` to give nothing.** Not one document in
  # that submodule carries a `/DSS` at all — `pdf-signature`'s `every_corpus_document_is_asked_
  # whether_it_carries_a_security_store` is the command that says so — so a run seeded from it
  # alone seeds zero, which is what this recipe would have looked like working. The population is
  # in the crawl, and `examples/signature_algorithm_census` prints how many documents hold one.
  # The two routes, because §12.8.4.2 puts the material in two places and says which comes first:
  # a document's own `/CRLs` and `/OCSPs` streams, and §12.8.3.3.2's `adbe-revocationInfoArchival`
  # attribute inside a signature value, which no scan of the file can see because the file states
  # it in hexadecimal inside a CMS object.
  # **The recogniser was calibrated against `openssl` rather than against itself** (trap 13):
  # `openssl crl -inform DER` and `openssl ocsp -respin` classify what it wrote, which is evidence
  # about the reading and never the definition of it (`CLAUDE.md` principle 5)
cd fuzz && cargo +nightly fuzz run x509         -- -runs=1000000  # the signer's certificate and
  # the verifications that run on the key inside it: `pdf_signature::x509` walks RFC 5280's
  # structure and `pdf_signature::pkcs1`, `pdf_signature::pss` and `pdf_signature::dsa` run the tree's only
  # loops whose trip counts come out of numbers in the file. Since the six-hundred-and-eighty-ninth
  # it also reaches `pdf_signature::ecdsa` and `pdf_signature::eddsa`, whose arms assert the same thing on
  # every signature shape a certificate's curve admits, including BSI TR-03111's plain `r ‖ s`. The property that matters is the last one — the target
  # verifies against a digest *it* chose, so `Ok(true)` would be a defect in the comparison rather
  # than a lucky input.
  # **Seed its corpus** from every certificate this tree already holds, which since the
  # eight-hundred-and-twenty-first is what `fuzz/seed_x509.py` collects, by three routes at once:
  #   find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \
  #     | python3 fuzz/seed_x509.py fuzz/corpus/x509 crates/pdf-model/src/*.rs -
  # The `-` is the list on standard input rather than `xargs`, because `xargs` runs the script once
  # per batch and each run counts only its own — a corpus this size gives thirty summaries and no
  # total. `-L` because a worktree's corpora are symbolic links to the primary checkout's.
  # **Point it at the whole disk rather than at `doc/pdf.js` alone**, which is what this line used
  # to say and is the whole reason this was the thinnest-seeded target in the tree. The population
  # it is entitled to is what `grep -alr /ByteRange corpus-cache doc/corpora doc/pdf.js/test/pdfs
  # | wc -l` prints, and naming one submodule asked for a fraction of it — a negative claim about a
  # corpus carries its population inside it whether or not it says so, which is `doc/habits.md`'s
  # rule under *Measuring* and was as true of a seeding recipe as of a ledger row (ADR 0751). The
  # script's summary line says how many certificates arrived by each route and `tools/fuzz.sh
  # --list` how many the corpus holds; no count is written here, for the reason ADR 0747 gives
  # about the seeder next door.
  # The three routes, because a certificate reaches a PDF in three unrelated ways: §12.8.3.3.1's
  # CMS object, walked structurally, which is the second implementation ADR 0229 wanted;
  # §12.8.4.3's `/DSS` `/Certs` and Table 255's `/Cert`, which a document states as objects of its
  # own and which are found by RFC 5280 section 4.1's opening bytes in the file and in its inflated
  # streams; and the hexadecimal in `crates/pdf-model/src/{x509,dsa,pss,ecdsa,eddsa}.rs`'s
  # `fixtures` modules. **That third route is what makes a clone's corpus complete**: the DSA
  # certificate is the only input that reaches `dsa::verify`, and the P-384, P-521,
  # brainpoolP256r1 and Ed25519 ones are the only inputs that reach the arms added in the
  # six-hundred-and-eighty-ninth session — until this route existed, this line asked a round to
  # re-make them with `openssl req -new -x509` by hand. The module documentation still has those
  # invocations, and any certificate at all is a legal input.
  # **RFC 5280 section 5.1's `CertificateList` is the near miss to know about**: a revocation list has
  # this same three-member shape, satisfies that RFC's section 4.1.1.2 rule that the two algorithm identifiers
  # agree, and sits in `/CRLs` immediately beside `/Certs` — so the second route reads as far as
  # `Validity`, where a certificate states two `Time`s and a revocation list one.
  # Clean at 1 000 000 in the three-hundred-and-ninety-second (ADR 0229)
cd fuzz && cargo +nightly fuzz run ccitt        -- -max_total_time=600 -timeout=20  # §7.4.6's fax
  # decoder of this tree's own (ADR 1349): six head bytes choose Table 11's parameters, the rest is
  # coded data. Seeded from the fax streams of the corpus; it was the one target here without a line,
  # so `tools/fuzz.sh --list` refused it and exited 1 until this line was written (ADR 1423).
cd fuzz && cargo +nightly fuzz run shaping      -- -max_total_time=600 -max_len=16384  # UAX #9 and
  # the Unicode Standard's cursive joining over a field value, §12.7.4.3 (ADRs 1413, 1414, 1417).
  # Seed it with `python3 fuzz/seed_shaping.py fuzz/corpus/shaping` — the UCD's bidirectional cases
  # and generated Arabic words; `-max_len` is raised because joining and rule L1 are walks over the
  # whole text, and a quadratic one shows only on a long value.
  # It reaches `Label` and the joining and bidirectional halves; `fold`, `decompose` and
  # `mark_class` are table lookups over the code points, which only the find bar calls with a
  # page's text, so the `find` target below is where they meet untrusted input (ADR 1495).
cd fuzz && cargo +nightly fuzz run jbig2        -- -max_total_time=600 -rss_limit_mb=2048 -timeout=30
cd fuzz && cargo +nightly fuzz run jpx          -- -max_total_time=600 -rss_limit_mb=2048 -timeout=30
  # §7.4.7's and §7.4.9's filters as the confined worker runs them — the codecs are `hayro-jbig2`
  # and `hayro-jpeg2000`, the framing, the prefix retry and the sample budget are `pdf-sandbox`'s —
  # reached through `Isolation::InProcess`, which calls the worker's own functions; `jpx` also reads
  # the same bytes with `pdf_model::jpeg2000`'s header reader. `-timeout=30` is the worker's own
  # request deadline, so a slow unit under it is a decode the viewer would have waited for and one
  # over it is one the worker would have been killed for. Seed both with `fuzz/seeds.sh`, which
  # frames the codestreams of the documents naming either filter the way each target reads them
  # and keeps the smallest of each shape `fuzz/seed_codecs.py` states (ADR 1571). libFuzzer stops at its first
  # timeout, and `hayro-jbig2` has inputs past the deadline (ADR 1424's third section), so a run
  # meant to go on past one adds `-fork=1 -ignore_timeouts=1` and reads what it leaves behind.
cd fuzz && cargo +nightly fuzz run xfdf         -- -max_total_time=600  # ISO 19444-1's XFDF and the
  # import it feeds, §12.7.6.4 (ADR 1297). Seeded from `crates/pdf-model/tests/xfdf/` by `fuzz/seeds.sh`.
cd fuzz && cargo +nightly fuzz run fetched_import -- -max_total_time=1200 -rss_limit_mb=2048 -timeout=20
  # a server's answer as a host hands it over, `Command::Respond` into `interact::import` (ADR
  # 1527): Annex O's `fdf` fetched, or §12.7.6.2's submission answered, against a form with a field
  # tree, a check box, a choice and a template, in front or behind. Every answer is said, and said
  # about the document it names. Seeded by `fuzz/seed_fetched_import.py` under eight routes each.
cd fuzz && cargo +nightly fuzz run linearize    -- -max_total_time=600 -rss_limit_mb=2048 -timeout=60
  # Annex F both ways: `linearize::state` on the input, and `serialize_linearized`'s file opened
  # again and found linearised with `/L` its length and `/N` the plan's pages (ADRs 1293, 1309).
  # Seeded by `fuzz/seeds.sh` with the smallest document of each file structure and order of page
  # count, `serialize`'s recipe with Annex F's plan beside it (ADR 1571).
cd fuzz && cargo +nightly fuzz run embed        -- -max_total_time=600 -rss_limit_mb=2048 -timeout=20
  # §9.9.1's `/FontFile2` program, §9.9.2's subset and the `CFF ` subsetter's charstring walk, over a
  # face the machine offers (ADRs 1425, 1438, 1449): a re-embedded subset renumbers nothing, the
  # kept glyphs are dense from `.notdef`, a subset carries the six-letter tag. Seeded by
  # `fuzz/seeds.sh` with the faces under `/usr/share/fonts`, which is the population on this disk.
cd fuzz && cargo +nightly fuzz run jpeg_bands   -- -max_total_time=600 -rss_limit_mb=2048 -timeout=20
  # §7.4.8's frames cut into bands at their restart intervals (ADR 1433) and at an entropy pass's
  # rows (ADR 1481), each against the whole frame's decoder through `pdf_model::image::
  # banded_decodes`, below the production floor (ADR 1495). Differential: a byte a band moves, or a
  # band decoded where the whole decoder refuses, is a finding. Seeded with the smallest
  # `DCTDecode` stream of 64 KiB or less of each frame shape — marker, precision, components and
  # their sampling, restart interval, `DNL` — out of documents of 4 MiB or less, and the two
  # modules' fixtures; `fuzz/seed_streams.py` says why each bound is the one it is (ADR 1559).
  # Until `doc/questions/Q227`'s fork is in, a run stops within minutes on `zune-jpeg`'s DC
  # multiply (`bitstream.rs` line 400, overflow checks on): a campaign adds `-fork=1
  # -ignore_crashes=1` and reads each crash's panic location, and only another location is a finding.
cd fuzz && cargo +nightly fuzz run find         -- -max_total_time=600 -rss_limit_mb=2048 -timeout=20
  # the find bar's match over a page's readback through `viewer_core::find_in_text`: §9.10.2's
  # presentation forms folded, canonical decompositions and marks a needle may leave off (ADRs
  # 1465, 1477) — the one caller of `pdf_font::shaping`'s `fold`, `decompose` and `mark_class` with
  # untrusted text. A literal occurrence of a lowercase needle is always found. Seeded by
  # `fuzz/seed_find.py` from the UCD's bidirectional lines and the cases the matcher was built for.
cd fuzz && cargo +nightly fuzz run meet         -- -max_total_time=600 -rss_limit_mb=2048 -timeout=20
  # §10.7.4's intersection of fills inside one pixel as the device computes it (ADR 1467), through
  # `raster_gpu::intersection`, against an exact piecewise integration in the target (ADR 1495).
  # Seeded by `fuzz/seed_meet.py` with the meet's own test shapes on a sixteenth-pixel grid.
cd fuzz && cargo +nightly fuzz run vfs_write    -- -max_total_time=600 -rss_limit_mb=2048 -timeout=20
  # RFC 0003 section 5.2's five write verbs over a document in memory through the in-process
  # worker: every commit begins with the file it replaces (§7.5.6), a deleted page is one page.
  # Seeded by `fuzz/seed_vfs_write.py` from the pdf.js documents under 32 KiB.
# **A campaign runs the targets without the sanitiser, under the lock and `tools/bounded.sh`.** The
# crates a fuzz target reaches forbid `unsafe`, so what AddressSanitizer adds over Rust's own checks
# is a dependency's unsafe code, and what it costs is the shadow memory that `tools/bounded.sh`'s
# `RLIMIT_DATA` refuses (the paragraph above). `-s none` keeps libFuzzer's coverage and overflow
# checks, runs about twice as fast, and fits the bound; the sanitised build stays the one to reach
# for when a dependency's crash needs its stack. Give each run a scratch corpus as its *first*
# directory and the seeded one second, so that what the run finds is written to the scratch one
# and a worktree's linked `fuzz/corpus` is read rather than grown (ADR 1423):
#   cargo +nightly fuzz build -O -s none
#   RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --data 4 --tree 4 -- \
#     <target dir>/x86_64-unknown-linux-gnu/release/<target> <scratch>/<target> fuzz/corpus/<target> \
#     -max_total_time=600 -rss_limit_mb=2048 -timeout=20 -jobs=1 -artifact_prefix=<scratch>/<target>-
# A target whose own line above states a larger ceiling runs under that one — `page`'s 4096 MB,
# which `ContentStreamCycleType3insideType3.pdf` among its seeds needs — and `jbig2` runs past
# `hayro-jbig2`'s known timeouts with its line's `-fork=1 -ignore_timeouts=1`. Read `INITED` for
# the disk's corpus and for fresh seeds before spending the run: a corpus far below what fresh
# seeds reach is a corpus to regenerate (ADRs 1423, 1495).
# `tools/state.sh fuzz` prints what the disk holds of every target: its seeds, and the crashes,
# timeouts and memory refusals sitting in `fuzz/artifacts/`.
```

**Sections of `tools/state.sh` a round runs by name**, each seconds long and each in `quick`, with
the argument in the comment above its function:

```sh
tools/state.sh navigation   # the four navigational documents against --bin pointers, overtaken
                            # and unread: their absent pointers, and each note's confirmed-unread claim
tools/state.sh departures   # every `departed` row's deciding ADR and what has cited it since
                            # (ADR 1166); run it before touching such a row
tools/state.sh cited        # each clause a source cites against its row's `code` list (ADR 1274)
tools/state.sh traps        # the trap index's rows, the group files' lengths, and which traps
                            # doc/history/ cites — the most cited and how many carry four-fifths
tools/state.sh remedies     # doc/todo/66's two halves: remedy sites not built, profiles that
                            # still say `does not carry out yet`
tools/state.sh instruments  # the examples this catalogue does not name
tools/state.sh main-checkout # what the main checkout holds that a merge does not carry — read
                            # only; doc/environment.md's *After a merge* is the commands (ADR 1440)
tools/state.sh oracle-held  # the oracle's held pages per verdict and group, from its constants and
                            # without the walk, and the next page's candidates (ADR 1512)
tools/state.sh binaries     # the main checkout's target/, the commit `install` built it from, and
                            # whether every file is still the one installed (ADR 1511)
cargo run -p conformance --bin unread   # the whole list navigation filters
cargo run -p conformance --bin pointers # every path pointer, by rung; an owner's
                            # uncommitted answer is its own rung, not an absent pointer
```

**`tools/batch.sh commit <message-file>`** is the merge's, not a round's: it stages a batch's whole
population by name, counts it, and commits, so a refused `git add` cannot let a partial commit
through (ADR 1313). A round commits nothing. **`tools/batch.sh install`** is the merge's too, run
after the fast-forward: it builds what a person runs from the commit `main` names and installs it
into the main checkout's `target/`, the commit and each file's SHA-256 beside it in
`target/installed-from` (ADR 1511). A round installs nothing there.

**Two measurements that are not gates, and each says why in its own header.**

```sh
# What it would cost to check this project's citations against the PDF instead of `doc/md/`.
# `doc/todo/48`'s item 5 and `doc/todo/63`'s success condition, with a number instead of a
# fear: it asks `tools/conformance`'s own two questions of both substrates and prints where
# they disagree. ~7 s. Its output is counts and clause numbers and no sentence of the
# standard, which is why the numbers may be written down (ADR 0187, ADR 0257).
cargo run --profile gates -p pdf-retrieve --example substitution_cost

# Which of this tree's quotations land on text Errata Collection 3 struck out. Not a gate for
# ADR 0252's reason — the checker must keep comparing against a conversion this project did
# not make. ~4 s over all fourteen documents. `doc/todo/01`'s twelfth sweep.
cargo run --profile gates -p spec-errata -- check doc/*.pdf
```

**Incomplete pages are compared and printed by the oracle but cannot fail it** — a page we
already say we cannot draw is expected to differ. **The denominator moves in both directions on
purpose**: it grows when reports stop firing (46 pages in session 21) and shrinks when a silence
ends (8 in session 17, 43 in session 8). A report must never be reached for as a way of making a
contradiction go away (trap 5).

Two classification counts are deliberately throwaway — scratch diagnostics do not belong in a
tree held to `clippy::pedantic`. **Whether a page's fonts are embedded** walks each `/Font`
resource and its `/DescendantFonts` for `/FontFile`, `/FontFile2`, `/FontFile3`. **The annotation
subtype breakdown** comes free from the corpus gate's own output:
`grep -o 'Annotation { detail: "[^"]*"' | sort | uniq -c`.

The oracle's first run on a fresh build directory is ~95 s and writes 319 MB of remembered
reference renders; **a warm run was ~30 s when that was written and is 102 s in the
four-hundred-and-forty-fifth**, at the same 99.7% hit rate over a 1.6 GB cache and with the machine
otherwise idle. **Read the printed hit rate rather than the
clock** — and it is the tell for something outside this tree: session 166 saw it at 85.7% on an
unchanged corpus, which was `poppler` being upgraded on this machine and every cached
`pdftoppm` render becoming a new key. Nothing about the verdicts moved. **The clock is now ours
rather than the subprocesses'**: the run prints *271 s ours, 90 s in the three reference renderers*
over 24 cores, where the ~30 s era's split was ~23 s of subprocess. The corpus gate (3.2 → 5.0 s) and
quorra's (25.1 → 39.0 s) moved by about the same factor, which points at the
three gates' common half — every one of the 974 first pages rasterised, and §11.4.7 drawing a
four-component page twice since ADR 0262. **If the clock ever becomes the constraint, that is where to
look and not at the subprocesses.** `PDFREF_CACHE=off` asks the three renderers again — how "the cache changes no verdict" is
re-checked; `PDFVIEWER_ORACLE_ONLY=a,b` compares only matching pages in 0.2 s and refuses to check
the ratchets, saying so. **`PDFREF_GS_CMYK_PROFILE=<file>` points `ghostscript` at another press**,
which is trap 9's shared-data removal made runnable: it is `-sDefaultCMYKProfile=` and nothing
else, it costs a full `gs` re-render because the cache keys on the invocation, and it is never set
by a gate. ADR 0773 has what it measured and the two controls a null result needs. `PDFVIEWER_CORPUS_TRACE=1` names each document as it starts, which is how
a hang is identified from a killed run.

**Two re-runs of the bound the oracle judges a text page by**, one differing fraction doing two
jobs — forming a consensus and flooring our own bound through `widened_to` — which ADR 0243
measured, ADR 0771 declined to split, ADR 0776 showed to be one knob, and ADR 0773 read the vector
row of. Both are walks, so both go behind the lock. The spread of the fixed bounds against the
references' own, and the substitution table beside it — one command, both tables:

```sh
PDFVIEWER_ORACLE_SPREAD=1 cargo test --profile gates -p pdf-model --test oracle -- \
    --ignored --nocapture the_fixed_bounds_against_the_references_own_spread
```

And the shared-profile removal: `doc/todo/02` §2's oracle line run three times, once under each of
these, because the cache keys on the invocation and re-renders `ghostscript` rather than answering
from the baseline's renders. The third is the control and must reproduce the baseline byte for
byte; the second is the null removal ADR 0773 needed a control for, since a CGATS copy is the same
press as the profile it replaces:

```sh
PDFREF_GS_CMYK_PROFILE=/usr/share/ghostscript/iccprofiles/ps_cmyk.icc        # a different press
PDFREF_GS_CMYK_PROFILE=<a CGATS copy>                                        # the same press: a null
PDFREF_GS_CMYK_PROFILE=/usr/share/ghostscript/iccprofiles/default_cmyk.icc   # the control
```

Cargo prints one line about `proc-macro-error2` being rejected by a future compiler. It arrives
through `iai-callgrind`, a dev-dependency reaching no shipped binary, and `deny.toml` records the
exception. Nothing to chase.

**`doc/pdf.js` is a submodule** (Apache-2.0, pinned at v6.1.200) holding the 974 PDFs. Optional to
clone — every test using it reports being skipped — but the ratchets mean nothing without it, so
CI must have it. **The time budget reports; it cannot enforce**: a Rust thread cannot be
cancelled, so a document that never returns hangs the suite rather than failing it.

## Which features a scope resolves, and what the shipped binary carries

Cargo unifies features across whatever is in the build, so **the resolved feature set is a property
of the invocation** and not of the tree. Three scopes matter here and they are genuinely different
invocations: the census's `-p viewer-core --test accessibility_census`, `--workspace`, and
`--release --bin quorra`. The question a round asks is whether the gate is measuring the
program a user gets.

It is answerable exactly, in about a minute, and the answer decays — so what is written down is the
command:

```sh
cargo +nightly test  --profile gates -p viewer-core --test accessibility_census \
                     --unit-graph -Z unstable-options > subset.json
cargo +nightly test  --workspace --profile gates  --unit-graph -Z unstable-options > workspace.json
cargo +nightly build --release --bin quorra   --unit-graph -Z unstable-options > shipped.json
```

Each unit in that JSON carries `pkg_id`, `mode`, `target.kind` and `features`. Take the transitive
closure of the root you care about — the unit whose `target.name` is `accessibility_census` and
`kind` is `["test"]`, or `quorra`/`["bin"]` — and compare `(package, mode, kind) → features`
between two files. Comparing the *whole* file instead is noise: the workspace graph contains
hundreds of crates the subset never builds, and `resolver = "3"` keeps a build-dependency's
features separate from a normal one's, so the same package legitimately appears twice.

**What it said in the seven-hundredth session** (ADR 0557 §3): the shipped binary differs from the
whole-workspace build in `either` and `serde` alone, both additive; the census's subset differs in
ten crates, every one of which was traced to its consumer and changes no value the program
computes. **That is a claim about today's dependency set** — a crate gaining a behaviour-changing
feature would falsify it, which is why the commands are here and the conclusion is in the ADR.

`--unit-graph` is nightly-only, which is why this is a method rather than a gate. The gate that
does exist for the neighbouring failure — a gate measuring a build whose *binaries* are incomplete
— is `tools/conformance/tests/sandbox_gates.rs`, and it is trap 16's.

**The examples no paragraph above names, one line each** — a census, a cost, a probe a later round
re-runs. Each is the example's own usage line and the first sentence of its own header, which says
what it measures; the header is the rest of the argument, and the ADR or ledger note that used the
number is where the number is. One that walks a corpus is a heavy walk and runs behind the lock
`doc/todo/02` names. Grouped by package, in the order `tools/state.sh instruments` lists them.

```sh
cargo run -p pdf-archive --example frontier
  # What the structural audit covers, at both of its granularities, and where it stops.
cargo run --release -p pdf-archive --example survey_cost -- FILE.pdf
  # What one survey of a document costs, and what a full report costs on top of it.
cargo run --release -p pdf-archive --example unreferenced -- doc/veraPDF-corpus
  # What ISO 19005's unreferenced-named-resource exemption reaches, and what nothing reaches.
cargo run --release -p pdf-font --example partial_to_unicode_census -- doc/pdf.js doc/corpora
  # How many substituted composite fonts state a `/ToUnicode` that omits codes their character
  # collection names.
cargo run --release -p pdf-font --example to_unicode_kind_census -- doc/pdf.js doc/corpora
  # How many font dictionaries state a `/ToUnicode` that is not the stream the clause requires.
cargo run --release -p pdf-font --example type1_encoding_census -- doc doc/pdf.js
  # How many bare Type 1 programs claim to encode codes their own `/Encoding` array never names.
cargo run --release -p pdf-model --example anisotropic_band_census -- doc/pdf.js/test/pdfs/*.pdf
  # How wide the sub-pixel substitution's band actually becomes, when the placement is anisotropic.
cargo run --release -p pdf-model --example annotation_group_census -- doc/pdf.js/test/pdfs/*.pdf
  # §12.5.6.2's `/IRT` and `/RT`, counted: how many annotations belong to a group, and how many of
  # them state a group attribute of their own.
cargo run --release -p pdf-model --example annotation_state_census -- doc/pdf.js/test/pdfs/*.pdf
  # §12.5.6.3's `/State` and `/StateModel`, counted: how many annotations a reviewer has ruled on.
cargo run --release -p pdf-model --example appearance_transparency_census
  # §12.5.5's transparency sentences, counted: what an annotation's appearance says about the group
  # it is composited as, and what the annotation says about how that group meets the page.
cargo run --release -p pdf-model --example attribute_owner_census -- doc/pdf.js/test/pdfs/*.pdf
  # §14.8.5.3's ranking, measured: how often two owners state one attribute, and who owns them.
cargo run --release -p pdf-model --example black_generation_census -- doc/pdf.js/test/pdfs/*.pdf
  # How many documents state §10.4.2.4's black generation or undercolour removal, and how many state
  # a *function* rather than naming the device's own.
cargo run --release -p pdf-model --example black_point_census
  # How many corpus pages state a Cal space's `/BlackPoint`, and how many state a real one.
cargo run --release -p pdf-model --example border_overhang_census
  # §12.5.4's "completely inside the annotation rectangle", measured in pixels rather than read.
cargo run --release -p pdf-model --example border_precedence_census
  # §12.5.4's borders, counted where this tree actually constructs one.
cargo run --release -p pdf-model --example callgrind_pages -- 20
  # Interprets a *run of distinct pages*, for deterministic instruction counting.
cargo run --release -p pdf-model --example cloudy_border_census
  # Table 169's cloudy border effect, counted where it is stated and where it is constructed.
cargo run -p pdf-model --example coincident_edge_probe
  # Which composition multiplies a rectangle's edge coverage by itself, and which does not.
cargo run --release -p pdf-model --example colour_key_mask_census -- <file.pdf>…
  # How many documents state §8.9.6.4's colour key `/Mask`, and how many of those are filtered.
cargo run -p pdf-model --example compare_rasters -- <left.png> <right.png>
  # Prints the oracle's own four measurements between two PNGs on disk.
cargo run --release -p pdf-model --example crop_box_census -- doc/pdf.js/test/pdfs/*.pdf
  # How many documents draw where ISO 32000-2 §14.11.2.1 says nothing shall be shown.
cargo run --release -p pdf-model --example damaged_dictionary_consumers -- <dir-or-file>...
  # Which of a file's damaged dictionaries something in the document *names*, and under what key.
cargo run --release -p pdf-model --example damaged_stream_census -- <dir-or-file>...
  # How many documents hold a stream that decodes only as far as its damage, and where.
cargo run --release -p pdf-model --example delegated_census -- doc/pdf.js/test/pdfs/*.pdf
  # How much of a page §6.3.2.2's instruction takes off it, over the corpus.
cargo run --release -p pdf-model --example display_list_digest -- doc/pdf.js/test/pdfs/*.pdf
  # A digest of every corpus document's first page, for proving a change drew nothing differently.
cargo run --release -p pdf-model --example element_bounds_census -- doc/pdf.js/test/pdfs/*.pdf
  # Table 379's `/BBox`, and the structure elements that reach an assistive technology with no place
  # at all.
cargo run --release -p pdf-model --example empty_font_program_census -- doc/pdf.js/test/pdfs/*.pdf
  # How many embedded font programs decode to no bytes at all, and what each one is.
cargo run --release -p pdf-model --example encryption_census -- doc/pdf.js/test/pdfs/*.pdf
  # ISO 32000-2 §7.6's populations, counted rather than remembered.
cargo run --release -p pdf-model --example file_attachment_census -- doc/pdf.js/test/pdfs/*.pdf
  # §12.5.6.15's `/FS`, counted against §7.7.4's `/EmbeddedFiles` tree.
cargo run --release -p pdf-model --example filter_census
  # How many corpus documents carry each sandboxed image codec, and how large each codestream is.
cargo run --release -p pdf-model --example fixed_print_census -- doc/pdf.js/test/pdfs/*.pdf
  # §12.5.6.22's watermark annotations, and how many of them state Table 193's `/FixedPrint`.
cargo run --release -p pdf-model --example font_cache_budget -- 100
  # What each font-cache budget gives up, over a run of pages.
cargo run --release -p pdf-model --example font_flags_census
  # How a corpus writes a font descriptor's `/Flags`, against what ISO 32000-2 §9.8.2 states of it.
cargo run --release -p pdf-model --example form_depth_cost -- [KIND] [DEPTH_A DEPTH_B]
  # What one nested form `XObject` costs in stack, measured rather than assumed.
cargo run --release -p pdf-model --example free_text_census -- doc/pdf.js/test/pdfs/*.pdf
  # §12.5.6.6's annotations as the corpus states them, and what Table 167 says about editing one.
cargo run --release -p pdf-model --example glyph_class_census
  # What a corpus writes in ISO 32000-2 §9.8.3.3's `/FD`, and which of it could change a page.
cargo run --release -p pdf-model --example group_space_census -- doc/pdf.js/test/pdfs/*.pdf
  # Which space a *painted* group actually composites in — §11.4.7's page group and §11.6.6's
  # inheritance, rather than whatever `/CS` a group dictionary happens to carry.
cargo run --release -p pdf-model --example image_prefix_census -- doc/pdf.js/test/pdfs
  # What `Document::image_stream` decodes, and how often one page asks it for the same bytes.
cargo run --release -p pdf-model --example image_region_census -- doc/pdf.js/test/pdfs/*.pdf
  # How much of an image a magnified view actually shows, and how much was decoded to show it.
cargo run --release -p pdf-model --example indexed_all_census -- <file.pdf>…
  # How the corpus states §8.6.6.3's `Indexed` spaces and §8.6.6.4's `/All` colourant.
cargo run --release -p pdf-model --example jpx_dump -- doc/pdf.js/test/pdfs/issue5475.pdf /tmp/jpx
  # Writes one document's `/JPXDecode` codestreams out as files, for work outside this tree.
cargo run --release -p pdf-model --example jpx_colour_census -- --list <paths.txt>
  # Every `JPXDecode` image's `colr` boxes by T.801 method and enumeration, and which decide (§7.4.9).
cargo run --release -p pdf-model --example kidless_node_census -- <file.pdf>…
  # How many documents state a page tree node with no `/Kids`, and what is beside it.
cargo run --release -p pdf-model --example list_continuation_census -- \
    $(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf') $(find -L doc/corpora corpus-cache -name '*.pdf') doc/*.pdf
  # Table 382's `/ContinuedList` and `/ContinuedFrom`: how many lists say they continue another.
cargo run --release -p pdf-model --example long_mitre_census
  # How often a document asks for a mitre a rasteriser's own join code will not draw.
cargo run --release -p pdf-model --example markup_text_census -- doc/pdf.js/test/pdfs/*.pdf
  # What a markup annotation offers §12.5.6.14's popup window to display.
cargo run --release -p pdf-model --example mcid_stream_census -- \
    $(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf') $(find -L doc/corpora -name '*.pdf') doc/*.pdf
  # How many pages number two content streams' marked-content sequences from zero, and collide.
cargo run --release -p pdf-model --example media_box_census -- <file.pdf>…
  # How many pages state no usable `/MediaBox` anywhere in their ancestry.
cargo run --release -p pdf-model --example mesh_census
  # How many corpus documents state a mesh shading, and how many of them state a `/Function`.
cargo run --release -p pdf-model --example mesh_triangle_census -- \
    doc/pdf.js/test/pdfs doc/corpora corpus-cache
  # How close a real mesh shading comes to the bound that stops one being read.
cargo run --release -p pdf-model --example name_dictionary_and_file_spec_census -- \
    doc/pdf.js/test/pdfs/*.pdf
  # Three entries two tables define and nothing in this tree consumed: `/Names /AP`, `/Thumb`,
  # `/EP`.
cargo run --release -p pdf-model --example nchannel_census -- @paths.txt
  # How many documents state an `NChannel` colour space, and how many of those name a spot
  # colourant.
cargo run --release -p pdf-model --example numeric_form_census -- <file.pdf>…
  # Which of ISO 32000-2 §7.3.3's numeric forms real documents actually write, and what they write
  # instead.
cargo run --release -p pdf-model --example object_metadata_census -- --pdfjs
  # §14.3.2's *object-level* metadata: a `/Metadata` stream on something other than the catalog.
cargo run --release -p pdf-model --example oc_usage_census -- doc/pdf.js/test/pdfs/*.pdf
  # Which of Table 100's usage categories the corpus actually asks for.
cargo run --release -p pdf-model --example open_annotation_census -- doc/pdf.js/test/pdfs/*.pdf
  # §12.5.6.4's `/Open`, counted against §12.5.6.14's.
cargo run -p pdf-model --example open_one -- <file.pdf> [scale] [out.png]
  # Opens, interprets and rasterises one document, for isolating a pathological file.
cargo run --release -p pdf-model --example operator_shape_census
  # Two negatives whose witness is a *shape in a content stream* rather than a name anywhere.
cargo run --release -p pdf-model --example own_ink > before.tsv
  # This backend's **own** ink on every tracked corpus first page, one line per document.
cargo run --release -p pdf-model --example pattern_state_census -- doc/pdf.js/test/pdfs/*.pdf
  # When a shading pattern's colours are resolved, and how many documents can tell the difference.
cargo run --release -p pdf-model --example point_rectangle_census -- <file.pdf>…
  # How many annotations state a `/Rect` covering no area, and what their subtype clause states.
cargo run --release -p pdf-model --example print_preference_census
  # §12.2's print half of Table 147, counted: which documents ask for something on paper.
cargo run --release -p pdf-model --example push_button_census -- doc/pdf.js/test/pdfs/*.pdf
  # Which of Table 192's push-button icon and caption entries the corpus actually states.
cargo run --release -p pdf-model --example raster_digest -- doc/pdf.js/test/pdfs/*.pdf
  # A digest of every corpus document's first page **as pixels**, for proving a change to a
  # rasteriser drew nothing differently.
cargo run --release -p pdf-model --example readback -- file.pdf [page]
  # What one page reads back as, which is §9.10.2's answer for every code it showed.
cargo run --release -p pdf-model --example rectangular_path_census -- <dir-or-file>...
  # How many fills state their region as *several* axis-aligned rectangles, and how many of those
  # put two of them in one device pixel.
cargo run --release -p pdf-model --example refused_action_census -- \
    doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf
  # Table 201's twenty action types, counted over every document this tree can reach, and what this
  # reader answers for each.
cargo run --release -p pdf-model --example refused_segment_census
  # How many real first pages state ISO 32000-2 §8.5.2.1's error, asked of the *interpreter*.
cargo run --profile gates -p pdf-model --example replace_cost -- <file.pdf> <page> [runs]
  # What re-placing §12.5.3's annotations would cost, against re-interpreting the page.
cargo run --profile gates -p pdf-model --example replacement_census -- <directory>…
  # Whether re-placing §12.5.3's annotations produces the page a whole interpretation does, over
  # every document of a corpus rather than over the eight pages a test can afford.
cargo run --release -p pdf-model --example required_entry_census -- doc/pdf.js/test/pdfs/*.pdf
  # Which of ISO 32000-2 §8.9.5.1 Table 87's *required* entries each image `XObject` states, and
  # which state one malformed — the census behind `crate::image`'s refusals.
cargo run --release -p pdf-model --example reset_form_census -- doc/pdf.js/test/pdfs/*.pdf
  # Which of Table 241's two spellings of `/Fields` the corpus's reset-form actions use.
cargo run --release -p pdf-model --example shading_grid_census -- \
    doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf doc/corpora-own/*.pdf
  # What share of a function-based shading's grid a magnified window actually shows.
cargo run --release -p pdf-model --example singular_transform_census -- <dir-or-file>...
  # How many documents paint under a matrix that has no inverse, and what it costs them.
cargo run --release -p pdf-model --example standing_count_census -- <dir-or-file>...
  # How many documents state a page count this reader cannot produce a page for, and why.
cargo run --release -p pdf-model --example structure_destination_census -- \
    doc/pdf.js/test/pdfs/*.pdf
  # The `/SD` entry, counted where the standard states it: on §12.6.4.2's go-to action, and on
  # §12.3.2.4's named destination.
cargo run --release -p pdf-model --example structure_tree_census -- <file.pdf>…
  # What §14.7's logical structure a corpus actually states, and where it is stated.
cargo run --release -p pdf-model --example sub_pixel_width_census -- <file.pdf> [page]
  # What a page's sub-pixel strokes are, and what their caps are worth in ink.
cargo run --release -p pdf-model --example substitute_stretch_census -- doc/pdf.js/test/pdfs/*.pdf
  # How far the corpus's substituted faces are from the widths their documents state.
cargo run --release -p pdf-model --example table_header_census -- doc/pdf.js/test/pdfs/*.pdf
  # §14.8.5.7's table attributes, counted over a corpus.
cargo run --release -p pdf-model --example tiling_type_census -- doc/pdf.js/test/pdfs/*.pdf
  # Table 74's `/TilingType`: which code a corpus asks for, and what the lattice does about it.
cargo run --profile gates -p pdf-model --example token_window_census -- doc
  # How large a window a content stream needs: the biggest single token, and the inline images.
cargo run --release -p pdf-model --example transfer_function_census -- doc/pdf.js/test/pdfs/*.pdf
  # How many documents state §10.5's transfer function, how many state a real one, and how many
  # paint a **shading** under it.
cargo run --release -p pdf-model --example type4_operator_census -- \
    doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf doc/corpora-own/*.pdf
  # Which of Table 42's operators the corpora's §7.10.5 programs actually reach, and what the two
  # defects of the five-hundred-and-thirty-fourth session cost where they were reached.
cargo run --release -p pdf-model --example type4_type_census -- \
    doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf doc/corpora-own/*.pdf
  # What a typed operand stack costs the files that exist, rather than in principle.
cargo run --release -p pdf-model --example uncovered_share -- <file.pdf> <page> <index> [scale]...
  # How much of what a page draws early still shines through what it draws later.
cargo run --release -p pdf-model --example unknown_subtype_census -- doc/pdf.js/test/pdfs
  # Annotations whose `/Subtype` is outside Table 171, and whether each states an appearance.
cargo run --profile gates -p pdf-model --example unnamed_code_census -- doc/pdf.js/test/pdfs/*.pdf
  # What the codes ISO 32000-2 §9.10.2 cannot name are made of, by which method could have named
  # them.
cargo run --release -p pdf-model --example unnamed_field_census -- <file.pdf>…
  # How many documents state a form field dictionary with no `/T`, and where in the tree it is.
cargo run --release -p pdf-model --example unreached_content_census -- \
    $(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf') $(find -L doc/corpora -name '*.pdf') doc/*.pdf
  # How much of a tagged page's readback no structure element reaches — §14.8.2.2.2's artifact by
  # absence, measured before it is computed.
cargo run --profile gates -p pdf-model --example window_lexer_spike -- <pdf> 0 whole
  # Road D's sink, as an experiment: a `Lexer` fed from a fixed window instead of a `Vec`.
cargo run --release -p pdf-render --example area_bench -- [runs]
  # What [`pdf_render::Image::area_averaged`] costs, against the serial shape it replaced.
cargo run --release -p pdf-transform --example r1175_separation_pixels
  # What choosing one definition of an ink over another does to the page, on the corpus witnesses.
cargo run --release -p pdf-vfs --example faces_on_the_port
  # What `doc/todo/59`'s resource port costs and what it buys, measured on the documents that named
  # it.
cargo run --profile gates -p pdf-vfs --example vfs_cost -- [DOCUMENT ...]
  # What a mount costs: a worker per generation, a question in each transport, and the peak.
cargo run --release -p render-raster --example atlas_squeeze -- [file.pdf] [page]
  # What a small glyph atlas does to a page of text, frame after frame.
cargo run --release -p render-raster --example clip_cost -- bug1844576 issue16473
  # What a page's clips cost each backend in ink — the instrument that says whether a clip is
  # *cutting* a mark or a backend is *losing* one at it.
cargo run --release -p render-raster --example coverage_lattice -- endchar issue15150
  # Where a backend's partial coverages *land* — the instrument that says whether a rasteriser
  # states an edge on a lattice or anywhere the geometry puts it.
cargo run --release -p render-raster --example device_under_confinement
  # What a graphics device does when the process holding it is confined.
cargo run --release -p render-raster --example edge_coverage_ladder
  # What each backend paints at the *edge* of a shape wider than a pixel — ISO 32000-2 §10.7.4.
cargo run --release -p render-raster --example encode_threads_across_the_seam -- \
    <file.pdf> [page] [scale] [1,2,4,…]
  # How many threads this host should let raster's geometry phase use, on *this* machine.
cargo run --release -p render-raster --example frame_race
  # The number `RENDER_LIBRARY.md` section 6.2 judges everything by: wall-clock `rasterize` time on
  # the dense page, all three backends, same display list, same target.
cargo run --release -p render-raster --example function_paint_census -- \
    doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf doc/corpora-own/*.pdf
  # How many of the type 1 shadings that exist a device will evaluate, and why not the rest.
cargo run --release -p render-raster --example image_phase
  # Whether each backend filters an image drawn at **exactly one device pixel per sample** — and
  # §10.7.4 says which answer is right: the centre of each device pixel is mapped back into source
  # space to decide its colour, and there is no averaging over the pixel area.
cargo run --release -p render-raster --example image_residual
  # `doc/QUORRA_FEEDBACK.md` section 46's residual reduction drawn without the page — and the
  # measurement that **excludes** it as the cause section 46 named.
cargo run --release -p render-raster --example lane_diff -- bug1743245 issue16500
  # One page through both of raster's coverage lanes, beside the CPU oracle.
cargo run --release -p render-raster --example mark_width
  # The instrument behind `doc/QUORRA_HAIRLINE_MARKS.md`: how many pixel rows a §10.7.4 mark
  # occupies, and how much ink it carries, per backend and per scale.
cargo run --release -p render-raster --example mitre_ladder
  # What each backend draws for a mitre at each ratio ISO 32000-2 §8.4.3.5 admits.
cargo run --release -p render-raster --example outline_stability -- \
    [file.pdf] [page] [scale] [atlas budget in bytes]
  # Are the resource identifiers this backend hands raster the same from one render of one display
  # list to the next?
cargo run --release -p render-raster --example quantum_diff
  # The measurement behind `tests/real_pages.rs`'s gates: the glyph-phase quantum's cost against the
  # CPU oracle on a real text page, per `RENDER_LIBRARY.md` section 4.5's "measured, never assumed".
cargo run --release -p render-raster --example rect_and_residue_census
  # Two facts about this corpus in one walk: how many of its fills are axis-aligned rectangles, and
  # what its clip chains cost the device.
cargo run --release -p render-raster --example sub_pixel_marks
  # What each backend does with a mark thinner than a pixel — the two numbers `doc/todo/11` called
  # *unmeasured*, and then the instrument that closed them.
cargo run --release -p render-raster --example viewport_refusal -- <file.pdf> [page] [scale] [w] [h]
  # Whether a page the 4× gate refuses is also refused by the *product*, whose target is a window.
cargo run --release -p viewer-confined --example confined_cancel -- [levels] [--finish] [--marks]
  # A hostile document, and a host taking its thread back — from **either** of the two places the
  # drawing can happen.
cargo run --release -p viewer-confined --example confined_page -- file.pdf [page] [out.png]
  # A page drawn in a confined process, with what each step of it cost.
cargo run --release -p viewer-confined --example confined_panels -- file.pdf [page]
  # A sidebar's worth of a document, read out of a process that cannot open a file.
cargo run --release -p viewer-confined --example confined_peak -- file.pdf [more.pdf …]
  # What a confined viewer's address space peaks at, against the ceiling it was given.
cargo run --release -p viewer-confined --example host_draw -- [--scale N] [--levels K] <file.pdf>…
  # What a page's marks cost the **host** that draws them, and whether anything predicts it.
cargo run --release -p viewer-confined --example list_against_raster -- [--scale N] <file.pdf>…
  # What a display list would cost to send, against what its raster costs.
cargo run --release -p viewer-core --example accessibility_cost -- \
    file.pdf [page] [repeats] [column]
  # What `Query::AccessibilityTree` costs, on the document whose size makes it a question.
cargo run --profile gates -p viewer-core --example find_cost -- \
    file.pdf needle [repeats] [split] [pages]
  # What a document-wide search costs, measured in `find_step` alone rather than in a host's loop.
cargo run --profile gates -p viewer-core --example zoom_cost -- <file.pdf> [ticks]
  # What one wheel notch costs the event thread, with and without §12.5.3's re-interpretation.
cargo run --release -p raster-gpu --example encode_threads [-- <adapter substring> [rounds] [thread counts, comma separated]]
  # What `encode: geometry` costs, and what more than one thread does to it.
cargo run --release -p raster-gpu --example function_compile [-- <adapter substring> [rounds]]
  # What a §7.10.5 program's generated shader costs to compile, by the program's length (ADR 0053).
cargo run --release -p raster-gpu --example function_paint
  # A feasibility spike: is a §7.10.5 function a paint the device should evaluate?
cargo run --release -p raster-gpu --example lane_placement
  # **Where each coverage lane puts a hairline, and what the sampled grid does to its ink.**.
cargo run --release -p raster-gpu --example outline_upload [-- <adapter substring> [rounds]]
  # What an outline upload costs, and where the GPU lane's conversion went (ADR 0075).
cargo run -p raster-gpu --example present_thread
  # **The proof of ADR 0056**: a page rendered on one thread while the window is presented from
  # another, and the pixels read back to show where the affine put them.
cargo run --release -p raster-gpu --example rect_lane
  # The caller's section 19 measurement: what does a `Rect` command save over a `Fill` of the same
  # four-edge outline?
cargo run --release -p raster-gpu --example residue_clip [-- <adapter substring> [rounds]]
  # What a page of curve-clipped marks costs to encode (ADR 0049).
cargo run --release -p raster-gpu --example retained [-- <adapter substring> [rounds]]
  # What an unchanged frame costs when it does not encode itself again (ADR 0048).
cargo run --release -p raster-gpu --example startup -- <adapter substring>
  # What bring-up costs, one step at a time — the brief section 7 measurement, attributable.
cargo run --release -p raster-gpu --example surface_measure
  # The two surface-tier numbers only a person at the real GPU can take.
DISPLAY=:77 cargo run -p raster-gpu --example window_smoke   # under Xvfb :77
  # The surface-path smoke test: a real window, real presents, and pixels a person (or `xwd`) can
  # look at.
```
