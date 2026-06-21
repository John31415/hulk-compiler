use crate::ast::{Expr, TypeFeaturesKind};
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedDecl, TypedDeclKind, TypedParam, TypedParamKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};
use crate::semantic::types::TypeId;
use crate::semantic::SemanticAnalyzer;

impl SemanticAnalyzer {
    pub fn instantiate_generic_method(
        &mut self,
        type_id: TypeId,
        method_name: &str,
        concrete_arg_types: &[TypeId],
        call_site_span: Span,
    ) -> Option<(String, TypeId)> {
        let feature = match self.ctx.get_pending_generic_method(type_id, method_name) {
            Some(f) => f,
            None => {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::UnknownMethod {
                            type_name: self.ctx.types.get(type_id).name.clone(),
                            method: method_name.to_string(),
                        },
                        call_site_span,
                    )
                    .into(),
                );
                return None;
            }
        };
        let (decl_params, return_type, body): (
            &Vec<(String, Option<String>)>,
            &Option<String>,
            &Expr,
        ) = match &feature.node {
            TypeFeaturesKind::Method {
                params,
                return_type,
                body,
                ..
            } => (params, return_type, body),
            _ => panic!("pending_generic_methods must only contain Method features"),
        };
        if decl_params.len() != concrete_arg_types.len() {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidMethodArity {
                        method: method_name.to_string(),
                        expected: decl_params.len(),
                        found: concrete_arg_types.len(),
                    },
                    call_site_span,
                )
                .into(),
            );
            return None;
        }
        let key = (
            type_id,
            method_name.to_string(),
            concrete_arg_types.to_vec(),
        );
        self.ctx.mark_method_in_progress(key.clone());
        let mangled_name =
            self.ctx
                .mangle_method_instance_name(type_id, method_name, concrete_arg_types);
        self.ctx.push_scope();
        self.ctx.current_type = Some(type_id);
        self.ctx.declare(Symbol {
            name: "self".to_string(),
            kind: SymbolKind::Variable,
            ty: SymbolType::Variable(type_id),
            span: feature.span,
        });
        let mut typed_params = Vec::new();
        for ((param_name, _), &concrete_ty) in decl_params.iter().zip(concrete_arg_types.iter()) {
            self.ctx.declare(Symbol {
                name: param_name.clone(),
                kind: SymbolKind::Parameter,
                ty: SymbolType::Variable(concrete_ty),
                span: feature.span,
            });
            typed_params.push(TypedParam::new(
                TypedParamKind {
                    name: param_name.clone(),
                    type_id: concrete_ty,
                },
                feature.span,
            ));
        }
        let declared_return = return_type.as_ref().and_then(|t| self.ctx.types.resolve(t));
        self.ctx.current_function_return = declared_return;
        self.ctx.current_method = Some(method_name.to_string());
        let body_type = self.analyze_expr(body);
        self.ctx.current_function_return = None;
        self.ctx.current_method = None;
        let final_return = match declared_return {
            Some(expected_return) => {
                if !self.ctx.types.is_subtype_of(body_type.ty, expected_return) {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::MethodReturnTypeMismatch {
                                method: method_name.to_string(),
                                expected: self.ctx.types.get(expected_return).name.clone(),
                                found: self.ctx.types.get(body_type.ty).name.clone(),
                            },
                            body_type.span,
                        )
                        .into(),
                    );
                }
                expected_return
            }
            None => body_type.ty,
        };
        self.ctx.current_type = None;
        self.ctx.pop_scope();
        self.ctx.unmark_method_in_progress(&key);
        let self_param = TypedParam::new(
            TypedParamKind {
                name: "self".to_string(),
                type_id,
            },
            feature.span,
        );
        let mut all_params = vec![self_param];
        all_params.extend(typed_params);
        let typed_decl = TypedDecl::new(
            TypedDeclKind::Function {
                name: mangled_name.clone(),
                params: all_params,
                return_type: final_return,
                body: body_type,
            },
            feature.span,
        );
        self.ctx.insert_method_instance(key, typed_decl);
        Some((mangled_name, final_return))
    }
}
