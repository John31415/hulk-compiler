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
