use crate::ast::Expr;
use crate::lexer::span::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedExpr, TypedExprKind};

impl SemanticAnalyzer {
    pub fn analyze_new(&mut self, type_name: &str, args: &Vec<Expr>, span: Span) -> TypedExpr {
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
                let object_type = self.resolve_builtin("Object");
                return TypedExpr::new(
                    TypedExprKind::New {
                        name: type_name.into(),
                        args: args.iter().map(|a| self.analyze_expr(a)).collect(),
                    },
                    object_type,
                    span,
                );
            }
        };

        if self.ctx.types.is_generic_template(instance_type_id) {
            return self.analyze_generic_new(type_name, args, span);
        }

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
        let mut typed_args = Vec::new();
        for (i, arg) in args.iter().enumerate() {
            let arg_type = self.analyze_expr(arg);
            if i < expected_params.len() {
                let param = &expected_params[i];
                let expected_type = param
                    .ty
                    .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                if !self.ctx.types.is_subtype_of(&self.ctx, arg_type.ty, expected_type) {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::ConstructorArgumentTypeMismatch {
                                type_name: type_name.to_string(),
                                param: param.name.to_string(),
                                expected: self.ctx.types.get(expected_type).name.clone(),
                                found: self.ctx.types.get(arg_type.ty).name.clone(),
                            },
                            arg.span,
                        )
                        .into(),
                    );
                }
            }
            typed_args.push(arg_type);
        }
        TypedExpr::new(
            TypedExprKind::New {
                name: type_name.into(),
                args: typed_args,
            },
            instance_type_id,
            span,
        )
    }

    fn analyze_generic_new(&mut self, type_name: &str, args: &[Expr], span: Span) -> TypedExpr {
        let object_type = self.resolve_builtin("Object");
        let molde_type_id = self
            .ctx
            .types
            .resolve(type_name)
            .expect("caller already confirmed this type resolves");
        let declared_arity = self.ctx.types.get_constructor_params(molde_type_id).len();
        if args.len() != declared_arity {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidConstructorArity {
                        type_name: type_name.to_string(),
                        expected: declared_arity,
                        found: args.len(),
                    },
                    span,
                )
                .into(),
            );
        }
        let mut typed_args = Vec::new();
        let mut instance_key_types = Vec::new();
        for arg in args {
            let arg_type = self.analyze_expr(arg);
            instance_key_types.push(arg_type.ty);
            typed_args.push(arg_type);
        }
        let key = (type_name.to_string(), instance_key_types.clone());
        let instance_type_id = if let Some(existing) = self.ctx.get_type_instance(&key) {
            existing
        } else if self.ctx.is_type_in_progress(&key) {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::UnknownType {
                        name: type_name.to_string(),
                    },
                    span,
                )
                .into(),
            );
            object_type
        } else {
            match self.instantiate_generic_type(type_name, &instance_key_types, span) {
                Some(id) => id,
                None => object_type,
            }
        };
        TypedExpr::new(
            TypedExprKind::New {
                name: self
                    .ctx
                    .mangle_instance_name(type_name, &instance_key_types),
                args: typed_args,
            },
            instance_type_id,
            span,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

    #[test]
    fn semantic_unit_test_new() {
        let source = r#"
type A(a: Number) {
    x = a;
}
        
{
    new John();
    new A(1);
    new A();
    new A(1, "hello");
    new A("hello");
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 4);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::UnknownType {
                name: "John".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::InvalidConstructorArity {
                type_name: "A".to_string(),
                expected: 1,
                found: 0
            }
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::InvalidConstructorArity {
                type_name: "A".to_string(),
                expected: 1,
                found: 2
            }
        );
        assert_eq!(
            analyzer.diagnostics[3].kind,
            SemanticErrorKind::ConstructorArgumentTypeMismatch {
                type_name: "A".to_string(),
                param: "a".to_string(),
                expected: "Number".to_string(),
                found: "String".to_string()
            }
        );
    }
}
