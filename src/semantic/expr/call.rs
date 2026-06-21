use crate::ast::Expr;
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedExpr, TypedExprKind};
use crate::semantic::symbols::SymbolType;
use crate::semantic::{analyzer::SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn analyze_call(&mut self, name: &str, args: &Vec<Expr>, span: Span) -> TypedExpr {
        let object_type = self.resolve_builtin("Object");
        if name == "print" {
            return self.analyze_print_call(args, span);
        }
        let symbol_ty = self.ctx.lookup(name).map(|s| s.ty.clone());
        let resolved = match symbol_ty {
            Some(SymbolType::Function { params, ret }) => CallResolution::Concrete {
                param_types: params,
                return_type: ret,
                call_name: name.to_string(),
            },
            Some(SymbolType::GenericFunction { .. }) => CallResolution::Generic,
            Some(SymbolType::Variable(_)) => {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::NotAFunction {
                            name: name.to_string(),
                        },
                        span,
                    )
                    .into(),
                );
                return TypedExpr::new(
                    TypedExprKind::Call {
                        name: name.into(),
                        args: vec![],
                    },
                    object_type,
                    span,
                );
            }
            None => {
                if let Some(current_type_id) = self.ctx.current_type {
                    if let Some((params, ret)) = self.ctx.types.lookup_method(current_type_id, name)
                    {
                        CallResolution::Concrete {
                            param_types: params,
                            return_type: ret,
                            call_name: name.to_string(),
                        }
                    } else {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::UndefinedFunction {
                                    name: name.to_string(),
                                },
                                span,
                            )
                            .into(),
                        );
                        return TypedExpr::new(
                            TypedExprKind::Call {
                                name: name.into(),
                                args: vec![],
                            },
                            object_type,
                            span,
                        );
                    }
                } else {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::UndefinedFunction {
                                name: name.to_string(),
                            },
                            span,
                        )
                        .into(),
                    );
                    return TypedExpr::new(
                        TypedExprKind::Call {
                            name: name.into(),
                            args: vec![],
                        },
                        object_type,
                        span,
                    );
                }
            }
        };
        match resolved {
            CallResolution::Concrete {
                param_types,
                return_type,
                call_name,
            } => {
                self.analyze_concrete_call(&call_name, name, args, &param_types, return_type, span)
            }
            CallResolution::Generic => self.analyze_generic_call(name, args, span),
        }
    }

    fn analyze_print_call(&mut self, args: &[Expr], span: Span) -> TypedExpr {
        let object_type = self.resolve_builtin("Object");
        let string_type = self.resolve_builtin("String");
        let number_type = self.resolve_builtin("Number");
        if args.len() != 1 {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidFunctionArity {
                        name: "print".to_string(),
                        expected: 1,
                        found: args.len(),
                    },
                    span,
                )
                .into(),
            );
            let typed_args: Vec<TypedExpr> = args.iter().map(|a| self.analyze_expr(a)).collect();
            return TypedExpr::new(
                TypedExprKind::Call {
                    name: "print".into(),
                    args: typed_args,
                },
                object_type,
                span,
            );
        }
        let arg_type = self.analyze_expr(&args[0]);
        let is_supported = arg_type.ty == string_type || arg_type.ty == number_type;
        if !is_supported {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::FunctionArgumentTypeMismatch {
                        name: "print".to_string(),
                        index: 1,
                        expected: "String' or 'Number".to_string(),
                        found: self.ctx.types.get(arg_type.ty).name.clone(),
                    },
                    args[0].span,
                )
                .into(),
            );
        }
        let return_ty = arg_type.ty;
        TypedExpr::new(
            TypedExprKind::Call {
                name: "print".into(),
                args: vec![arg_type],
            },
            return_ty,
            span,
        )
    }

    fn analyze_concrete_call(
        &mut self,
        call_name: &str,
        display_name: &str,
        args: &[Expr],
        param_types: &[TypeId],
        return_type: TypeId,
        span: Span,
    ) -> TypedExpr {
        if args.len() != param_types.len() {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidFunctionArity {
                        name: display_name.to_string(),
                        expected: param_types.len(),
                        found: args.len(),
                    },
                    span,
                )
                .into(),
            );
        }
        let mut typed_args = Vec::new();
        for (i, arg) in args.iter().enumerate() {
            let arg_type = self.analyze_expr(arg);
            if i < param_types.len() {
                let expected_type = param_types[i];
                if !self.ctx.types.is_subtype_of(arg_type.ty, expected_type) {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::FunctionArgumentTypeMismatch {
                                name: display_name.to_string(),
                                index: i + 1,
                                expected: self.ctx.types.get(expected_type).name.clone(),
                                found: self.ctx.types.get(arg_type.ty).name.clone(),
                            },
                            arg.span,
                        )
                        .into(),
                    );
                }
            }
            typed_args.push(arg_type);
        }
        TypedExpr::new(
            TypedExprKind::Call {
                name: call_name.into(),
                args: typed_args,
            },
            return_type,
            span,
        )
    }

    fn analyze_generic_call(&mut self, name: &str, args: &[Expr], span: Span) -> TypedExpr {
        let object_type = self.resolve_builtin("Object");
        let (param_types_decl, declared_arity) = match self.ctx.lookup(name).map(|s| &s.ty) {
            Some(SymbolType::GenericFunction { param_types, .. }) => {
                (param_types.clone(), param_types.len())
            }
            _ => unreachable!("analyze_generic_call called for a non-generic symbol"),
        };
        if args.len() != declared_arity {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidFunctionArity {
                        name: name.to_string(),
                        expected: declared_arity,
                        found: args.len(),
                    },
                    span,
                )
                .into(),
            );
        }
        let mut typed_args = Vec::new();
        let mut instance_key_types = Vec::new();
        for (i, arg) in args.iter().enumerate() {
            let arg_type = self.analyze_expr(arg);
            let declared = param_types_decl.get(i).copied().flatten();
            match declared {
                Some(expected_type) => {
                    if !self.ctx.types.is_subtype_of(arg_type.ty, expected_type) {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::FunctionArgumentTypeMismatch {
                                    name: name.to_string(),
                                    index: i + 1,
                                    expected: self.ctx.types.get(expected_type).name.clone(),
                                    found: self.ctx.types.get(arg_type.ty).name.clone(),
                                },
                                arg.span,
                            )
                            .into(),
                        );
                    }
                    instance_key_types.push(expected_type);
                }
                None => {
                    instance_key_types.push(arg_type.ty);
                }
            }
            typed_args.push(arg_type);
        }
        let key = (name.to_string(), instance_key_types.clone());
        let (mangled_name, return_type) = if let Some(existing) = self.ctx.get_instance(&key) {
            let ret = existing.node_return_type();
            (
                self.ctx.mangle_instance_name(name, &instance_key_types),
                ret,
            )
        } else if self.ctx.is_in_progress(&key) {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::GenericInferenceFailed {
                        function: name.to_string(),
                    },
                    span,
                )
                .into(),
            );
            (name.to_string(), object_type)
        } else {
            match self.instantiate_generic_function(name, &instance_key_types, span) {
                Some((mangled_name, ret)) => (mangled_name, ret),
                None => (name.to_string(), object_type),
            }
        };
        TypedExpr::new(
            TypedExprKind::Call {
                name: mangled_name,
                args: typed_args,
            },
            return_type,
            span,
        )
    }

    pub fn analyze_base_call(&mut self, name: &str, args: &Vec<Expr>, span: Span) -> TypedExpr {
        let object_type = self.resolve_builtin("Object");
        let current_type_id = match self.ctx.current_type {
            Some(id) => id,
            None => {
                self.diagnostics
                    .push(SemanticError::new(SemanticErrorKind::InvalidBaseUsage, span).into());
                return TypedExpr::new(
                    TypedExprKind::Call {
                        name: name.into(),
                        args: vec![],
                    },
                    object_type,
                    span,
                );
            }
        };
        let current_method_name = match &self.ctx.current_method {
            Some(m) => m,
            None => {
                self.diagnostics
                    .push(SemanticError::new(SemanticErrorKind::InvalidBaseUsage, span).into());
                return TypedExpr::new(
                    TypedExprKind::Call {
                        name: name.into(),
                        args: vec![],
                    },
                    object_type,
                    span,
                );
            }
        };
        if !args.is_empty() {
            self.diagnostics
                .push(SemanticError::new(SemanticErrorKind::BaseTakesNoArguments, span).into());
        }
        let type_id = match self.find_closest_ancestor_method(current_type_id, current_method_name)
        {
            Some(ancestor_return_type) => ancestor_return_type,
            None => {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::UndefinedBaseMethod {
                            type_name: self.ctx.types.get(current_type_id).name.clone(),
                            method: current_method_name.to_string(),
                        },
                        span,
                    )
                    .into(),
                );
                self.ctx.types.resolve("Object").unwrap()
            }
        };
        TypedExpr::new(
            TypedExprKind::Call {
                name: name.into(),
                args: vec![],
            },
            type_id,
            span,
        )
    }

    fn find_closest_ancestor_method(&self, type_id: TypeId, method_name: &str) -> Option<TypeId> {
        let mut current_id = type_id;
        while let Some(parent_id) = self.ctx.types.get_parent(current_id) {
            if let Some(return_type_id) = self
                .ctx
                .types
                .get_method_return_type(parent_id, method_name)
            {
                return Some(return_type_id);
            }
            current_id = parent_id;
        }
        None
    }
}

enum CallResolution {
    Concrete {
        param_types: Vec<TypeId>,
        return_type: TypeId,
        call_name: String,
    },
    Generic,
}

#[cfg(test)]
mod tests {
    use crate::semantic::{error::SemanticErrorKind, test_utils::parse_program, SemanticAnalyzer};

    #[test]
    fn semantic_unit_test_call_function_err() {
        let source = r#"
type A() {
    B() => C();
}

{
    let x = 1 in {
        x(42);
        y(42);
    };
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 3);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::UndefinedFunction {
                name: "C".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::NotAFunction {
                name: "x".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::UndefinedFunction {
                name: "y".to_string()
            }
        );
    }

    #[test]
    fn semantic_unit_test_call_function_arity_err() {
        let source = r#"
function A() {
    42;
}

function B(b) {
    42;
}

{
    A(1);
    B();
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 2);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::InvalidFunctionArity {
                name: "A".to_string(),
                expected: 0,
                found: 1
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::InvalidFunctionArity {
                name: "B".to_string(),
                expected: 1,
                found: 0
            }
        );
    }

    #[test]
    fn semantic_unit_test_call_function_type_mismatch_err() {
        let source = r#"
function A(a: Number, b: String, c: Boolean) {
    42;
}

{
    A(true, 1, "hello");
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 3);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::FunctionArgumentTypeMismatch {
                name: "A".to_string(),
                index: 1,
                expected: "Number".to_string(),
                found: "Boolean".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::FunctionArgumentTypeMismatch {
                name: "A".to_string(),
                index: 2,
                expected: "String".to_string(),
                found: "Number".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::FunctionArgumentTypeMismatch {
                name: "A".to_string(),
                index: 3,
                expected: "Boolean".to_string(),
                found: "String".to_string()
            }
        );
    }

    #[test]
    fn semantic_unit_test_call_function_base_err() {
        let source = r#" 
type A {
    f() => 42;
}
        
type B inherits A {
    a = base();

    f() => base(1);

    g() => base();
} 

{
    let x = base() in 42;
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 4);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::InvalidBaseUsage
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::BaseTakesNoArguments
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::UndefinedBaseMethod {
                type_name: "B".to_string(),
                method: "g".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[3].kind,
            SemanticErrorKind::InvalidBaseUsage
        );
    }
}
