use crate::ast::{Expr, InterpolationSegment};
use crate::lexer::Lexer;
use crate::token::Token;
use crate::value::Value;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    const fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> Option<&Token> {
        if self.pos < self.tokens.len() {
            Some(&self.tokens[self.pos])
        } else {
            None
        }
    }

    const fn advance(&mut self) {
        self.pos += 1;
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        if self.current() == Some(expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, found {:?}",
                expected,
                self.current()
            ))
        }
    }

    fn parse(&mut self) -> Result<Expr, String> {
        let result = self.parse_or()?;
        if self.current().is_some() {
            return Err(format!("Unexpected token: {:#?}", self.current()));
        }
        Ok(result)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while self.current() == Some(&Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        while self.current() == Some(&Token::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let left = self.parse_additive()?;

        match self.current() {
            Some(Token::Equal) => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Eq(Box::new(left), Box::new(right)))
            }
            Some(Token::NotEqual) => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Ne(Box::new(left), Box::new(right)))
            }
            Some(Token::Greater) => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Gt(Box::new(left), Box::new(right)))
            }
            Some(Token::GreaterEqual) => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Ge(Box::new(left), Box::new(right)))
            }
            Some(Token::Less) => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Lt(Box::new(left), Box::new(right)))
            }
            Some(Token::LessEqual) => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Le(Box::new(left), Box::new(right)))
            }
            _ => Ok(left),
        }
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            match self.current() {
                Some(Token::Plus) => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expr::Add(Box::new(left), Box::new(right));
                }
                Some(Token::Minus) => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expr::Sub(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            match self.current() {
                Some(Token::Multiply) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::Mul(Box::new(left), Box::new(right));
                }
                Some(Token::Divide) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::Div(Box::new(left), Box::new(right));
                }
                Some(Token::Modulo) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::Mod(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.current() {
            Some(Token::Not) => {
                self.advance();
                let value = self.parse_unary()?;
                Ok(Expr::Not(Box::new(value)))
            }
            Some(Token::Minus) => {
                self.advance();
                let value = self.parse_unary()?;
                Ok(Expr::Neg(Box::new(value)))
            }
            Some(Token::Plus) => {
                self.advance();
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_string_content(raw_string: &str) -> Result<Vec<InterpolationSegment>, String> {
        let mut segments = Vec::new();
        let mut literal = String::new();
        let chars: Vec<char> = raw_string.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                '{' => {
                    if i + 1 < chars.len() && chars[i + 1] == '{' {
                        literal.push('{');
                        i += 2;
                    } else {
                        if !literal.is_empty() {
                            segments.push(InterpolationSegment::Literal(literal.clone()));
                            literal.clear();
                        }

                        i += 1;
                        let expr_start = i;
                        let mut brace_count = 1;

                        while i < chars.len() && brace_count > 0 {
                            match chars[i] {
                                '{' => brace_count += 1,
                                '}' => brace_count -= 1,
                                _ => {}
                            }
                            if brace_count > 0 {
                                i += 1;
                            }
                        }

                        if brace_count != 0 {
                            return Err("Unclosed brace in interpolated string".to_string());
                        }

                        let expr_str = chars[expr_start..i].iter().collect::<String>();

                        if expr_str.trim().is_empty() {
                            return Err("Empty expression in interpolated string".to_string());
                        }

                        let expr = parse_expr(&expr_str)?;
                        segments.push(InterpolationSegment::Expression(Box::new(expr)));

                        i += 1;
                    }
                }
                '}' => {
                    if i + 1 < chars.len() && chars[i + 1] == '}' {
                        literal.push('}');
                        i += 2;
                    } else {
                        return Err("Unmatched closing brace in interpolated string".to_string());
                    }
                }
                _ => {
                    literal.push(chars[i]);
                    i += 1;
                }
            }
        }

        if !literal.is_empty() {
            segments.push(InterpolationSegment::Literal(literal));
        }

        if segments.is_empty() {
            segments.push(InterpolationSegment::Literal(String::new()));
        }

        Ok(segments)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.current() {
            Some(Token::Number(n)) => {
                let value = *n;
                self.advance();
                Ok(Expr::Const(Value::Number(value)))
            }
            Some(Token::Boolean(b)) => {
                let value = *b;
                self.advance();
                Ok(Expr::Const(Value::Boolean(value)))
            }
            Some(Token::String(s)) => {
                let value = s.clone();
                self.advance();

                if value.contains('{') || value.contains('}') {
                    let segments = Self::parse_string_content(&value)?;

                    if segments.len() == 1
                        && let InterpolationSegment::Literal(lit) = &segments[0]
                    {
                        return Ok(Expr::Const(Value::String(lit.clone())));
                    }

                    Ok(Expr::InterpolatedString(segments))
                } else {
                    Ok(Expr::Const(Value::String(value)))
                }
            }
            Some(Token::Variable(name)) => {
                let name_cloned = name.clone();
                self.advance();
                Ok(Expr::Load(name_cloned))
            }
            Some(Token::LeftParen) => {
                self.advance();
                let value = self.parse_or()?;
                self.expect(&Token::RightParen)?;
                Ok(value)
            }
            Some(token) => Err(format!("Unexpected token: {token:?}")),
            None => Err("Unexpected end of expression".to_string()),
        }
    }
}

/// # Errors
///
/// Returns an error if the expression is invalid.
pub fn parse_expr(expr: &str) -> Result<Expr, String> {
    if expr.trim().is_empty() {
        return Err("Empty expression".to_string());
    }

    let mut lexer = Lexer::new(expr, '@');
    let tokens = lexer.tokenize()?;

    if tokens.is_empty() {
        return Err("Empty expression".to_string());
    }

    let mut parser = Parser::new(tokens);
    parser.parse()
}
