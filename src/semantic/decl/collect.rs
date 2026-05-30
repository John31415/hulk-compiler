use crate::ast::{Decl, DeclKind};
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};

impl SemanticAnalyzer {
    pub fn collect_declarations(&mut self, decls: &[Decl]) {
        for decl in decls {
            match &decl.node {
                DeclKind::Function { name, .. } => {
                    let ok = self.ctx.declare(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Function,
                        ty: SymbolType::Unknown,
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
