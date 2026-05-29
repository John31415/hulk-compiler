use crate::ast::Expr;
use crate::diagnostics::Diagnostic;
use crate::lexer::span::Span;
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Option<Box<Expr>>,
        span: Span,
    ) -> TypeId {
        let bool_type = self.resolve_builtin("Boolean");
        let cond_type = self.check_expr(condition);
        if cond_type != bool_type {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "'If' condition must be 'Boolean', but found '{}'",
                    self.ctx.types.get(cond_type).name
                ),
                condition.span,
            ));
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
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "'While' condition must be 'Boolean', but found '{}'",
                    self.ctx.types.get(cond_type).name
                ),
                condition.span,
            ));
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
