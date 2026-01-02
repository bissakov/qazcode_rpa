#[derive(Debug, Clone, PartialEq)]
pub enum Token {
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
