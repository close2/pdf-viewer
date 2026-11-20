# 3336 — A line cap or join code out of range is clipped into it

Session 2158. Status: accepted. Context: audit 2155's finding D01.

§8.4.1: "Parameters that are numeric values, such as the current colour, line width, and miter limit,
shall be clipped into valid range, if necessary." Tables 53 and 54 give the cap and the join the codes
0, 1 and 2. `line_cap` and `line_join` sent any other code to the initial value and called that §8.4.1's
answer; the sentence names no initial value.

**Decision.** The codes are numbers, so they are clipped: below 0 to 0, above 2 to 2. `3 J` is the
projecting square cap and `7 j` the bevel; `-1 J` the butt cap. Both routes — `J`/`j` and `/LC`/`/LJ` —
share the two functions. `pdf-model/tests/line_parameters.rs` pins it.
