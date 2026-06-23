pub mod assign;
pub mod binary;
pub mod block;
pub mod call;
pub mod control_flow;
pub mod let_expr;
pub mod new;
pub mod postfix;
pub mod primary;
pub mod unary;

use inkwell::values::BasicValueEnum;

use crate::semantic::{
    SemanticAnalyzer,
    hir::{TypedExpr, TypedExprKind},
};

use super::{Backend, BackendError, BackendResult};

impl<'ctx> Backend<'ctx> {
    pub fn compile_expr(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        match &expr.node {
            TypedExprKind::Literal(..) => self.compile_literal(expr),
            TypedExprKind::Variable(..) => self.compile_variable(expr),
            TypedExprKind::Block(..) => self.compile_block(expr, sema),
            TypedExprKind::Call { .. } => self.compile_call(expr, sema),
            TypedExprKind::PropertyAccess { .. } => self.compile_property_access(expr, sema),
            TypedExprKind::MethodCall { .. } => self.compile_method_call(expr, sema),
            TypedExprKind::Unary { .. } => self.compile_unary(expr, sema),
            TypedExprKind::Binary { .. } => self.compile_binary(expr, sema),
            TypedExprKind::Is { .. } => self.compile_is(expr, sema),
            TypedExprKind::As { .. } => self.compile_as(expr, sema),
            TypedExprKind::Let { .. } => self.compile_let(expr, sema),
            TypedExprKind::If { .. } => self.compile_if(expr, sema),
            TypedExprKind::While { .. } => self.compile_while(expr, sema),
            TypedExprKind::Assign { .. } => self.compile_assign(expr, sema),
            TypedExprKind::New { .. } => self.compile_new(expr, sema),
        }
    }
}
