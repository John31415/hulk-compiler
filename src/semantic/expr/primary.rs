use crate::ast::{Literal, LiteralKind};
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedExpr, TypedExprKind};
use crate::semantic::{SemanticAnalyzer, symbols::SymbolType};

impl SemanticAnalyzer {
    pub fn analyze_literal(&mut self, lit: &Literal, span: Span) -> TypedExpr {
        let type_id = match lit.node {
            LiteralKind::Number(_) => self.resolve_builtin("Number"),
            LiteralKind::String(_) => self.resolve_builtin("String"),
            LiteralKind::Bool(_) => self.resolve_builtin("Boolean"),
        };
        TypedExpr::new(TypedExprKind::Literal(lit.node.clone()), type_id, span)
    }

    pub fn analyze_variable(&mut self, name: &str, span: Span) -> TypedExpr {
        let type_id = if let Some(symbol) = self.ctx.lookup(name) {
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
        };
        TypedExpr::new(TypedExprKind::Variable(name.into()), type_id, span)
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

    #[test]
    fn semantic_unit_test_primary() {
        let source = r#"
function f() => 42;

let x = a in f;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 2);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::UndefinedVariable {
                name: "a".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::NotAVariable {
                name: "f".to_string()
            }
        );
    }
}
