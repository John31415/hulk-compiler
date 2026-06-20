#![allow(dead_code)]

use super::types::TypeId;
use crate::lexer::Span;

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Variable,
    Function,
    Parameter,
    Attribute,
}

#[derive(Debug, Clone)]
pub enum SymbolType {
    Variable(TypeId),
    Function {
        params: Vec<TypeId>,
        ret: TypeId,
    },
    GenericFunction {
        param_types: Vec<Option<TypeId>>,
        ret_type: Option<TypeId>,
    },
}

impl SymbolType {
    pub fn is_generic(&self) -> bool {
        matches!(self, SymbolType::GenericFunction { .. })
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: SymbolType,
    pub span: Span,
}
