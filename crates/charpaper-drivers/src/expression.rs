// Blender runs a driver's formula itself, without Python, when it keeps to a small subset of
// Python it calls simple expressions (BLI_expr_pylike_eval.cc): numbers, the driver's
// variables, arithmetic, comparisons, `and`, `or`, `not`, `a if c else b`, and a set of math
// functions. That subset is all the app runs; anything else is refused when parsed. Values are
// worked out as Python does, in 64-bit floats, with True as 1 and False as 0.

use std::f64::consts::E;
use std::f64::consts::PI;

/// A parsed formula, evaluated against the driver's variables in the order they were named.
#[derive(Debug, Clone, PartialEq)]
pub struct Expression(Node);

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Number(f64),
    Variable(usize),
    Negate(Box<Node>),
    Not(Box<Node>),
    Binary(Op, Box<Node>, Box<Node>),
    And(Box<Node>, Box<Node>),
    Or(Box<Node>, Box<Node>),
    // Python chains comparisons: `a < b < c` is `a < b and b < c`, with b worked out once.
    Compare(Box<Node>, Vec<(Comparison, Node)>),
    Choose { condition: Box<Node>, then: Box<Node>, otherwise: Box<Node> },
    Call(Function, Vec<Node>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Power,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Comparison {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Function {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Exp,
    Log,
    Log10,
    Log2,
    Sqrt,
    Pow,
    Hypot,
    Floor,
    Ceil,
    Trunc,
    Round,
    Abs,
    Fmod,
    Radians,
    Degrees,
    Min,
    Max,
    Clamp,
    Lerp,
    Smoothstep,
}

impl Function {
    // The function, and how many arguments it takes, from least to most.
    fn named(name: &str) -> Option<(Self, usize, usize)> {
        use Function::*;
        let found = match name {
            "sin" => (Sin, 1, 1),
            "cos" => (Cos, 1, 1),
            "tan" => (Tan, 1, 1),
            "asin" => (Asin, 1, 1),
            "acos" => (Acos, 1, 1),
            "atan" => (Atan, 1, 1),
            "atan2" => (Atan2, 2, 2),
            "exp" => (Exp, 1, 1),
            "log" => (Log, 1, 2),
            "log10" => (Log10, 1, 1),
            "log2" => (Log2, 1, 1),
            "sqrt" => (Sqrt, 1, 1),
            "pow" => (Pow, 2, 2),
            "hypot" => (Hypot, 2, 2),
            "floor" => (Floor, 1, 1),
            "ceil" => (Ceil, 1, 1),
            "trunc" | "int" => (Trunc, 1, 1),
            "round" => (Round, 1, 2),
            "abs" | "fabs" => (Abs, 1, 1),
            "fmod" => (Fmod, 2, 2),
            "radians" => (Radians, 1, 1),
            "degrees" => (Degrees, 1, 1),
            "min" => (Min, 1, usize::MAX),
            "max" => (Max, 1, usize::MAX),
            "clamp" => (Clamp, 1, 3),
            "lerp" => (Lerp, 3, 3),
            "smoothstep" => (Smoothstep, 3, 3),
            _ => return None,
        };
        Some(found)
    }

    fn apply(self, a: &[f64]) -> f64 {
        use Function::*;
        match self {
            Sin => a[0].sin(),
            Cos => a[0].cos(),
            Tan => a[0].tan(),
            Asin => a[0].asin(),
            Acos => a[0].acos(),
            Atan => a[0].atan(),
            Atan2 => a[0].atan2(a[1]),
            Exp => a[0].exp(),
            Log if a.len() == 2 => a[0].ln() / a[1].ln(),
            Log => a[0].ln(),
            Log10 => a[0].log10(),
            Log2 => a[0].log2(),
            Sqrt => a[0].sqrt(),
            Pow => a[0].powf(a[1]),
            Hypot => a[0].hypot(a[1]),
            Floor => a[0].floor(),
            Ceil => a[0].ceil(),
            Trunc => a[0].trunc(),
            Round if a.len() == 2 => {
                let scale = 10f64.powf(a[1].trunc());
                (a[0] * scale).round_ties_even() / scale
            }
            Round => a[0].round_ties_even(),
            Abs => a[0].abs(),
            Fmod => a[0] % a[1],
            Radians => a[0].to_radians(),
            Degrees => a[0].to_degrees(),
            Min => a.iter().copied().fold(f64::INFINITY, f64::min),
            Max => a.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            Clamp if a.len() == 3 => a[0].max(a[1]).min(a[2]),
            // Blender's one-argument clamp is to 0..1; two arguments mean nothing to it.
            Clamp if a.len() == 1 => a[0].clamp(0.0, 1.0),
            Clamp => f64::NAN,
            Lerp => a[0] + (a[1] - a[0]) * a[2],
            Smoothstep => {
                let t = ((a[2] - a[0]) / (a[1] - a[0])).clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            }
        }
    }
}

impl Expression {
    /// Parses `text`, whose only names besides Blender's functions and constants are
    /// `variables`.
    pub fn parse(text: &str, variables: &[&str]) -> Result<Self, String> {
        let tokens = tokens(text)?;
        let mut parser = Parser { tokens: &tokens, at: 0, variables };
        let node = parser.choose()?;
        match parser.peek() {
            None => Ok(Self(node)),
            Some(token) => Err(format!("unexpected {token} in {text:?}")),
        }
    }

    pub fn evaluate(&self, variables: &[f32]) -> f64 {
        self.0.evaluate(variables)
    }
}

impl Node {
    fn evaluate(&self, vars: &[f32]) -> f64 {
        let truth = |value: bool| if value { 1.0 } else { 0.0 };
        match self {
            Node::Number(n) => *n,
            Node::Variable(index) => vars.get(*index).map_or(f64::NAN, |&v| v as f64),
            Node::Negate(a) => -a.evaluate(vars),
            Node::Not(a) => truth(a.evaluate(vars) == 0.0),
            Node::Binary(op, a, b) => {
                let (a, b) = (a.evaluate(vars), b.evaluate(vars));
                match op {
                    Op::Add => a + b,
                    Op::Subtract => a - b,
                    Op::Multiply => a * b,
                    Op::Divide if b == 0.0 => f64::NAN,
                    Op::Divide => a / b,
                    // Python's remainder takes the divisor's sign.
                    Op::Remainder if b == 0.0 => f64::NAN,
                    Op::Remainder => a - b * (a / b).floor(),
                    Op::Power => a.powf(b),
                }
            }
            // Like Python, they give one of their operands rather than True or False.
            Node::And(a, b) => {
                let a = a.evaluate(vars);
                if a == 0.0 { a } else { b.evaluate(vars) }
            }
            Node::Or(a, b) => {
                let a = a.evaluate(vars);
                if a != 0.0 { a } else { b.evaluate(vars) }
            }
            Node::Compare(first, rest) => {
                let mut left = first.evaluate(vars);
                for (comparison, node) in rest {
                    let right = node.evaluate(vars);
                    let holds = match comparison {
                        Comparison::Less => left < right,
                        Comparison::LessOrEqual => left <= right,
                        Comparison::Greater => left > right,
                        Comparison::GreaterOrEqual => left >= right,
                        Comparison::Equal => left == right,
                        Comparison::NotEqual => left != right,
                    };
                    if !holds {
                        return 0.0;
                    }
                    left = right;
                }
                1.0
            }
            Node::Choose { condition, then, otherwise } => {
                if condition.evaluate(vars) != 0.0 {
                    then.evaluate(vars)
                } else {
                    otherwise.evaluate(vars)
                }
            }
            Node::Call(function, args) => {
                let values: Vec<f64> = args.iter().map(|a| a.evaluate(vars)).collect();
                function.apply(&values)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Name(String),
    Symbol(&'static str),
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Token::Number(n) => write!(f, "{n}"),
            Token::Name(name) => write!(f, "{name:?}"),
            Token::Symbol(symbol) => write!(f, "{symbol:?}"),
        }
    }
}

// Longest first, so `**` isn't read as two `*`.
const SYMBOLS: [&str; 15] =
    ["**", "<=", ">=", "==", "!=", "+", "-", "*", "/", "%", "(", ")", ",", "<", ">"];

fn tokens(text: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c.is_whitespace() {
            rest = &rest[c.len_utf8()..];
        } else if c.is_ascii_digit()
            || (c == '.' && rest[1..].starts_with(|d: char| d.is_ascii_digit()))
        {
            let length = number_length(rest);
            let number =
                rest[..length].parse().map_err(|_| format!("bad number {:?}", &rest[..length]))?;
            tokens.push(Token::Number(number));
            rest = &rest[length..];
        } else if c.is_alphabetic() || c == '_' {
            let length =
                rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
            tokens.push(Token::Name(rest[..length].to_string()));
            rest = &rest[length..];
        } else if let Some(symbol) = SYMBOLS.iter().find(|s| rest.starts_with(**s)) {
            tokens.push(Token::Symbol(symbol));
            rest = &rest[symbol.len()..];
        } else {
            return Err(format!("{c:?} isn't part of a simple expression"));
        }
    }
    Ok(tokens)
}

// Digits with at most one point, then an optional exponent: 2, 2.5, .5, 2., 1e-3.
fn number_length(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
        i += 1;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        if j < bytes.len() && bytes[j].is_ascii_digit() {
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    i
}

// One function per level of Python's precedence, loosest first.
struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
    variables: &'a [&'a str],
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn eat_symbol(&mut self, symbol: &str) -> bool {
        let found = matches!(self.peek(), Some(Token::Symbol(s)) if *s == symbol);
        self.at += usize::from(found);
        found
    }

    fn eat_name(&mut self, name: &str) -> bool {
        let found = matches!(self.peek(), Some(Token::Name(n)) if n == name);
        self.at += usize::from(found);
        found
    }

    fn expect_symbol(&mut self, symbol: &str) -> Result<(), String> {
        if self.eat_symbol(symbol) {
            return Ok(());
        }
        match self.peek() {
            Some(token) => Err(format!("expected {symbol:?}, found {token}")),
            None => Err(format!("expected {symbol:?} at the end")),
        }
    }

    fn choose(&mut self) -> Result<Node, String> {
        let then = self.or()?;
        if !self.eat_name("if") {
            return Ok(then);
        }
        let condition = self.or()?;
        if !self.eat_name("else") {
            return Err("`if` without `else`".into());
        }
        let otherwise = self.choose()?;
        Ok(Node::Choose {
            condition: Box::new(condition),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
        })
    }

    fn or(&mut self) -> Result<Node, String> {
        let mut node = self.and()?;
        while self.eat_name("or") {
            node = Node::Or(Box::new(node), Box::new(self.and()?));
        }
        Ok(node)
    }

    fn and(&mut self) -> Result<Node, String> {
        let mut node = self.not()?;
        while self.eat_name("and") {
            node = Node::And(Box::new(node), Box::new(self.not()?));
        }
        Ok(node)
    }

    fn not(&mut self) -> Result<Node, String> {
        if self.eat_name("not") {
            return Ok(Node::Not(Box::new(self.not()?)));
        }
        self.compare()
    }

    fn compare(&mut self) -> Result<Node, String> {
        let first = self.sum()?;
        let mut rest = Vec::new();
        loop {
            let comparison = match self.peek() {
                Some(Token::Symbol("<")) => Comparison::Less,
                Some(Token::Symbol("<=")) => Comparison::LessOrEqual,
                Some(Token::Symbol(">")) => Comparison::Greater,
                Some(Token::Symbol(">=")) => Comparison::GreaterOrEqual,
                Some(Token::Symbol("==")) => Comparison::Equal,
                Some(Token::Symbol("!=")) => Comparison::NotEqual,
                _ => break,
            };
            self.at += 1;
            rest.push((comparison, self.sum()?));
        }
        if rest.is_empty() { Ok(first) } else { Ok(Node::Compare(Box::new(first), rest)) }
    }

    fn sum(&mut self) -> Result<Node, String> {
        let mut node = self.term()?;
        loop {
            let op = if self.eat_symbol("+") {
                Op::Add
            } else if self.eat_symbol("-") {
                Op::Subtract
            } else {
                return Ok(node);
            };
            node = Node::Binary(op, Box::new(node), Box::new(self.term()?));
        }
    }

    fn term(&mut self) -> Result<Node, String> {
        let mut node = self.unary()?;
        loop {
            let op = if self.eat_symbol("*") {
                Op::Multiply
            } else if self.eat_symbol("/") {
                Op::Divide
            } else if self.eat_symbol("%") {
                Op::Remainder
            } else {
                return Ok(node);
            };
            node = Node::Binary(op, Box::new(node), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Node, String> {
        if self.eat_symbol("-") {
            return Ok(Node::Negate(Box::new(self.unary()?)));
        }
        if self.eat_symbol("+") {
            return self.unary();
        }
        self.power()
    }

    // `**` binds tighter than a minus on its left and looser than one on its right, and
    // groups from the right: -2**2 is -4, 2**-1 is 0.5, 2**3**2 is 512.
    fn power(&mut self) -> Result<Node, String> {
        let base = self.primary()?;
        if self.eat_symbol("**") {
            return Ok(Node::Binary(Op::Power, Box::new(base), Box::new(self.unary()?)));
        }
        Ok(base)
    }

    fn primary(&mut self) -> Result<Node, String> {
        let token = self.peek().cloned().ok_or("the expression ends too soon")?;
        self.at += 1;
        match token {
            Token::Number(n) => Ok(Node::Number(n)),
            Token::Symbol("(") => {
                let node = self.choose()?;
                self.expect_symbol(")")?;
                Ok(node)
            }
            Token::Name(name) if self.eat_symbol("(") => self.call(&name),
            Token::Name(name) => self.name(&name),
            token => Err(format!("unexpected {token}")),
        }
    }

    fn name(&self, name: &str) -> Result<Node, String> {
        if let Some(index) = self.variables.iter().position(|v| *v == name) {
            return Ok(Node::Variable(index));
        }
        match name {
            "pi" => Ok(Node::Number(PI)),
            "e" => Ok(Node::Number(E)),
            "True" => Ok(Node::Number(1.0)),
            "False" => Ok(Node::Number(0.0)),
            _ => Err(format!("{name:?} is neither a variable of the driver nor a constant")),
        }
    }

    fn call(&mut self, name: &str) -> Result<Node, String> {
        let (function, least, most) = Function::named(name)
            .ok_or_else(|| format!("{name:?} isn't a function of simple expressions"))?;
        let mut args = Vec::new();
        if !self.eat_symbol(")") {
            loop {
                args.push(self.choose()?);
                if self.eat_symbol(")") {
                    break;
                }
                self.expect_symbol(",")?;
            }
        }
        if args.len() < least || args.len() > most {
            return Err(format!("{name} can't take {} argument(s)", args.len()));
        }
        Ok(Node::Call(function, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str, names: &[&str], vars: &[f32]) -> f64 {
        Expression::parse(text, names).unwrap().evaluate(vars)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn pose_reader() {
        let text = "max(0.0, 1.0 - d / 2.07445)";
        assert!(close(value(text, &["d"], &[0.0]), 1.0));
        assert!(close(value(text, &["d"], &[1.0]), 1.0 - 1.0 / 2.07445));
        assert!(close(value(text, &["d"], &[3.0]), 0.0));
    }

    #[test]
    fn precedence_as_in_python() {
        assert!(close(value("1 + 2 * 3", &[], &[]), 7.0));
        assert!(close(value("(1 + 2) * 3", &[], &[]), 9.0));
        assert!(close(value("-2 ** 2", &[], &[]), -4.0));
        assert!(close(value("2 ** -1", &[], &[]), 0.5));
        assert!(close(value("2 ** 3 ** 2", &[], &[]), 512.0));
        assert!(close(value("7 - 2 - 1", &[], &[]), 4.0));
        assert!(close(value("8 / 2 / 2", &[], &[]), 2.0));
    }

    #[test]
    fn remainder_follows_the_divisor() {
        assert!(close(value("-7 % 3", &[], &[]), 2.0));
        assert!(close(value("7 % -3", &[], &[]), -2.0));
        assert!(close(value("fmod(-7, 3)", &[], &[]), -1.0));
    }

    #[test]
    fn logic_gives_operands() {
        assert!(close(value("0 or 5", &[], &[]), 5.0));
        assert!(close(value("2 and 3", &[], &[]), 3.0));
        assert!(close(value("0 and 3", &[], &[]), 0.0));
        assert!(close(value("not 0", &[], &[]), 1.0));
        assert!(close(value("1 < 2 < 3", &[], &[]), 1.0));
        assert!(close(value("1 < 3 < 2", &[], &[]), 0.0));
        assert!(close(value("1 if a > 0.5 else 2", &["a"], &[0.7]), 1.0));
        assert!(close(value("1 if a > 0.5 else 2", &["a"], &[0.2]), 2.0));
    }

    #[test]
    fn blender_functions() {
        assert!(close(value("clamp(1.5)", &[], &[]), 1.0));
        assert!(close(value("clamp(5, 0, 2)", &[], &[]), 2.0));
        assert!(close(value("lerp(2, 4, 0.25)", &[], &[]), 2.5));
        assert!(close(value("smoothstep(0, 1, 0.5)", &[], &[]), 0.5));
        assert!(close(value("degrees(pi)", &[], &[]), 180.0));
        assert!(close(value("round(2.5)", &[], &[]), 2.0));
        assert!(close(value("int(-1.7)", &[], &[]), -1.0));
        assert!(close(value("min(3, a, 5)", &["a"], &[1.0]), 1.0));
        assert!(close(value("log(8, 2)", &[], &[]), 3.0));
        assert!(close(value(".5 + 1e-1 + 2.", &[], &[]), 2.6));
    }

    #[test]
    fn division_by_zero_has_no_value() {
        assert!(value("1 / a", &["a"], &[0.0]).is_nan());
    }

    #[test]
    fn refuses_what_needs_python() {
        assert!(Expression::parse("bpy.data.objects", &[]).is_err());
        assert!(Expression::parse("var[0]", &["var"]).is_err());
        assert!(Expression::parse("frame / 10", &[]).is_err());
        assert!(Expression::parse("noise(1)", &[]).is_err());
        assert!(Expression::parse("min(", &[]).is_err());
        assert!(Expression::parse("1 +", &[]).is_err());
        assert!(Expression::parse("lerp(1, 2)", &[]).is_err());
    }
}
