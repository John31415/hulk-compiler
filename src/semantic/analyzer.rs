use super::context::SemanticContext;
use crate::ast::{Decl, Expr};
use crate::diagnostics::Diagnostic;

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
        self.collect_declarations(decls);
        self.check_declarations(decls);
        if let Some(expr) = entry {
            self.check_expr(expr);
        }
    }

    pub fn push_declarations(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}
