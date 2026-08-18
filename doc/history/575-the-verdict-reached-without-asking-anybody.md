# 575 — the oracle's `no render` bucket is the one verdict reached without asking the references, and one of its pages was ours

Date: 2026-08-18. ADR: [0410](../adr/0410-the-verdict-reached-without-asking-anybody.md).

Took the robustness denominator, from the oracle's own verdict buckets. `contradicted` and
`ambiguous` are diagnosed to the last page and have been for two hundred rounds, so sizing them took
a minute and produced nothing to pick — but the *summary* did: the gate prints seven verdicts and
holds two. `no render` was one of the five nobody watched, and it is the one where a defect is worst,
because a page in it is a page a person is shown nothing of.

`examine` returns as soon as `render_ours` fails, one line before `render_references`. So on a
`no render` page the three reference renderers are never invoked, which makes it the one verdict this
gate reaches with the other programs' opinions unasked — and a page three readers draw and we do not
was indistinguishable there from a page nobody can read. `doc/HANDOVER.md`'s trap 1 has called the
count "a to-do list of pages nobody has looked at" since the hundred-and-seventy-seventh session, and
the sentence was exactly true for four hundred rounds.

The whole bucket was put to `pdftoppm`, `mutool` and `gs` by hand, with `tools/pdfref`'s own
invocations copied verbatim so that every one is explicit about the page box. Most of it is the
standard working: §7.6.4.1's password on eight pages, which all three references refuse in the same
words — that is four independent derivations of §7.6.4.3's key agreeing the *default* password is not
these documents', which is a claim about eight files nothing here could previously make — plus two
encryptions ISO 32000-2 states no algorithm for and six page trees that yield nothing.

Three pages were something else. `issue19517.pdf` is past this gate's own `PIXEL_BUDGET` and the
program draws it perfectly: 12 608 × 16 806 at ink 172.597 against 172.602, 172.599 and 172.599 —
agreement with all three to 0.005 of 255 on a page the oracle has never judged. The budget stays; what
was wrong is that the bucket named the program when the instrument was what declined.
`Brotli-Prototype-FileA.pdf` is two references implementing a filter ISO 32000-2 does not define. And
`boundingBox_invalid.pdf` page 1 was ours.

That file states `/MediaBox [0 0 0 0]`, and §7.9.5 says in a NOTE that a rectangle may have zero width
or height — so the array passes every test a rectangle reader can apply, and Table 31's requirement
that it "define the boundaries of the physical medium" is the one it fails. `MediaBoxSubstitution`
had two variants, `Absent` and `NotARectangle`, which between them name everything a *rectangle*
reader can complain about; the value that fails Table 31 while satisfying §7.9.5 fell through both,
`TargetSpec::for_page` refused the page as degenerate, and the document was unviewable. The
generalising half is one clause over: §14.11.2's other four boxes have had the empty-box rule since
they were read, because each *defaults* to a larger one and can fall back — the media box is what all
four fall back to, so it is the member of the family with nothing behind it and the one whose rule was
missing.

It draws now, on the A4 substitute ADR 0389 chose and kept against three references' 612 × 792, with
the report saying so. The check needs no reference treated as truth: ink 0.63587 over 596 × 842 is
319 099 units of mark against `poppler`'s 317 570 and `mutool`'s 318 698 over 612 × 792 — the same
marks on a different sheet, 0.5% apart.

The bucket is held by name now, in four `NO_RENDER_*` groups over *all* pages rather than the complete
ones, since a page that renders nothing is never complete. `examples/media_box_census` learned the
third kind and was run over every corpus on this disk: zero witnesses in 919 995 crawled pages, one in
the 974, and it is the file pdf.js built to carry this defect and captioned *Empty /MediaBox*.

Files: `crates/pdf-model/src/page.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/src/content/report.rs`, `crates/pdf-model/tests/oracle.rs`,
`crates/pdf-model/tests/page_geometry.rs`, `crates/pdf-model/examples/media_box_census.rs`,
`doc/conformance/ledger.toml` (§7.7.3.4, §7.9.5, §14.11.2.1), `doc/oracle-and-corpus.md` §3d,
`doc/HANDOVER.md` trap 1, `doc/todo/README.md`.
