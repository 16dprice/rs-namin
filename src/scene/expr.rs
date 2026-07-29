//! A tiny math-expression evaluator for function plotting: one variable
//! (`x`), the usual arithmetic (`+ - * / ^`, `^` right-associative), unary
//! minus, implicit multiplication (`2x`, `x sin(x)`, `2(x+1)`), parentheses,
//! the constants `pi`/`tau`/`e`, and a fixed set of one-argument functions.
//!
//! Parsing is recursive descent over a token list; evaluation walks the AST.
//! Both are allocation-light and fast enough to re-parse per frame (the
//! inspector spawns specs every frame), though `Plot` caches the parsed
//! expression anyway.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Func {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Exp,
    Ln,
    Log10,
    Sqrt,
    Abs,
    Floor,
    Ceil,
    Sign,
}

impl Func {
    fn from_name(name: &str) -> Option<Func> {
        Some(match name {
            "sin" => Func::Sin,
            "cos" => Func::Cos,
            "tan" => Func::Tan,
            "asin" => Func::Asin,
            "acos" => Func::Acos,
            "atan" => Func::Atan,
            "sinh" => Func::Sinh,
            "cosh" => Func::Cosh,
            "tanh" => Func::Tanh,
            "exp" => Func::Exp,
            "ln" => Func::Ln,
            "log" => Func::Log10,
            "sqrt" => Func::Sqrt,
            "abs" => Func::Abs,
            "floor" => Func::Floor,
            "ceil" => Func::Ceil,
            "sign" => Func::Sign,
            _ => return None,
        })
    }

    fn eval(self, v: f32) -> f32 {
        match self {
            Func::Sin => v.sin(),
            Func::Cos => v.cos(),
            Func::Tan => v.tan(),
            Func::Asin => v.asin(),
            Func::Acos => v.acos(),
            Func::Atan => v.atan(),
            Func::Sinh => v.sinh(),
            Func::Cosh => v.cosh(),
            Func::Tanh => v.tanh(),
            Func::Exp => v.exp(),
            Func::Ln => v.ln(),
            Func::Log10 => v.log10(),
            Func::Sqrt => v.sqrt(),
            Func::Abs => v.abs(),
            Func::Floor => v.floor(),
            Func::Ceil => v.ceil(),
            Func::Sign => v.signum(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f32),
    X,
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Call(Func, Box<Expr>),
}

impl Expr {
    /// Evaluate at `x`. Domain errors follow IEEE float semantics (NaN/inf);
    /// callers treat non-finite results as "no sample here".
    pub fn eval(&self, x: f32) -> f32 {
        match self {
            Expr::Num(n) => *n,
            Expr::X => x,
            Expr::Add(a, b) => a.eval(x) + b.eval(x),
            Expr::Sub(a, b) => a.eval(x) - b.eval(x),
            Expr::Mul(a, b) => a.eval(x) * b.eval(x),
            Expr::Div(a, b) => a.eval(x) / b.eval(x),
            Expr::Pow(a, b) => a.eval(x).powf(b.eval(x)),
            Expr::Neg(a) => -a.eval(x),
            Expr::Call(f, a) => f.eval(a.eval(x)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f32),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

fn tokenize(src: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => i += 1,
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '^' => {
                tokens.push(Token::Caret);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '0'..='9' | '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let n: f32 = text.parse().map_err(|_| format!("bad number {text:?}"))?;
                tokens.push(Token::Num(n));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                tokens.push(Token::Ident(chars[start..i].iter().collect()));
            }
            other => return Err(format!("unexpected character {other:?}")),
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// sum := product (("+" | "-") product)*
    fn parse_sum(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_product()?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.next();
                    left = Expr::Add(Box::new(left), Box::new(self.parse_product()?));
                }
                Some(Token::Minus) => {
                    self.next();
                    left = Expr::Sub(Box::new(left), Box::new(self.parse_product()?));
                }
                _ => return Ok(left),
            }
        }
    }

    /// product := unary (("*" | "/") unary | unary_adjacent)*
    /// where an adjacent atom-start token means implicit multiplication
    /// (`2x`, `2(x+1)`, `x sin(x)`).
    fn parse_product(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.next();
                    left = Expr::Mul(Box::new(left), Box::new(self.parse_unary()?));
                }
                Some(Token::Slash) => {
                    self.next();
                    left = Expr::Div(Box::new(left), Box::new(self.parse_unary()?));
                }
                Some(Token::Num(_)) | Some(Token::Ident(_)) | Some(Token::LParen) => {
                    left = Expr::Mul(Box::new(left), Box::new(self.parse_unary()?));
                }
                _ => return Ok(left),
            }
        }
    }

    /// unary := "-" unary | power
    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.peek() == Some(&Token::Minus) {
            self.next();
            return Ok(Expr::Neg(Box::new(self.parse_unary()?)));
        }
        self.parse_power()
    }

    /// power := atom ("^" unary)?   (right-associative, so 2^3^2 = 2^9)
    fn parse_power(&mut self) -> Result<Expr, String> {
        let base = self.parse_atom()?;
        if self.peek() == Some(&Token::Caret) {
            self.next();
            return Ok(Expr::Pow(Box::new(base), Box::new(self.parse_unary()?)));
        }
        Ok(base)
    }

    /// atom := number | "x" | constant | func "(" sum ")" | "(" sum ")"
    fn parse_atom(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Token::Num(n)) => Ok(Expr::Num(n)),
            Some(Token::Ident(name)) => match name.as_str() {
                "x" => Ok(Expr::X),
                "pi" => Ok(Expr::Num(std::f32::consts::PI)),
                "tau" => Ok(Expr::Num(std::f32::consts::TAU)),
                "e" => Ok(Expr::Num(std::f32::consts::E)),
                _ => {
                    let func = Func::from_name(&name).ok_or_else(|| format!("unknown name {name:?}"))?;
                    if self.next() != Some(Token::LParen) {
                        return Err(format!("expected '(' after {name}"));
                    }
                    let arg = self.parse_sum()?;
                    if self.next() != Some(Token::RParen) {
                        return Err(format!("missing ')' after {name}(...)"));
                    }
                    Ok(Expr::Call(func, Box::new(arg)))
                }
            },
            Some(Token::LParen) => {
                let inner = self.parse_sum()?;
                if self.next() != Some(Token::RParen) {
                    return Err("missing ')'".to_string());
                }
                Ok(inner)
            }
            Some(other) => Err(format!("unexpected token {other:?}")),
            None => Err("unexpected end of expression".to_string()),
        }
    }
}

/// Parse an expression in the variable `x`.
pub fn parse(src: &str) -> Result<Expr, String> {
    let tokens = tokenize(src)?;
    if tokens.is_empty() {
        return Err("empty expression".to_string());
    }
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.parse_sum()?;
    if parser.pos != parser.tokens.len() {
        return Err(format!("unexpected trailing input at token {}", parser.pos + 1));
    }
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str, x: f32) -> f32 {
        parse(src).unwrap().eval(x)
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(eval("1 + 2 * 3", 0.0), 7.0);
        assert_eq!(eval("(1 + 2) * 3", 0.0), 9.0);
        assert_eq!(eval("10 - 4 - 3", 0.0), 3.0); // left-assoc
        assert_eq!(eval("12 / 4 / 3", 0.0), 1.0);
    }

    #[test]
    fn power_is_right_associative_and_binds_tighter_than_neg() {
        assert_eq!(eval("2^3^2", 0.0), 512.0);
        assert_eq!(eval("-2^2", 0.0), -4.0);
        assert_eq!(eval("2^-1", 0.0), 0.5);
    }

    #[test]
    fn variable_and_constants() {
        assert_eq!(eval("x^2 + 1", 3.0), 10.0);
        assert!((eval("pi", 0.0) - std::f32::consts::PI).abs() < 1e-6);
        assert!((eval("tau / 2 - pi", 0.0)).abs() < 1e-6);
        assert!((eval("ln(e)", 0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn functions() {
        assert!((eval("sin(pi / 2)", 0.0) - 1.0).abs() < 1e-6);
        assert!((eval("sqrt(x)", 16.0) - 4.0).abs() < 1e-6);
        assert!((eval("abs(-3)", 0.0) - 3.0).abs() < 1e-6);
        assert!((eval("log(100)", 0.0) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn implicit_multiplication() {
        assert_eq!(eval("2x", 3.0), 6.0);
        assert_eq!(eval("2(x + 1)", 3.0), 8.0);
        assert!((eval("x sin(pi/2)", 5.0) - 5.0).abs() < 1e-6);
        assert_eq!(eval("(x+1)(x-1)", 3.0), 8.0);
    }

    #[test]
    fn unary_minus() {
        assert_eq!(eval("-x", 2.0), -2.0);
        assert_eq!(eval("3 - -x", 2.0), 5.0);
        assert_eq!(eval("--x", 2.0), 2.0);
    }

    #[test]
    fn domain_errors_are_nonfinite_not_panics() {
        assert!(eval("1 / x", 0.0).is_infinite());
        assert!(eval("ln(x)", -1.0).is_nan());
        assert!(eval("sqrt(x)", -4.0).is_nan());
    }

    #[test]
    fn parse_errors() {
        assert!(parse("").is_err());
        assert!(parse("x +").is_err());
        assert!(parse("(x").is_err());
        assert!(parse("x)").is_err());
        assert!(parse("foo(x)").is_err());
        assert!(parse("sin x").is_err());
        assert!(parse("1..2").is_err());
        assert!(parse("x $ 2").is_err());
    }
}
