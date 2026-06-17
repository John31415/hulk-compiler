use super::{builtin::install_builtins, context::SemanticContext, error::SemanticError};
use crate::ast::{Decl, Expr};

pub struct SemanticAnalyzer {
    pub ctx: SemanticContext,
    pub diagnostics: Vec<SemanticError>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            ctx: SemanticContext::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn analyze_program(&mut self, decls: &[Decl], entry: &Expr) {
        install_builtins(&mut self.ctx);
        self.collect_declarations(decls);
        self.check_declarations(decls);
        self.check_expr(entry);
    }

    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}
