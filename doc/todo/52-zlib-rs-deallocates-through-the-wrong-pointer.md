# `zlib-rs` deallocates through a pointer its allocation was not made through

Status: **owed upstream**, and declined by the tests themselves with the reason beside each.
Priority: 52 — blocked on a dependency. Nothing in this tree can fix it.
Corpus: — (every deflated stream runs this code; nothing has ever gone wrong at run time)
Clauses: §7.4.4's `FlateDecode`, only as the caller.
Code: `crates/pdf-syntax/src/filter.rs` — `mod tests`' own doc comment and three
`#[cfg_attr(miri, ignore)]`. **Not `.github/workflows/ci.yml` any more**, and the move is the
correction below.

## What Miri says

`cargo +nightly miri test -p pdf-syntax --lib`, on `filter::tests::flate_falls_back_to_raw_deflate`:

```
error: Undefined Behavior: deallocating while item [Unique for <…>] is strongly protected
   --> zlib-rs-0.6.6/src/allocate.rs:185:14
    |
185 |     unsafe { std::alloc::dealloc(ptr.cast(), layout) };
```

through `zlib_rs::deflate::end` from `<zlib_rs::stable::Deflate as Drop>::drop`. Under Tree
Borrows the same line says "deallocation through <…> (root of the allocation) at alloc…[0x0] is
forbidden", so **it is not an artefact of Stacked Borrows being the experimental model** — both
aliasing disciplines reject it, which is the evidence worth putting in the report.

## The skip was a substring, and it was wrong in both directions

This file used to say the CI line skipped "the two tests whose names contain `flate`". **It
skipped three**: `an_inflate_never_buys_a_buffer_past_the_bound` contains `flate` inside `inflate`,
so a test about a bomb's *buffer* — ADR 0354's, exactly the kind of arithmetic an interpreter is
worth pointing at — was excluded by an accident of spelling that nobody could see from either end.
It does drive `zlib-rs` and does have to decline; that it declined for no stated reason is the
point.

The other direction cost more. A name filter in a workflow can only ever exclude what somebody
remembered to write there, and when a **second** dependency's unsafe appeared — `crossbeam-epoch`
0.9.20, reached through `rayon` from `pdf-render`'s divided reduction — the Miri job simply began
failing, and went on failing across five pushes with nothing on this machine able to say so. So
each test that must not run under the interpreter now declines by itself, naming its dependency
and its aliasing rule; the CI line carries no `--skip` at all. ADR 0450.

## Why it is here and not in `10`–`19`

Nothing in this tree is wrong, and nothing observably misbehaves: the code has run on every
deflated stream in a 974-document corpus without a fault. What Miri objects to is a provenance
rule, and the machine code an optimiser emits from that pattern is very likely fine today. "Very
likely fine today" is exactly what a provenance bug is before it is a miscompilation.

`zlib-rs` was chosen (`doc/stack.md`'s table) as "pure Rust, ~C speed" — the whole reason it is
here rather than `miniz` or C `zlib` is that it is Rust. A soundness bug in it is worth reporting
for that reason above any other.

## What to do

1. **Reduce it.** The failing test is small already; the reproducer wanted upstream is a
   `zlib-rs`-only one — construct a `Deflate`, drop it, run under Miri — with no PDF in it.
2. **Check it against `zlib-rs` `main`** before reporting; 0.6.6 is what `Cargo.lock` pins and the
   allocator shim is exactly the kind of code that gets rewritten.
3. **Report it** at <https://github.com/trifectatechfoundation/zlib-rs>, with both models' messages.
4. **Take the three `#[cfg_attr(miri, ignore)]`s off** when a fixed version is released — and the
   note above `mod tests` with them — then delete this file. **There is a fourth in the file and it
   is not one of them**: `an_lzw_bomb_costs_the_window_rather_than_its_decode` declines for its own
   *size* rather than for a dependency's unsafe (ADR 0463), and it stays when these three go. Its
   reason is in its doc comment, which is the whole point of a declination living on its test.

## What not to do

Do not switch `flate2` to another backend over this. The C backends have the same class of bug and
cannot be checked for it at all, which is the argument that put `zlib-rs` here.
