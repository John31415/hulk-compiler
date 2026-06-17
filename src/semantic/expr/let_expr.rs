use crate::ast::Expr;
use crate::lexer::span::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedExpr, TypedExprKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};

impl SemanticAnalyzer {
    pub fn analyze_let(
        &mut self,
        name: &str,
        type_name: &Option<String>,
        value: &Expr,
        body: &Expr,
        span: Span,
    ) -> TypedExpr {
        let value_type = self.analyze_expr(value);
        let var_type = match type_name {
            Some(t_name) => match self.ctx.types.resolve(t_name) {
                Some(id) => {
                    if !self.ctx.types.is_subtype_of(value_type.ty, id) {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::LetBindingTypeMismatch {
                                    found: self.ctx.types.get(value_type.ty).name.clone(),
                                    expected: t_name.to_string(),
                                },
                                value.span,
                            )
                            .into(),
                        );
                    }
                    id
                }
                None => {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::UnknownTypeInLetAnnotation {
                                name: t_name.to_string(),
                            },
                            span,
                        )
                        .into(),
                    );
                    self.ctx.types.resolve("Object").unwrap()
                }
            },
            None => value_type.ty,
        };
        self.ctx.push_scope();
        self.ctx.declare(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Variable,
            ty: SymbolType::Variable(var_type),
            span,
        });
        let body_type = self.analyze_expr(body);
        let type_id = body_type.ty;
        self.ctx.pop_scope();
        TypedExpr::new(
            TypedExprKind::Let {
                name: name.into(),
                value: Box::new(value_type),
                body: Box::new(body_type),
            },
            type_id,
            span,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

    #[test]
    fn semantic_unit_test_let_expr() {
        let source = r#"
{
    let x: Number = "hello" in 42;
    let y: String = true in 42;
    let z: John = 1 in 42;
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 3);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::LetBindingTypeMismatch {
                expected: "Number".to_string(),
                found: "String".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::LetBindingTypeMismatch {
                expected: "String".to_string(),
                found: "Boolean".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::UnknownTypeInLetAnnotation {
                name: "John".to_string()
            }
        );
    }
}
