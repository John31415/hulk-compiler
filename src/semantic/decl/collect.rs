use crate::ast::{Decl, DeclKind};
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};

impl SemanticAnalyzer {
    pub fn collect_declarations(&mut self, decls: &[Decl]) {
        let object_type = self.ctx.types.resolve("Object").unwrap();
        for decl in decls {
            match &decl.node {
                DeclKind::Function { name, params, .. } => {
                    let dummy_params = vec![object_type; params.len()];
                    let ok = self.ctx.declare(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Function,
                        ty: SymbolType::Function {
                            params: dummy_params,
                            ret: object_type,
                        },
                        span: decl.span,
                    });
                    if !ok {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::DuplicateFunction {
                                    name: name.to_string(),
                                },
                                decl.span,
                            )
                            .into(),
                        );
                    }
                }
                DeclKind::Type { name, .. } => {
                    let ok = self.ctx.types.insert(name.clone(), None).is_some();
                    if !ok {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::DuplicateType {
                                    name: name.to_string(),
                                },
                                decl.span,
                            )
                            .into(),
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

    #[test]
    fn semantic_unit_test_duplicate_function_err() {
        let source = r#"
function f() {}
function f(x: Number) {}

type A {}
type A(a: String) {}

42;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(
            program.node.decls.as_deref().unwrap_or(&[]),
            Some(&program.node.body),
        );
        assert_eq!(analyzer.diagnostics.len(), 2);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::DuplicateFunction {
                name: "f".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::DuplicateType {
                name: "A".to_string()
            }
        );
    }
}
