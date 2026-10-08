# OpenType's script tag registry, and where this list came from

`script-tags.txt` is the registry of OpenType script tags, one tag and the registry's name for the
script per line. `crates/pdf-font/src/pairs.rs` chooses a program's `GPOS` script table by the tag
registered for the script a run of rich text is written in (ADR 1696), and the test
`every_script_reaches_a_registered_tag` reads every code `Scripts.txt` names against this list, so
the mapping in `pairs::tags` cannot name a tag the registry does not hold.

## Where it came from

<https://learn.microsoft.com/en-us/typography/opentype/spec/scripttags>, the page titled "Script
tags (OpenType 1.9.1)", fetched on 2026-10-08. Its table's first two columns are kept, in its own
order; the third, remarks, is not. The registry prints the tag `'yi  '` with no-break spaces "to
ensure that two space characters are displayed" and says the tag's bytes are `0x79 0x69 0x20 0x20`,
so every tag here is written with ordinary spaces.

    sha256  0fa89c4e9ffca684b80bb6c846f70851943e3173689c5b3de42511e6d0224923  script-tags.txt

Only a test reads it; nothing is compiled in from it.
