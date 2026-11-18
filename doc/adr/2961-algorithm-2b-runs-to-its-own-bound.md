# 2961 — Algorithm 2.B runs to its own bound

Session 2072. Status: **implemented.** Context: review 2038 finding D18, ISO 32000-2 §7.6.4.3.4.

## 1. The finding

Algorithm 2.B repeats steps (a) to (d) 64 times, and then step (e): "Look at the very last byte of E. If the value
of that byte (taken as an unsigned integer) is greater than the round number - 32, repeat steps (a-d) again." `crypt::hash_2b` stopped at 256 rounds as a guard against a file
choosing how long the reader computes, and returned the hash reached so far, which for a valid password needing
more rounds would be a key that does not authenticate.

## 2. The decision

The ceiling is removed. The clause bounds itself: the byte is at most 255, so the condition is false once 287
repetitions are done, whatever E is. A file therefore never chose more than 287 repetitions, and a ceiling of this reader's below that bound could only answer a valid password wrongly. No valid input needing
more than 256 rounds is known (NOTE 3 expects 65 to 80); the change removes a wrong answer, not an observed one.

## 3. Cost

The worst case an adversarial salt could ask for rises from 256 to 287 repetitions, 12 % more, and reaching it
requires E's last byte to stay high across more than 220 repetitions, which no salt chosen without breaking SHA-2 does.
