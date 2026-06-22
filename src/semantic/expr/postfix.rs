use crate::ast::{Expr, ExprKind};
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedExpr, TypedExprKind};
use crate::semantic::SemanticAnalyzer;

impl SemanticAnalyzer {
    pub fn analyze_property_access(&mut self, obj: &Expr, property: &str, span: Span) -> TypedExpr {
        let obj_expr = self.analyze_expr(obj);
        let is_self_access = match &obj.node {
            ExprKind::Variable(name) => name == "self",
            _ => false,
        };
        if !is_self_access {
            self.diagnostics
                .push(SemanticError::new(SemanticErrorKind::InvalidPropertyAccess, span).into());
            return TypedExpr::new(
                TypedExprKind::PropertyAccess {
                    obj: Box::new(obj_expr),
                    property: property.into(),
                },
                self.resolve_builtin("Object"),
                span,
            );
        }
        if self.ctx.current_method.is_none() {
            self.diagnostics
                .push(SemanticError::new(SemanticErrorKind::InvalidPropertyAccess, span).into());
            return TypedExpr::new(
                TypedExprKind::PropertyAccess {
                    obj: Box::new(obj_expr),
                    property: property.into(),
                },
                self.resolve_builtin("Object"),
                span,
            );
        }
        let attr_type_id = if let Some(t) = self.ctx.types.lookup_attribute(obj_expr.ty, property) {
            t
        } else {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::UnknownAttribute {
                        type_name: self.ctx.types.get(obj_expr.ty).name.clone(),
                        attribute: property.to_string(),
                    },
                    span,
                )
                .into(),
            );
            self.resolve_builtin("Object")
        };
        TypedExpr::new(
            TypedExprKind::PropertyAccess {
                obj: Box::new(obj_expr),
                property: property.into(),
            },
            attr_type_id,
            span,
        )
    }

    pub fn analyze_method_call(
        &mut self,
        obj: &Expr,
        method: &str,
        args: &Vec<Expr>,
        span: Span,
    ) -> TypedExpr {
        let obj_expr = self.analyze_expr(obj);
        if self
            .ctx
            .get_pending_generic_method(obj_expr.ty, method)
            .is_some()
        {
            return self.analyze_generic_method_call(obj_expr, method, args, span);
        }
        let mut typed_args = Vec::new();
        let type_id = if let Some((param_types, return_type)) =
            self.ctx.types.lookup_method(obj_expr.ty, method)
        {
            if args.len() != param_types.len() {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::InvalidMethodArity {
                            method: method.to_string(),
                            expected: param_types.len(),
                            found: args.len(),
                        },
                        span,
                    )
                    .into(),
                );
            }
            for (i, arg) in args.iter().enumerate() {
                let arg_type = self.analyze_expr(arg);
                if i < param_types.len() {
                    let expected_type = param_types[i];
                    if !self.ctx.types.is_subtype_of(&self.ctx, arg_type.ty, expected_type) {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::MethodArgumentTypeMismatch {
                                    method: method.to_string(),
                                    index: i + 1,
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
            return_type
        } else {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::UnknownMethod {
                        type_name: self.ctx.types.get(obj_expr.ty).name.clone(),
                        method: method.to_string(),
                    },
                    span,
                )
                .into(),
            );
            self.resolve_builtin("Object")
        };
        TypedExpr::new(
            TypedExprKind::MethodCall {
                obj: Box::new(obj_expr),
                method: method.into(),
                args: typed_args,
            },
            type_id,
            span,
        )
    }

    fn analyze_generic_method_call(
        &mut self,
        obj_expr: TypedExpr,
        method: &str,
        args: &[Expr],
        span: Span,
    ) -> TypedExpr {
        let object_type = self.resolve_builtin("Object");
        let obj_type_id = obj_expr.ty;
        let mut typed_args = Vec::new();
        let mut concrete_arg_types = Vec::new();
        for arg in args {
            let arg_type = self.analyze_expr(arg);
            concrete_arg_types.push(arg_type.ty);
            typed_args.push(arg_type);
        }
        let key = (obj_type_id, method.to_string(), concrete_arg_types.clone());
        let (mangled_name, return_type) = if let Some(existing) = self.ctx.get_method_instance(&key)
        {
            let ret = existing.node_return_type();
            (
                self.ctx
                    .mangle_method_instance_name(obj_type_id, method, &concrete_arg_types),
                ret,
            )
        } else if self.ctx.is_method_in_progress(&key) {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::GenericInferenceFailed {
                        function: method.to_string(),
                    },
                    span,
                )
                .into(),
            );
            (method.to_string(), object_type)
        } else {
            match self.instantiate_generic_method(obj_type_id, method, &concrete_arg_types, span) {
                Some((name, ret)) => (name, ret),
                None => (method.to_string(), object_type),
            }
        };
        let mut call_args = vec![obj_expr];
        call_args.extend(typed_args);
        TypedExpr::new(
            TypedExprKind::Call {
                name: mangled_name,
                args: call_args,
            },
            return_type,
            span,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{error::SemanticErrorKind, test_utils::parse_program, SemanticAnalyzer};

    #[test]
    fn semantic_unit_test_postfix_property_access() {
        let source = r#"
type A {
    a = "a";

    f() {
        let p = new A() in p.a;
    }

    b = self.a;

    g() => self.c;

    h(n: Number) => n;
}
        
let a = new A() in {
    a.h();
    a.h(true);
    a.i();
};
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 6);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::InvalidPropertyAccess
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::InvalidPropertyAccess
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::UnknownAttribute {
                type_name: "A".to_string(),
                attribute: "c".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[3].kind,
            SemanticErrorKind::InvalidMethodArity {
                method: "h".to_string(),
                expected: 1,
                found: 0
            }
        );
        assert_eq!(
            analyzer.diagnostics[4].kind,
            SemanticErrorKind::MethodArgumentTypeMismatch {
                method: "h".to_string(),
                index: 1,
                expected: "Number".to_string(),
                found: "Boolean".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[5].kind,
            SemanticErrorKind::UnknownMethod {
                type_name: "A".to_string(),
                method: "i".to_string()
            }
        );
    }
}
