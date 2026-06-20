use serde::Serialize;

use crate::{
    ast::{BinaryOp, LiteralKind, UnaryOp},
    lexer::Span,
};

use super::types::TypeId;

#[derive(Debug, Clone, Serialize)]
pub struct TypedSpanned<T> {
    pub node: T,
    pub ty: TypeId,
    pub span: Span,
}

impl<T> TypedSpanned<T> {
    pub fn new(node: T, ty: TypeId, span: Span) -> Self {
        Self { node, ty, span }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DeclSpanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> DeclSpanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedProgram {
    pub node: TypedProgramKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedProgramKind {
    pub decls: Option<Vec<TypedDecl>>,
    pub body: TypedExpr,
    pub monomorphized_functions: Vec<TypedDecl>,
}

pub type TypedDecl = DeclSpanned<TypedDeclKind>;

#[derive(Debug, Clone, Serialize)]
pub enum TypedDeclKind {
    Function {
        name: String,
        params: Vec<TypedParam>,
        return_type: TypeId,
        body: TypedExpr,
    },
    Type {
        name: String,
        params: Option<Vec<TypedParam>>,
        parent: Option<TypedInheritInfo>,
        features: Vec<TypedTypeFeature>,
        type_id: TypeId,
    },
}

impl TypedDecl {
    pub fn node_return_type(&self) -> TypeId {
        match &self.node {
            TypedDeclKind::Function { return_type, .. } => *return_type,
            TypedDeclKind::Type { .. } => {
                panic!("node_return_type called on a TypedDeclKind::Type")
            }
        }
    }
}

pub type TypedExpr = TypedSpanned<TypedExprKind>;

#[derive(Debug, Clone, Serialize)]
pub enum TypedExprKind {
    Literal(LiteralKind),
    Variable(String),
    New {
        name: String,
        args: Vec<TypedExpr>,
    },
    Block(Vec<TypedExpr>),
    Call {
        name: String,
        args: Vec<TypedExpr>,
    },
    PropertyAccess {
        obj: Box<TypedExpr>,
        property: String,
    },
    MethodCall {
        obj: Box<TypedExpr>,
        method: String,
        args: Vec<TypedExpr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<TypedExpr>,
    },
    Binary {
        left_expr: Box<TypedExpr>,
        op: BinaryOp,
        right_expr: Box<TypedExpr>,
    },
    Is {
        expr: Box<TypedExpr>,
        target_type: TypeId,
    },
    As {
        expr: Box<TypedExpr>,
        target_type: TypeId,
    },
    Let {
        name: String,
        value: Box<TypedExpr>,
        body: Box<TypedExpr>,
    },
    If {
        condition: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Option<Box<TypedExpr>>,
    },
    While {
        condition: Box<TypedExpr>,
        body: Box<TypedExpr>,
    },
    For {
        var: String,
        iterable: Box<TypedExpr>,
        body: Box<TypedExpr>,
    },
    Assign {
        target: Box<TypedExpr>,
        value: Box<TypedExpr>,
    },
}

pub type TypedTypeFeature = DeclSpanned<TypedTypeFeatureKind>;

#[derive(Debug, Clone, Serialize)]
pub enum TypedTypeFeatureKind {
    Attribute {
        name: String,
        type_id: TypeId,
        default: Option<TypedExpr>,
    },
    Method {
        name: String,
        params: Vec<TypedParam>,
        return_type: TypeId,
        body: TypedExpr,
    },
}

pub type TypedParam = DeclSpanned<TypedParamKind>;

#[derive(Debug, Clone, Serialize)]
pub struct TypedParamKind {
    pub name: String,
    pub type_id: TypeId,
}

pub type TypedInheritInfo = DeclSpanned<TypedInheritInfoKind>;

#[derive(Debug, Clone, Serialize)]
pub struct TypedInheritInfoKind {
    pub parent_type: TypeId,
    pub args: Option<Vec<TypedExpr>>,
}
