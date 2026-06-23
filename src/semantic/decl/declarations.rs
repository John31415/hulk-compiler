use crate::ast::{Decl, DeclKind};
use crate::semantic::SemanticAnalyzer;
use crate::semantic::hir::TypedDecl;
use crate::semantic::symbols::SymbolType;

impl SemanticAnalyzer {
    pub fn analyze_declarations(&mut self, decls: &[Decl]) -> Option<Vec<TypedDecl>> {
        let ok = self.register_signatures(decls);
        if !ok {
            return Some(vec![]);
        }
        self.check_circular_inheritance(decls);
        let ok = self.check_circular_protocols_extension(decls);
        if ok {
            self.collect_extended_methods(decls);
        }
        self.resolve_constructor_signatures();
        let mut typed_decls = Vec::new();
        for decl in decls {
            match &decl.node {
                DeclKind::Function { name, .. } => {
                    let is_generic = matches!(
                        self.ctx.lookup(name).map(|s| &s.ty),
                        Some(SymbolType::GenericFunction { .. })
                    ) || self.ctx.generic_decls.contains_key(name);
                    if is_generic {
                        continue;
                    }
                    typed_decls.push(self.analyze_function(decl));
                }
                DeclKind::Type { name, .. } => {
                    let resolved_id = self.ctx.types.resolve(name);
                    let is_generic = resolved_id
                        .map(|id| self.ctx.types.is_generic_template(id))
                        .unwrap_or(false);
                    if is_generic {
                        continue;
                    }
                    typed_decls.push(self.analyze_type(decl));
                }
                _ => continue,
            }
        }
        (!typed_decls.is_empty()).then_some(typed_decls)
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

    #[test]
    fn semantic_unit_test_check_function_err() {
        let source = r#"
function a(): Number {
    42;
    "hello";
}

42;
    "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 1);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::FunctionReturnTypeMismatch {
                function: "a".to_string(),
                expected: "Number".to_string(),
                found: "String".to_string(),
            }
        );
    }

    #[test]
    fn semantic_unit_test_check_type_err() {
        let source = r#"
type A(a: B, a: Number) {}

42;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(program);
        assert!(
            analyzer.diagnostics.is_empty(),
            "{:?}",
            analyzer.diagnostics
        );
        assert!(result.unwrap().node.monomorphized_types.is_empty());
    }

    #[test]
    fn semantic_unit_test_check_type_inheritance_err() {
        let source = r#"
type B(a: Number) {}
type C inherits B(1, 2) {}
type D inherits B("hello") {}
type E inherits John {}

42;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 3);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::InvalidInheritanceArity {
                parent: "B".to_string(),
                expected: 1,
                found: 2
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::InheritanceArgumentTypeMismatch {
                parent: "B".to_string(),
                param: "a".to_string(),
                expected: "Number".to_string(),
                found: "String".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::UnknownParentType {
                child: "E".to_string(),
                parent: "John".to_string()
            }
        );
    }

    #[test]
    fn semantic_unit_test_check_type_attributes_err() {
        let source = r#"
type E {
    e: John = 1;
    f: Number = "hello";
    f = 2;
}

42;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 3);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::UnknownTypeInAttribute {
                type_name: "John".to_string(),
                attribute: "e".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::AttributeTypeMismatch {
                attribute: "f".to_string(),
                expected: "Number".to_string(),
                found: "String".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::DuplicateAttribute {
                type_name: "E".to_string(),
                attribute: "f".to_string()
            }
        );
    }

    #[test]
    fn semantic_unit_test_check_type_method_err() {
        let source = r#"
type E {
    m(a: John): John => 1;
    n(): Number => "hello";
}

42;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 3);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::UnknownTypeInMethodParameter {
                method: "m".to_string(),
                param: "a".to_string(),
                type_name: "John".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::UnknownReturnTypeInMethod {
                method: "m".to_string(),
                type_name: "John".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::MethodReturnTypeMismatch {
                method: "n".to_string(),
                expected: "Number".to_string(),
                found: "String".to_string()
            }
        );
    }

    #[test]
    fn semantic_unit_test_check_type_method_override_ok() {
        let source = r#"
type A() {
    f(x: Number): Number => x;
}

type B inherits A {
    f(): String => "hello";
}

type C inherits A {
    f(x: Boolean): Number => 1;
}

42;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        for semantic_error in &analyzer.diagnostics {
            println!("{:?}", semantic_error);
        }
        assert_eq!(analyzer.diagnostics.len(), 3);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::InvalidOverrideArity {
                method: "f".to_string(),
                found: 0,
                expected: 1
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::InvalidOverrideReturnType {
                method: "f".to_string(),
                found: "String".to_string(),
                expected: "Number".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::InvalidOverrideParameterType {
                method: "f".to_string(),
                param_name: "x".to_string(),
                found: "Boolean".to_string(),
                expected: "Number".to_string()
            }
        );
    }
}
