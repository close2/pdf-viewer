# 526 — The line a comment ends at

**Finding.** §7.10.5.1 lists what a type 4 program may contain and "Comments" is one of the five
entries, so a commented program is a conforming program — and this tree refused one. The compiler
spaced `{`, `}` and `%` apart, split the **whole stream** on white space, and then skipped exactly
one token after each PERCENT SIGN, a rule its own comment described as "the closest safe
approximation" to §7.2.4's "all characters after the PERCENT SIGN and up to but not including the
end-of-the-line marker". It is not safe: where the next word is prose the function is refused
loudly (the project owner's `doc/corpora-own/type4_pi.pdf` opens `% BBP Math for Pi …`, and `Math`
is not in Table 42), and where a comment quotes the arithmetic it documents — `% dup 3 mul` — every
word after the first was compiled into the program with nothing reported. Comments are now cut line
by line before tokenising, which is sound *here* for a reason the same list gives: with "No
composite data structures (such as strings or arrays)" there are no string literals, so a PERCENT
SIGN in a type 4 program can only start a comment.

**Date.** 2026-08-14.
**ADR.** [0361](../adr/0361-the-line-a-comment-ends-at.md).
**Touched.** `crates/pdf-model/src/function.rs` (`without_comments`, `compile_postscript`,
`compile_block`'s `%` arm removed, three tests), `crates/pdf-model/tests/shadings.rs` (three tests),
`crates/test-scenes/src/type4.rs` (new) and `src/lib.rs`,
`crates/pdf-model/examples/type4_comment_census.rs` (new), `doc/conformance/ledger.toml` (§7.2.4,
§7.10.5, §7.10.5.1), `doc/corpora-own/README.md`, `doc/verify.md`, `doc/adr/0361-*` (new), this file.

## The page, looked at

`pdf-retrieve page doc/corpora-own/type4_pi.pdf 0` went from one `Shading` report and a blank page
to `complete`, and the page draws **3.141** — ten rectangles the program's own comments name
(`% Rect 10,25 85,95`), black where the function returns `dup sub` and white where it returns
`dup div`. The value is checkable by arithmetic and was: 4/1 − 2/4 − 1/5 − 1/6 = 3.1333333…, plus
(4/9 − 2/12 − 1/13 − 1/14)/16 = 0.0080891…, is 3.1414224…, which is π to 1.7 × 10⁻⁴ and exactly
3.141 after the program's own `1000 mul truncate 1000 div`.

## The census, and what it costs

`cargo run --release -p pdf-model --example type4_comment_census` over `doc/pdf.js`, the four
`doc/corpora` submodules, `doc/corpora-own` and the whole `SafeDocs` cache — 67 461 documents,
7 352 type 4 functions in 2 098 of them — finds **two** programs containing a PERCENT SIGN: the
owner's file, `refused`, and one `SafeDocs` tint transform, `harmless`. **Nought silently
mis-compiled**, and the reason is the finding: a producer does not comment a generated tint
transform, so the exposure was to files written by hand.

The `harmless` one earned a test of its own. `cc-main-2021-31/5097152.pdf` object 19 is
`{\r0 %c\r0 %m\r0 %y\r3 index %k\r5 -1 roll pop\r}` — comments of one word each, which is why the
old rule survived, and **CARRIAGE RETURN line endings with no LINE FEED in the file**. A fix that
had cut each LINE-FEED-delimited line at its first PERCENT SIGN would have read that whole program
as one comment and returned `{ 0`, four outputs short and in silence. §7.2.3's marker is either
byte, and the code and the test both say so.

## Nothing else moved, measured rather than argued

`display_list_digest` over all 974 corpus documents is **byte-identical** before and after — same
command count, `Debug` length and hash on every one of the 974 lines — taken by `git diff > patch`,
`git apply -R`, run, `git apply` (never `git stash`; session 523 paid for that). So no corpus or
oracle page can move, `doc/todo/00` step 7's ink sweep has nothing to sweep and the quorra lanes
have nothing to compare. The census says why: no corpus document carries a commented type 4 program.

## Gates, verbatim

```text
cargo fmt --all --check                                   clean
cargo clippy --workspace --all-targets                    silent of lints
cargo nextest run --workspace                             1918 tests run: 1918 passed, 15 skipped
cargo test --workspace --doc                              1 passed, 0 failed, 1 ignored
corpus    974 documents in 11.4s: 0 unopenable, 8 locked, 2 encrypted beyond us,
          6 pageless, 61 incomplete, 0 slow
          codes reaching no glyph in silence 5/2; reaching a blank glyph 57/9;
          §9.10.2 could not name 1228/43
oracle    1794 pages in 44.4s (1694 we call complete, 100 incomplete)
          agrees 906/863   contradicted 67/66   ambiguous 786/755
          our geometry 1/0   reference geometry 2/2   not comparable 13/8   no render 19/0
text      974 documents in 33.2s: 25 skipped, 58 incomplete and not gated;
          overall 99.3% (24016/24195 words), 22 below 90%
          10969/11163 word boxes in bounds (98.26%), 486 of 508 documents fully in bounds
          PDFBox: doc/corpora/pdfbox is not checked out — skipped, as §2 says it may be
dates / xmp / jpeg2000 / conformance                      ok
quorra    956 pages compared in 89.8s: 930 agree, 24 differ, 2 refused, 18 not comparable
```

The gpu lane was not run and is not owed: this round took no quorra release and did not touch the
zoom path, and the digest above establishes that it draws the same lists.

**Not done, and why.** `doc/todo/02` §5's release binaries were not installed into the main tree's
`target/`: this is an unmerged worktree, and putting unmerged rendering in front of a person is the
merge round's decision rather than this one's.

## Two things worth keeping

**A rule implemented twice is a rule that can be right once.** §7.2.4 has been right in
`pdf-syntax`'s lexer since the first commit — `skip_whitespace` skips comments and white space
together *because* the clause makes them one thing, and the ledger row said so and named that file.
The type 4 compiler is a second lexer for a second grammar; it never goes through `lexer.rs`, and
nobody had asked the clause a second time. The row names both files now, which is what would have
found this.

**A comment admitting an approximation is a defect report nobody filed.** "Skipping one token is
the closest safe approximation" was written by somebody who knew the rule and could not reach it
from where the code stood. The fix was to move the work earlier — strip, then split — rather than
to approximate better; the question to ask of such a comment is what would have to happen *before*
this point for the exact rule to be available.
