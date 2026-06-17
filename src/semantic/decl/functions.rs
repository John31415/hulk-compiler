use crate::ast::{Decl, DeclKind};
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedDecl, TypedDeclKind, TypedParam, TypedParamKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};

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
                    Some(id) => id,
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
        if !self.ctx.types.is_subtype_of(body_type.ty, expected_return) {
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
}
