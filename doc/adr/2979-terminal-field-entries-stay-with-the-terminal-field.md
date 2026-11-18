# 2979 — `/TI`, `/I`, `/RV`, `/DS` and a choice field's `/Opt` are read no higher than the terminal field

Session 2075. Status: **implemented.** Context: review 2038 (findings B26, B63, B64, B65), ADR 2448 (the
inheritance memo). Clauses: ISO 32000-2 §12.7.4.1, §12.7.4.3 Table 228, §12.7.5.4 Tables 230 and 234.

§12.7.4.1 gives a field its parent's value for the attributes designated inheritable, and Table 228's `/DS` and
`/RV`, and Table 234's `/TI`, `/I` and `/Opt`, are not. `READ_UP_THE_CHAIN` read all of them from the nearest
ancestor stating one, because a widget that is not merged with its field holds none of them itself. They now stop at
the terminal field: each field's resolved entries carry its own of these and never its parent's, and a widget that
states `/T` — a widget merged with its terminal field — does not read them from the non-terminal field above it.
`/Opt` is two entries under one name: Table 230's check-box and radio-button `/Opt` is inheritable and still read up
the chain; Table 234's is local when the resolved `/FT` is `/Ch`, and its absence presents no choices, as the row
says.

Separately, §12.7.5.4's `/V` names items by display text and `/I` decides between equal labels. A `/V` naming one
label twice selected the first `/I` index twice, which collapsed to one. Each value now takes an occurrence no
earlier value took.
