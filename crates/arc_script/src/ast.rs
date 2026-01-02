use crate::value::Value;

#[derive(Debug, Clone)]
pub enum InterpolationSegment {
    Literal(String),
    Expression(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Const(Value),
    Load(String),

    Add(Box<Self>, Box<Self>),
    Sub(Box<Self>, Box<Self>),
    Mul(Box<Self>, Box<Self>),
    Div(Box<Self>, Box<Self>),
    Mod(Box<Self>, Box<Self>),
    Neg(Box<Self>),

    Eq(Box<Self>, Box<Self>),
    Ne(Box<Self>, Box<Self>),
    Gt(Box<Self>, Box<Self>),
    Ge(Box<Self>, Box<Self>),
    Lt(Box<Self>, Box<Self>),
    Le(Box<Self>, Box<Self>),

    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Not(Box<Self>),

    InterpolatedString(Vec<InterpolationSegment>),
}
