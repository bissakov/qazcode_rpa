use crate::{constants::UiConstants, node_graph::VariableValue, variables::Variables};

#[derive(Debug, Clone)]
pub enum Expr {
    Const(VariableValue),
    Load(String),

    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),

    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),

    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
}

trait ValueExt {
    fn to_bool(&self) -> Result<bool, String>;
    fn to_number(&self) -> Result<f64, String>;
}

impl ValueExt for VariableValue {
    fn to_bool(&self) -> Result<bool, String> {
        match self {
            VariableValue::Boolean(b) => Ok(*b),
            _ => Err(format!("Cannot convert {self:?} to boolean")),
        }
    }

    fn to_number(&self) -> Result<f64, String> {
        match self {
            VariableValue::Number(n) => Ok(*n),
            VariableValue::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
            VariableValue::String(s) => s
                .parse::<f64>()
                .map_err(|_| format!("Cannot convert string '{s}' to number")),
            VariableValue::Undefined => Err("Cannot convert undefined value to number".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Boolean(bool),
    String(String),
    Variable(String),
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
}

struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn current(&self) -> Option<char> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        while let Some(ch) = self.current() {
            if ch.is_ascii_digit() || ch == '.' {
                self.advance();
            } else {
                break;
            }
        }
        let num_str: String = self.input[start..self.pos].iter().collect();
        num_str
            .parse::<f64>()
            .map_err(|_| format!("Invalid number: {num_str}"))
    }

    fn read_identifier(&mut self) -> String {
        let start = self.pos;
        while let Some(ch) = self.current() {
            if ch.is_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        self.input[start..self.pos].iter().collect()
    }

    fn read_string(&mut self, quote: char) -> Result<String, String> {
        self.advance();
        let start = self.pos;
        while let Some(ch) = self.current() {
            if ch == quote {
                let result: String = self.input[start..self.pos].iter().collect();
                self.advance();
                return Ok(result);
            }
            self.advance();
        }
        Err("Unterminated string".to_string())
    }

    fn read_variable(&mut self) -> Result<String, String> {
        self.advance();
        let start = self.pos;
        while let Some(ch) = self.current() {
            if ch == UiConstants::VARIABLE_PLACEHOLDER_CLOSE {
                let var_name: String = self.input[start..self.pos].iter().collect();
                self.advance();
                if var_name.is_empty() {
                    return Err("Empty variable name".to_string());
                }
                return Ok(var_name);
            }
            self.advance();
        }
        Err("Unterminated variable placeholder".to_string())
    }

    fn next_token(&mut self) -> Result<Option<Token>, String> {
        self.skip_whitespace();

        let Some(ch) = self.current() else {
            return Ok(None);
        };

        let token = match ch {
            UiConstants::VARIABLE_PLACEHOLDER_OPEN => {
                let var_name = self.read_variable()?;
                Token::Variable(var_name)
            }
            '"' | '\'' => {
                let s = self.read_string(ch)?;
                Token::String(s)
            }
            '+' => {
                self.advance();
                Token::Plus
            }
            '-' => {
                self.advance();
                Token::Minus
            }
            '*' => {
                self.advance();
                Token::Multiply
            }
            '/' => {
                self.advance();
                Token::Divide
            }
            '%' => {
                self.advance();
                Token::Modulo
            }
            '(' => {
                self.advance();
                Token::LeftParen
            }
            ')' => {
                self.advance();
                Token::RightParen
            }
            '=' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Token::Equal
                } else {
                    return Err("Invalid operator '=', use '==' for equality".to_string());
                }
            }
            '!' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Token::NotEqual
                } else {
                    Token::Not
                }
            }
            '>' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Token::GreaterEqual
                } else {
                    Token::Greater
                }
            }
            '<' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Token::LessEqual
                } else {
                    Token::Less
                }
            }
            '&' => {
                self.advance();
                if self.current() == Some('&') {
                    self.advance();
                    Token::And
                } else {
                    return Err("Invalid operator '&', use '&&' for logical AND".to_string());
                }
            }
            '|' => {
                self.advance();
                if self.current() == Some('|') {
                    self.advance();
                    Token::Or
                } else {
                    return Err("Invalid operator '|', use '||' for logical OR".to_string());
                }
            }
            _ if ch.is_ascii_digit() => {
                let num = self.read_number()?;
                Token::Number(num)
            }
            _ if ch.is_alphabetic() => {
                let ident = self.read_identifier();
                match ident.as_str() {
                    "true" => Token::Boolean(true),
                    "false" => Token::Boolean(false),
                    "AND" => Token::And,
                    "OR" => Token::Or,
                    "NOT" => Token::Not,
                    _ => return Err(format!("Unknown identifier: {ident}")),
                }
            }
            _ => return Err(format!("Unexpected character: {ch}")),
        };

        Ok(Some(token))
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token()? {
            tokens.push(token);
        }
        Ok(tokens)
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> Option<&Token> {
        if self.pos < self.tokens.len() {
            Some(&self.tokens[self.pos])
        } else {
            None
        }
    }

    fn advance(&mut self) {
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

    fn parse(&mut self, variables: &mut Variables) -> Result<Expr, String> {
        let result = self.parse_or(variables)?;
        if self.current().is_some() {
            return Err(format!("Unexpected token: {:#?}", self.current()));
        }
        Ok(result)
    }

    fn parse_or(&mut self, variables: &mut Variables) -> Result<Expr, String> {
        let mut left = self.parse_and(variables)?;
        while self.current() == Some(&Token::Or) {
            self.advance();
            let right = self.parse_and(variables)?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self, variables: &mut Variables) -> Result<Expr, String> {
        let mut left = self.parse_comparison(variables)?;
        while self.current() == Some(&Token::And) {
            self.advance();
            let right = self.parse_comparison(variables)?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self, variables: &mut Variables) -> Result<Expr, String> {
        let left = self.parse_additive(variables)?;

        match self.current() {
            Some(Token::Equal) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(Expr::Eq(Box::new(left), Box::new(right)))
            }
            Some(Token::NotEqual) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(Expr::Ne(Box::new(left), Box::new(right)))
            }
            Some(Token::Greater) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(Expr::Gt(Box::new(left), Box::new(right)))
            }
            Some(Token::GreaterEqual) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(Expr::Ge(Box::new(left), Box::new(right)))
            }
            Some(Token::Less) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(Expr::Lt(Box::new(left), Box::new(right)))
            }
            Some(Token::LessEqual) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(Expr::Le(Box::new(left), Box::new(right)))
            }
            _ => Ok(left),
        }
    }

    fn parse_additive(&mut self, variables: &mut Variables) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative(variables)?;
        loop {
            match self.current() {
                Some(Token::Plus) => {
                    self.advance();
                    let right = self.parse_multiplicative(variables)?;
                    left = Expr::Add(Box::new(left), Box::new(right));
                }
                Some(Token::Minus) => {
                    self.advance();
                    let right = self.parse_multiplicative(variables)?;
                    left = Expr::Sub(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self, variables: &mut Variables) -> Result<Expr, String> {
        let mut left = self.parse_unary(variables)?;
        loop {
            match self.current() {
                Some(Token::Multiply) => {
                    self.advance();
                    let right = self.parse_unary(variables)?;
                    left = Expr::Mul(Box::new(left), Box::new(right));
                }
                Some(Token::Divide) => {
                    self.advance();
                    let right = self.parse_unary(variables)?;
                    left = Expr::Div(Box::new(left), Box::new(right));
                }
                Some(Token::Modulo) => {
                    self.advance();
                    let right = self.parse_unary(variables)?;
                    left = Expr::Mod(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self, variables: &mut Variables) -> Result<Expr, String> {
        match self.current() {
            Some(Token::Not) => {
                self.advance();
                let value = self.parse_unary(variables)?;
                Ok(Expr::Not(Box::new(value)))
            }
            Some(Token::Minus) => {
                self.advance();
                let value = self.parse_unary(variables)?;
                Ok(Expr::Neg(Box::new(value)))
            }
            Some(Token::Plus) => {
                self.advance();
                self.parse_unary(variables)
            }
            _ => self.parse_primary(variables),
        }
    }

    fn parse_primary(&mut self, variables: &mut Variables) -> Result<Expr, String> {
        match self.current() {
            Some(Token::Number(n)) => {
                let value = *n;
                self.advance();
                Ok(Expr::Const(VariableValue::Number(value)))
            }
            Some(Token::Boolean(b)) => {
                let value = *b;
                self.advance();
                Ok(Expr::Const(VariableValue::Boolean(value)))
            }
            Some(Token::String(s)) => {
                let value = s.clone();
                self.advance();
                Ok(Expr::Const(VariableValue::String(value)))
            }
            Some(Token::Variable(name)) => {
                let name_cloned = name.clone();
                self.advance();
                Ok(Expr::Load(name_cloned))
            }
            Some(Token::LeftParen) => {
                self.advance();
                let value = self.parse_or(variables)?;
                self.expect(&Token::RightParen)?;
                Ok(value)
            }
            Some(token) => Err(format!("Unexpected token: {token:?}")),
            None => Err("Unexpected end of expression".to_string()),
        }
    }
}

pub fn parse_expr(expression: &str, variables: &mut Variables) -> Result<Expr, String> {
    if expression.trim().is_empty() {
        return Err("Empty expression".to_string());
    }

    let mut lexer = Lexer::new(expression);
    let tokens = lexer.tokenize()?;

    if tokens.is_empty() {
        return Err("Empty expression".to_string());
    }

    let mut parser = Parser::new(tokens);
    parser.parse(variables)
}

pub fn eval_expr(expr: &Expr, variables: &Variables) -> Result<VariableValue, String> {
    match expr {
        Expr::Const(v) => Ok(v.clone()),

        Expr::Load(name) => {
            let v = variables.get(name);
            match v {
                Some(val) if !matches!(val, VariableValue::Undefined) => Ok(val.clone()),
                _ => Err(format!("Undefined variable: {}", name)),
            }
        }

        Expr::Add(a, b) => Ok(VariableValue::Number(
            eval_expr(a, variables)?.to_number()? + eval_expr(b, variables)?.to_number()?,
        )),

        Expr::Sub(a, b) => Ok(VariableValue::Number(
            eval_expr(a, variables)?.to_number()? - eval_expr(b, variables)?.to_number()?,
        )),

        Expr::Mul(a, b) => Ok(VariableValue::Number(
            eval_expr(a, variables)?.to_number()? * eval_expr(b, variables)?.to_number()?,
        )),

        Expr::Div(a, b) => {
            let rhs = eval_expr(b, variables)?.to_number()?;
            if rhs.abs() < f64::EPSILON {
                return Err("Division by zero".into());
            }
            Ok(VariableValue::Number(
                eval_expr(a, variables)?.to_number()? / rhs,
            ))
        }

        Expr::Mod(a, b) => {
            let rhs = eval_expr(b, variables)?.to_number()?;
            if rhs.abs() < f64::EPSILON {
                return Err("Division by zero".into());
            }
            Ok(VariableValue::Number(
                eval_expr(a, variables)?.to_number()? % rhs,
            ))
        }

        Expr::Neg(e) => Ok(VariableValue::Number(
            -eval_expr(e, variables)?.to_number()?,
        )),

        Expr::Eq(a, b) => Ok(VariableValue::Boolean(
            eval_expr(a, variables)? == eval_expr(b, variables)?,
        )),

        Expr::Ne(a, b) => Ok(VariableValue::Boolean(
            eval_expr(a, variables)? != eval_expr(b, variables)?,
        )),

        Expr::Gt(a, b) => Ok(VariableValue::Boolean(
            eval_expr(a, variables)?.to_number()? > eval_expr(b, variables)?.to_number()?,
        )),

        Expr::Ge(a, b) => Ok(VariableValue::Boolean(
            eval_expr(a, variables)?.to_number()? >= eval_expr(b, variables)?.to_number()?,
        )),

        Expr::Lt(a, b) => Ok(VariableValue::Boolean(
            eval_expr(a, variables)?.to_number()? < eval_expr(b, variables)?.to_number()?,
        )),

        Expr::Le(a, b) => Ok(VariableValue::Boolean(
            eval_expr(a, variables)?.to_number()? <= eval_expr(b, variables)?.to_number()?,
        )),

        Expr::And(a, b) => Ok(VariableValue::Boolean(
            eval_expr(a, variables)?.to_bool()? && eval_expr(b, variables)?.to_bool()?,
        )),

        Expr::Or(a, b) => Ok(VariableValue::Boolean(
            eval_expr(a, variables)?.to_bool()? || eval_expr(b, variables)?.to_bool()?,
        )),

        Expr::Not(e) => Ok(VariableValue::Boolean(!eval_expr(e, variables)?.to_bool()?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic() {
        let mut variables = Variables::new();

        {
            let expression = "2 + 3";
            let expr = parse_expr(expression, &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Number(5.0)
            );
        }

        {
            let expression = "10 - 4";
            let expr = parse_expr(expression, &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Number(5.0)
            );
        }

        {
            let expression = "3 * 4";
            let expr = parse_expr(expression, &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Number(6.0)
            );
        }

        {
            let expression = "15 / 3";
            let expr = parse_expr(expression, &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Number(5.0)
            );
        }

        {
            let expression = "10 % 3";
            let expr = parse_expr(expression, &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Number(1.0)
            );
        }
    }

    #[test]
    fn test_parentheses() {
        let mut variables = Variables::new();

        {
            let expr = parse_expr("(2 + 3) * 4", &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Number(20.0)
            );
        }

        {
            let expr = parse_expr("2 + (3 * 4)", &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Number(14.0)
            );
        }
    }

    #[test]
    fn test_comparison() {
        let mut variables = Variables::new();

        for (expr_str, expected) in [
            ("5 > 3", true),
            ("5 < 3", false),
            ("5 >= 5", true),
            ("5 <= 4", false),
            ("5 == 5", true),
            ("5 != 3", true),
        ] {
            let expr = parse_expr(expr_str, &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Boolean(expected)
            );
        }
    }

    #[test]
    fn test_boolean() {
        let mut variables = Variables::new();

        for (expr_str, expected) in [
            ("true && true", true),
            ("true && false", false),
            ("true || false", true),
            ("!true", false),
            ("!false", true),
        ] {
            let expr = parse_expr(expr_str, &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Boolean(expected)
            );
        }
    }

    #[test]
    fn test_boolean_uppercase() {
        let mut variables = Variables::new();

        for (expr_str, expected) in [
            ("true AND true", true),
            ("true OR false", true),
            ("NOT false", true),
        ] {
            let expr = parse_expr(expr_str, &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Boolean(expected)
            );
        }
    }

    #[test]
    fn test_variables() {
        let mut variables = Variables::new();

        variables.create_variable("x", crate::variables::VariableScope::Global);
        variables.set("x", VariableValue::Number(10.0));

        variables.create_variable("y", crate::variables::VariableScope::Global);
        variables.set("y", VariableValue::Number(5.0));

        {
            let expr = parse_expr("{x} + {y}", &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Number(15.0)
            );
        }

        {
            let expr = parse_expr("{x} > {y}", &mut variables).unwrap();
            assert_eq!(
                eval_expr(&expr, &variables).unwrap(),
                VariableValue::Boolean(true)
            );
        }
    }

    #[test]
    fn test_complex() {
        let mut variables = Variables::new();

        variables.create_variable("a", crate::variables::VariableScope::Global);
        variables.set("a", VariableValue::Number(10.0));

        variables.create_variable("b", crate::variables::VariableScope::Global);
        variables.set("b", VariableValue::Number(5.0));

        let expr = parse_expr("({a} + {b}) * 2 > 20", &mut variables).unwrap();
        assert_eq!(
            eval_expr(&expr, &variables).unwrap(),
            VariableValue::Boolean(true)
        );
    }

    #[test]
    fn test_errors() {
        let mut variables = Variables::new();

        assert!(parse_expr("", &mut variables).is_err());
        assert!(parse_expr("2 +", &mut variables).is_err());
        assert!(parse_expr("(2 + 3", &mut variables).is_err());
        assert!(parse_expr("2 + 3)", &mut variables).is_err());

        let expr = parse_expr("10 / 0", &mut variables).unwrap();
        assert!(eval_expr(&expr, &variables).is_err());

        let expr = parse_expr("{undefined}", &mut variables).unwrap();
        assert!(eval_expr(&expr, &variables).is_err());
    }

    #[test]
    fn test_boolean_strict() {
        let mut variables = Variables::new();

        for expr_str in ["yes", "and", "or", "not"] {
            assert!(parse_expr(expr_str, &mut variables).is_err());
        }
    }
}
