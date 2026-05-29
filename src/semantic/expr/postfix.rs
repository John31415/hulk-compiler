use crate::ast::Expr;
use crate::diagnostics::Diagnostic;
use crate::lexer::span::Span;
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_property_access(&mut self, obj: &Expr, property: &str, span: Span) -> TypeId {
        let obj_type = self.check_expr(obj);
        if let Some(attr_type_id) = self.ctx.types.lookup_attribute(obj_type, property) {
            attr_type_id
        } else {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "Type '{}' has no attribute named '{}'",
                    self.ctx.types.get(obj_type).name,
                    property,
                ),
                span,
            ));
            self.ctx.types.resolve("Object").unwrap()
        }
    }

    pub fn check_method_call(
        &mut self,
        obj: &Expr,
        method: &str,
        args: &Vec<Expr>,
        span: Span,
    ) -> TypeId {
        let obj_type = self.check_expr(obj);
        if let Some((param_types, return_type)) = self.ctx.types.lookup_method(obj_type, method) {
            if args.len() != param_types.len() {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "Method '{}' expects '{}' arguments, but '{}' were provided",
                        method,
                        param_types.len(),
                        args.len(),
                    ),
                    span,
                ));
            }
            for (i, arg) in args.iter().enumerate() {
                let arg_type = self.check_expr(arg);
                if i < param_types.len() {
                    let expected_type = param_types[i];
                    if !self.ctx.types.is_subtype_of(arg_type, expected_type) {
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "Type mismatch in method '{}' call: argument '{}' expects '{}', found '{}'",
                                method,
                                i + 1,
                                self.ctx.types.get(expected_type).name,
                                self.ctx.types.get(arg_type).name,
                            ),
                            arg.span,
                        ));
                    }
                }
            }
            return_type
        } else {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "Type '{}' has no method named '{}'",
                    self.ctx.types.get(obj_type).name,
                    method,
                ),
                span,
            ));
            self.ctx.types.resolve("Object").unwrap()
        }
    }
}
