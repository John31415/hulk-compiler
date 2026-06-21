pub mod decl_types;
pub mod functions;
pub mod methods;

use crate::semantic::{
    hir::{TypedDecl, TypedDeclKind, TypedProgram},
    SemanticAnalyzer,
};

use super::{Backend, BackendError, BackendResult};

impl<'ctx> Backend<'ctx> {
    pub fn declare_program(
        &mut self,
        program: &TypedProgram,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<()> {
        if let Some(decls) = &program.node.decls {
            self.declare_top_level(decls, sema)?;
        }
        for type_instance_decl in &program.node.monomorphized_types {
            self.declare_type(type_instance_decl, sema)?;
        }
        for instance_decl in &program.node.monomorphized_functions {
            self.declare_function(instance_decl)?;
        }
        for method_instance_decl in &program.node.monomorphized_methods {
            self.declare_function(method_instance_decl)?;
        }
        Ok(())
    }

    pub fn compile_program(
        &mut self,
        program: &TypedProgram,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<()> {
        self.declare_program(program, sema)?;
        if let Some(decls) = &program.node.decls {
            self.compile_top_level(decls, sema)?;
        }
        for type_instance_decl in &program.node.monomorphized_types {
            self.compile_type(type_instance_decl, sema)?;
        }
        for instance_decl in &program.node.monomorphized_functions {
            self.compile_function(instance_decl, sema)?;
        }
        for method_instance_decl in &program.node.monomorphized_methods {
            self.compile_function(method_instance_decl, sema)?;
        }
        let i32_type = self.llvm_context.i32_type();
        let main_fn_type = i32_type.fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_fn_type, None);
        let entry_block = self.llvm_context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry_block);
        self.current_function = Some(main_fn);
        let _entry_value = self.compile_expr(&program.node.body, sema)?;
        self.builder
            .build_return(Some(&i32_type.const_int(0, false)))
            .map_err(|_| BackendError::InvalidExpression)?;
        Ok(())
    }

    fn declare_top_level(
        &mut self,
        decls: &[TypedDecl],
        sema: &SemanticAnalyzer,
    ) -> BackendResult<()> {
        for decl in decls {
            match &decl.node {
                TypedDeclKind::Function { .. } => {
                    self.declare_function(decl)?;
                }
                TypedDeclKind::Type { .. } => {
                    self.declare_type(decl, sema)?;
                }
            }
        }
        Ok(())
    }

    fn compile_top_level(
        &mut self,
        decls: &[TypedDecl],
        sema: &SemanticAnalyzer,
    ) -> BackendResult<()> {
        for decl in decls {
            match &decl.node {
                TypedDeclKind::Function { .. } => {
                    self.compile_function(decl, sema)?;
                }
                TypedDeclKind::Type { .. } => {
                    self.compile_type(decl, sema)?;
                }
            }
        }
        Ok(())
    }
}
