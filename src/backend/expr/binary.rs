use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

use crate::semantic::types::TypeId;
use crate::{
    ast::BinaryOpKind,
    semantic::{
        SemanticAnalyzer,
        hir::{TypedExpr, TypedExprKind},
    },
};

use super::{Backend, BackendError, BackendResult};

impl<'ctx> Backend<'ctx> {
    pub fn compile_binary(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        if let TypedExprKind::Binary {
            left_expr,
            op,
            right_expr,
        } = &expr.node
        {
            let lhs = self.compile_expr(left_expr, sema)?;
            let rhs = self.compile_expr(right_expr, sema)?;
            match op.node {
                BinaryOpKind::Add
                | BinaryOpKind::Sub
                | BinaryOpKind::Mul
                | BinaryOpKind::Div
                | BinaryOpKind::Pow
                | BinaryOpKind::Less
                | BinaryOpKind::Greater
                | BinaryOpKind::LessEqual
                | BinaryOpKind::GreaterEqual => {
                    if let (BasicValueEnum::FloatValue(lhs_f), BasicValueEnum::FloatValue(rhs_f)) =
                        (lhs, rhs)
                    {
                        match op.node {
                            BinaryOpKind::Add => {
                                let res = self
                                    .builder
                                    .build_float_add(lhs_f, rhs_f, "add_tmp")
                                    .map_err(|_| BackendError::InvalidExpression)?;
                                return Ok(BasicValueEnum::FloatValue(res));
                            }
                            BinaryOpKind::Sub => {
                                let res = self
                                    .builder
                                    .build_float_sub(lhs_f, rhs_f, "sub_tmp")
                                    .map_err(|_| BackendError::InvalidExpression)?;
                                return Ok(BasicValueEnum::FloatValue(res));
                            }
                            BinaryOpKind::Mul => {
                                let res = self
                                    .builder
                                    .build_float_mul(lhs_f, rhs_f, "mul_tmp")
                                    .map_err(|_| BackendError::InvalidExpression)?;
                                return Ok(BasicValueEnum::FloatValue(res));
                            }
                            BinaryOpKind::Div => {
                                let res = self
                                    .builder
                                    .build_float_div(lhs_f, rhs_f, "div_tmp")
                                    .map_err(|_| BackendError::InvalidExpression)?;
                                return Ok(BasicValueEnum::FloatValue(res));
                            }
                            BinaryOpKind::Pow => {
                                let f64_type = lhs_f.get_type();
                                let fn_type =
                                    f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
                                let pow_fn =
                                    self.module.get_function("llvm.pow.f64").unwrap_or_else(|| {
                                        self.module.add_function("llvm.pow.f64", fn_type, None)
                                    });
                                let call = self
                                    .builder
                                    .build_call(pow_fn, &[lhs_f.into(), rhs_f.into()], "pow_tmp")
                                    .map_err(|_| BackendError::InvalidExpression)?;
                                let res =
                                    call.try_as_basic_value().unwrap_basic().into_float_value();
                                return Ok(BasicValueEnum::FloatValue(res));
                            }
                            BinaryOpKind::Less
                            | BinaryOpKind::Greater
                            | BinaryOpKind::LessEqual
                            | BinaryOpKind::GreaterEqual => {
                                let pred = match op.node {
                                    BinaryOpKind::Less => inkwell::FloatPredicate::OLT,
                                    BinaryOpKind::Greater => inkwell::FloatPredicate::OGT,
                                    BinaryOpKind::LessEqual => inkwell::FloatPredicate::OLE,
                                    BinaryOpKind::GreaterEqual => inkwell::FloatPredicate::OGE,
                                    _ => unreachable!(),
                                };
                                let res = self
                                    .builder
                                    .build_float_compare(pred, lhs_f, rhs_f, "cmp_tmp")
                                    .map_err(|_| BackendError::InvalidExpression)?;
                                return Ok(BasicValueEnum::IntValue(res));
                            }
                            _ => return Err(BackendError::InvalidExpression),
                        }
                    } else {
                        return Err(BackendError::InvalidExpression);
                    }
                }
                BinaryOpKind::Concat | BinaryOpKind::ConcatSpace => {
                    let lhs_str = self.ensure_string(lhs)?;
                    let rhs_str = self.ensure_string(rhs)?;
                    let ptr_type = lhs_str.get_type();
                    let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
                    let fn_name = match op.node {
                        BinaryOpKind::Concat => "hulk_string_concat",
                        BinaryOpKind::ConcatSpace => "hulk_string_concat_space",
                        _ => unreachable!(),
                    };
                    let concat_fn = self
                        .module
                        .get_function(fn_name)
                        .unwrap_or_else(|| self.module.add_function(fn_name, fn_type, None));
                    let call_site = self
                        .builder
                        .build_call(concat_fn, &[lhs_str.into(), rhs_str.into()], "concat_tmp")
                        .map_err(|_| BackendError::InvalidExpression)?;
                    let res_ptr = call_site
                        .try_as_basic_value()
                        .unwrap_basic()
                        .into_pointer_value();
                    return Ok(BasicValueEnum::PointerValue(res_ptr));
                }
                BinaryOpKind::DoubleEqual | BinaryOpKind::NotEqual => {
                    let is_equal = matches!(op.node, BinaryOpKind::DoubleEqual);
                    match (lhs, rhs) {
                        (BasicValueEnum::FloatValue(lhs_f), BasicValueEnum::FloatValue(rhs_f)) => {
                            let pred = if is_equal {
                                inkwell::FloatPredicate::OEQ
                            } else {
                                inkwell::FloatPredicate::UNE
                            };
                            let res = self
                                .builder
                                .build_float_compare(pred, lhs_f, rhs_f, "f_cmp_tmp")
                                .map_err(|_| BackendError::InvalidExpression)?;
                            return Ok(BasicValueEnum::IntValue(res));
                        }
                        (BasicValueEnum::IntValue(lhs_i), BasicValueEnum::IntValue(rhs_i)) => {
                            let pred = if is_equal {
                                inkwell::IntPredicate::EQ
                            } else {
                                inkwell::IntPredicate::NE
                            };
                            let res = self
                                .builder
                                .build_int_compare(pred, lhs_i, rhs_i, "b_cmp_tmp")
                                .map_err(|_| BackendError::InvalidExpression)?;
                            return Ok(BasicValueEnum::IntValue(res));
                        }
                        (
                            BasicValueEnum::PointerValue(lhs_p),
                            BasicValueEnum::PointerValue(rhs_p),
                        ) => {
                            let pred = if is_equal {
                                inkwell::IntPredicate::EQ
                            } else {
                                inkwell::IntPredicate::NE
                            };
                            let res = self
                                .builder
                                .build_int_compare(pred, lhs_p, rhs_p, "ref_cmp_tmp")
                                .map_err(|_| BackendError::InvalidExpression)?;
                            return Ok(BasicValueEnum::IntValue(res));
                        }
                        _ => return Err(BackendError::InvalidExpression),
                    }
                }
                BinaryOpKind::And | BinaryOpKind::Or => {
                    if let (BasicValueEnum::IntValue(lhs_i), BasicValueEnum::IntValue(rhs_i)) =
                        (lhs, rhs)
                    {
                        let res = match op.node {
                            BinaryOpKind::And => self
                                .builder
                                .build_and(lhs_i, rhs_i, "and_tmp")
                                .map_err(|_| BackendError::InvalidExpression)?,
                            BinaryOpKind::Or => self
                                .builder
                                .build_or(lhs_i, rhs_i, "or_tmp")
                                .map_err(|_| BackendError::InvalidExpression)?,
                            _ => unreachable!(),
                        };
                        return Ok(BasicValueEnum::IntValue(res));
                    } else {
                        return Err(BackendError::InvalidExpression);
                    }
                }
            }
        }
        Err(BackendError::InvalidExpression)
    }

    pub fn compile_is(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        let (operand_expr, target_type_id) = match &expr.node {
            TypedExprKind::Is {
                expr: operand,
                target_type,
            } => (operand.as_ref(), target_type),
            _ => return Err(BackendError::InvalidExpression),
        };
        let bool_type = self.llvm_context.bool_type();
        let operand_type_id = operand_expr.ty;
        let is_primitive = |type_id| -> bool {
            type_id == TypeId(0)
                || type_id == TypeId(1)
                || type_id == TypeId(2)
                || type_id == TypeId(3)
        };
        if is_primitive(operand_type_id) {
            self.compile_expr(operand_expr, sema)?;
            let result = operand_type_id == *target_type_id;
            return Ok(BasicValueEnum::IntValue(
                bool_type.const_int(result as u64, false),
            ));
        }
        if is_primitive(*target_type_id) {
            self.compile_expr(operand_expr, sema)?;
            return Ok(BasicValueEnum::IntValue(bool_type.const_int(0, false)));
        }

        let obj_val = self.compile_expr(operand_expr, sema)?;
        let obj_ptr = obj_val.into_pointer_value();
        let actual_type_tag = self.load_type_tag(obj_ptr, operand_expr.ty)?;

        let valid_type_ids = self.types.subtypes_of(*target_type_id);
        let tag_type = self.llvm_context.i32_type();
        let mut is_instance = bool_type.const_int(0, false);
        for valid_id in valid_type_ids {
            let tag_const = tag_type.const_int(valid_id.0 as u64, false);
            let eq = self
                .builder
                .build_int_compare(IntPredicate::EQ, actual_type_tag, tag_const, "tag_eq")
                .map_err(|_| BackendError::InvalidExpression)?;
            is_instance = self
                .builder
                .build_or(is_instance, eq, "is_or")
                .map_err(|_| BackendError::InvalidExpression)?;
        }
        Ok(BasicValueEnum::IntValue(is_instance))
    }

    pub fn compile_as(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        let (operand_expr, target_type_id) = match &expr.node {
            TypedExprKind::As {
                expr: operand,
                target_type,
            } => (operand.as_ref(), target_type),
            _ => return Err(BackendError::InvalidExpression),
        };
        let obj_val = self.compile_expr(operand_expr, sema)?;
        let target_llvm_ty = self.types.get_llvm_type(*target_type_id);
        match obj_val {
            BasicValueEnum::IntValue(int_val) => {
                if target_llvm_ty.is_int_type() {
                    Ok(BasicValueEnum::IntValue(int_val))
                } else {
                    Err(BackendError::InvalidExpression)
                }
            }
            BasicValueEnum::FloatValue(float_val) => {
                if target_llvm_ty.is_float_type() {
                    Ok(BasicValueEnum::FloatValue(float_val))
                } else {
                    Err(BackendError::InvalidExpression)
                }
            }
            BasicValueEnum::PointerValue(_) => {
                if target_llvm_ty.is_pointer_type() {
                    let res = self
                        .builder
                        .build_bit_cast(obj_val, target_llvm_ty, "ptr_cast")
                        .map_err(|_| BackendError::InvalidExpression)?;
                    Ok(res)
                } else {
                    Err(BackendError::InvalidExpression)
                }
            }
            _ => Err(BackendError::InvalidExpression),
        }
    }

    fn load_type_tag(
        &mut self,
        obj_ptr: inkwell::values::PointerValue<'ctx>,
        static_type_id: TypeId,
    ) -> BackendResult<inkwell::values::IntValue<'ctx>> {
        let ptr_ty = self.llvm_context.ptr_type(AddressSpace::default());
        let i32_ty = self.llvm_context.i32_type();

        let object_struct_type = self
            .types
            .get_layout(static_type_id)
            .ok_or(BackendError::InvalidExpression)?
            .struct_type;
        let vtable_field_ptr = self
            .builder
            .build_struct_gep(object_struct_type, obj_ptr, 0, "vtable_field_ptr")
            .map_err(|_| BackendError::InvalidExpression)?;
        let vtable_ptr = self
            .builder
            .build_load(ptr_ty, vtable_field_ptr, "load_vtable_ptr")
            .map_err(|_| BackendError::InvalidExpression)?
            .into_pointer_value();

        let vtable_struct_type = self
            .types
            .get_layout(static_type_id)
            .and_then(|l| l.vtable_struct_type)
            .or_else(|| {
                self.types
                    .layouts
                    .values()
                    .find_map(|l| l.vtable_struct_type)
            })
            .ok_or(BackendError::InvalidExpression)?;

        let tag_ptr = self
            .builder
            .build_struct_gep(vtable_struct_type, vtable_ptr, 0, "tag_field_ptr")
            .map_err(|_| BackendError::InvalidExpression)?;
        let tag_val = self
            .builder
            .build_load(i32_ty, tag_ptr, "load_type_tag")
            .map_err(|_| BackendError::InvalidExpression)?;
        Ok(tag_val.into_int_value())
    }

    fn ensure_string(&mut self, value: BasicValueEnum<'ctx>) -> BackendResult<PointerValue<'ctx>> {
        match value {
            BasicValueEnum::PointerValue(ptr) => Ok(ptr),
            BasicValueEnum::FloatValue(float_val) => {
                let f64_type = float_val.get_type();
                let ptr_type = self.llvm_context.ptr_type(inkwell::AddressSpace::from(0));
                let fn_type = ptr_type.fn_type(&[f64_type.into()], false);
                let conv_fn = self
                    .module
                    .get_function("hulk_number_to_string")
                    .unwrap_or_else(|| {
                        self.module
                            .add_function("hulk_number_to_string", fn_type, None)
                    });
                let call_site = self
                    .builder
                    .build_call(conv_fn, &[float_val.into()], "num_to_str_tmp")
                    .map_err(|_| BackendError::InvalidExpression)?;
                Ok(call_site
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value())
            }
            _ => Err(BackendError::InvalidExpression),
        }
    }
}
