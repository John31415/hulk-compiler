use crate::ast::Expr;
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_let(
        &mut self,
        name: &str,
        type_name: &Option<String>,
        value: &Expr,
        body: &Expr,
        span: Span,
    ) -> TypeId {
        let value_type = self.check_expr(value);
        let var_type = match type_name {
            Some(t_name) => match self.ctx.types.resolve(t_name) {
                Some(id) => {
                    if !self.ctx.types.is_subtype_of(value_type, id) {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::LetBindingTypeMismatch {
                                    found: self.ctx.types.get(value_type).name.clone(),
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
            None => value_type,
        };
        self.ctx.push_scope();
        self.ctx.declare(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Variable,
            ty: SymbolType::Variable(var_type),
            span,
        });
        let body_type = self.check_expr(body);
        self.ctx.pop_scope();
        body_type
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

    #[test]
    fn semantic_unit_test_control_flow() {
        let source = r#"
{
    let x: Number = "hello" in 42;
    let y: String = true in 42;
    let z: John = 1 in 42;
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(
            program.node.decls.as_deref().unwrap_or(&[]),
            Some(&program.node.body),
        );
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
