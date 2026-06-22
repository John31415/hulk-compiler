use crate::ast::{Decl, DeclKind};
use crate::lexer::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedDecl, TypedDeclKind, TypedParam, TypedParamKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};
use crate::semantic::types::TypeId;

impl SemanticAnalyzer {
    pub fn analyze_function(&mut self, function_decl: &Decl) -> TypedDecl {
        let (name, params, return_type, body) = match &function_decl.node {
            DeclKind::Function {
                name,
                params,
                return_type,
                body,
            } => (name, params, return_type, body),
            _ => panic!("Expected function declaration"),
        };
        self.ctx.push_scope();
        let mut typed_params = Vec::new();
        for (param_name, param_type_opt) in params {
            let param_type_id = match param_type_opt {
                Some(type_name) => match self.ctx.types.resolve(type_name) {
                    Some(id) => {
                        if self.ctx.types.get(id).is_protocol() {
                            self.diagnostics.push(
                                SemanticError::new(
                                    SemanticErrorKind::ProtocolNotAllowedAsParameterType {
                                        type_name: type_name.to_string(),
                                        param_name: param_name.to_string(),
                                    },
                                    function_decl.span,
                                )
                                .into(),
                            );
                            self.ctx.types.resolve("Object").unwrap()
                        } else {
                            id
                        }
                    }
                    None => {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::UnknownTypeInParameter {
                                    type_name: type_name.to_string(),
                                    param_name: param_name.to_string(),
                                },
                                function_decl.span,
                            )
                            .into(),
                        );
                        self.ctx.types.resolve("Object").unwrap()
                    }
                },
                None => self.ctx.types.resolve("Object").unwrap(),
            };
            let param_symbol = Symbol {
                name: param_name.clone(),
                kind: SymbolKind::Parameter,
                ty: SymbolType::Variable(param_type_id),
                span: function_decl.span,
            };
            if !self.ctx.declare(param_symbol) {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::DuplicateParameter {
                            function: name.to_string(),
                            param: param_name.to_string(),
                        },
                        function_decl.span,
                    )
                    .into(),
                );
            }
            let typed_param = TypedParam::new(
                TypedParamKind {
                    name: param_name.clone(),
                    type_id: param_type_id,
                },
                function_decl.span,
            );
            typed_params.push(typed_param);
        }
        let expected_return = match return_type {
            Some(type_name) => match self.ctx.types.resolve(type_name) {
                Some(id) => id,
                None => {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::UnknownReturnType {
                                function: name.to_string(),
                                type_name: type_name.to_string(),
                            },
                            function_decl.span,
                        )
                        .into(),
                    );
                    self.ctx.types.resolve("Object").unwrap()
                }
            },
            None => self.ctx.types.resolve("Object").unwrap(),
        };
        self.ctx.current_function_return = Some(expected_return);
        let body_type = self.analyze_expr(body);
        if !self
            .ctx
            .types
            .is_subtype_of(&self.ctx, body_type.ty, expected_return)
        {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::FunctionReturnTypeMismatch {
                        function: name.to_string(),
                        expected: self.ctx.types.get(expected_return).name.clone(),
                        found: self.ctx.types.get(body_type.ty).name.clone(),
                    },
                    body.span,
                )
                .into(),
            );
        }
        self.ctx.current_function_return = None;
        self.ctx.pop_scope();
        TypedDecl::new(
            TypedDeclKind::Function {
                name: name.clone(),
                params: typed_params,
                return_type: expected_return,
                body: body_type,
            },
            function_decl.span,
        )
    }

    pub fn instantiate_generic_function(
        &mut self,
        name: &str,
        concrete_types: &[TypeId],
        call_site_span: Span,
    ) -> Option<(String, TypeId)> {
        let function_decl = match self.ctx.generic_decls.get(name).cloned() {
            Some(decl) => decl,
            None => {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::UndefinedFunction {
                            name: name.to_string(),
                        },
                        call_site_span,
                    )
                    .into(),
                );
                return None;
            }
        };
        let (decl_name, params, return_type, body) = match &function_decl.node {
            DeclKind::Function {
                name,
                params,
                return_type,
                body,
            } => (name, params, return_type, body),
            _ => panic!("Expected function declaration in generic_decls"),
        };
        let mangled_name = self.ctx.mangle_instance_name(decl_name, concrete_types);
        let key = (decl_name.clone(), concrete_types.to_vec());
        self.ctx.mark_in_progress(key.clone());
        self.ctx.push_scope();
        let mut typed_params = Vec::new();
        for (i, (param_name, param_type_opt)) in params.iter().enumerate() {
            let param_type_id = concrete_types
                .get(i)
                .copied()
                .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
            let _ = param_type_opt;
            let param_symbol = Symbol {
                name: param_name.clone(),
                kind: SymbolKind::Parameter,
                ty: SymbolType::Variable(param_type_id),
                span: function_decl.span,
            };
            if !self.ctx.declare(param_symbol) {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::DuplicateParameter {
                            function: decl_name.to_string(),
                            param: param_name.to_string(),
                        },
                        function_decl.span,
                    )
                    .into(),
                );
            }
            let typed_param = TypedParam::new(
                TypedParamKind {
                    name: param_name.clone(),
                    type_id: param_type_id,
                },
                function_decl.span,
            );
            typed_params.push(typed_param);
        }
        let declared_return = return_type.as_ref().and_then(|t| self.ctx.types.resolve(t));
        self.ctx.current_function_return = declared_return;
        let body_type = self.analyze_expr(body);
        self.ctx.current_function_return = None;
        let final_return = match declared_return {
            Some(expected_return) => {
                if !self
                    .ctx
                    .types
                    .is_subtype_of(&self.ctx, body_type.ty, expected_return)
                {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::FunctionReturnTypeMismatch {
                                function: decl_name.to_string(),
                                expected: self.ctx.types.get(expected_return).name.clone(),
                                found: self.ctx.types.get(body_type.ty).name.clone(),
                            },
                            body.span,
                        )
                        .into(),
                    );
                }
                expected_return
            }
            None => body_type.ty,
        };
        self.ctx.pop_scope();
        self.ctx.unmark_in_progress(&key);
        let typed_decl = TypedDecl::new(
            TypedDeclKind::Function {
                name: mangled_name.clone(),
                params: typed_params,
                return_type: final_return,
                body: body_type,
            },
            function_decl.span,
        );
        self.ctx.insert_instance(key, typed_decl);
        Some((mangled_name, final_return))
    }
}
