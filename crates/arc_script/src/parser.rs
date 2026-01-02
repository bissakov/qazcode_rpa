use crate::token::Token;

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
