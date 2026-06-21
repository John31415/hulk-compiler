use super::{
    builtin::install_builtins, context::SemanticContext, error::SemanticError, hir::TypedProgram,
};
use crate::{ast::Program, semantic::hir::TypedProgramKind};

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

    pub fn analyze_program(
        &mut self,
        program: Program,
    ) -> Result<TypedProgram, Vec<SemanticError>> {
        let decls = program.node.decls.as_deref().unwrap_or(&[]);
        let entry = &program.node.body;
        install_builtins(&mut self.ctx);
        self.collect_declarations(decls);
        let typed_decls = self.analyze_declarations(decls);
        let typed_entry = self.analyze_expr(entry);
        let monomorphized_functions = self
            .ctx
            .instantiation_order
            .iter()
            .map(|key| {
                self.ctx
                    .generic_instances
                    .get(key)
                    .cloned()
                    .expect("instantiation_order key must exist in generic_instances")
            })
            .collect();
        let monomorphized_types = self
            .ctx
            .type_instantiation_order
            .iter()
            .map(|key| {
                self.ctx
                    .generic_type_instance_decls
                    .get(key)
                    .cloned()
                    .expect(
                        "type_instantiation_order key must exist in generic_type_instance_decls",
                    )
            })
            .collect();
        let monomorphized_methods =
            self.ctx
                .method_instantiation_order
                .iter()
                .map(|key| {
                    self.ctx.generic_method_instances.get(key).cloned().expect(
                        "method_instantiation_order key must exist in generic_method_instances",
                    )
                })
                .collect();
        if self.has_errors() {
            return Err(self.diagnostics.clone());
        }
        let hir = TypedProgram {
            node: TypedProgramKind {
                decls: typed_decls,
                body: typed_entry,
                monomorphized_functions,
                monomorphized_types,
                monomorphized_methods,
            },
            span: program.span,
        };
        Ok(hir)
    }

    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}
