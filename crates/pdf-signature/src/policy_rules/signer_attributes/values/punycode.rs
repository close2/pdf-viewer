//! RFC 3492's Punycode, the encoding RFC 5890 section 2.3.2.1 makes an A-label of a U-label:
//! section 6's encoding and decoding procedures under section 5's parameter values, with section
//! 6.4's overflow handled by checked arithmetic, which fails rather than wraps.

/// Section 5's parameter values for Punycode.
const BASE: u32 = 36;
const T_MIN: u32 = 1;
const T_MAX: u32 = 26;
const SKEW: u32 = 38;
const DAMP: u32 = 700;
const INITIAL_BIAS: u32 = 72;
const INITIAL_N: u32 = 0x80;

/// Section 6.1's bias adaptation.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`points` is at least one and `delta` a checked `u32`: the halving, the division and the \
              sum stay below `u32::MAX`, and the loop divides `delta` until `k`, a few multiples \
              of 36, is returned"
)]
fn adapt(delta: u32, points: u32, first: bool) -> u32 {
    let mut delta = if first { delta / DAMP } else { delta / 2 };
    delta += delta / points;
    let mut k = 0;
    while delta > ((BASE - T_MIN) * T_MAX) / 2 {
        delta /= BASE - T_MIN;
        k += BASE;
    }
    k + (((BASE - T_MIN + 1) * delta) / (delta + SKEW))
}

/// Section 6's threshold `t` for the digit position whose `k` is given.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`bias + T_MAX` is a bias `adapt` returned plus 26, and `k - bias` runs only where \
              `k` exceeds `bias`"
)]
fn threshold(k: u32, bias: u32) -> u32 {
    if k <= bias {
        T_MIN
    } else if k >= bias + T_MAX {
        T_MAX
    } else {
        k - bias
    }
}

/// Section 5: digit values 0 to 25 are `a` to `z`, 26 to 35 are `0` to `9`. The encoder emits
/// the lowercase forms.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`value` is below `BASE`, so each sum is an ASCII letter or digit"
)]
fn digit(value: u32) -> char {
    let offset = if value < 26 {
        u32::from(b'a') + value
    } else {
        u32::from(b'0') + value - 26
    };
    char::from_u32(offset).unwrap_or('a')
}

/// Section 5: a digit's value, either case accepted.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "each subtraction is from an octet the arm has matched as at least its base, and \
              each sum is below 36"
)]
fn value_of(octet: u8) -> Option<u32> {
    match octet {
        b'a'..=b'z' => Some(u32::from(octet - b'a')),
        b'A'..=b'Z' => Some(u32::from(octet - b'A')),
        b'0'..=b'9' => Some(u32::from(octet - b'0') + 26),
        _ => None,
    }
}

/// Section 6.3: `input` encoded, or `None` on section 6.4's overflow.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every sum that grows with the input is checked; what is not is bounded: `next - n` \
              with `next` at least `n`, `handled + 1` with `handled` below `total`, a threshold \
              `t` at most `q` and below `BASE`, and `k` grown by `BASE` once per digit of `q`, \
              which each digit divides by at least ten"
)]
pub(super) fn encode(input: &str) -> Option<String> {
    let points: Vec<u32> = input.chars().map(u32::from).collect();
    let mut output: String = input.chars().filter(char::is_ascii).collect();
    let basic = u32::try_from(output.len()).ok()?;
    let total = u32::try_from(points.len()).ok()?;
    if basic > 0 {
        output.push('-');
    }
    let (mut n, mut delta, mut bias, mut handled) = (INITIAL_N, 0_u32, INITIAL_BIAS, basic);
    while handled < total {
        let next = points.iter().copied().filter(|&point| point >= n).min()?;
        delta = delta.checked_add((next - n).checked_mul(handled + 1)?)?;
        n = next;
        for &point in &points {
            if point < n {
                delta = delta.checked_add(1)?;
            }
            if point == n {
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = threshold(k, bias);
                    if q < t {
                        break;
                    }
                    output.push(digit(t + (q - t) % (BASE - t)));
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                output.push(digit(q));
                bias = adapt(delta, handled + 1, handled == basic);
                delta = 0;
                handled += 1;
            }
        }
        delta = delta.checked_add(1)?;
        n = n.checked_add(1)?;
    }
    Some(output)
}

/// Section 6.2: `input` decoded, or `None` where it is not Punycode — a code point outside ASCII,
/// a character that is no digit, a basic code point decoded from the deltas, an overflow, or a
/// value that is no Unicode scalar value.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every sum and product that grows with the input is checked; `i - old` with `i` \
              at least `old`, `i += 1` with `i` below the output's length after the remainder, \
              and `k` grown by `BASE` while a checked `weight` multiplies by at least ten"
)]
pub(super) fn decode(input: &str) -> Option<String> {
    if !input.is_ascii() {
        return None;
    }
    let (basic, deltas) = match input.rfind('-') {
        Some(at) => (&input[..at], &input[at + 1..]),
        None => ("", input),
    };
    let mut output: Vec<u32> = basic.bytes().map(u32::from).collect();
    let (mut n, mut i, mut bias) = (INITIAL_N, 0_u32, INITIAL_BIAS);
    let mut digits = deltas.bytes().peekable();
    while digits.peek().is_some() {
        let old = i;
        let mut weight = 1_u32;
        let mut k = BASE;
        loop {
            let value = value_of(digits.next()?)?;
            i = i.checked_add(value.checked_mul(weight)?)?;
            let t = threshold(k, bias);
            if value < t {
                break;
            }
            weight = weight.checked_mul(BASE - t)?;
            k += BASE;
        }
        let length = u32::try_from(output.len()).ok()?.checked_add(1)?;
        bias = adapt(i - old, length, old == 0);
        n = n.checked_add(i / length)?;
        i %= length;
        if n < INITIAL_N {
            return None;
        }
        output.insert(usize::try_from(i).ok()?, n);
        i += 1;
    }
    output.into_iter().map(char::from_u32).collect()
}
