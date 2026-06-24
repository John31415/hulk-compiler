use std::fmt;

use crate::lexer::Span;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(kind: T, span: Span) -> Self {
        Self { node: kind, span }
    }
}

pub type Program = Spanned<ProgramKind>;

#[derive(Debug, Clone, Serialize)]
pub struct ProgramKind {
    pub decls: Option<Vec<Decl>>,
    pub body: Expr,
}

pub type Decl = Spanned<DeclKind>;

#[derive(Debug, Clone, Serialize)]
pub enum TypeAnnotation {
    Named { name: String, span: Span },
    Star { name: String, span: Span },
}

#[derive(Debug, Clone, Serialize)]
pub enum DeclKind {
    Function {
        name: String,
        params: Vec<(String, Option<TypeAnnotation>)>,
        return_type: Option<TypeAnnotation>,
        body: Expr,
    },
    Type {
        name: String,
        params: Option<Vec<(String, Option<TypeAnnotation>)>>,
        parent: Option<InheritInfo>,
        features: Vec<TypeFeatures>,
    },
    Protocol {
        name: String,
        parents: Option<Vec<String>>,
        methods: Vec<ProtocolMethods>,
    },
}

pub type Expr = Spanned<ExprKind>;

#[derive(Debug, Clone, Serialize)]
pub enum ExprKind {
    Literal(Literal),
    Variable(String),
    New {
        type_name: String,
        args: Vec<Expr>,
    },
    Block(Vec<Expr>),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    PropertyAccess {
        obj: Box<Expr>,
        property: String,
    },
    MethodCall {
        obj: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left_expr: Box<Expr>,
        op: BinaryOp,
        right_expr: Box<Expr>,
    },
    Is {
        expr: Box<Expr>,
        type_name: String,
    },
    As {
        expr: Box<Expr>,
        type_name: String,
    },
    Let {
        name: String,
        type_name: Option<TypeAnnotation>,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    While {
        condition: Box<Expr>,
        body: Box<Expr>,
    },
    For {
        var: String,
        iterable: Box<Expr>,
        body: Box<Expr>,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
}

pub type ProtocolMethods = Spanned<ProtocolMethodsKind>;

#[derive(Debug, Clone, Serialize)]
pub struct ProtocolMethodsKind {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub return_type: String,
}

pub type TypeFeatures = Spanned<TypeFeaturesKind>;

#[derive(Debug, Clone, Serialize)]
pub enum TypeFeaturesKind {
    Attribute {
        name: String,
        type_name: Option<TypeAnnotation>,
        default: Option<Expr>,
    },
    Method {
        name: String,
        params: Vec<(String, Option<TypeAnnotation>)>,
        return_type: Option<TypeAnnotation>,
        body: Expr,
    },
}

pub type InheritInfo = Spanned<InheritInfoKind>;

#[derive(Debug, Clone, Serialize)]
pub struct InheritInfoKind {
    pub parent_name: String,
    pub args: Option<Vec<Expr>>,
}

pub type Literal = Spanned<LiteralKind>;

#[derive(Debug, Clone, Serialize)]
pub enum LiteralKind {
    Number(f64),
    String(String),
    Bool(bool),
}

pub type UnaryOp = Spanned<UnaryOpKind>;

#[derive(Debug, Clone, Serialize)]
pub enum UnaryOpKind {
    Not,
    Neg,
}

pub type BinaryOp = Spanned<BinaryOpKind>;

#[derive(Debug, Clone, Serialize)]
pub enum BinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Concat,
    ConcatSpace,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    DoubleEqual,
    NotEqual,
    And,
    Or,
}

impl fmt::Display for BinaryOpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOpKind::Add => write!(f, "+"),
            BinaryOpKind::Sub => write!(f, "-"),
            BinaryOpKind::Mul => write!(f, "*"),
            BinaryOpKind::Div => write!(f, "/"),
            BinaryOpKind::Mod => write!(f, "%"),
            BinaryOpKind::Pow => write!(f, "^"),
            BinaryOpKind::Concat => write!(f, "@"),
            BinaryOpKind::ConcatSpace => write!(f, "@@"),
            BinaryOpKind::Less => write!(f, "<"),
            BinaryOpKind::Greater => write!(f, ">"),
            BinaryOpKind::LessEqual => write!(f, "<="),
            BinaryOpKind::GreaterEqual => write!(f, ">="),
            BinaryOpKind::DoubleEqual => write!(f, "=="),
            BinaryOpKind::NotEqual => write!(f, "!="),
            BinaryOpKind::And => write!(f, "&"),
            BinaryOpKind::Or => write!(f, "|"),
        }
    }
}
