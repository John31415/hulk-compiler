use crate::ast::{Decl, DeclKind};
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};

impl SemanticAnalyzer {
    pub fn check_circular_inheritance(&mut self, decls: &[Decl]) {
        for decl in decls {
            if let DeclKind::Type { name, .. } = &decl.node {
                if let Some(start_id) = self.ctx.types.resolve(name) {
                    let mut current = self.ctx.types.get_parent(start_id);
                    let mut visited = vec![start_id];
                    while let Some(parent_id) = current {
                        if visited.contains(&parent_id) {
                            self.diagnostics.push(
                                SemanticError::new(
                                    SemanticErrorKind::CyclicInheritance {
                                        name: name.to_string(),
                                    },
                                    decl.span,
                                )
                                .into(),
                            );
                            self.ctx.types.set_parent(start_id, None);
                            break;
                        }
                        visited.push(parent_id);
                        current = self.ctx.types.get_parent(parent_id);
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
    fn semantic_unit_test_circular_inheritance_ok() {
        let source = r#"
type A inherits B {}
type B inherits C {}
type C inherits D {}
type E inherits D {}
type F inherits A {}
type X inherits E {}
type D {}

42;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(
            program.node.decls.as_deref().unwrap_or(&[]),
            Some(&program.node.body),
        );
        assert_eq!(analyzer.diagnostics.len(), 0);
    }

    #[test]
    fn semantic_unit_test_circular_inheritance_err() {
        let source = r#"
type A inherits B {}
type B inherits C {}
type C inherits D {}
type E inherits D {}
type F inherits A {}
type X inherits E {}
type D inherits F {}

42;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_program(
            program.node.decls.as_deref().unwrap_or(&[]),
            Some(&program.node.body),
        );
        assert_eq!(analyzer.diagnostics.len(), 1);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::CyclicInheritance {
                name: "A".to_string()
            }
        );
    }
}
