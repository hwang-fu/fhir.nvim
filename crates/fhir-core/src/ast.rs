use rust_decimal::Decimal;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    This,
    Identifier(String),
    Member {
        base: Box<Expr>,
        name: String,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        base: Option<Box<Expr>>,
        name: String,
        args: Vec<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    TypeTest {
        op: TypeOp,
        expr: Box<Expr>,
        type_name: String,
    },
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Boolean(bool),
    Integer(i64),
    Decimal(Decimal),
    Str(String),
    Date(String),
    DateTime(String),
    Quantity(Decimal, String),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    In,
    Union,
    Concat,
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    Xor,
    Implies,
    Equiv,
    NotEquiv,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypeOp {
    Is,
    As,
}
