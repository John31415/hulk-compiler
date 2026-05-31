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
