use crate::ast::Expr;
use crate::diagnostics::Diagnostic;
use crate::lexer::span::Span;
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_new(&mut self, type_name: &str, args: &Vec<Expr>, span: Span) -> TypeId {
        let instance_type_id = match self.ctx.types.resolve(type_name) {
            Some(id) => id,
            None => {
                self.diagnostics.push(Diagnostic::error(
                    format!("Cannot instantiate non-existent type '{}'", type_name,),
                    span,
                ));
                return self.ctx.types.resolve("Object").unwrap();
            }
        };
        let expected_params = self.ctx.types.infos[instance_type_id.0]
            .constructor_params
            .clone();
        if args.len() != expected_params.len() {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "Type '{}' constructor expects '{}' arguments, but '{}' were provided",
                    type_name,
                    expected_params.len(),
                    args.len(),
                ),
                span,
            ));
        }
        for (i, arg) in args.iter().enumerate() {
            let arg_type = self.check_expr(arg);
            if i < expected_params.len() {
                let param = &expected_params[i];
                let expected_type = param
                    .ty
                    .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                if !self.ctx.types.is_subtype_of(arg_type, expected_type) {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "Type mismatch in instatiation of '{}': parameter '{}' expects '{}', found '{}'",
                            type_name,
                            param.name,
                            self.ctx.types.get(expected_type).name,
                            self.ctx.types.get(arg_type).name,
                        ),
                        arg.span,
                    ));
                }
            }
        }
        instance_type_id
    }
}
