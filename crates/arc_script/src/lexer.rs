use crate::token::Token;

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    variable_sigil: char,
}

impl Lexer {
    fn new(input: &str, variable_sigil: char) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            variable_sigil,
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

        match self.current() {
            Some(ch) if is_alpha(ch) => {}
            _ => return Err("Invalid variable name after '$'".to_string()),
        }

        while let Some(ch) = self.current() {
            if is_alpha(ch) {
                self.advance();
            } else {
                break;
            }
        }

        let name: String = self.input[start..self.pos].iter().collect();

        if name.is_empty() {
            Err("Empty variable name after '$'".to_string())
        } else {
            Ok(name)
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, String> {
        self.skip_whitespace();

        let Some(ch) = self.current() else {
            return Ok(None);
        };

        let token = match ch {
            c if c == self.variable_sigil => {
                let var_name = self.read_variable()?;
                Token::Variable(var_name)
            }
            '"' => {
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
            '\'' => {
                return Err("Single-quoted strings are not allowed. Use double quotes.".to_string());
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

fn is_alpha(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}
