use crate::ast::{Literal, LiteralKind};
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::{SemanticAnalyzer, symbols::SymbolType, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_literal(&mut self, lit: &Literal) -> TypeId {
        let type_name = match lit.node {
            LiteralKind::Number(_) => "Number",
            LiteralKind::String(_) => "String",
            LiteralKind::Bool(_) => "Boolean",
        };
        self.ctx
            .types
            .resolve(type_name)
            .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap())
    }

    pub fn check_variable(&mut self, name: &str, span: Span) -> TypeId {
        if let Some(symbol) = self.ctx.lookup(name) {
            match symbol.ty {
                SymbolType::Variable(type_id) => type_id,
                SymbolType::Function { .. } => {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::NotAVariable {
                                name: name.to_string(),
                            },
                            span,
                        )
                        .into(),
                    );
                    self.ctx.types.resolve("Object").unwrap()
                }
            }
        } else {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::UndefinedVariable {
                        name: name.to_string(),
                    },
                    span,
                )
                .into(),
            );
            self.ctx.types.resolve("Object").unwrap()
        }
    }
}
