use crate::ast::Expr;
use crate::diagnostics::Diagnostic;
use crate::lexer::span::Span;
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
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "Type mismatch in let binding: cannot assign '{}' to explicit type '{}'",
                                self.ctx.types.get(value_type).name,
                                t_name,
                            ),
                            value.span,
                        ));
                    }
                    id
                }
                None => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("Non-existent type '{}' in let type annotation", t_name,),
                        span,
                    ));
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
