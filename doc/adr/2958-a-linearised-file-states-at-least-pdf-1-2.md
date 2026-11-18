# 2958 — A linearised file states at least PDF 1.2

Session 2072. Status: **implemented.** Context: review 2038 finding B55, ISO 32000-2 §F.3.2, Errata Collection 3
Issue #331 (`doc/errata-read.md`), ADR 1225.

## 1. The finding

§F.3.2 as published says linearisation "may be applied to any PDF file of version 1.1 or greater". Issue #331,
`Review/Completed`, strikes the `1.1` and writes `1.2`. `pdf_syntax::linearize` wrote the source's own header, so a
PDF 1.0 or 1.1 source came out linearised under a header below the amended floor. The audit's two tests
(`linearizing_pdf_1_0_…` and `…_1_1_refuses_or_raises_the_header_to_the_ec3_floor`) accept either a refusal or a raise.

## 2. The decision: raise, do not refuse

`write_linearized` takes the larger of the version it would have written and `LINEARIZATION_FLOOR` (1.2). A raise is
chosen over a refusal because nothing a 1.0 or 1.1 file can contain is outside what a 1.2 file may contain: the
header then states a version the content meets, and a user asking for a linearised copy of an old file gets one.
A refusal would turn a version number into a lost operation, which the erratum does not ask for: it bounds which
files *are* linearised, and a file written at 1.2 is one.

## 3. Cost

A linearised copy of a 1.0 or 1.1 document no longer carries its source's version byte for byte. The version is
not content, and §7.5.2 makes the header the statement of the version the file conforms to.
