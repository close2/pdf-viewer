//! How much native stack Boa's parser and bytecompiler would need for a script, estimated from its
//! tokens before either is handed it (ADR 1626).
//!
//! Boa 0.22 parses and compiles by recursion and states no depth limit of its own, so a script
//! deep enough overflows the thread it runs on — which in the confined worker is the process, and
//! in [`crate::Engine`] the realm's thread. Brackets are the costly case and
//! [`crate::Budget::nesting`] bounds them; this bounds everything else as well. Measured on Boa
//! 0.22 in an optimised build, the largest construct that survives a thread of 2, 4 and 8 MiB
//! (the stack per level is the difference between two sizes, divided by the levels it bought):
//!
//! | construct | per level | levels at 8 MiB |
//! |---|---|---|
//! | `(` `[` `{a:` `` `${ `` | 22.8–25.6 KiB | 332–364 |
//! | a nested `function(){return …}` | 51.8 KiB | 162 |
//! | `!` `-` `typeof` (one token) | 3.4 KiB | 2 496 |
//! | `x=>` | 4.4 KiB | 1 888 |
//! | `{` as a block | 3.4 KiB | 2 464 |
//! | `if(x)` and `else if(…){}` | 2.7 KiB | 3 072 |
//! | `1?1:` | 2.7 KiB | 3 136 |
//! | `=a`, `**1`, `new` | 1.5 KiB | 5 568–5 760 |
//! | `+1`, `&&1`, `,1`, `()`, `.a` | 0.75–0.81 KiB | 10 368–11 136 |
//!
//! Each token is given the cost of the dearest construct it can begin: an opening bracket
//! [`BRACKET`], a prefix operator [`PREFIX`], `=>` [`ARROW`], a statement keyword [`STATEMENT`],
//! `?` [`CONDITIONAL`], an assignment or `**` [`ASSIGNMENT`], and anything else [`TOKEN`]. The
//! estimate is the dearest point of the script: the costs summed along the path from the start of
//! the statement it is in, through every bracket still open, to the token. So the sum resets where
//! ECMAScript's grammar makes two things siblings rather than one inside the other — a `;`; a line
//! break between two operands, where automatic semicolon insertion ends a statement; a comma
//! between the elements of an array, the properties of an object literal or the arguments of a
//! call — and a closing bracket gives back what its opening cost.
//!
//! **The scan reads ECMAScript's tokens as its lexical grammar fixes them, with one reading of its
//! own**: whether a `/` begins a regular expression or divides depends on the syntax around it,
//! and the scan takes it to divide after an operand and to begin a regular expression anywhere
//! else. A slash after `yield`, `await` or `let` used as a name is where that can be wrong, and a
//! script built to exploit it overflows the worker's stack and is contained by the worker's
//! process, as every script was before this bound (ADR 1609).

/// What an opening bracket costs: the dearest bracket measured, 25.6 KiB, with room over it.
pub const BRACKET: u64 = 27 << 10;
/// What a prefix operator costs — `!`, `~`, a unary `+` or `-`, `++` or `--` before an operand,
/// `typeof`, `void`, `delete`, `new`, `await`, `yield`, `...`: 3.4 KiB measured.
pub const PREFIX: u64 = 3584;
/// What `=>` costs: 4.4 KiB measured for `x=>`, of which the name is a token of its own.
pub const ARROW: u64 = 4608;
/// What a statement keyword costs — `if`, `else`, `while`, `for`, `do`, `with`, `switch`, `try`,
/// `catch`, `finally`: 2.7 KiB measured for a nested `if`.
pub const STATEMENT: u64 = 2867;
/// What `?` costs: 2.7 KiB measured for a level of `1?1:`.
pub const CONDITIONAL: u64 = 2765;
/// What an assignment operator and `**` cost: 1.5 KiB measured.
pub const ASSIGNMENT: u64 = 1536;
/// What every other token costs: 0.81 KiB measured, the dearest of the binary operators, member
/// accesses and calls.
pub const TOKEN: u64 = 820;

/// The estimate for `script`, in bytes of native stack.
#[must_use]
pub fn estimate(script: &str) -> u64 {
    Scan::new(script).run()
}

/// What a token is to the scan's two decisions: whether a `/` after it divides, and whether a line
/// break after it can end a statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// The start of the script, or after `;`: nothing precedes.
    Start,
    /// An operand's end: a name, a literal, a closing bracket.
    Operand,
    /// The `)` closing the head of `if`, `while`, `for`, `with`, `switch` or `catch`: a statement
    /// follows, inside the one the head began.
    Head,
    /// An operator, a punctuator or a keyword that is not an operand.
    Operator,
}

/// Which bracket an entry of the stack was opened by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bracket {
    /// `(`.
    Paren,
    /// `[`.
    Square,
    /// `{`.
    Brace,
    /// `${` inside a template literal.
    Substitution,
}

/// One bracket still open.
#[derive(Debug, Clone, Copy)]
struct Open {
    /// Which.
    bracket: Bracket,
    /// The sum before it opened, which closing it gives back.
    before: u64,
    /// The sum just inside it, which a sibling's start returns to.
    base: u64,
    /// Whether a comma directly inside it separates siblings.
    siblings: bool,
    /// Whether it is the head of a statement keyword.
    head: bool,
}

/// The scan's state.
struct Scan<'a> {
    /// The script.
    bytes: &'a [u8],
    /// The next byte.
    at: usize,
    /// The brackets open, innermost last.
    open: Vec<Open>,
    /// The sum at the current token.
    sum: u64,
    /// The dearest sum met.
    deepest: u64,
    /// The previous token's class.
    previous: Class,
    /// The previous token's text, where it was a keyword or a punctuator.
    word: &'a str,
    /// Whether a line break came since the previous token.
    broken: bool,
    /// Whether only white space and comments came since the last line break.
    line_start: bool,
}

/// The punctuators, longest first, so that the first that matches is the longest.
const PUNCTUATORS: [&str; 52] = [
    ">>>=", "...", "===", "!==", "**=", "<<=", ">>=", ">>>", "&&=", "||=", "??=", "=>", "==", "!=",
    "<=", ">=", "&&", "||", "??", "?.", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
    "**", "<<", ">>", ";", ",", "<", ">", "+", "-", "*", "/", "%", "&", "|", "^", "!", "~", "?",
    ":", "=", ".", "@",
];

impl<'a> Scan<'a> {
    /// A scan of `script` from its start.
    fn new(script: &'a str) -> Self {
        Self {
            bytes: script.as_bytes(),
            at: 0,
            open: Vec::new(),
            sum: 0,
            deepest: 0,
            previous: Class::Start,
            word: "",
            broken: false,
            line_start: true,
        }
    }

    /// Scans to the end and answers the dearest sum.
    fn run(mut self) -> u64 {
        while self.at < self.bytes.len() {
            self.step();
        }
        self.deepest
    }

    /// The byte at `at + ahead`, or 0 past the end.
    fn peek(&self, ahead: usize) -> u8 {
        self.bytes
            .get(self.at.saturating_add(ahead))
            .copied()
            .unwrap_or(0)
    }

    /// The character at the current byte, decoded from at most its own four bytes.
    ///
    /// Never from the rest of the script: a decode of everything left at every character made a
    /// scan of a long run of characters outside ASCII quadratic, which the `script` fuzz target
    /// found as a timeout in a string compiled at run time.
    fn character(&self) -> Option<char> {
        let end = self.at.saturating_add(4).min(self.bytes.len());
        let window = self.bytes.get(self.at..end)?;
        let valid = match std::str::from_utf8(window) {
            Ok(valid) => valid,
            Err(error) => std::str::from_utf8(window.get(..error.valid_up_to())?).ok()?,
        };
        valid.chars().next()
    }

    /// The text from `start` to the current byte.
    fn text(&self, start: usize) -> &'a str {
        std::str::from_utf8(self.bytes.get(start..self.at).unwrap_or_default()).unwrap_or("")
    }

    /// The base a sibling's start returns to: the innermost open bracket's, or nothing.
    fn base(&self) -> u64 {
        self.open.last().map_or(0, |open| open.base)
    }

    /// Adds a token's cost.
    fn add(&mut self, cost: u64) {
        self.sum = self.sum.saturating_add(cost);
        self.deepest = self.deepest.max(self.sum);
    }

    /// An operand begins: where a line break follows an operand, automatic semicolon insertion
    /// ends the statement before it, and the sum goes back to its base.
    fn operand_begins(&mut self) {
        if self.broken && self.previous == Class::Operand {
            self.sum = self.base();
        }
    }

    /// Reads one token, or white space, or a comment.
    fn step(&mut self) {
        let byte = self.peek(0);
        match byte {
            b'\n' | b'\r' => {
                self.at = self.at.saturating_add(1);
                self.broken = true;
                self.line_start = true;
            }
            b' ' | b'\t' | 0x0B | 0x0C => self.at = self.at.saturating_add(1),
            b'/' if self.peek(1) == b'/' => self.line_comment(),
            b'/' if self.peek(1) == b'*' => self.block_comment(),
            b'<' if self.bytes.get(self.at..self.at.saturating_add(4)) == Some(b"<!--") => {
                self.line_comment();
            }
            b'-' if self.line_start
                && self.bytes.get(self.at..self.at.saturating_add(3)) == Some(b"-->") =>
            {
                self.line_comment();
            }
            b'\'' | b'"' => {
                self.operand_begins();
                self.string(byte);
                self.token(Class::Operand, "", TOKEN);
            }
            b'`' => {
                self.at = self.at.saturating_add(1);
                self.template();
            }
            b'/' if self.previous != Class::Operand => {
                self.operand_begins();
                self.regular_expression();
                self.token(Class::Operand, "", TOKEN);
            }
            b'0'..=b'9' => self.number(),
            b'.' if self.peek(1).is_ascii_digit() => self.number(),
            b'(' | b'[' | b'{' => self.open_bracket(byte),
            b')' | b']' | b'}' => self.close_bracket(byte),
            _ if byte >= 0x80 => self.non_ascii(),
            _ if byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$' | b'\\' | b'#') => {
                self.name();
            }
            _ => self.punctuator(),
        }
    }

    /// Records a token of `class` and `cost`.
    fn token(&mut self, class: Class, word: &'a str, cost: u64) {
        self.add(cost);
        self.previous = class;
        self.word = word;
        self.broken = false;
        self.line_start = false;
    }

    /// `//` to the end of the line, and the two HTML-like comments Annex B adds.
    fn line_comment(&mut self) {
        while self.at < self.bytes.len() && !matches!(self.peek(0), b'\n' | b'\r') {
            self.at = self.at.saturating_add(1);
        }
    }

    /// `/* … */`, which counts as a line break where it holds one.
    fn block_comment(&mut self) {
        self.at = self.at.saturating_add(2);
        while self.at < self.bytes.len() {
            if self.peek(0) == b'*' && self.peek(1) == b'/' {
                self.at = self.at.saturating_add(2);
                return;
            }
            if matches!(self.peek(0), b'\n' | b'\r') {
                self.broken = true;
            }
            self.at = self.at.saturating_add(1);
        }
    }

    /// A string literal opened by `quote`, to its closing quote or its line's end.
    fn string(&mut self, quote: u8) {
        self.at = self.at.saturating_add(1);
        while self.at < self.bytes.len() {
            match self.peek(0) {
                b'\\' => self.at = self.at.saturating_add(2),
                b'\n' | b'\r' => return,
                byte if byte == quote => {
                    self.at = self.at.saturating_add(1);
                    return;
                }
                _ => self.at = self.at.saturating_add(1),
            }
        }
    }

    /// A regular expression literal, to its closing slash outside a class, or its line's end.
    fn regular_expression(&mut self) {
        self.at = self.at.saturating_add(1);
        let mut class = false;
        while self.at < self.bytes.len() {
            match self.peek(0) {
                b'\\' => self.at = self.at.saturating_add(2),
                b'\n' | b'\r' => return,
                b'[' => {
                    class = true;
                    self.at = self.at.saturating_add(1);
                }
                b']' => {
                    class = false;
                    self.at = self.at.saturating_add(1);
                }
                b'/' if !class => {
                    self.at = self.at.saturating_add(1);
                    while self.peek(0).is_ascii_alphanumeric() {
                        self.at = self.at.saturating_add(1);
                    }
                    return;
                }
                _ => self.at = self.at.saturating_add(1),
            }
        }
    }

    /// A template literal's text, from just after its backtick or a substitution's `}`, to its
    /// closing backtick or its next `${`.
    fn template(&mut self) {
        while self.at < self.bytes.len() {
            match self.peek(0) {
                b'\\' => self.at = self.at.saturating_add(2),
                b'`' => {
                    self.at = self.at.saturating_add(1);
                    self.token(Class::Operand, "", TOKEN);
                    return;
                }
                b'$' if self.peek(1) == b'{' => {
                    self.at = self.at.saturating_add(2);
                    let before = self.sum;
                    self.add(BRACKET);
                    self.open.push(Open {
                        bracket: Bracket::Substitution,
                        before,
                        base: self.sum,
                        siblings: false,
                        head: false,
                    });
                    self.previous = Class::Operator;
                    self.word = "${";
                    self.broken = false;
                    return;
                }
                _ => self.at = self.at.saturating_add(1),
            }
        }
    }

    /// A numeric literal.
    fn number(&mut self) {
        self.operand_begins();
        let start = self.at;
        while self.at < self.bytes.len() {
            let byte = self.peek(0);
            let exponent_sign = matches!(byte, b'+' | b'-')
                && matches!(self.bytes.get(self.at.wrapping_sub(1)), Some(b'e' | b'E'))
                && !self.text(start).starts_with("0x")
                && !self.text(start).starts_with("0X");
            if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_') || exponent_sign {
                self.at = self.at.saturating_add(1);
            } else {
                break;
            }
        }
        self.token(Class::Operand, "", TOKEN);
    }

    /// A character outside ASCII: the two line separators and the spaces ECMAScript names are
    /// white space, and anything else is taken as part of a name.
    fn non_ascii(&mut self) {
        match self.character() {
            Some('\u{2028}' | '\u{2029}') => {
                self.at = self.at.saturating_add(3);
                self.broken = true;
                self.line_start = true;
            }
            Some(character) if character.is_whitespace() || character == '\u{FEFF}' => {
                self.at = self.at.saturating_add(character.len_utf8());
            }
            _ => self.name(),
        }
    }

    /// A name, or a keyword.
    fn name(&mut self) {
        let start = self.at;
        while self.at < self.bytes.len() {
            let byte = self.peek(0);
            if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'#') {
                self.at = self.at.saturating_add(1);
            } else if byte == b'\\' {
                self.at = self.at.saturating_add(2);
            } else if byte >= 0x80 {
                match self.character() {
                    Some(character)
                        if !character.is_whitespace()
                            && !matches!(character, '\u{2028}' | '\u{2029}' | '\u{FEFF}') =>
                    {
                        self.at = self.at.saturating_add(character.len_utf8());
                    }
                    Some(_) => break,
                    None => self.at = self.at.saturating_add(1),
                }
            } else {
                break;
            }
        }
        if self.at == start {
            self.at = self.at.saturating_add(1);
        }
        let word = self.text(start);
        // A name after `.` or `?.` is a property's, whatever it spells.
        let property = matches!(self.word, "." | "?.");
        let (class, cost) = if property {
            (Class::Operand, TOKEN)
        } else {
            // A name is an operand, `this`, `null`, `true`, `false` and `super` among them.
            match word {
                "typeof" | "void" | "delete" | "new" | "await" | "yield" => {
                    (Class::Operator, PREFIX)
                }
                "if" | "else" | "while" | "for" | "do" | "with" | "switch" | "try" | "catch"
                | "finally" => (Class::Operator, STATEMENT),
                "in" | "instanceof" | "var" | "const" | "function" | "class" | "return"
                | "throw" | "case" | "default" | "break" | "continue" | "debugger" | "import"
                | "export" | "extends" => (Class::Operator, TOKEN),
                _ => (Class::Operand, TOKEN),
            }
        };
        // A line break before `else`, `catch`, `finally`, `while` or a binary keyword does not end
        // a statement: each continues the one before it.
        let continues = !property
            && matches!(
                word,
                "else" | "catch" | "finally" | "while" | "in" | "instanceof"
            );
        if !continues {
            self.operand_begins();
        }
        self.token(class, word, cost);
    }

    /// An opening bracket.
    fn open_bracket(&mut self, byte: u8) {
        let head = byte == b'('
            && self.previous == Class::Operator
            && matches!(
                self.word,
                "if" | "while" | "for" | "with" | "switch" | "catch"
            );
        let siblings = match byte {
            b'[' => self.previous != Class::Operand,
            // A call's arguments or a function's parameters.
            b'(' => !head && (self.previous == Class::Operand || self.word == "function"),
            // An object literal follows an operator; a block follows a statement's head, `=>`,
            // `:` (a label's or a case's), a keyword that takes a block, or nothing.
            _ => {
                self.previous == Class::Operator
                    && !matches!(
                        self.word,
                        "=>" | ":" | "else" | "do" | "try" | "finally" | ";"
                    )
            }
        };
        let bracket = match byte {
            b'(' => Bracket::Paren,
            b'[' => Bracket::Square,
            _ => Bracket::Brace,
        };
        if byte != b'(' && byte != b'[' {
            // A block or an object literal after a line break may begin a statement.
            self.operand_begins();
        }
        self.at = self.at.saturating_add(1);
        let before = self.sum;
        self.add(BRACKET);
        self.open.push(Open {
            bracket,
            before,
            base: self.sum,
            siblings,
            head,
        });
        self.previous = Class::Operator;
        self.word = "";
        self.broken = false;
        self.line_start = false;
    }

    /// A closing bracket: the sum goes back to what it was before the bracket opened, plus the
    /// bracketed group as one operand. A bracket that closes none open, or closes another kind, is
    /// a syntax error the parser will name; the scan keeps its sum, which errs towards the dearer.
    fn close_bracket(&mut self, byte: u8) {
        self.at = self.at.saturating_add(1);
        let closes = match byte {
            b')' => Bracket::Paren,
            b']' => Bracket::Square,
            _ => Bracket::Brace,
        };
        let top = self.open.last().copied();
        match top {
            Some(open) if open.bracket == Bracket::Substitution && closes == Bracket::Brace => {
                self.open.pop();
                self.sum = open.before;
                self.template();
            }
            Some(open) if open.bracket == closes => {
                self.open.pop();
                self.sum = open.before;
                let class = if open.head {
                    Class::Head
                } else {
                    Class::Operand
                };
                self.token(class, "", TOKEN);
            }
            _ => self.token(Class::Operand, "", TOKEN),
        }
    }

    /// A punctuator.
    fn punctuator(&mut self) {
        let rest = self.bytes.get(self.at..).unwrap_or_default();
        let found = PUNCTUATORS
            .iter()
            .find(|punctuator| rest.starts_with(punctuator.as_bytes()))
            .copied()
            .unwrap_or("");
        // `?.` before a digit is `?` and a number: ECMAScript's own disambiguation.
        let found = if found == "?." && self.peek(2).is_ascii_digit() {
            "?"
        } else {
            found
        };
        if found.is_empty() {
            self.at = self.at.saturating_add(1);
            return;
        }
        self.at = self.at.saturating_add(found.len());
        let prefix_position = self.previous != Class::Operand;
        let cost = match found {
            ";" => {
                self.sum = self.base();
                self.previous = Class::Start;
                self.word = ";";
                self.broken = false;
                self.line_start = false;
                return;
            }
            "," => {
                if self.open.last().is_some_and(|open| open.siblings) {
                    self.sum = self.base();
                    self.previous = Class::Operator;
                    self.word = ",";
                    self.broken = false;
                    return;
                }
                TOKEN
            }
            "=>" => ARROW,
            "?" => CONDITIONAL,
            "!" | "~" | "..." => PREFIX,
            "+" | "-" | "++" | "--" if prefix_position => PREFIX,
            "**" | "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "**=" | "<<=" | ">>=" | ">>>="
            | "&=" | "|=" | "^=" | "&&=" | "||=" | "??=" => ASSIGNMENT,
            _ => TOKEN,
        };
        // A postfix `++` or `--` ends an operand; everything else leaves an operator before what
        // follows.
        let class = if matches!(found, "++" | "--") && !prefix_position {
            Class::Operand
        } else {
            Class::Operator
        };
        self.token(class, found, cost);
    }
}

#[cfg(test)]
mod tests {
    use super::{BRACKET, PREFIX, TOKEN, estimate};

    #[test]
    fn a_chain_without_brackets_costs_a_token_a_term() {
        let chain = format!("var x = 1{};", "+1".repeat(20_000));
        assert!(estimate(&chain) >= 40_000 * TOKEN);
        let unary = format!("var x = {}1;", "!".repeat(3000));
        assert!(estimate(&unary) >= 3000 * PREFIX);
    }

    #[test]
    fn statements_are_siblings_and_cost_only_their_own_length() {
        let script = "var x = 1 + 2;\n".repeat(5000);
        assert!(estimate(&script) < 10 * TOKEN);
        let without = "x = 1 + 2\ny = x * 2\n".repeat(5000);
        assert!(
            estimate(&without) < 10 * TOKEN,
            "a line break between operands ends one"
        );
        let functions = "function f() { return 1; }\n".repeat(2000);
        assert!(estimate(&functions) < BRACKET * 2);
    }

    #[test]
    fn a_list_s_elements_are_siblings_and_a_sequence_s_are_not() {
        let array = format!("var t = [{}];", "1, ".repeat(20_000));
        assert!(estimate(&array) < BRACKET + 10 * TOKEN);
        let object = format!("var t = {{{}}};", "a: 1, ".repeat(20_000));
        assert!(estimate(&object) < BRACKET + 10 * TOKEN);
        let call = format!("f({});", "1, ".repeat(20_000));
        assert!(estimate(&call) < BRACKET + 10 * TOKEN);
        let sequence = format!("var t = ({});", "1, ".repeat(20_000));
        assert!(estimate(&sequence) > 20_000 * TOKEN);
    }

    #[test]
    fn what_a_string_comment_or_expression_holds_is_not_read_as_code() {
        let hidden = format!(
            "x = '{}'; /* {} */ y = /;[;]/; // ;",
            ";".repeat(99),
            "(".repeat(99)
        );
        assert!(estimate(&hidden) < 10 * TOKEN);
        // A `;` inside a string does not end the chain it sits in.
        let chain = format!("var x = 1{};", "+';'".repeat(5000));
        assert!(estimate(&chain) > 5000 * TOKEN);
    }

    #[test]
    fn an_else_if_chain_on_its_own_lines_nests() {
        let chain = format!("if (x) {{}}\n{}", "else if (x) {}\n".repeat(2000));
        assert!(estimate(&chain) > 2000 * 2 * super::STATEMENT);
    }

    /// The `script` fuzz target's timeout: a run of six hundred thousand characters outside ASCII
    /// compiled with `Function`, which a decode of the whole remainder per character made
    /// quadratic.
    #[test]
    fn a_long_run_of_characters_outside_ascii_is_scanned_once() {
        let started = std::time::Instant::now();
        let unit = format!("{}!", "\u{FFFD}".repeat(125));
        let long = format!("return {}1", unit.repeat(5000));
        let _ = estimate(&long);
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }

    #[test]
    fn brackets_cost_what_they_nest() {
        let nested = format!("x = {}1{};", "(".repeat(100), ")".repeat(100));
        assert!(estimate(&nested) >= 100 * BRACKET);
        let template = format!("x = {}1{};", "`${".repeat(50), "}`".repeat(50));
        assert!(estimate(&template) >= 50 * BRACKET);
    }
}
