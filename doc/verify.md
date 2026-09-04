# Verify it — every instrument in this tree, and when to run it

Status: **standing** — the catalogue. `doc/todo/02-every-round.md` §2 is the round's own gate
sequence and **owns those commands**; this file owns everything a round runs when it has a reason
to, which is most of what is here.

Read by: whoever needs `cargo deny`, a fuzz target, a callgrind counter, a cross-target check, a
census example or the AT-SPI recipe. `doc/HANDOVER.md`'s reading table is the pointer to this file.

**The gate sequence is not repeated here, deliberately.** Two documents stating the same commands
is how they drift apart, and they had: this list said 1369 tests where the gate printed 1371, and
omitted `render-quorra`'s corpus gate altogether. `doc/todo/02` §2 is the one copy.

**Nothing here runs in a fresh clone until the specifications are unpacked**, which is one command
and is in `doc/environment.md`.

```sh
cargo run -p conformance --bin ledger      # regenerates rows, keeps every status
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
# And the Windows *read path* runs here, which is the only way to test it from Linux: the two
# implementations are chosen by `#[cfg(unix)]` / `#[cfg(not(unix))]`, so rewriting those two
# attributes compiles the thread-and-channel one on this machine. ADR 0194 has the recipe; all 19
# sandbox tests and the whole corpus gate pass through it.
cargo build --profile gates -p viewer-confined --bins   # trap 10: the line below runs one test
tools/bounded.sh --data 12 --tree 12 -- \
  cargo test --profile gates -p viewer-confined --test awkward_classes -- --ignored --nocapture
  # **the other confined program, over the same population as `pdf-vfs`'s read walk** (ADR 0879):
  # a document of each of `corpus-classes`'s ten classes, from every corpus root on this disk,
  # opened as a descriptor and drawn through `pdf-view-worker`, three page turns apiece. What
  # fails it is a **death** — `killed by signal N` — because a system call the filter forbids costs
  # this program the page a person is reading, where it costs a mount one generated file. Not a
  # `doc/todo/02` §2 gate: `pdf-vfs`'s read walk gates the same class of defect every round over a
  # wider set of *questions*, and this is the run a round that touches the confinement, the
  # interpreter's dependencies or `pdf-sandbox`'s allow-list owes. 198 documents in 13.5 s; with
  # `no_machine_fonts()` taken out of `viewer_confined::worker::confine` it reports 28 deaths in
  # six of the ten classes, which is how it was shown to be able to fail (trap 13)
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
cargo run --release -p render-quorra --example zoom_ladder -- [file.pdf] [page] [out-dir]
  # the two backends compared up a ladder of magnifications and back down, through one device,
  # switching coverage lanes where `viewer-ui` does. `doc/QUORRA_FEEDBACK.md` §11
PDFVIEWER_QUORRA_COVERAGE=gpu PDFVIEWER_QUORRA_SCALE=4 \
  cargo test --profile gates -p render-quorra --test corpus -- --ignored --nocapture
  # §2's quorra gate pointed at the *other* coverage lane — the one `viewer-ui` switches to past
  # ten times magnification, and the one no gate had ever run over the corpus. Either knob turns
  # the ratchets off and the run says so; a value that is neither `cpu` nor `gpu` is a panic
  # rather than a silent default. `PDFVIEWER_QUORRA_SCALE=4` is the interesting pairing, because
  # the lane exists for magnification: ADR 0283 took its refusals from 36 to 12 there.
FIRST_FRAME_COVERAGE=gpu cargo run --release -p render-quorra --example first_frame -- [page] [scale]
  # what the first frame costs that the tenth does not, on either lane (ADRs 0179, 0283)
cargo run --release -p viewer-gtk --example outline_census -- [file.pdf]
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
cargo run --release -p pdf-model --example group_blit_census -- doc/pdf.js/test/pdfs/*.pdf
  # what each first page's transparency groups would cost to composite, in blitted pixels, and it
  # is `pdf_render::group_blit_demand` itself rather than a second reading of the same idea — the
  # instrument that sized `MAX_GROUP_BLIT_PIXELS` (ADR 0780). **It interprets and never
  # rasterises**, which is what lets it be run over a population holding `poppler-978-0.pdf`, whose
  # 73 047 page-spanning groups take some 640 s to draw and 2.5 s to interpret. Run over a corpus
  # in several processes with `xargs -0 -n 40 -P 8`; the summaries add
cargo run --release -p render-quorra --example filtered_edge_colour
  # what each of the three backends' image filters does to the *colour* of a partly transparent
  # sample — §8.9.6.2's "smooth the edges of the mask, not … the painted colour values", which is
  # the difference between filtering premultiplied and filtering straight. One scene, because it is
  # the only shape that separates them: every image in both cross-backend suites is opaque, and on
  # an opaque raster the two arithmetics agree. The CPU backend and vello depart from the painted
  # colour by 0 and quorra by 131 of 255, which is the shipped rasteriser (ADR 0697,
  # `doc/todo/55`, `doc/QUORRA_FEEDBACK.md` §39)
cargo run --release -p render-quorra --example sampled_lane_column -- [--scale N] doc/pdf.js/test/pdfs/*.pdf
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
cargo run --release -p pdf-model --example signature_algorithm_census -- @/tmp/paths
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
cargo run --release -p pdf-model --example press_census -- <dir>/*.pdf    # one process per archive
  # which press §11.4.7 gives a page and whether `crate::icc` can evaluate the profile behind it —
  # its `A2B` out, and since ADR 0796 the `B2A` §8.6.5.5 requires of a blending-space profile,
  # counted as carried and as evaluated, side by side, so that an encoding this tree does not
  # read is a number rather than a silence. It shares nothing between documents, so its answer is a function of the files and two runs are
  # byte-identical — which is what `tools/safedocs survey` could not say while the press table was
  # a process-wide budget (ADR 0416). `--sample` measures what a grid of a given side departs from
  # evaluating the profile, which is what `PRESS_SIDE` is answerable to (ADR 0272)
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
cargo run --release -p pdf-model     --example open_cost -- [file.pdf]
  # where the *launch path's* document half goes: §7.5's xref, the page tree, §12.3.3's outline,
  # §12.8's signatures, each on its own. ADR 0179, doc/todo/42
DISPLAY=:77 target/pdf-viewer-gtk --trace=launch,frames [file.pdf]   # and where a *native* host's
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
cargo run --release -p render-quorra --example bring_up  -- [all|vulkan|gl]
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
# **`fuzz/corpus` and `fuzz/artifacts` are gitignored, so whether a target is seeded is a fact
# about this disk and not about the repository** — no gate can read it out of the tree, which is
# why the wrapper asks the directory. Two consequences a round meets. A **fresh worktree had
# neither directory at all** until `tools/worktree.sh` was taught to link them, so every fuzz run a
# parallel round made started from nothing and said nothing about it. And a **clone has to
# re-seed**, from the scripts in `fuzz/` and the recipes under each target below; a corpus is not
# recoverable from the history because it was never in it.
tools/fuzz.sh lexer                               # or, without the two questions, by hand:
cd fuzz && cargo +nightly fuzz run lexer         -- -runs=50000   # needs nightly
cd fuzz && cargo +nightly fuzz run cmap          -- -runs=50000   # §9.7's CMap parser
cd fuzz && cargo +nightly fuzz run crypt         -- -runs=50000   # §7.6's algorithms
cd fuzz && cargo +nightly fuzz run variable_text -- -runs=50000   # §12.7.4.3's /DA and layout
cd fuzz && cargo +nightly fuzz run forms_data    -- -runs=50000   # §12.7.8's FDF, §7.9.4's dates
cd fuzz && cargo +nightly fuzz run object        -- -runs=50000   # §7.3's object grammar
cd fuzz && cargo +nightly fuzz run document      -- -runs=50000   # §7.5's file structure
cd fuzz && cargo +nightly fuzz run page -- -runs=50000 -fork=6 -rss_limit_mb=4096 -timeout=60
  # **clauses 8, 9 and 11** — a whole document through `pdf_model::interpret`, which nothing
  # reached until the four-hundred-and-twenty-eighth: `nm` finds `pdf_model::interpret` in one of
  # the other thirteen binaries and it calls it on a page with no `/Resources` (ADR 0264).
  # **Seed its corpus first**, and from real documents, because libFuzzer will not invent a header,
  # a page tree, a content stream and a resource dictionary that agree with each other:
  #   find corpus-cache/safedocs doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \
  #     | xargs -0 python3 fuzz/seed_page.py fuzz/corpus/page
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
  # libFuzzer called 15 s is 0.8 s in `target/pdf-retrieve`, which is ASan, the debug assertions
  # and six forks sharing 24 cores.
cd fuzz && cargo +nightly fuzz run xmp           -- -runs=50000   # §14.3.2's XMP, the tree's
  # only XML. Its corpus is seeded with all 318 packets the pdf.js documents decode to
cd fuzz && cargo +nightly fuzz run sfnt          -- -runs=50000   # §9.6.3's two glyph-table repairs
  # **seed its corpus with real fonts** — 60 `/FontFile2` streams out of `doc/pdf.js/test/pdfs/`
  # into `fuzz/corpus/sfnt/`. Unseeded it never forms a table directory and tests nothing; seeded
  # it produced two crashers in its first minute (ADR 0175)
# §14.7's tree on a real accessibility bus, which is the only way to check the AccessKit bridge
# end to end from here. A session bus, at-spi's own bus and registry, Xvfb, and `busctl` walking
# `org.a11y.atspi.Accessible` from the registry root — a real client rather than this program's
# own types. ADR 0214 has the script; the shape is:
#   dbus-run-session -- bash -c '/usr/lib/at-spi-bus-launcher --launch-immediately & sleep 3
#     ADDR=$(busctl --user --json=short call org.a11y.Bus /org/a11y/bus org.a11y.Bus GetAddress …)
#     DISPLAY=:99 AT_SPI_BUS_ADDRESS=$ADDR /usr/lib/at-spi2-registryd & sleep 2
#     DISPLAY=:99 pdf-viewer doc/PDF20_AN001-BPC.pdf & sleep 6
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
# **And since ADR 0623 the recipe applies to all three windows** — `./target/pdf-viewer`,
# `./target/pdf-viewer-gtk` and `./target/pdf-viewer-qt` each publish §14.7's tree, so the same walk
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
# `pdf-viewer`, `pdf-viewer-gtk` and `pdf-viewer-qt`. **Read `GetState` back after *each*
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
  # `pdf_model::der`'s X.690 reader and `pdf_model::cms`'s RFC 5652 SignedData, the tree's only
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
cd fuzz && cargo +nightly fuzz run x509         -- -runs=1000000  # the signer's certificate and
  # the verifications that run on the key inside it: `pdf_model::x509` walks RFC 5280's
  # structure and `pdf_model::pkcs1`, `pdf_model::pss` and `pdf_model::dsa` run the tree's only
  # loops whose trip counts come out of numbers in the file. Since the six-hundred-and-eighty-ninth
  # it also reaches `pdf_model::ecdsa` and `pdf_model::eddsa`, whose arms assert the same thing on
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
  # own and which are found by RFC 5280 §4.1's opening bytes in the file and in its inflated
  # streams; and the hexadecimal in `crates/pdf-model/src/{x509,dsa,pss,ecdsa,eddsa}.rs`'s
  # `fixtures` modules. **That third route is what makes a clone's corpus complete**: the DSA
  # certificate is the only input that reaches `dsa::verify`, and the P-384, P-521,
  # brainpoolP256r1 and Ed25519 ones are the only inputs that reach the arms added in the
  # six-hundred-and-eighty-ninth session — until this route existed, this line asked a round to
  # re-make them with `openssl req -new -x509` by hand. The module documentation still has those
  # invocations, and any certificate at all is a legal input.
  # **RFC 5280 §5.1's `CertificateList` is the near miss to know about**: a revocation list has
  # this same three-member shape, satisfies §4.1.1.2's rule that the two algorithm identifiers
  # agree, and sits in `/CRLs` immediately beside `/Certs` — so the second route reads as far as
  # `Validity`, where a certificate states two `Time`s and a revocation list one.
  # Clean at 1 000 000 in the three-hundred-and-ninety-second (ADR 0229)
```

**Two measurements that are not gates, and each says why in its own header.**

```sh
# What it would cost to check this project's citations against the PDF instead of `doc/md/`.
# `doc/todo/48`'s item 5 and `doc/todo/36`'s success condition, with a number instead of a
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
`--release --bin pdf-viewer`. The question a round asks is whether the gate is measuring the
program a user gets.

It is answerable exactly, in about a minute, and the answer decays — so what is written down is the
command:

```sh
cargo +nightly test  --profile gates -p viewer-core --test accessibility_census \
                     --unit-graph -Z unstable-options > subset.json
cargo +nightly test  --workspace --profile gates  --unit-graph -Z unstable-options > workspace.json
cargo +nightly build --release --bin pdf-viewer   --unit-graph -Z unstable-options > shipped.json
```

Each unit in that JSON carries `pkg_id`, `mode`, `target.kind` and `features`. Take the transitive
closure of the root you care about — the unit whose `target.name` is `accessibility_census` and
`kind` is `["test"]`, or `pdf-viewer`/`["bin"]` — and compare `(package, mode, kind) → features`
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
