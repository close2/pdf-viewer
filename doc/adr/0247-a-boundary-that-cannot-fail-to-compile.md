# ADR 0247 — A boundary that cannot fail to compile

Status: accepted, 2026-08-09 (session 411).

## Context

`doc/todo/30`'s order is the project owner's: **GTK4 first**, **Qt second**, **`viewer-ffi` last**,
under one sentence —

> Do not freeze a C ABI until two Rust consumers have shaken the API out.

Both landed (ADRs 0244, 0246) and neither added a message. The condition is met, and the file named
**three amendments to take first**, all of one shape: a place where the boundary relies on a *Rust*
compiler check that a C caller cannot get. This round takes those three, and then builds the ABI.

The whole subject of this ADR is one sentence: **a C caller cannot fail to compile.** Everything
`viewer-core` gets from leaving its enums exhaustive — *"a new `Event` should fail to compile in
every consumer"* — is unavailable across a C boundary, and what replaces it has to be argued rather
than assumed. A C ABI is also the one mistake that cannot be taken back, which is why the order put
it last.

---

## Part one — the three amendments

### Amendment 1 — `pdf_render::RasterFormat` stops being `#[non_exhaustive]`

**Taken.** One attribute removed, four consumers simplified.

The rule is `doc/ui-boundary.md`'s and it is about `viewer-core`'s *vocabulary*. `Raster` crosses
that vocabulary inside `Rendered::Raster` and `Answer::Frame`, so its `format` field is part of the
vocabulary whichever crate declares the enum — and it was `#[non_exhaustive]`, which buys the
opposite of the rule. What that cost, counted:

| consumer | what it had to write |
|---|---|
| `viewer-gtk` | a catch-all arm and a `PixelError::UnknownFormat` variant |
| `viewer-qt` | the same arm and the same variant, written a second time |
| `viewer-ui` | `SoftwareError::Format(RasterFormat)` and two `!=` checks on the composite path |
| `viewer-confined` | an `Uncarried` refusal on the *encode* side of the wire format |

Four runtime refusals for a condition no build could reach, and a fifth consumer — a C ABI — that
could not have written even the refusal. All four are now an **exhaustive `match` with one arm**,
which compiles to nothing and fails to build the day a second layout arrives. `viewer-confined`'s
*decode* side keeps its refusal, because a byte arriving from the confined process is a claim rather
than a variant.

**What it costs is stated rather than hidden**: adding a second pixel layout to `pdf-render` is now
a breaking change for every consumer of that crate. That is the intended price. A raster's bytes
mean something, and a consumer that has not been told what they mean must not composite them.

### Amendment 2 — `Answer::Outline` is owned

**Taken.** `Answer::Outline(&'a Outline)` → `Answer::Outline(Outline)`.

ADR 0244 recorded the asymmetry and said no change was owed, because "the borrow costs the host one
clone it was going to make". ADR 0246 tested that and found it *worse* on Qt, where
`QAbstractItemModel` admits no laziness. This round counted the consumers, and the answer is that
the borrow saved nobody anything: **all five clone it** — `viewer-ui`'s sidebar, `viewer-gtk`'s
`GtkTreeListModel`, `viewer-qt`'s flattened `Vec<QtRow>`, `viewer-confined`'s pipe, and the C ABI
which has to own it by construction. A panel outlives the query that filled it.

Measured rather than asserted, by `viewer-host --example outline_census`, median of five:

| document | rows | `Query::Outline`, whole answer |
|---|---|---|
| `PDF20_AN001-BPC.pdf` | 14 | **481 ns** |
| `ISO_32000-2_sponsored_EC3.pdf` | 988 | **80.7 µs** |

Against ADR 0246's 3.66 ms to build the three panel models those 988 rows go into, the clone is 2%
of what a host does with it. Three call sites got shorter; one doc comment in `viewer-ui` lost the
reason it gave and kept the better one it also gave.

### Amendment 3 — a field's value says whether it *is* the field's value

**Taken, and it turned out to be a live bug rather than a documentation gap.**

`doc/todo/30` said the fix was "one sentence in a doc comment". It is not, and the reason is what
the round found. ADR 0201's rule has a host write a field's value back into its control after every
keystroke, because §12.7.5.3's `DoNotScroll` means the field can take less than was typed. Table 231
bit 14's field answers with bullets — *"[c]haracters typed from the keyboard shall instead be echoed
in some unreadable form, such as asterisks or bullet characters"* — so a host that followed the rule
would send the bullets as the next value.

**`viewer-ui` did exactly that.** It is a tier-2 host with no native secure entry, its `aimed`
function read the value back for every keystroke, and nothing in it had ever mentioned Table 231
bit 14: typing `a` into a password field holding `secret` produced `Edit::SetField` with `••••••a`.
The two native hosts ship the exception because each was written *after* reading two doc comments
and noticing they interact; the host that predates both shipped the bug. That is the strongest
possible argument that a doc sentence was the wrong fix.

So the **variant changed shape**, which is the precedent ADR 0167 set and which nothing being
`#[non_exhaustive]` exists to make cheap:

```rust
Answer::Field { name, value: Option<pdf_model::view::ShownValue> }
```

`ShownValue` is `{ text: String, obscured: bool }`, produced by the *same* reading that makes the
bullets, so the flag cannot disagree with the string it describes. `Answer::Fields`'s `FormField`
carries the same type, so a host cannot learn the exception from one question and miss it in the
other. Every consumer failed to compile, which is what the shape change is for, and each was fixed
where it stood:

- **`viewer-ui`** refuses to put the keyboard in a password field at all, and says so by name. It
  draws its own page and has no secure control to type into; a host that cannot read a value back
  cannot obey ADR 0201, and offering the field anyway would be worse than declining it (trap 5).
- **`viewer-gtk`**'s `write_back` refuses on `shown.obscured` rather than on `ControlKind::Entry {
  password: false, .. }`, which was the host inferring from a control's kind what the answer now
  states.
- **`viewer-qt`** carries `obscured` across the `cxx` bridge and `window.cpp` guards the write-back
  switch with it, instead of leaving the password entry out of the switch by hand.

### And the clause had a second sentence nobody had read

`doc/todo/02` §1 asks a round to take from both tracks, and reading §12.7.5.3 against the code for
amendment 3 found this, three lines below the sentence that caused it:

> NOTE To protect password confidentiality, it is imperative that PDF processors never store the
> value of the text field in the PDF file if this flag is set.

**This tree stored it.** `ViewState::save` wrote Table 226's `/V` for every edited field, so a person
who typed a password into a form and saved got their password in the file in clear text. A NOTE is
informative and this one is obeyed anyway, because the alternative is that.

`ViewState::save` now returns `Written { bytes, withheld }` and writes **neither** the value nor the
appearance for such a field — neither half, so the producer's `/V` and the producer's `/AP` stay as
they were and go on agreeing, where writing one without the other would leave a widget drawing
something §12.7.2 says its `/V` should decide. `withheld` names each field, and `viewer-core` turns
it into `Event::Reported`, because a save that quietly dropped what a person typed is the silence
this project exists against. `a_password_fields_typed_value_is_not_written_into_the_file` searches
the saved **bytes** for the characters rather than reading the value back — reading it back answers
bullets whether or not the secret reached the file, which would have made the test pass while the
bug stood.

---

## Part two — the ABI

`crates/viewer-ffi`: one crate, **39 entry points**, `cdylib` + `staticlib` + `rlib`, a hand-written
header, and a C program that drives it. The four questions `doc/todo/30` asks, each with what it
costs a compiled C caller.

### 1. How does a C caller send a command? — one function each

**Decided: a function per command, never a tagged union.**

The alternative is one `pdfv_command` struct with a tag and a union, which is how a message-passing
API is usually spelled. It is wrong here for a reason entirely about C: **a union's size is part of
the ABI.** A command added later changes the size of a type every caller has already compiled, and
an old caller passing an old-sized struct to a new library is undefined behaviour no diagnostic
catches. A function added later is a symbol an old caller never looks up.

*Cost to a compiled C caller of a new `Command`: nothing.* It keeps working, unrecompiled, and does
not have the new command.

### 2. How does it receive events and answers? — an owned batch it frees

**Decided: `Viewer::handle`'s iterator is drained into a `Vec` before the entry point returns.**

`Viewer::handle` borrows the viewer and `Viewer::query` may too. Neither survives this boundary: a
caller holding a borrow and calling back in is `&mut Viewer` held twice, which is the aliasing
hazard `viewer-qt` needed its `Busy` guard for (ADR 0246 §3) and which nothing on this side would
notice. A batch ends the borrow before the caller sees anything, so **re-entrancy stops being a rule
anybody has to keep** and becomes a property of the shape.

**A callback was the other candidate and is refused**: `void (*)(const pdfv_event *, void *)` saves
an allocation and buys the caller a chance to call `pdfv_go_to_page` from inside the dispatch of the
events `pdfv_open` produced.

Structured answers take the same shape — `pdfv_outline` is owned — and amendment 2 is what makes
that cheap rather than a second copy.

*Cost of a variant gaining a field:* a new accessor function; the old one goes on answering what it
always answered.

### 3. How does it hand back a raster? — through the request that asked for it

**Decided: the render request is an opaque handle; this crate holds `render-cpu` to draw it.**

`Event::NeedsRender` carries an `Arc<DisplayList>`, which is clauses 8 and 9 in a data structure. It
is not a thing to put in a header, and a C caller has no use for it that is not "draw this" — which
is what the request already means. So `pdfv_event_render_request` hands over an opaque handle the
caller may **move to another thread**, `pdfv_render_request_rasterise` draws it there, and
`pdfv_render_ready_raster` takes the request back beside the raster.

Taking the request back is how `RenderToken` stays out of the header: it is opaque in Rust, and a
caller returns what it was given. The round trip is kept as a round trip rather than hidden inside
`pdfv_open`, because the whole point of `NeedsRender`/`RenderReady` is that the *host* decides where
the work runs.

Pixels reach the caller by **copy into a buffer the caller owns** (`pdfv_frame_copy`, sized from
`pdfv_frame_info`). No pointer into the viewer's memory is ever handed out, so there is no lifetime
for a C program to get wrong, and the cost is the one copy `doc/ui-boundary.md` prices tier 1 at.

### 4. How does it learn about a variant added later? — it names it, describes it, and counts it

**This is the question the other two hosts never had to answer**, and the answer has three parts and
one admission.

- **Names.** `pdfv_event_kind_name` answers for any number, `"unknown"` for one this build does not
  define.
- **Descriptions.** `pdfv_events_describe` gives **one sentence for every event, whatever its kind**,
  including kinds the caller was compiled before. An unknown event can therefore be *logged* rather
  than dropped in silence — trap 5 in the only form C leaves available. `EventKind::of` is
  exhaustive over `viewer_core::Event` with no catch-all arm, so a message added to that crate fails
  to compile in `kinds.rs` and cannot reach a caller unnamed.
- **A count.** The header states `PDFV_EVENT_KIND_COUNT`; the library answers
  `pdfv_event_kind_count()`; `pdfv_abi_check` compares them and the ABI version. A caller that runs
  it in `main` has converted *"fails to compile in every consumer"* into **"fails to start, once,
  naming the number that moved"**.

**The admission**: that is weaker. It is a runtime check a caller may decline to make, where the Rust
rule is a build failure nobody can decline. It is the strongest thing available, and saying so is
better than implying a C boundary keeps a guarantee it cannot keep.

*What each kind of change costs a compiled C caller:*

| change | cost |
|---|---|
| a new `Command` | nothing |
| a new `Event` | nothing until it meets one; then a `default:` arm that says what it does not understand |
| a variant gaining a field | a new accessor function, when it wants the field |
| a **struct passed by value** gaining a field | **a recompilation it has no way of knowing it needs** |

The last row is why `PDFV_ABI_VERSION` exists and why this header has exactly **two** such structs
(`pdfv_geometry`, `pdfv_frame`), both small and both output-only.

### Errors, because a C caller cannot see a `Result`

Every fallible entry point returns a `pdfv_status` as `int32_t`; `pdfv_status_message` turns one into
a sentence; out-parameters are written only on `PDFV_OK`. Nothing returns a sentinel that has to be
told apart from data — `pdfv_event_opened` on a `PageChanged` answers `PDFV_WRONG_KIND` and not
zeroes, because zero is a page count. A *document* that will not open is not a status at all: it is
`Event::OpenFailed` carrying `pdf-syntax`'s own words, which is a fact about the file and belongs in
the channel every other fact about it uses.

### No `cbindgen`, and the drift is bought back by a test

**Decided: the header is hand-written.** It is the artefact a C programmer reads, with the reason for
each shape beside it; a generated header is a derivative of Rust types that a person then has to read
anyway. What a generator buys is that it cannot drift, and that is bought back:

- `tests/header_and_library_agree.rs` reads both files and asserts that every `#[unsafe(no_mangle)]`
  entry point is declared exactly once in the header and every `pdfv_` function the header declares
  exists in the Rust, **and that every `PDFV_` constant is the number the Rust enumeration gives
  it**. The second is the one that would fail silently: `#define PDFV_EVENT_SAVED 12u` beside a Rust
  `Saved = 11` compiles, links, runs, and acts on the wrong events. Checked by making that exact
  edit; the test fails and names it.
- `tests/a_c_program_drives_the_abi.rs` hands the header to `cc` with `-Wall -Wextra -Werror` and
  the symbols to a linker.

The consequence: **no new dependency, no `deny.toml` entry, and nothing to record in
`doc/third-party-data.md`.** `cargo deny check` is clean on all four checks.

### Where the `unsafe` is, and how much

`doc/todo/30` reserves the permission for this crate. Being the crate the rule names is not a licence
to stop counting, so the position is exact and is enforced by `tests/unsafe_position.rs`:

- the crate root **denies** `unsafe_code` and lifts it **once**, on `pub mod abi`;
- **every `unsafe` token in the crate is in `src/abi.rs`**, and each is one of three forms: the one
  `#![allow(unsafe_op_in_unsafe_fn)]`, **39** `#[unsafe(no_mangle)]` attributes, and **35**
  signatures — 33 `pub unsafe extern "C" fn` entry points and two `unsafe fn` helpers;
- **there is no `unsafe` block anywhere in the crate.**

Two positions inside that are decisions rather than accidents. **Every entry point taking a pointer
is `pub unsafe extern "C" fn`**: C does not see the word, and Rust does — this crate is an `rlib` as
well as a `cdylib`, and a safe `extern "C" fn` that dereferenced its arguments would be an unsound
API for a Rust caller. And **`unsafe_op_in_unsafe_fn` is lifted for that module**, because these
bodies are *entirely* the unsafe operation: a block around three lines of a four-line shim marks
nothing out. What replaces the lint is the count above.

`viewer-qt/tests/unsafe_position.rs` predicted this crate and asked to be amended when it arrived; it
now asserts that **exactly two** crates in the tree lift the denial, each with a test on where its
`unsafe` sits. `viewer-ffi`'s own test additionally asserts that `pdf-syntax`, `pdf-model`,
`pdf-font`, `pdf-render`, `render-cpu`, `viewer-core` and `viewer-host` still hold
`#![forbid(unsafe_code)]` — which is `doc/todo/30`'s claim that the compiler-enforced rule survives
this crate's arrival, checked rather than promised.

**No PDF parsing happens behind an `unsafe`.** This crate touches messages.

### A tree is the one shape a C ABI cannot hand over as itself

§12.3.3's outline crosses **flattened**: depth first, with a depth on each row. That is not a
compromise invented here — it is what `viewer-qt` already builds internally, because
`QAbstractItemModel` must answer for any node at any moment (ADR 0246). The second host having
needed the same shape is what makes it a finding.

The rows come from `viewer_host::panel`, unchanged. ADR 0246 decision 3 said a native host on this
boundary is mostly not toolkit code; a C host is a native host, and it takes the same four modules'
worth of decision-making the other two do.

---

## Evidence: a C program, compiled with `gcc`, run headless

`c/open_a_page.c`, 300 lines, `gcc -std=c11 -Wall -Wextra -Werror -O2`, linked against the release
`cdylib`. Numbers read off the run, median of five.

| what | `PDF20_AN001-BPC.pdf` (5 pages, 173 159 B) | `ISO_32000-2_sponsored_EC3.pdf` (1023 pages, 25 MB) |
|---|---|---|
| `pdfv_abi_check` | `abi 1 (header 1), 15 event kind(s) (header 15)` | the same |
| `pdfv_open` | **4.42 ms**, 3 events: `Opened`, `PageChanged`, `NeedsRender` | **63.1 ms**, `Opened says document 1 has 1023 page(s)` |
| first page drawn and handed back | **12.3 ms** | **76.3 ms** |
| §12.4.2's label, off `pdfv_events_describe` | `page 1 of 5, labelled Cover` | — |
| `pdfv_page_geometry` | `595.3x841.9 user units at 1.188, 708x1000 px, origin 46.0,-0.0` | — |
| `pdfv_outline_read` | **14 rows**, depth 0 and 1, each with §7.3.10's two numbers | **988 rows** |
| page turn (`PDFV_PAGE_NEXT`) | 2 events, page 2 drawn in **7.39 ms** | page 2 drawn in **12.8 ms** |
| `pdfv_frame_info` | `page 2, 708x1000, format 0, 2832000 byte(s)` | `2828000 byte(s)` |
| `pdfv_frame_copy` | **1025 µs — 2.8 GB/s** | **149 µs — 18.9 GB/s** |
| the page is not blank | 13 113 of 708 000 pixels are not white | 52 728 of 707 000 |
| `pdfv_frame_copy` into one byte | `PDFV_BUFFER_TOO_SMALL`, nothing written | the same |

Three of those are worth a sentence.

**The two copy rates are the same copy.** 2.8 GB/s against 18.9 GB/s for 2.8 MB of `memcpy` is not
the boundary: the note's run reaches the copy having done almost nothing, so `malloc(2 832 000)`
returns fresh pages and the copy takes their first-touch faults; the ISO run has built a 988-row
outline first and the allocator is warm. ADR 0246 measured the same split on both toolkits — 2.6 to
3.6 GB/s cold, 11.5 to 12.0 GB/s warm — and this is a third instrument agreeing.

**A 1023-page document opens in 63 ms against 4.4 ms for a five-page one, and that is not a page
count.** The file is 145 times larger in *bytes*; `CLAUDE.md`'s rule is that a 500-page document must
open no slower than a 5-page one, and what this measures is a 25 MB file rather than 1023 pages. The
same ratio appears on the Qt host (ADR 0246: 193.8 ms against 105.9 ms with a window in the way).

**The header collided with itself on the first compile.** `typedef struct pdfv_frame_info` beside
`int32_t pdfv_frame_info(…)` is a redeclaration error, because C has one namespace for a struct tag
and a function. Nothing in the Rust could have shown that, and it is the plainest possible
demonstration of why the round demanded a C program rather than an `extern "C"` surface with tests
in Rust.

## Consequences

- **The three amendments are taken and one of them was a bug**, not a documentation gap: `viewer-ui`
  corrupted a password field's value on every keystroke, and the shape change is what surfaced it.
- **A fourth thing came out of the same clause**: this program wrote a person's typed password into
  the file it saved, against Table 231 bit 14's own NOTE. It no longer does, and it says so.
- **`RasterFormat` is no longer `#[non_exhaustive]`**, and four consumers lost a runtime refusal for
  a condition a build now catches.
- **The ABI is 39 entry points and it is not the whole vocabulary.** `doc/todo/30` carries what is
  left — the pointer and selection channel, §12.7's form, the edit and save messages, the layer and
  attachment panels, §12.4.4's transitions — and every one of them is a *function to add*, which
  costs a compiled caller nothing. That is the property the shape was chosen for, and it is why
  stopping here is honest rather than half-built.
- **`viewer-ffi` cross-compiles**, to both `x86_64-pc-windows-msvc` and `aarch64-apple-darwin`,
  unlike either toolkit host. A C ABI over a pure-Rust core binds no platform, which is the point of
  having one.
- **Two crates in the tree now lift `deny(unsafe_code)`**, each with a test on where its `unsafe`
  sits, and every crate that touches PDF bytes still forbids it — checked, in
  `viewer-ffi/tests/unsafe_position.rs`, rather than asserted in a document.
