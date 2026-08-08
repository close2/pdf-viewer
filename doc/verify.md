# Verify it — every instrument in this tree, and when to run it

Status: **standing** — the catalogue. `doc/todo/02-every-round.md` §2 is the round's own gate
sequence and **owns those commands**; this file owns everything a round runs when it has a reason
to, which is most of what is here.

Read by: whoever needs `cargo deny`, a fuzz target, a callgrind counter, a cross-target check, a
census example or the AT-SPI recipe. `doc/HANDOVER.md`'s "Verify it" is the pointer to this file.

**The gate sequence is not repeated here, deliberately.** Two documents stating the same commands
is how they drift apart, and they had: this list said 1369 tests where the gate printed 1371, and
omitted `render-quorra`'s corpus gate altogether. `doc/todo/02` §2 is the one copy.

**Nothing here runs in a fresh clone until the specifications are unpacked**, which is one command
and is in `doc/HANDOVER.md`'s "What to do next".

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
  # `--all-targets` rather than `--bins` since the three-hundred-and-eighty-fourth: `DEFAULT_BACKEND`
  # is a `#[cfg(windows)]` constant and the test that states it is DX12 lives in the binary's own
  # `mod tests`, which `--bins` does not build. `-p viewer-ui` has no `criterion` in its dev tree,
  # so this one does cross-compile where `--workspace --all-targets` does not.
  # `--workspace --all-targets` does *not* cross-compile here: `criterion` pulls `alloca`, whose
  # build script needs a C toolchain for the target and this machine has neither MSVC nor macOS's.
  # The CI runners do, which is why the `platforms` job builds the binaries and these two check
  # what a benchmark does not reach.
  # **`viewer-gtk` is deliberately in none of these**, and asking for it says why: `glib-sys`'s
  # build script needs a cross-compiling `pkg-config` wrapper, and the target would need GTK 4's
  # own development files, which is a platform package manager's job rather than a Rust target's.
  # A host binds a platform and is checked on it — the same rule that makes `viewer-accessibility`
  # Linux-only in its own manifest (ADRs 0214, 0244).
# And the Windows *read path* runs here, which is the only way to test it from Linux: the two
# implementations are chosen by `#[cfg(unix)]` / `#[cfg(not(unix))]`, so rewriting those two
# attributes compiles the thread-and-channel one on this machine. ADR 0194 has the recipe; all 19
# sandbox tests and the whole corpus gate pass through it.
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
  # (ADR 0236). A profile cannot see either number — it counts commands, not what a command covers
cargo run --release -p pdf-model --example field_flag_census -- doc/pdf.js/test/pdfs/*.pdf
  # which of §12.7's twenty field flags any real document states (ADR 0197)
cargo run --release -p pdf-model --example variable_text_census -- doc/pdf.js/test/pdfs/*.pdf
  # what §12.7.4.3 actually lays text out for, which the two censuses above cannot say: 622 widgets
  # of a text field or a combo box, 305 with no /AP /N stream, and 73 of §12.5.6.6's free text
  # annotations — with each one's /DA font classified by what its descriptor says about a baseline,
  # and §12.7.5.4's list boxes counted beside them. `font_metric_census` counts the fonts a *page*
  # draws with, which is a different population and was mistaken for this one (ADR 0240)
cargo run --release -p pdf-model --example presentation_census -- doc/pdf.js/test/pdfs/*.pdf doc/*.pdf
  # what any real document says about §12.4.4: 978 opened, 1971 pages walked, and **not one**
  # states a /Trans, a /Dur or a /PresSteps — asked of the page tree rather than of the raw bytes,
  # so a /Trans inside an object stream would have counted. `--example presentation_fixture` writes
  # the three-slide document that therefore has to stand in for one (ADR 0230)
cargo run --release -p pdf-model --example luminosity_mask_census -- doc/pdf.js/test/pdfs/*.pdf
  # what a §11.5.3 mask group is painted *with*, against what its /CS declares — 87 groups on
  # this corpus, 39 blending in /DeviceCMYK and 36 in /DeviceGray, and not one setting a `k`
  # colour, which is what turned a report's condition into the departure itself (ADR 0217)
cargo run --release -p render-gpu --example frame_split -- [file.pdf] [page] [scale]
  # where a GPU frame's time goes: encoding, the whole frame, and the same target drawn from a
  # list of one rectangle. doc/RENDER_LIBRARY.md §6.1
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_open [file.pdf]  # §7.5's xref alone, in instructions rather
  # than in a wall clock that moves by 2× between runs of the same binary. ADR 0180
cargo run --release -p pdf-model     --example open_cost -- [file.pdf]
  # where the *launch path's* document half goes: §7.5's xref, the page tree, §12.3.3's outline,
  # §12.8's signatures, each on its own. ADR 0179, doc/todo/42
cargo run --release -p render-quorra --example bring_up  -- [all|vulkan|gl]
  # and where its device half goes: instance, adapter, device — one measurement per process,
  # because a second instance in one process is measured with the loader already warm
cargo build --release -p hayro-compare --bins && \
  cargo run --release -p hayro-compare --bin hayro-speed -- doc/pdf.js/test/pdfs/*.pdf   # ~45 min
cargo run --release -p hayro-compare --bin hayro-speed -- --per-document ...  # one line per file,
  # which is how a renderer that is a *program* rather than a crate is joined to the table (ADR 0136)
cd fuzz && cargo +nightly fuzz run lexer         -- -runs=50000   # needs nightly
cd fuzz && cargo +nightly fuzz run cmap          -- -runs=50000   # §9.7's CMap parser
cd fuzz && cargo +nightly fuzz run crypt         -- -runs=50000   # §7.6's algorithms
cd fuzz && cargo +nightly fuzz run variable_text -- -runs=50000   # §12.7.4.3's /DA and layout
cd fuzz && cargo +nightly fuzz run forms_data    -- -runs=50000   # §12.7.8's FDF, §7.9.4's dates
cd fuzz && cargo +nightly fuzz run object        -- -runs=50000   # §7.3's object grammar
cd fuzz && cargo +nightly fuzz run document      -- -runs=50000   # §7.5's file structure
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
#     AT_SPI_BUS_ADDRESS=$ADDR /usr/lib/at-spi2-registryd & sleep 2
#     DISPLAY=:99 pdf-viewer doc/PDF20_AN001-BPC.pdf & sleep 6
#     busctl --address=$ADDR call org.a11y.atspi.Registry /org/a11y/atspi/accessible/root \
#       org.a11y.atspi.Accessible GetChildren'
# `org.a11y.Status IsEnabled` is already true here; where it is not, set it before the viewer
# starts or every adapter stays inactive by design. **Orca is not installed on this machine**, so
# what a person on a desktop still has to do is run one and listen.
cd fuzz && cargo +nightly fuzz run fragment      -- -runs=50000   # Annex O's fragment identifier,
  # and the only untrusted input here that no document carries: it arrives with the request
cd fuzz && cargo +nightly fuzz run confined_wire -- -runs=1000000 -rss_limit_mb=1024
  # the confined viewer's four decoders (ADR 0223). The one target whose input is not a document
  # but a *process*: `pdf-view-worker` runs hostile files behind seccomp and writes its answers to
  # a host that is not confined. **Seed its corpus first**, with `fuzz/seed_confined_wire.py` —
  # a second implementation of the frame layer, which spawns the release worker and keeps every
  # payload it wrote: 83 of them from five documents and all 25 questions. Unseeded it never gets
  # past a one-byte discriminant into an outline's tree or a thumbnail's samples:
  #   cargo build --release -p viewer-confined --bins
  #   python3 fuzz/seed_confined_wire.py target/pdf-view-worker fuzz/corpus/confined_wire \
  #     doc/PDF20_AN002-AF.pdf doc/PDF-Declarations.pdf doc/ISO_32000-2_sponsored_EC3.pdf \
  #     doc/PDF20_AN001-BPC.pdf doc/pdf.js/test/pdfs/issue15716.pdf
cd fuzz && cargo +nightly fuzz run cms          -- -runs=50000   # §12.8.3.3's signature value:
  # `pdf_model::der`'s X.690 reader and `pdf_model::cms`'s RFC 5652 SignedData, the tree's only
  # ASN.1. **Seed its corpus** with the eleven `/Contents` blobs the nine signed corpus documents
  # hold; four of them are indefinite-length BER, which is the shape a from-scratch input never
  # forms. Clean at 1 000 000 in the three-hundred-and-seventy-seventh (ADR 0215) and again in the
  # three-hundred-and-ninety-second, after its `SignerInfo` gained a signature and an identifier
cd fuzz && cargo +nightly fuzz run x509         -- -runs=1000000  # the signer's certificate and
  # the RSA verification over the key inside it: `pdf_model::x509` walks RFC 5280's structure and
  # `pdf_model::pkcs1` runs the tree's only loop whose trip count comes out of a number in the
  # file. The property that matters is the last one — the target verifies against a digest *it*
  # chose, so `Ok(true)` would be a defect in the comparison rather than a lucky input.
  # **Seed its corpus** with the 22 certificates the corpus's signatures carry:
  #   python3 fuzz/seed_x509.py fuzz/corpus/x509 doc/pdf.js/test/pdfs/*.pdf
  # plus any certificate at all — the round's two vectors are an RSA and a P-256 one from
  # `openssl req -new -x509`. Clean at 1 000 000 in the three-hundred-and-ninety-second (ADR 0229)
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
reference renders; every run after it is ~30 s. **Read the printed hit rate rather than the
clock** — and it is the tell for something outside this tree: session 166 saw it at 85.7% on an
unchanged corpus, which was `poppler` being upgraded on this machine and every cached
`pdftoppm` render becoming a new key. Nothing about the verdicts moved. Of that ~30 s, ~23 s is the three external renderers at a 99.7% hit rate; the rest is
ours — roughly 600 s of processor time over 24 cores on our own render, the comparison and the
artefacts. **If 30 s ever becomes the constraint, that is where to look and not at the
subprocesses.** `PDFREF_CACHE=off` asks the three renderers again — how "the cache changes no verdict" is
re-checked; `PDFVIEWER_ORACLE_ONLY=a,b` compares only matching pages in 0.2 s and refuses to check
the ratchets, saying so. `PDFVIEWER_CORPUS_TRACE=1` names each document as it starts, which is how
a hang is identified from a killed run.

Cargo prints one line about `proc-macro-error2` being rejected by a future compiler. It arrives
through `iai-callgrind`, a dev-dependency reaching no shipped binary, and `deny.toml` records the
exception. Nothing to chase.

**`doc/pdf.js` is a submodule** (Apache-2.0, pinned at v6.1.200) holding the 974 PDFs. Optional to
clone — every test using it reports being skipped — but the ratchets mean nothing without it, so
CI must have it. **The time budget reports; it cannot enforce**: a Rust thread cannot be
cancelled, so a document that never returns hangs the suite rather than failing it.
