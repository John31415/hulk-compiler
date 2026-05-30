use crate::ast::Expr;
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_new(&mut self, type_name: &str, args: &Vec<Expr>, span: Span) -> TypeId {
        let instance_type_id = match self.ctx.types.resolve(type_name) {
            Some(id) => id,
            None => {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::UnknownType {
                            name: type_name.to_string(),
                        },
                        span,
                    )
                    .into(),
                );
                return self.ctx.types.resolve("Object").unwrap();
            }
        };
        let expected_params = self.ctx.types.infos[instance_type_id.0]
            .constructor_params
            .clone();
        if args.len() != expected_params.len() {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidConstructorArity {
                        type_name: type_name.to_string(),
                        expected: expected_params.len(),
                        found: args.len(),
                    },
                    span,
                )
                .into(),
            );
        }
        for (i, arg) in args.iter().enumerate() {
            let arg_type = self.check_expr(arg);
            if i < expected_params.len() {
                let param = &expected_params[i];
                let expected_type = param
                    .ty
                    .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                if !self.ctx.types.is_subtype_of(arg_type, expected_type) {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::ConstructorArgumentTypeMismatch {
                                type_name: type_name.to_string(),
                                param: param.name.to_string(),
                                expected: self.ctx.types.get(expected_type).name.clone(),
                                found: self.ctx.types.get(arg_type).name.clone(),
                            },
                            arg.span,
                        )
                        .into(),
                    );
                }
            }
        }
        instance_type_id
    }
}
