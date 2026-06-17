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
        analyzer.analyze_program(
            program.node.decls.as_deref().unwrap_or(&[]),
            &program.node.body,
        );
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
