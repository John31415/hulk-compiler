use super::context::SemanticContext;
use crate::ast::{Decl, Expr};
use crate::diagnostics::Diagnostic;
use crate::semantic::builtin::install_builtins;

pub struct SemanticAnalyzer {
    pub ctx: SemanticContext,
    pub diagnostics: Vec<Diagnostic>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            ctx: SemanticContext::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn analyze_program(&mut self, decls: &[Decl], entry: Option<&Expr>) {
        install_builtins(&mut self.ctx);
        self.collect_declarations(decls);
        self.check_declarations(decls);
        if let Some(expr) = entry {
            self.check_expr(expr);
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}
