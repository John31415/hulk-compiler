use crate::ast::Expr;
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Option<Box<Expr>>,
        _span: Span,
    ) -> TypeId {
        let bool_type = self.resolve_builtin("Boolean");
        let cond_type = self.check_expr(condition);
        if cond_type != bool_type {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidConditionType {
                        found: self.ctx.types.get(cond_type).name.clone(),
                    },
                    condition.span,
                )
                .into(),
            );
        }
        let then_type = self.check_expr(then_branch);
        match else_branch {
            Some(else_expr) => {
                let else_type = self.check_expr(else_expr);
                self.ctx.types.find_lca(then_type, else_type)
            }
            None => self.resolve_builtin("Object"),
        }
    }

    pub fn check_while(&mut self, condition: &Expr, body: &Expr) -> TypeId {
        let bool_type = self.resolve_builtin("Boolean");
        let cond_type = self.check_expr(condition);
        if cond_type != bool_type {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidWhileCondition {
                        found: self.ctx.types.get(cond_type).name.clone(),
                    },
                    condition.span,
                )
                .into(),
            );
        }
        let body_type = self.check_expr(body);
        body_type
    }

    pub fn check_for(&mut self, var: &str, iterable: &Expr, body: &Expr, span: Span) -> TypeId {
        let _iterable_type = self.check_expr(iterable);
        let loop_var_type = self.resolve_builtin("Object");
        self.ctx.push_scope();
        self.ctx.declare(Symbol {
            name: var.to_string(),
            kind: SymbolKind::Variable,
            ty: SymbolType::Variable(loop_var_type),
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
    if(42) { 42; } else { 42; };
    while("hello") { 42; };
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(
            program.node.decls.as_deref().unwrap_or(&[]),
            &program.node.body,
        );
        assert_eq!(analyzer.diagnostics.len(), 2);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::InvalidConditionType {
                found: "Number".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::InvalidWhileCondition {
                found: "String".to_string()
            }
        );
    }
}
