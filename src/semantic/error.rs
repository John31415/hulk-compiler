use crate::{
    diagnostics::{Diagnostic, Label},
    lexer::span::Span,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticErrorKind {
    DuplicateType {
        name: String,
    },
    DuplicateFunction {
        name: String,
    },
    DuplicateParameter {
        function: String,
        param: String,
    },
    TypeMismatch {
        expected: String,
        found: String,
    },
    CyclicInheritance {
        name: String,
    },
    InvalidAssignmentTarget,
    InvalidCast {
        from: String,
        to: String,
    },
    InvalidConditionType {
        found: String,
    },
    InvalidBinaryOperation {
        operator: String,
        left: String,
        right: String,
    },
    IncomparableTypes {
        left: String,
        right: String,
    },
    ImpossibleTypeCheck {
        expr: String,
        target: String,
    },
    UnknownType {
        name: String,
    },
    InvalidOperatorOperand {
        operator: String,
        expected: String,
        found: String,
    },
    NotAFunction {
        name: String,
    },
    UndefinedFunction {
        name: String,
    },
    InvalidFunctionArity {
        name: String,
        expected: usize,
        found: usize,
    },
    FunctionArgumentTypeMismatch {
        name: String,
        index: usize,
        expected: String,
        found: String,
    },
    InvalidBaseUsage,
    BaseTakesNoArguments,
    UndefinedBaseMethod {
        type_name: String,
        method: String,
    },
    InvalidWhileCondition {
        found: String,
    },
    LetBindingTypeMismatch {
        expected: String,
        found: String,
    },
    UnknownTypeInLetAnnotation {
        name: String,
    },
    InvalidConstructorArity {
        type_name: String,
        expected: usize,
        found: usize,
    },
    ConstructorArgumentTypeMismatch {
        type_name: String,
        param: String,
        expected: String,
        found: String,
    },
    UnknownAttribute {
        type_name: String,
        attribute: String,
    },
    InvalidMethodArity {
        method: String,
        expected: usize,
        found: usize,
    },
    MethodArgumentTypeMismatch {
        method: String,
        index: usize,
        expected: String,
        found: String,
    },
    UnknownMethod {
        type_name: String,
        method: String,
    },
    NotAVariable {
        name: String,
    },
    UndefinedVariable {
        name: String,
    },
    InvalidUnaryOperation {
        operator: String,
        operand: String,
    },
    InvalidInheritanceFromPrimitive {
        child: String,
        parent: String,
    },
    UnknownTypeInParameter {
        type_name: String,
        param_name: String,
    },
    UnknownReturnType {
        function: String,
        type_name: String,
    },
    FunctionReturnTypeMismatch {
        function: String,
        expected: String,
        found: String,
    },
    UnknownTypeInFunctionParameter {
        function: String,
        param: String,
        type_name: String,
    },
    DuplicateConstructorParameter {
        type_name: String,
        param: String,
    },
    InvalidInheritanceArity {
        parent: String,
        expected: usize,
        found: usize,
    },
    InheritanceArgumentTypeMismatch {
        parent: String,
        param: String,
        expected: String,
        found: String,
    },
    UnknownParentType {
        child: String,
        parent: String,
    },
    UnknownTypeInAttribute {
        type_name: String,
        attribute: String,
    },
    AttributeTypeMismatch {
        attribute: String,
        expected: String,
        found: String,
    },
    DuplicateAttribute {
        type_name: String,
        attribute: String,
    },
    UnknownReturnTypeInMethod {
        method: String,
        type_name: String,
    },
    MethodReturnTypeMismatch {
        method: String,
        expected: String,
        found: String,
    },
    UnknownTypeInMethodParameter {
        method: String,
        param: String,
        type_name: String,
    },
    InvalidOverrideArity {
        method: String,
        found: usize,
        expected: usize,
    },
    InvalidOverrideReturnType {
        method: String,
        found: String,
        expected: String,
    },
    InvalidOverrideParameterType {
        method: String,
        param_name: String,
        found: String,
        expected: String,
    },
    InvalidPropertyAccess,
}

impl SemanticErrorKind {
    pub fn message(&self) -> String {
        match self {
            Self::DuplicateType { name } => format!("duplicate type '{}'", name),
            Self::DuplicateFunction { name } => format!("duplicate function '{}'", name),
            Self::UnknownType { name } => {
                format!("type '{}' does not exist in the current scope", name)
            }
            Self::TypeMismatch { expected, found } => {
                format!("type mismatch: expected '{}', found '{}'", expected, found)
            }
            Self::CyclicInheritance { name } => {
                format!("cyclic inheritance detected at type '{}'", name)
            }
            Self::InvalidAssignmentTarget => "invalid assignment target".to_string(),
            Self::InvalidCast { from, to } => {
                format!(
                    "cannot cast expression of type '{}' to completely unrelated type '{}'",
                    from, to
                )
            }
            Self::InvalidConditionType { found } => {
                format!("'if' condition must be 'Boolean', but found '{}'", found)
            }
            Self::InvalidBinaryOperation {
                operator,
                left,
                right,
            } => {
                format!(
                    "operator '{}' requires both operands to be of compatible types: either both Strings, or one String and one Number. Received left: '{}', right: '{}'",
                    operator, left, right,
                )
            }
            Self::IncomparableTypes { left, right } => {
                format!(
                    "cannot compare completely unrelated types: '{}' and '{}'",
                    left, right
                )
            }
            Self::ImpossibleTypeCheck { expr, target } => {
                format!(
                    "expression of type '{}' can never be an instance of '{}'",
                    expr, target
                )
            }
            Self::InvalidOperatorOperand {
                operator,
                expected,
                found,
            } => {
                format!(
                    "operator '{}' expects type '{}', but found '{}'",
                    operator, expected, found
                )
            }
            Self::NotAFunction { name } => {
                format!("identifier '{}' is a variable, not a function", name)
            }
            Self::UndefinedFunction { name } => {
                format!("function or method '{}' is not defined in this scope", name)
            }
            Self::InvalidFunctionArity {
                name,
                expected,
                found,
            } => {
                format!(
                    "function '{}' expects '{}' arguments, but '{}' were provided",
                    name, expected, found
                )
            }
            Self::FunctionArgumentTypeMismatch {
                name,
                index,
                expected,
                found,
            } => {
                format!(
                    "type mismatch in call to '{}': argument '{}' expects '{}', found '{}'",
                    name, index, expected, found
                )
            }
            Self::InvalidBaseUsage => "base() can only be used inside a type method".to_string(),
            Self::BaseTakesNoArguments => "base() does not take explicit arguments".to_string(),
            Self::UndefinedBaseMethod { type_name, method } => {
                format!(
                    "no ancestor of type '{}' implements the method '{}'",
                    type_name, method
                )
            }
            Self::InvalidWhileCondition { found } => {
                format!("'while' condition must be 'Boolean', but found '{}'", found)
            }
            Self::LetBindingTypeMismatch { expected, found } => {
                format!(
                    "type mismatch in let binding: cannot assign '{}' to explicit type '{}'",
                    found, expected
                )
            }
            Self::UnknownTypeInLetAnnotation { name } => {
                format!("non-existent type '{}' in let type annotation", name)
            }
            Self::InvalidConstructorArity {
                type_name,
                expected,
                found,
            } => {
                format!(
                    "type '{}' constructor expects '{}' arguments, but '{}' were provided",
                    type_name, expected, found
                )
            }
            Self::ConstructorArgumentTypeMismatch {
                type_name,
                param,
                expected,
                found,
            } => {
                format!(
                    "type mismatch in instantiation of '{}': parameter '{}' expects '{}', found '{}'",
                    type_name, param, expected, found
                )
            }
            Self::UnknownAttribute {
                type_name,
                attribute,
            } => {
                format!(
                    "type '{}' has no attribute named '{}'",
                    type_name, attribute
                )
            }
            Self::InvalidMethodArity {
                method,
                expected,
                found,
            } => {
                format!(
                    "method '{}' expects '{}' arguments, but '{}' were provided",
                    method, expected, found
                )
            }
            Self::MethodArgumentTypeMismatch {
                method,
                index,
                expected,
                found,
            } => {
                format!(
                    "type mismatch in method '{}' call: argument '{}' expects '{}', found '{}'",
                    method, index, expected, found
                )
            }
            Self::UnknownMethod { type_name, method } => {
                format!("type '{}' has no method named '{}'", type_name, method)
            }
            Self::NotAVariable { name } => {
                format!("identifier '{}' is a function, not a variable", name)
            }
            Self::UndefinedVariable { name } => {
                format!(
                    "variable, parameter or attribute '{}' is not defined in this scope",
                    name
                )
            }
            Self::InvalidUnaryOperation { operator, operand } => {
                format!(
                    "operator '{}' cannot be applied to type '{}'",
                    operator, operand
                )
            }
            Self::InvalidInheritanceFromPrimitive { child, parent } => {
                format!(
                    "type '{}' cannot inherit from primitive type '{}'",
                    child, parent
                )
            }
            Self::UnknownTypeInParameter {
                type_name,
                param_name,
            } => {
                format!(
                    "non-existent type '{}' for parameter '{}'",
                    type_name, param_name
                )
            }
            Self::DuplicateParameter { function, param } => {
                format!(
                    "parameter '{}' is already defined in function '{}'",
                    param, function
                )
            }
            Self::UnknownReturnType {
                function,
                type_name,
            } => {
                format!(
                    "non-existent return type '{}' in function '{}'",
                    type_name, function
                )
            }
            Self::FunctionReturnTypeMismatch {
                function,
                expected,
                found,
            } => {
                format!(
                    "type mismatch in '{}': body returns '{}' but expected '{}'",
                    function, found, expected
                )
            }
            Self::UnknownTypeInFunctionParameter {
                function,
                param,
                type_name,
            } => {
                format!(
                    "non-existent type '{}' for parameter '{}' in '{}'",
                    type_name, param, function
                )
            }
            Self::DuplicateConstructorParameter { type_name, param } => {
                format!(
                    "parameter '{}' is already defined in the constructor of type '{}'",
                    param, type_name
                )
            }
            Self::InvalidInheritanceArity {
                parent,
                expected,
                found,
            } => {
                format!(
                    "type '{}' expects '{}' arguments in its constructor, but '{}' were passed",
                    parent, expected, found
                )
            }
            Self::InheritanceArgumentTypeMismatch {
                parent,
                param,
                expected,
                found,
            } => {
                format!(
                    "incompatible type in inheritance argument '{}' of '{}': inferred '{}' but expected '{}'",
                    param, parent, found, expected
                )
            }
            Self::UnknownParentType { child, parent } => {
                format!(
                    "type '{}' attempts to inherit from non-existent type '{}'",
                    child, parent
                )
            }
            Self::UnknownTypeInAttribute {
                type_name,
                attribute,
            } => {
                format!(
                    "non-existent type '{}' for attribute '{}'",
                    type_name, attribute
                )
            }
            Self::AttributeTypeMismatch {
                attribute,
                expected,
                found,
            } => {
                format!(
                    "incompatible type in attribute '{}': cannot assign '{}' to '{}'",
                    attribute, found, expected
                )
            }
            Self::DuplicateAttribute {
                type_name,
                attribute,
            } => {
                format!(
                    "attribute '{}' is already defined in type '{}' or its ancestors",
                    attribute, type_name
                )
            }
            Self::UnknownReturnTypeInMethod { method, type_name } => {
                format!(
                    "non-existent return type '{}' in method '{}'",
                    type_name, method
                )
            }
            Self::MethodReturnTypeMismatch {
                method,
                expected,
                found,
            } => {
                format!(
                    "method '{}' must return '{}' but its body returns '{}'",
                    method, expected, found
                )
            }
            Self::UnknownTypeInMethodParameter {
                method,
                param,
                type_name,
            } => {
                format!(
                    "non-existent type '{}' for parameter '{}' in method '{}'",
                    type_name, param, method
                )
            }
            Self::InvalidOverrideArity {
                method,
                found,
                expected,
            } => {
                format!(
                    "override method '{}' has '{}' parameter(s), but its parent declaration expects '{}'",
                    method, found, expected
                )
            }
            Self::InvalidOverrideReturnType {
                method,
                found,
                expected,
            } => {
                format!(
                    "the return type '{}' of override method '{}' does not match the parent's expected return type '{}'",
                    found, method, expected
                )
            }
            Self::InvalidOverrideParameterType {
                method,
                param_name,
                found,
                expected,
            } => {
                format!(
                    "parameter '{}' of override method '{}' has type '{}', but parent method expects '{}'",
                    param_name, method, found, expected
                )
            }
            Self::InvalidPropertyAccess => {
                format!("properties are private and can only be accessed via 'self'",)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: Span,
}

impl SemanticError {
    pub fn new(kind: SemanticErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl From<SemanticError> for Diagnostic {
    fn from(value: SemanticError) -> Self {
        let message = value.kind.message();
        Diagnostic::error(message.clone(), value.span).with_label(Label::new(message, value.span))
    }
}
