use crate::ast::{Expr, ExprKind};
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_property_access(&mut self, obj: &Expr, property: &str, span: Span) -> TypeId {
        let is_self_access = match &obj.node {
            ExprKind::Variable(name) => name == "self",
            _ => false,
        };
        if !is_self_access {
            self.diagnostics
                .push(SemanticError::new(SemanticErrorKind::InvalidPropertyAccess, span).into());
            return self.ctx.types.resolve("Object").unwrap();
        }
        if self.ctx.current_method.is_none() {
            self.diagnostics
                .push(SemanticError::new(SemanticErrorKind::InvalidPropertyAccess, span).into());
            return self.ctx.types.resolve("Object").unwrap();
        }
        let obj_type = self.check_expr(obj);
        if let Some(attr_type_id) = self.ctx.types.lookup_attribute(obj_type, property) {
            attr_type_id
        } else {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::UnknownAttribute {
                        type_name: self.ctx.types.get(obj_type).name.clone(),
                        attribute: property.to_string(),
                    },
                    span,
                )
                .into(),
            );
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
                let arg_type = self.check_expr(arg);
                if i < param_types.len() {
                    let expected_type = param_types[i];
                    if !self.ctx.types.is_subtype_of(arg_type, expected_type) {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::MethodArgumentTypeMismatch {
                                    method: method.to_string(),
                                    index: i + 1,
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
            return_type
        } else {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::UnknownMethod {
                        type_name: self.ctx.types.get(obj_type).name.clone(),
                        method: method.to_string(),
                    },
                    span,
                )
                .into(),
            );
            self.ctx.types.resolve("Object").unwrap()
        }
    }
}
