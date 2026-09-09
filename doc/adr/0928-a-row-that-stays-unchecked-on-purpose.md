# 0928 — A row that stays unchecked on purpose

Session 940. Status: **accepted**. Recorded under the questions-directory rule: when an `A` file
appears, the round that acts on it records the decision in an ADR.

## The answer

`Q51` asked whether to buy the JPEG 2000 specifications. It was asked when none were held, narrowed
on 2026-09-07 when the owner obtained **ISO/IEC 15444-1:2000** for free and five of the seven rows
of ISO 19005-2 §6.2.8.3 / ISO 19005-4 §6.2.7.3 came off `Check::Unchecked` on the strength of it
(ADR 0925), and answered on 2026-09-09:

> Part 2, ISO/IEC 15444-2, is not in `doc/`, and I am not buying it.
>
> Leave the baseline-feature row `Unchecked` with its truthful reason. Part 2 is bought only if we
> ever want a conforming-validator claim with no gaps — and that is a new question, asked then.

## The decision

**`graphics/jpeg2000-uses-the-baseline-feature-set` stays `Check::Unchecked`, and its reason stops
pointing at an open question.** The reason cited `doc/questions/Q51`, which reads as *pending*; it
now cites `A51`, which reads as *settled*. The row is not a debt, not a gap to be closed when
somebody gets around to it, and not something a later round should try to be clever about. It is a
requirement whose defining text this project has decided not to hold.

The other row that stays unchecked in that clause — `jpeg2000-device-colour-obeys-the-colour-rules`
— is **not** covered by this answer and must not be filed under it. It is unchecked for a different
and more interesting reason: neither part of ISO 19005 says which enumerated colour space makes an
image *effectively* a device space, and buying anything would not settle it. Its reason already
says so.

## Two things the next reader should not have to rediscover

**The later editions do not help, and I checked rather than assumed.**
`doc/ISO-IEC-15444-1-2016.pdf` and `doc/ISO-IEC-15444-1-2019.pdf` sit beside the 2000 edition and
look like the third and fourth editions of part 1. They are iTeh **STANDARD PREVIEW** files:
fifteen pages of title page, copyright notice, foreword and table of contents. Prepared with
`tools/spec-md.py` they come to about 5 300 words each and contain the strings `colr` and `EnumCS`
exactly zero times. They cannot settle the edition question, and a future round tempted by their
filenames should stop here.

**The edition discrepancy is therefore permanent and is handled where it occurs.** ISO 19005
permits a `METH` of 3 where the 2000 edition of part 1 defines only 1 and 2; it names enumerated
colour spaces 12 and 19 where that edition's table defines only 16 and 17; and it gives `APPROX` a
meaning where I.5.3.3 says the field shall be zero and readers shall ignore it. In all three the
rule implemented is ISO 19005's, each constant in `crates/pdf-archive/src/table/graphics.rs` says
so above itself, and that is the end state rather than a workaround pending a purchase.

## What it would take to change this

A `conforming-validator` claim with no gaps, wanted by somebody for a reason. `A51` says that is a
new question to be asked then — not a reopening of this one.
