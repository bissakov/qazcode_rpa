use crate::{constants::UiConstants, node_graph::VariableValue, variables::Variables};

trait ValueExt {
    fn to_bool(&self) -> Result<bool, String>;
    fn to_number(&self) -> Result<f64, String>;
}

impl ValueExt for VariableValue {
    fn to_bool(&self) -> Result<bool, String> {
        match self {
            VariableValue::Boolean(b) => Ok(*b),
            _ => Err(format!("Cannot convert {:?} to boolean", self)),
        }
    }

    fn to_number(&self) -> Result<f64, String> {
        match self {
            VariableValue::Number(n) => Ok(*n),
            VariableValue::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
            VariableValue::String(s) => s
                .parse::<f64>()
                .map_err(|_| format!("Cannot convert string '{}' to number", s)),
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
            .map_err(|_| format!("Invalid number: {}", num_str))
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

        let ch = match self.current() {
            Some(c) => c,
            None => return Ok(None),
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
                    _ => return Err(format!("Unknown identifier: {}", ident)),
                }
            }
            _ => return Err(format!("Unexpected character: {}", ch)),
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

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.current() == Some(&expected) {
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

    fn parse(&mut self, variables: &mut Variables) -> Result<VariableValue, String> {
        let result = self.parse_or(variables)?;
        if self.current().is_some() {
            return Err(format!("Unexpected token: {:?}", self.current()));
        }
        Ok(result)
    }

    fn parse_or(&mut self, variables: &mut Variables) -> Result<VariableValue, String> {
        let mut left = self.parse_and(variables)?;
        while self.current() == Some(&Token::Or) {
            self.advance();
            let right = self.parse_and(variables)?;
            let left_bool = left.to_bool()?;
            let right_bool = right.to_bool()?;
            left = VariableValue::Boolean(left_bool || right_bool);
        }
        Ok(left)
    }

    fn parse_and(&mut self, variables: &mut Variables) -> Result<VariableValue, String> {
        let mut left = self.parse_comparison(variables)?;
        while self.current() == Some(&Token::And) {
            self.advance();
            let right = self.parse_comparison(variables)?;
            let left_bool = left.to_bool()?;
            let right_bool = right.to_bool()?;
            left = VariableValue::Boolean(left_bool && right_bool);
        }
        Ok(left)
    }

    fn parse_comparison(&mut self, variables: &mut Variables) -> Result<VariableValue, String> {
        let left = self.parse_additive(variables)?;

        match self.current() {
            Some(Token::Equal) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(VariableValue::Boolean(self.compare_values(&left, &right)?))
            }
            Some(Token::NotEqual) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(VariableValue::Boolean(!self.compare_values(&left, &right)?))
            }
            Some(Token::Greater) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(VariableValue::Boolean(
                    left.to_number()? > right.to_number()?,
                ))
            }
            Some(Token::GreaterEqual) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(VariableValue::Boolean(
                    left.to_number()? >= right.to_number()?,
                ))
            }
            Some(Token::Less) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(VariableValue::Boolean(
                    left.to_number()? < right.to_number()?,
                ))
            }
            Some(Token::LessEqual) => {
                self.advance();
                let right = self.parse_additive(variables)?;
                Ok(VariableValue::Boolean(
                    left.to_number()? <= right.to_number()?,
                ))
            }
            _ => Ok(left),
        }
    }

    fn compare_values(&self, left: &VariableValue, right: &VariableValue) -> Result<bool, String> {
        match (left, right) {
            (VariableValue::Number(l), VariableValue::Number(r)) => {
                Ok((l - r).abs() < f64::EPSILON)
            }
            (VariableValue::Boolean(l), VariableValue::Boolean(r)) => Ok(l == r),
            (VariableValue::String(l), VariableValue::String(r)) => Ok(l == r),
            _ => {
                let l_num = left.to_number()?;
                let r_num = right.to_number()?;
                Ok((l_num - r_num).abs() < f64::EPSILON)
            }
        }
    }

    fn parse_additive(&mut self, variables: &mut Variables) -> Result<VariableValue, String> {
        let mut left = self.parse_multiplicative(variables)?;
        loop {
            match self.current() {
                Some(Token::Plus) => {
                    self.advance();
                    let right = self.parse_multiplicative(variables)?;
                    left = VariableValue::Number(left.to_number()? + right.to_number()?);
                }
                Some(Token::Minus) => {
                    self.advance();
                    let right = self.parse_multiplicative(variables)?;
                    left = VariableValue::Number(left.to_number()? - right.to_number()?);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self, variables: &mut Variables) -> Result<VariableValue, String> {
        let mut left = self.parse_unary(variables)?;
        loop {
            match self.current() {
                Some(Token::Multiply) => {
                    self.advance();
                    let right = self.parse_unary(variables)?;
                    left = VariableValue::Number(left.to_number()? * right.to_number()?);
                }
                Some(Token::Divide) => {
                    self.advance();
                    let right = self.parse_unary(variables)?;
                    let divisor = right.to_number()?;
                    if divisor.abs() < f64::EPSILON {
                        return Err("Division by zero".to_string());
                    }
                    left = VariableValue::Number(left.to_number()? / divisor);
                }
                Some(Token::Modulo) => {
                    self.advance();
                    let right = self.parse_unary(variables)?;
                    let divisor = right.to_number()?;
                    if divisor.abs() < f64::EPSILON {
                        return Err("Modulo by zero".to_string());
                    }
                    left = VariableValue::Number(left.to_number()? % divisor);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self, variables: &mut Variables) -> Result<VariableValue, String> {
        match self.current() {
            Some(Token::Not) => {
                self.advance();
                let value = self.parse_unary(variables)?;
                Ok(VariableValue::Boolean(!value.to_bool()?))
            }
            Some(Token::Minus) => {
                self.advance();
                let value = self.parse_unary(variables)?;
                Ok(VariableValue::Number(-value.to_number()?))
            }
            Some(Token::Plus) => {
                self.advance();
                self.parse_unary(variables)
            }
            _ => self.parse_primary(variables),
        }
    }

    fn parse_primary(&mut self, variables: &mut Variables) -> Result<VariableValue, String> {
        match self.current() {
            Some(Token::Number(n)) => {
                let value = *n;
                self.advance();
                Ok(VariableValue::Number(value))
            }
            Some(Token::Boolean(b)) => {
                let value = *b;
                self.advance();
                Ok(VariableValue::Boolean(value))
            }
            Some(Token::String(s)) => {
                let value = s.clone();
                self.advance();
                Ok(VariableValue::String(value))
            }
            Some(Token::Variable(name)) => {
                let var_name = name.clone();
                self.advance();

                let id = variables.id(var_name.as_str());
                let value = variables.get(id).clone();
                if matches!(value, VariableValue::Undefined) {
                    Err(format!("Undefined variable: {}", var_name))
                } else {
                    Ok(value)
                }
            }
            Some(Token::LeftParen) => {
                self.advance();
                let value = self.parse_or(variables)?;
                self.expect(Token::RightParen)?;
                Ok(value)
            }
            Some(token) => Err(format!("Unexpected token: {:?}", token)),
            None => Err("Unexpected end of expression".to_string()),
        }
    }
}

pub fn evaluate(expression: &str, variables: &mut Variables) -> Result<VariableValue, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic() {
        let mut variables = Variables::new();
        assert_eq!(
            evaluate("2 + 3", &mut variables).unwrap(),
            VariableValue::Number(5.0)
        );
        assert_eq!(
            evaluate("10 - 4", &mut variables).unwrap(),
            VariableValue::Number(6.0)
        );
        assert_eq!(
            evaluate("3 * 4", &mut variables).unwrap(),
            VariableValue::Number(12.0)
        );
        assert_eq!(
            evaluate("15 / 3", &mut variables).unwrap(),
            VariableValue::Number(5.0)
        );
        assert_eq!(
            evaluate("10 % 3", &mut variables).unwrap(),
            VariableValue::Number(1.0)
        );
    }

    #[test]
    fn test_parentheses() {
        let mut variables = Variables::new();
        assert_eq!(
            evaluate("(2 + 3) * 4", &mut variables).unwrap(),
            VariableValue::Number(20.0)
        );
        assert_eq!(
            evaluate("2 + (3 * 4)", &mut variables).unwrap(),
            VariableValue::Number(14.0)
        );
    }

    #[test]
    fn test_comparison() {
        let mut variables = Variables::new();
        assert_eq!(
            evaluate("5 > 3", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
        assert_eq!(
            evaluate("5 < 3", &mut variables).unwrap(),
            VariableValue::Boolean(false)
        );
        assert_eq!(
            evaluate("5 >= 5", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
        assert_eq!(
            evaluate("5 <= 4", &mut variables).unwrap(),
            VariableValue::Boolean(false)
        );
        assert_eq!(
            evaluate("5 == 5", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
        assert_eq!(
            evaluate("5 != 3", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
    }

    #[test]
    fn test_boolean() {
        let mut variables = Variables::new();
        assert_eq!(
            evaluate("true && true", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
        assert_eq!(
            evaluate("true && false", &mut variables).unwrap(),
            VariableValue::Boolean(false)
        );
        assert_eq!(
            evaluate("true || false", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
        assert_eq!(
            evaluate("!true", &mut variables).unwrap(),
            VariableValue::Boolean(false)
        );
        assert_eq!(
            evaluate("!false", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
    }

    #[test]
    fn test_boolean_uppercase() {
        let mut variables = Variables::new();
        assert_eq!(
            evaluate("true AND true", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
        assert_eq!(
            evaluate("true OR false", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
        assert_eq!(
            evaluate("NOT false", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
    }

    #[test]
    fn test_variables() {
        let mut variables = Variables::new();

        let id = variables.id("x");
        variables.set(id, VariableValue::Number(10.0));

        let id = variables.id("y");
        variables.set(id, VariableValue::Number(5.0));

        assert_eq!(
            evaluate("{x} + {y}", &mut variables).unwrap(),
            VariableValue::Number(15.0)
        );
        assert_eq!(
            evaluate("{x} > {y}", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
    }

    #[test]
    fn test_complex() {
        let mut variables = Variables::new();

        let id = variables.id("a");
        variables.set(id, VariableValue::Number(10.0));

        let id = variables.id("b");
        variables.set(id, VariableValue::Number(5.0));

        assert_eq!(
            evaluate("({a} + {b}) * 2 > 20", &mut variables).unwrap(),
            VariableValue::Boolean(true)
        );
    }

    #[test]
    fn test_errors() {
        let mut variables = Variables::new();
        assert!(evaluate("", &mut variables).is_err());
        assert!(evaluate("2 +", &mut variables).is_err());
        assert!(evaluate("(2 + 3", &mut variables).is_err());
        assert!(evaluate("2 + 3)", &mut variables).is_err());
        assert!(evaluate("10 / 0", &mut variables).is_err());
        assert!(evaluate("{undefined}", &mut variables).is_err());
    }

    #[test]
    fn test_boolean_strict() {
        let mut variables = Variables::new();
        assert!(evaluate("yes", &mut variables).is_err());
        assert!(evaluate("and", &mut variables).is_err());
        assert!(evaluate("or", &mut variables).is_err());
        assert!(evaluate("not", &mut variables).is_err());
    }
}
