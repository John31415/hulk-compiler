use super::types::TypeId;
use crate::lexer::Span;

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Variable,
    Function,
    Type,
    Parameter,
    Attribute,
    Method,
}

#[derive(Debug, Clone)]
pub enum SymbolType {
    Variable(TypeId),
    Function {
        params: Vec<TypeId>,
        ret: TypeId,
    },
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: SymbolType,
    pub span: Span,
}
