//! `FormCalc`'s syntactic grammar: XFA 3.3 chapter 25, *Syntactic Grammar* and *Basic
//! Expressions* (pages 1058–1076), productions 27 to 105, cited and paraphrased.
//!
//! The whole grammar is parsed, so that a script outside the subset [`crate::eval`] runs is
//! refused by the construct it uses rather than by a syntax error it does not have. The parser
//! descends recursively and counts its depth against [`MAX_NESTING`], and keeps the depth of the
//! binary operators in the tree it builds within [`MAX_OPERATOR_DEPTH`], because a chain
//! `1 + 1 + …` is folded by a loop yet is a tree one level deeper per operator, which the
//! evaluator and the drop walk recursively. So a hostile script's nesting or chain costs a
//! refusal and never the stack.

use std::fmt;

use crate::lex::{self, Keyword, Kind, LexError, Punct, Token};

/// How deep expressions may nest before a script is refused: far past any script a form
/// designer writes, and far inside the stack a recursive descent of this grammar uses.
pub const MAX_NESTING: usize = 64;

/// How deep binary operators may stand in the expression tree before a script is refused: the
/// most binary nodes on any path from the root to a leaf, counted through parentheses, blocks and
/// arguments alike, because the evaluator and the drop recurse down every such path on the stack
/// of the thread they run on. A chain of `n` operators is `n` deep, and so is a chain whose first
/// operand is a parenthesised chain of its own, summed. 200 is about thirty times the longest
/// chain in the 502 distinct `FormCalc` scripts the census printed whole, which is 7 operators,
/// and a 256 KiB thread walks a path that deep (ADR 2482).
pub const MAX_OPERATOR_DEPTH: usize = 200;

/// One expression of production 29.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Production 11's number literal.
    Number(f64),
    /// Production 8's string literal.
    String(String),
    /// Production 12's `null`.
    Null,
    /// `nan`, `inf` or `infinity`, which page 1057 says `FormCalc` does not support as values.
    NotANumber(usize),
    /// Production 90's accessor, read for its value.
    Accessor(Accessor),
    /// Production 100's function call: the name as written, the arguments, the byte offset.
    Call(String, Vec<Expr>, usize),
    /// Production 80's unary expression.
    Unary(Unary, Box<Expr>),
    /// Productions 68 to 78's binary expressions.
    Binary(Binary, Box<Expr>, Box<Expr>),
    /// Production 87's assignment.
    Assign(Accessor, Box<Expr>),
    /// Production 84's variable declaration, with its initial value if it states one.
    Var(String, Option<Box<Expr>>),
    /// Production 84's user-defined function: its name, its parameters, its body.
    Func(String, Vec<String>, Vec<Expr>),
    /// Production 94: each `if`/`elseif` condition with its list, then the `else` list.
    If(Vec<(Expr, Vec<Expr>)>, Option<Vec<Expr>>),
    /// Production 97.
    While(Box<Expr>, Vec<Expr>),
    /// Production 95: the variable, whether it is declared, its start, `downto`, the end, the
    /// step, the body.
    For(For),
    /// Production 96: the variable, the argument list, the body.
    Foreach(String, Vec<Expr>, Vec<Expr>),
    /// Production 98.
    Block(Vec<Expr>),
    /// Production 92.
    Break,
    /// Production 93.
    Continue,
}

/// Production 95's parts.
#[derive(Debug, Clone, PartialEq)]
pub struct For {
    /// The loop variable.
    pub variable: String,
    /// Whether the loop declares it with `var`.
    pub declares: bool,
    /// Its start value.
    pub start: Box<Expr>,
    /// `downto` rather than `upto`.
    pub down: bool,
    /// The end value.
    pub end: Box<Expr>,
    /// The step, `1` or `-1` by default (page 1072).
    pub step: Option<Box<Expr>>,
    /// The body.
    pub body: Vec<Expr>,
}

/// Production 81's operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unary {
    /// `-`.
    Minus,
    /// `+`.
    Plus,
    /// `not`.
    Not,
}

/// Productions 69 to 79's operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binary {
    /// `|`, `or`.
    Or,
    /// `&`, `and`.
    And,
    /// `==`, `eq`.
    Equal,
    /// `<>`, `ne`.
    NotEqual,
    /// `<`, `lt`.
    Less,
    /// `<=`, `le`.
    LessEqual,
    /// `>`, `gt`.
    Greater,
    /// `>=`, `ge`.
    GreaterEqual,
    /// `+`.
    Add,
    /// `-`.
    Subtract,
    /// `*`.
    Multiply,
    /// `/`.
    Divide,
}

/// Production 90's accessor: containers joined by separators, optionally ending in `.*`.
#[derive(Debug, Clone, PartialEq)]
pub struct Accessor {
    /// The containers in order; the first has [`Separator::First`].
    pub steps: Vec<Step>,
    /// Whether it ends in production 88's `.*`, every child of what it names.
    pub every_child: bool,
    /// The byte offset it begins at.
    pub at: usize,
}

/// One container of production 91.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// What joins it to the step before.
    pub separator: Separator,
    /// Its name, or `this` for the keyword of that name.
    pub name: String,
    /// Its occurrence.
    pub index: Index,
    /// The arguments, where the container is production 103's method call.
    pub call: Option<Vec<Expr>>,
}

/// Production 90's separators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separator {
    /// The first container, joined to nothing.
    First,
    /// `.`, a child.
    Dot,
    /// `..`, a descendant.
    DotDot,
    /// `.#`, a property by its class.
    DotHash,
}

/// The occurrence a container names (page 1067's notation table).
#[derive(Debug, Clone, PartialEq)]
pub enum Index {
    /// No index: the occurrence that bears the referencing object's own number.
    Same,
    /// `[*]`, every occurrence.
    Every,
    /// `[e]`, the occurrence `e` evaluates to.
    At(Box<Expr>),
    /// `[+e]` or `[-e]`, relative to the referencing object's occurrence; `true` for `-`.
    Relative(bool, Box<Expr>),
}

/// Why a script is not a production 27 `FormCalculation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Its text is not a sequence of tokens.
    Lex(LexError),
    /// A token the grammar does not allow where it stands, at this byte offset.
    Unexpected(String, usize),
    /// The script ended where the grammar needs more.
    UnexpectedEnd,
    /// It nests deeper than [`MAX_NESTING`].
    TooDeep,
    /// Its binary operators stand deeper than [`MAX_OPERATOR_DEPTH`] in the expression tree.
    OperatorsTooDeep,
    /// A reserved keyword of production 16 that no production of the syntactic grammar uses:
    /// `throw`, `return` or `exit`, at this byte offset.
    Reserved(&'static str, usize),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lex(error) => error.fmt(f),
            Self::Unexpected(token, at) => write!(f, "unexpected {token} at byte {at}"),
            Self::UnexpectedEnd => f.write_str("the script ends inside an expression"),
            Self::TooDeep => write!(f, "the script nests deeper than {MAX_NESTING} expressions"),
            Self::OperatorsTooDeep => write!(
                f,
                "the script's operators stand deeper than {MAX_OPERATOR_DEPTH} in its expression tree"
            ),
            Self::Reserved(word, at) => write!(
                f,
                "the reserved keyword {word} at byte {at} belongs to no production of the grammar"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(error: LexError) -> Self {
        Self::Lex(error)
    }
}

/// Production 27: the script's expression list.
///
/// # Errors
///
/// [`ParseError`] where the text is not a `FormCalculation`.
pub fn parse(source: &str) -> Result<Vec<Expr>, ParseError> {
    let mut parser = Parser {
        tokens: lex::tokens(source)?,
        at: 0,
        depth: 0,
        height: 0,
    };
    let list = parser.list()?;
    match parser.tokens.get(parser.at) {
        None => Ok(list),
        Some(token) => Err(unexpected(token)),
    }
}

fn unexpected(token: &Token) -> ParseError {
    let shown = match &token.kind {
        Kind::Number(n) => format!("number {n}"),
        Kind::String(_) => "string literal".to_owned(),
        Kind::Identifier(name) => format!("identifier {name}"),
        Kind::Keyword(keyword) => format!("keyword {keyword:?}").to_ascii_lowercase(),
        Kind::Punct(punct) => format!("`{}`", punct.spelling()),
    };
    ParseError::Unexpected(shown, token.at)
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    depth: usize,
    /// The deepest binary operator of what has been parsed since [`Parser::binary`] last cleared
    /// it: the operand being parsed, every expression nested in it included.
    height: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Kind> {
        self.tokens.get(self.at).map(|t| &t.kind)
    }

    fn offset(&self) -> usize {
        self.tokens.get(self.at).map_or(usize::MAX, |t| t.at)
    }

    /// Moves past the token just looked at; the index stays within the token list, so saturating
    /// is only the lint's proof.
    fn step(&mut self) {
        self.at = self.at.saturating_add(1);
    }

    fn bump(&mut self) -> Result<Token, ParseError> {
        let token = self
            .tokens
            .get(self.at)
            .cloned()
            .ok_or(ParseError::UnexpectedEnd)?;
        self.step();
        Ok(token)
    }

    fn eat_keyword(&mut self, keyword: Keyword) -> bool {
        if self.peek() == Some(&Kind::Keyword(keyword)) {
            self.step();
            true
        } else {
            false
        }
    }

    fn eat_punct(&mut self, punct: Punct) -> bool {
        if self.peek() == Some(&Kind::Punct(punct)) {
            self.step();
            true
        } else {
            false
        }
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> Result<(), ParseError> {
        let token = self.bump()?;
        if token.kind == Kind::Keyword(keyword) {
            Ok(())
        } else {
            Err(unexpected(&token))
        }
    }

    fn expect_punct(&mut self, punct: Punct) -> Result<(), ParseError> {
        let token = self.bump()?;
        if token.kind == Kind::Punct(punct) {
            Ok(())
        } else {
            Err(unexpected(&token))
        }
    }

    fn identifier(&mut self) -> Result<String, ParseError> {
        let token = self.bump()?;
        match token.kind {
            Kind::Identifier(name) => Ok(name),
            _ => Err(unexpected(&token)),
        }
    }

    fn nest(&mut self) -> Result<(), ParseError> {
        self.depth = self.depth.saturating_add(1);
        if self.depth > MAX_NESTING {
            return Err(ParseError::TooDeep);
        }
        Ok(())
    }

    /// Production 28: expressions until a keyword that closes the list around them, or the end.
    fn list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut list = Vec::new();
        while let Some(kind) = self.peek() {
            if matches!(
                kind,
                Kind::Keyword(
                    Keyword::Endif
                        | Keyword::Else
                        | Keyword::Elseif
                        | Keyword::Endwhile
                        | Keyword::Endfor
                        | Keyword::End
                        | Keyword::Endfunc
                )
            ) {
                break;
            }
            list.push(self.expression()?);
        }
        Ok(list)
    }

    /// Production 29.
    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.nest()?;
        let at = self.offset();
        let expression = match self.peek() {
            Some(Kind::Keyword(Keyword::If)) => self.if_expression(),
            Some(Kind::Keyword(Keyword::While)) => self.while_expression(),
            Some(Kind::Keyword(Keyword::For)) => self.for_expression(),
            Some(Kind::Keyword(Keyword::Foreach)) => self.foreach_expression(),
            Some(Kind::Keyword(Keyword::Do)) => {
                self.step();
                let body = self.list()?;
                self.expect_keyword(Keyword::End)?;
                Ok(Expr::Block(body))
            }
            Some(Kind::Keyword(Keyword::Var)) => {
                self.step();
                let name = self.identifier()?;
                let init = if self.eat_punct(Punct::Assign) {
                    Some(Box::new(self.simple()?))
                } else {
                    None
                };
                Ok(Expr::Var(name, init))
            }
            Some(Kind::Keyword(Keyword::Func)) => self.func(),
            Some(Kind::Keyword(Keyword::Break)) => {
                self.step();
                Ok(Expr::Break)
            }
            Some(Kind::Keyword(Keyword::Continue)) => {
                self.step();
                Ok(Expr::Continue)
            }
            Some(Kind::Keyword(Keyword::Throw)) => Err(ParseError::Reserved("throw", at)),
            Some(Kind::Keyword(Keyword::Return)) => Err(ParseError::Reserved("return", at)),
            Some(Kind::Keyword(Keyword::Exit)) => Err(ParseError::Reserved("exit", at)),
            _ => {
                let simple = self.simple()?;
                if self.peek() == Some(&Kind::Punct(Punct::Assign)) {
                    let Expr::Accessor(target) = simple else {
                        return Err(unexpected(&self.tokens[self.at]));
                    };
                    self.step();
                    Ok(Expr::Assign(target, Box::new(self.simple()?)))
                } else {
                    Ok(simple)
                }
            }
        };
        self.depth = self.depth.saturating_sub(1);
        expression
    }

    fn condition(&mut self) -> Result<Expr, ParseError> {
        self.expect_punct(Punct::OpenParen)?;
        let condition = self.simple()?;
        self.expect_punct(Punct::CloseParen)?;
        Ok(condition)
    }

    /// Production 94.
    fn if_expression(&mut self) -> Result<Expr, ParseError> {
        self.step();
        let mut arms = Vec::new();
        let condition = self.condition()?;
        self.expect_keyword(Keyword::Then)?;
        arms.push((condition, self.list()?));
        let mut otherwise = None;
        loop {
            if self.eat_keyword(Keyword::Elseif) {
                let condition = self.condition()?;
                self.expect_keyword(Keyword::Then)?;
                arms.push((condition, self.list()?));
            } else if self.eat_keyword(Keyword::Else) {
                otherwise = Some(self.list()?);
                self.expect_keyword(Keyword::Endif)?;
                break;
            } else {
                self.expect_keyword(Keyword::Endif)?;
                break;
            }
        }
        Ok(Expr::If(arms, otherwise))
    }

    /// Production 97.
    fn while_expression(&mut self) -> Result<Expr, ParseError> {
        self.step();
        let condition = self.condition()?;
        self.expect_keyword(Keyword::Do)?;
        let body = self.list()?;
        self.expect_keyword(Keyword::Endwhile)?;
        Ok(Expr::While(Box::new(condition), body))
    }

    /// Production 95, with page 1073's examples' spellings: the assignment may declare its
    /// variable with `var`, and the end is a simple expression (`upto 100`) where the production
    /// writes an accessor.
    fn for_expression(&mut self) -> Result<Expr, ParseError> {
        self.step();
        let declares = self.eat_keyword(Keyword::Var);
        let variable = self.identifier()?;
        self.expect_punct(Punct::Assign)?;
        let start = Box::new(self.simple()?);
        let down = if self.eat_keyword(Keyword::Downto) {
            true
        } else {
            self.expect_keyword(Keyword::Upto)?;
            false
        };
        let end = Box::new(self.simple()?);
        let step = if self.eat_keyword(Keyword::Step) {
            Some(Box::new(self.simple()?))
        } else {
            None
        };
        self.expect_keyword(Keyword::Do)?;
        let body = self.list()?;
        self.expect_keyword(Keyword::Endfor)?;
        Ok(Expr::For(For {
            variable,
            declares,
            start,
            down,
            end,
            step,
            body,
        }))
    }

    /// Production 96.
    fn foreach_expression(&mut self) -> Result<Expr, ParseError> {
        self.step();
        let variable = self.identifier()?;
        self.expect_keyword(Keyword::In)?;
        self.expect_punct(Punct::OpenParen)?;
        let arguments = self.arguments()?;
        self.expect_keyword(Keyword::Do)?;
        let body = self.list()?;
        self.expect_keyword(Keyword::Endfor)?;
        Ok(Expr::Foreach(variable, arguments, body))
    }

    /// Production 84's `func`.
    fn func(&mut self) -> Result<Expr, ParseError> {
        self.step();
        let name = self.identifier()?;
        self.expect_punct(Punct::OpenParen)?;
        let mut parameters = Vec::new();
        if !self.eat_punct(Punct::CloseParen) {
            loop {
                parameters.push(self.identifier()?);
                if self.eat_punct(Punct::CloseParen) {
                    break;
                }
                self.expect_punct(Punct::Comma)?;
            }
        }
        self.expect_keyword(Keyword::Do)?;
        let body = self.list()?;
        self.expect_keyword(Keyword::Endfunc)?;
        Ok(Expr::Func(name, parameters, body))
    }

    /// Production 102's argument list after its `(`, through its `)`.
    fn arguments(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut arguments = Vec::new();
        if self.eat_punct(Punct::CloseParen) {
            return Ok(arguments);
        }
        loop {
            arguments.push(self.simple()?);
            if self.eat_punct(Punct::CloseParen) {
                return Ok(arguments);
            }
            self.expect_punct(Punct::Comma)?;
        }
    }

    /// Production 67.
    fn simple(&mut self) -> Result<Expr, ParseError> {
        self.nest()?;
        let expression = self.binary(0);
        self.depth = self.depth.saturating_sub(1);
        expression
    }

    /// Productions 68 to 79 by precedence climbing, lowest first (page 1061): `|`, `&`,
    /// equality, relational, additive, multiplicative; each level associates to the left.
    fn binary(&mut self, level: usize) -> Result<Expr, ParseError> {
        if level == 6 {
            return self.unary();
        }
        // Each operand's height is measured from zero; what the enclosing operand had already
        // reached is restored, as the larger of the two, once this expression is built.
        let outer = std::mem::take(&mut self.height);
        let mut left = self.binary(level.saturating_add(1))?;
        let mut height = self.height;
        while let Some(operator) = self.peek().and_then(|kind| operator_at(level, kind)) {
            self.step();
            self.height = 0;
            let right = self.binary(level.saturating_add(1))?;
            height = height.max(self.height).saturating_add(1);
            if height > MAX_OPERATOR_DEPTH {
                return Err(ParseError::OperatorsTooDeep);
            }
            left = Expr::Binary(operator, Box::new(left), Box::new(right));
        }
        self.height = outer.max(height);
        Ok(left)
    }

    /// Production 80.
    fn unary(&mut self) -> Result<Expr, ParseError> {
        let operator = match self.peek() {
            Some(Kind::Punct(Punct::Minus)) => Unary::Minus,
            Some(Kind::Punct(Punct::Plus)) => Unary::Plus,
            Some(Kind::Keyword(Keyword::Not)) => Unary::Not,
            _ => return self.primary(),
        };
        self.step();
        self.nest()?;
        let operand = self.unary();
        self.depth = self.depth.saturating_sub(1);
        Ok(Expr::Unary(operator, Box::new(operand?)))
    }

    /// Production 82, with production 100's function call told from production 90's accessor
    /// by what follows the first identifier.
    fn primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.bump()?;
        match token.kind {
            Kind::Number(n) => Ok(Expr::Number(n)),
            Kind::String(text) => Ok(Expr::String(text)),
            Kind::Keyword(Keyword::Null) => Ok(Expr::Null),
            Kind::Keyword(Keyword::Nan | Keyword::Inf | Keyword::Infinity) => {
                Ok(Expr::NotANumber(token.at))
            }
            Kind::Punct(Punct::OpenParen) => {
                let inner = self.simple()?;
                self.expect_punct(Punct::CloseParen)?;
                Ok(inner)
            }
            Kind::Identifier(name) if self.peek() == Some(&Kind::Punct(Punct::OpenParen)) => {
                self.step();
                let arguments = self.arguments()?;
                if self.continues_accessor() {
                    // `name(...)` followed by `.` is a method call heading an accessor.
                    let first = Step {
                        separator: Separator::First,
                        name,
                        index: Index::Same,
                        call: Some(arguments),
                    };
                    self.accessor(first, token.at)
                } else {
                    Ok(Expr::Call(name, arguments, token.at))
                }
            }
            Kind::Identifier(name) => {
                let first = self.container(Separator::First, name)?;
                self.accessor(first, token.at)
            }
            Kind::Keyword(Keyword::This) => {
                let first = self.container(Separator::First, "this".to_owned())?;
                self.accessor(first, token.at)
            }
            _ => Err(unexpected(&token)),
        }
    }

    fn continues_accessor(&self) -> bool {
        matches!(
            self.peek(),
            Some(Kind::Punct(
                Punct::Dot | Punct::DotDot | Punct::DotHash | Punct::DotStar
            ))
        )
    }

    /// Production 90 after its first container.
    fn accessor(&mut self, first: Step, at: usize) -> Result<Expr, ParseError> {
        let mut steps = vec![first];
        let mut every_child = false;
        while self.continues_accessor() {
            let separator = match self.bump()?.kind {
                Kind::Punct(Punct::Dot) => Separator::Dot,
                Kind::Punct(Punct::DotDot) => Separator::DotDot,
                Kind::Punct(Punct::DotHash) => Separator::DotHash,
                _ => {
                    every_child = true;
                    break;
                }
            };
            let token = self.bump()?;
            let name = match token.kind {
                Kind::Identifier(name) => name,
                // A property may share a keyword's spelling (`.#value` never does, but a
                // container named like an operator keyword is reachable only this way).
                Kind::Keyword(keyword) => format!("{keyword:?}").to_ascii_lowercase(),
                _ => return Err(unexpected(&token)),
            };
            let step = if self.eat_punct(Punct::OpenParen) {
                Step {
                    separator,
                    name,
                    index: Index::Same,
                    call: Some(self.arguments()?),
                }
            } else {
                self.container(separator, name)?
            };
            steps.push(step);
        }
        Ok(Expr::Accessor(Accessor {
            steps,
            every_child,
            at,
        }))
    }

    /// Production 91's optional index after a container's name.
    fn container(&mut self, separator: Separator, name: String) -> Result<Step, ParseError> {
        let index = if self.eat_punct(Punct::OpenBracket) {
            let index = if self.eat_punct(Punct::Star) {
                Index::Every
            } else if self.peek() == Some(&Kind::Punct(Punct::Plus))
                || self.peek() == Some(&Kind::Punct(Punct::Minus))
            {
                let back = self.peek() == Some(&Kind::Punct(Punct::Minus));
                self.step();
                Index::Relative(back, Box::new(self.simple()?))
            } else {
                Index::At(Box::new(self.simple()?))
            };
            self.expect_punct(Punct::CloseBracket)?;
            index
        } else {
            Index::Same
        };
        Ok(Step {
            separator,
            name,
            index,
            call: None,
        })
    }
}

/// The operator `kind` spells at precedence `level`, 0 the loosest.
fn operator_at(level: usize, kind: &Kind) -> Option<Binary> {
    let operator = match kind {
        Kind::Punct(Punct::Bar) | Kind::Keyword(Keyword::Or) => Binary::Or,
        Kind::Punct(Punct::Ampersand) | Kind::Keyword(Keyword::And) => Binary::And,
        Kind::Punct(Punct::Equal) | Kind::Keyword(Keyword::Eq) => Binary::Equal,
        Kind::Punct(Punct::NotEqual) | Kind::Keyword(Keyword::Ne) => Binary::NotEqual,
        Kind::Punct(Punct::Less) | Kind::Keyword(Keyword::Lt) => Binary::Less,
        Kind::Punct(Punct::LessEqual) | Kind::Keyword(Keyword::Le) => Binary::LessEqual,
        Kind::Punct(Punct::Greater) | Kind::Keyword(Keyword::Gt) => Binary::Greater,
        Kind::Punct(Punct::GreaterEqual) | Kind::Keyword(Keyword::Ge) => Binary::GreaterEqual,
        Kind::Punct(Punct::Plus) => Binary::Add,
        Kind::Punct(Punct::Minus) => Binary::Subtract,
        Kind::Punct(Punct::Star) => Binary::Multiply,
        Kind::Punct(Punct::Slash) => Binary::Divide,
        _ => return None,
    };
    let at = match operator {
        Binary::Or => 0,
        Binary::And => 1,
        Binary::Equal | Binary::NotEqual => 2,
        Binary::Less | Binary::LessEqual | Binary::Greater | Binary::GreaterEqual => 3,
        Binary::Add | Binary::Subtract => 4,
        Binary::Multiply | Binary::Divide => 5,
    };
    (at == level).then_some(operator)
}
