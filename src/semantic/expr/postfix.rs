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

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

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
        analyzer.analyze_program(
            program.node.decls.as_deref().unwrap_or(&[]),
            &program.node.body,
        );
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
