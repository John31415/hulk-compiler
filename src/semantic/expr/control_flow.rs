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
        let object_type = self.resolve_builtin("Object");
        let bool_type = self.resolve_builtin("Boolean");
        let iterable_expr = self.analyze_expr(iterable);
        let iterable_protocol = self
            .ctx
            .types
            .resolve("Iterable")
            .expect("'Iterable' protocol is missing from the type table.");
        if !self
            .ctx
            .types
            .is_subtype_of(&self.ctx, iterable_expr.ty, iterable_protocol)
        {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::TypeMismatch {
                        expected: self.ctx.types.get(iterable_protocol).name.clone(),
                        found: self.ctx.types.get(iterable_expr.ty).name.clone(),
                    },
                    iterable.span,
                )
                .into(),
            );
        }
        let (_, loop_var_type) = self
            .ctx
            .types
            .lookup_method(iterable_expr.ty, "current")
            .expect("'Iterable' subtype without current()");
        self.ctx.push_scope();
        self.ctx.declare(Symbol {
            name: var.to_string(),
            kind: SymbolKind::Variable,
            ty: SymbolType::Variable(loop_var_type),
            span,
        });
        let body_type = self.analyze_expr(body);
        let result_ty = body_type.ty;
        self.ctx.pop_scope();
        let iter_name = format!("__iter_{}_{}", span.start, span.end);
        let iter_var_expr = TypedExpr::new(
            TypedExprKind::Variable(iter_name.clone()),
            iterable_expr.ty,
            span,
        );
        let next_call = TypedExpr::new(
            TypedExprKind::MethodCall {
                obj: Box::new(iter_var_expr.clone()),
                method: "next".to_string(),
                args: vec![],
            },
            bool_type,
            span,
        );
        let current_call = TypedExpr::new(
            TypedExprKind::MethodCall {
                obj: Box::new(iter_var_expr.clone()),
                method: "current".to_string(),
                args: vec![],
            },
            loop_var_type,
            span,
        );
        let inner_let = TypedExpr::new(
            TypedExprKind::Let {
                name: var.to_string(),
                value: Box::new(current_call),
                body: Box::new(body_type),
            },
            result_ty,
            span,
        );
        let while_expr = TypedExpr::new(
            TypedExprKind::While {
                condition: Box::new(next_call),
                body: Box::new(inner_let),
            },
            object_type,
            span,
        );
        TypedExpr::new(
            TypedExprKind::Let {
                name: iter_name,
                value: Box::new(iterable_expr),
                body: Box::new(while_expr),
            },
            object_type,
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
