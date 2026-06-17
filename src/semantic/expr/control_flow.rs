use crate::ast::Expr;
use crate::lexer::span::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedExpr, TypedExprKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};

impl SemanticAnalyzer {
    pub fn analyze_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Option<Box<Expr>>,
        span: Span,
    ) -> TypedExpr {
        let bool_type = self.resolve_builtin("Boolean");
        let cond_type = self.analyze_expr(condition);
        if cond_type.ty != bool_type {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidConditionType {
                        found: self.ctx.types.get(cond_type.ty).name.clone(),
                    },
                    condition.span,
                )
                .into(),
            );
        }
        let then_type = self.analyze_expr(then_branch);
        let else_type = else_branch
            .as_ref()
            .map(|expr| Box::new(self.analyze_expr(expr)));
        let type_id = match else_branch {
            Some(else_expr) => {
                let else_type = self.analyze_expr(else_expr);
                self.ctx.types.find_lca(then_type.ty, else_type.ty)
            }
            None => self.resolve_builtin("Object"),
        };
        TypedExpr::new(
            TypedExprKind::If {
                condition: Box::new(cond_type),
                then_branch: Box::new(then_type),
                else_branch: else_type,
            },
            type_id,
            span,
        )
    }

    pub fn analyze_while(&mut self, condition: &Expr, body: &Expr, span: Span) -> TypedExpr {
        let bool_type = self.resolve_builtin("Boolean");
        let cond_type = self.analyze_expr(condition);
        if cond_type.ty != bool_type {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidWhileCondition {
                        found: self.ctx.types.get(cond_type.ty).name.clone(),
                    },
                    condition.span,
                )
                .into(),
            );
        }
        let body_type = self.analyze_expr(body);
        let type_id = body_type.ty;
        TypedExpr::new(
            TypedExprKind::While {
                condition: Box::new(cond_type),
                body: Box::new(body_type),
            },
            type_id,
            span,
        )
    }

    pub fn analyze_for(
        &mut self,
        var: &str,
        iterable: &Expr,
        body: &Expr,
        span: Span,
    ) -> TypedExpr {
        let iterable_type = self.analyze_expr(iterable);
        let loop_var_type = self.resolve_builtin("Object");
        self.ctx.push_scope();
        self.ctx.declare(Symbol {
            name: var.to_string(),
            kind: SymbolKind::Variable,
            ty: SymbolType::Variable(loop_var_type),
            span,
        });
        let body_type = self.analyze_expr(body);
        let type_id = body_type.ty;
        self.ctx.pop_scope();
        TypedExpr::new(
            TypedExprKind::For {
                var: var.into(),
                iterable: Box::new(iterable_type),
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
    fn semantic_unit_test_control_flow() {
        let source = r#"
{
    if(42) { 42; } else { 42; };
    while("hello") { 42; };
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
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
