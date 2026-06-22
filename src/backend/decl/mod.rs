pub mod decl_types;
pub mod functions;
pub mod methods;

use std::collections::{HashMap, VecDeque};

use crate::semantic::{
    SemanticAnalyzer,
    hir::{TypedDecl, TypedDeclKind, TypedProgram},
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
        for decl in &program.node.monomorphized_types {
            self.compile_type_struct_only(decl, sema)?;
        }
        for decl in &program.node.monomorphized_types {
            self.compile_type_methods(decl, sema)?;
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

    fn topo_sort_types<'a>(decls: &'a [TypedDecl]) -> Vec<&'a TypedDecl> {
        let mut type_decls: Vec<(usize, &TypedDecl)> = Vec::new();
        let mut other_decls: Vec<&TypedDecl> = Vec::new();
        for decl in decls {
            if matches!(&decl.node, TypedDeclKind::Type { .. }) {
                type_decls.push((type_decls.len(), decl));
            } else {
                other_decls.push(decl);
            }
        }
        let n = type_decls.len();
        let id_to_idx: HashMap<_, usize> = type_decls
            .iter()
            .enumerate()
            .filter_map(|(i, (_, d))| {
                if let TypedDeclKind::Type { type_id, .. } = &d.node {
                    Some((*type_id, i))
                } else {
                    None
                }
            })
            .collect();
        let mut in_degree = vec![0usize; n];
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, (_, decl)) in type_decls.iter().enumerate() {
            if let TypedDeclKind::Type { parent, .. } = &decl.node {
                if let Some(inherit) = parent {
                    let parent_type_id = inherit.node.parent_type;
                    if let Some(&j) = id_to_idx.get(&parent_type_id) {
                        in_degree[i] += 1;
                        children[j].push(i);
                    }
                }
            }
        }
        let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
        let mut sorted: Vec<&TypedDecl> = Vec::with_capacity(n);
        while let Some(i) = queue.pop_front() {
            sorted.push(type_decls[i].1);
            for &child in &children[i] {
                in_degree[child] -= 1;
                if in_degree[child] == 0 {
                    queue.push_back(child);
                }
            }
        }
        if sorted.len() < n {
            for (i, (_, decl)) in type_decls.iter().enumerate() {
                if in_degree[i] > 0 {
                    sorted.push(decl);
                }
            }
        }
        other_decls.into_iter().chain(sorted).collect()
    }

    fn declare_top_level(
        &mut self,
        decls: &[TypedDecl],
        sema: &SemanticAnalyzer,
    ) -> BackendResult<()> {
        let ordered = Self::topo_sort_types(decls);
        for decl in ordered {
            match &decl.node {
                TypedDeclKind::Function { .. } => self.declare_function(decl)?,
                TypedDeclKind::Type { .. } => self.declare_type(decl, sema)?,
            }
        }
        Ok(())
    }

    fn compile_top_level(
        &mut self,
        decls: &[TypedDecl],
        sema: &SemanticAnalyzer,
    ) -> BackendResult<()> {
        let ordered = Self::topo_sort_types(decls);
        for decl in &ordered {
            if matches!(&decl.node, TypedDeclKind::Type { .. }) {
                self.compile_type_struct_only(decl, sema)?;
            }
        }
        for decl in &ordered {
            if matches!(&decl.node, TypedDeclKind::Type { .. }) {
                self.compile_type_methods(decl, sema)?;
            }
        }
        for decl in &ordered {
            if matches!(&decl.node, TypedDeclKind::Function { .. }) {
                self.compile_function(decl, sema)?;
            }
        }
        Ok(())
    }
}
