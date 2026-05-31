use crate::ast::TypeFeaturesKind;
use crate::ast::{Decl, DeclKind};
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};
use crate::semantic::types::ConstructorParam;

impl SemanticAnalyzer {
    pub fn register_signatures(&mut self, decls: &[Decl]) {
        for decl in decls {
            match &decl.node {
                DeclKind::Function {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let mut param_types = Vec::new();
                    for (_, param_type_opt) in params {
                        let p_type = param_type_opt
                            .as_ref()
                            .and_then(|t| self.ctx.types.resolve(t))
                            .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                        param_types.push(p_type);
                    }
                    let ret_type = return_type
                        .as_ref()
                        .and_then(|t| self.ctx.types.resolve(t))
                        .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                    let new_ty = SymbolType::Function {
                        params: param_types,
                        ret: ret_type,
                    };
                    if !self.ctx.update_symbol_type(name, new_ty.clone()) {
                        self.ctx.declare(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::Function,
                            ty: new_ty,
                            span: decl.span,
                        });
                    }
                }
                DeclKind::Type {
                    name,
                    params,
                    parent,
                    features,
                } => {
                    let current_type_id =
                        self.ctx.types.resolve(name).expect(
                            "Type should have already been registered in collect_declarations",
                        );
                    if let Some(inherit_info) = parent {
                        if let Some(parent_type_id) =
                            self.ctx.types.resolve(&inherit_info.node.parent_name)
                        {
                            let parent_name = &inherit_info.node.parent_name;
                            if parent_name == "Number"
                                || parent_name == "String"
                                || parent_name == "Boolean"
                            {
                                self.diagnostics.push(
                                    SemanticError::new(
                                        SemanticErrorKind::InvalidInheritanceFromPrimitive {
                                            child: name.to_string(),
                                            parent: parent_name.to_string(),
                                        },
                                        inherit_info.span,
                                    )
                                    .into(),
                                );
                                self.ctx.types.set_parent(current_type_id, None);
                            } else {
                                self.ctx
                                    .types
                                    .set_parent(current_type_id, Some(parent_type_id));
                            }
                        }
                    }
                    let mut constructor_params = Vec::new();
                    if let Some(param_list) = params {
                        for (p_name, p_type_opt) in param_list {
                            let p_type =
                                p_type_opt.as_ref().and_then(|t| self.ctx.types.resolve(t));
                            constructor_params.push(ConstructorParam {
                                name: p_name.clone(),
                                ty: p_type,
                            });
                        }
                    }
                    self.ctx
                        .types
                        .set_constructor_params(current_type_id, constructor_params);
                    for feature in features {
                        if let TypeFeaturesKind::Method {
                            name: method_name,
                            params: method_params,
                            return_type,
                            ..
                        } = &feature.node
                        {
                            let mut p_types = Vec::new();
                            for (_, p_type_opt) in method_params {
                                let p_id = p_type_opt
                                    .as_ref()
                                    .and_then(|t| self.ctx.types.resolve(t))
                                    .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                                p_types.push(p_id);
                            }
                            let r_id = return_type
                                .as_ref()
                                .and_then(|t| self.ctx.types.resolve(t))
                                .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                            let method_symbol = Symbol {
                                name: method_name.clone(),
                                kind: SymbolKind::Function,
                                ty: SymbolType::Function {
                                    params: p_types,
                                    ret: r_id,
                                },
                                span: feature.span,
                            };
                            self.ctx.types.insert_method(current_type_id, method_symbol);
                        }
                    }
                }
            }
        }
    }
}
