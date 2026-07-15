use std::collections::HashMap;

use crate::ast::{Block, BinaryOp, Expr, ExprKind, Function, Program, Type};
use inkwell::AddressSpace;
use inkwell::FloatPredicate;
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{InitializationConfig, Target, TargetMachine};
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum};

type ValueWithType<'ctx> = (Option<BasicValueEnum<'ctx>>, Type);

pub struct Generator {
    types: HashMap<usize, Type>,
    current_scope: Vec<HashMap<String, BasicValueEnum<'static>>>,
    context: &'static Context,
    module: Module<'static>,
    builder: Builder<'static>,
    next_label: usize,
}

impl Generator {
    pub fn new(types: &HashMap<usize, Type>) -> Self {
        let context = Box::new(Context::create());
        let context: &'static Context = Box::leak(context);
        let module = context.create_module("funC-module");
        let builder = context.create_builder();

        Self {
            types: types.clone(),
            current_scope: vec![HashMap::new()],
            context,
            module,
            builder,
            next_label: 0,
        }
    }

    pub fn generate(mut self, program: &Program) -> String {
        let triple = init_target_machine();
        self.module.set_triple(&triple);
        self.declare_runtime();

        for function in &program.functions {
            self.emit_function(function);
        }

        self.module.print_to_string().to_string()
    }

    fn declare_runtime(&self) {
        let malloc_type = self
            .i8_ptr_type()
            .fn_type(&[self.context.i64_type().into()], false);
        self.module
            .add_function("malloc", malloc_type, None);

        let free_type = self
            .context
            .void_type()
            .fn_type(&[self.i8_ptr_type().into()], false);
        self.module
            .add_function("free", free_type, None);
    }

    fn i8_ptr_type(&self) -> inkwell::types::PointerType {
        self.context.i8_type().ptr_type(AddressSpace::default())
    }

    fn llvm_type(&self, ty: &Type) -> Option<BasicTypeEnum<'static>> {
        Some(match ty {
            Type::Void => return None,
            Type::Bool => self.context.bool_type().as_basic_type_enum(),
            Type::I8 => self.context.i8_type().as_basic_type_enum(),
            Type::I16 => self.context.i16_type().as_basic_type_enum(),
            Type::I32 => self.context.i32_type().as_basic_type_enum(),
            Type::I64 => self.context.i64_type().as_basic_type_enum(),
            Type::U8 => self.context.i8_type().as_basic_type_enum(),
            Type::U16 => self.context.i16_type().as_basic_type_enum(),
            Type::U32 => self.context.i32_type().as_basic_type_enum(),
            Type::U64 => self.context.i64_type().as_basic_type_enum(),
            Type::F32 => self.context.f32_type().as_basic_type_enum(),
            Type::F64 => self.context.f64_type().as_basic_type_enum(),
            Type::Pointer(_) => self.i8_ptr_type().as_basic_type_enum(),
        })
    }

    fn next_label(&mut self, prefix: &str) -> String {
        let id = self.next_label;
        self.next_label += 1;
        format!("{}_{}", prefix, id)
    }

    fn emit_function(&mut self, function: &Function) {
        let fn_params = function
            .params
            .iter()
            .map(|p| self.llvm_type(&p.ty).expect("invalid parameter type"))
            .collect::<Vec<_>>();
        let fn_type = if function.return_type == Type::Void {
            self.context
                .void_type()
                .fn_type(&fn_params.iter().map(|t| (*t).into()).collect::<Vec<_>>(), false)
        } else {
            self.llvm_type(&function.return_type)
                .expect("invalid return type")
                .fn_type(
                    &fn_params.iter().map(|t| (*t).into()).collect::<Vec<_>>(),
                    false,
                )
        };

        let compiled = self.module.add_function(&function.name, fn_type, None);
        let entry = self.context.append_basic_block(compiled, "entry");
        self.builder.position_at_end(entry);

        self.current_scope.push(HashMap::new());
        for (index, param) in function.params.iter().enumerate() {
            let arg = compiled
                .get_nth_param(index as u32)
                .expect("missing function parameter");
            self.current_scope
                .last_mut()
                .unwrap()
                .insert(param.name.clone(), arg);
        }

        let (ret_value, _ret_type) = self.emit_block(&function.body);
        if function.return_type == Type::Void {
            self.builder.build_return(None).unwrap();
        } else if let Some(value) = ret_value {
            self.builder.build_return(Some(&value)).unwrap();
        } else {
            let fallback = self
                .numeric_zero_value(&function.return_type)
                .expect("cannot build fallback zero");
            self.builder.build_return(Some(&fallback)).unwrap();
        }
        self.current_scope.pop();
    }

    fn emit_block(&mut self, block: &Block) -> (Option<BasicValueEnum<'static>>, Type) {
        self.current_scope.push(HashMap::new());
        let mut value = None;
        let mut ty = Type::Void;
        for expr in &block.expressions {
            let current = self.emit_expr(expr);
            if current.0.is_some() {
                value = current.0;
                ty = current.1;
            }
        }
        self.current_scope.pop();
        (value, ty)
    }

    fn emit_expr(&mut self, expr: &Expr) -> ValueWithType<'static> {
        let ty = self.types.get(&expr.id).cloned().unwrap_or(Type::Void);
        match &expr.kind {
            ExprKind::Let { name, value, .. } => {
                let emitted = self.emit_expr(value);
                if let Some(value) = emitted.0 {
                    self.current_scope
                        .last_mut()
                        .unwrap()
                        .insert(name.clone(), value);
                }
                emitted
            }
            ExprKind::Store(value, ptr) => {
                let rhs = self.emit_expr(value);
                let ptr = self.emit_expr(ptr);
                let ptr = ptr.0.expect("store on non-value pointer").into_pointer_value();
                self.builder
                    .build_store(ptr, rhs.0.expect("store without RHS"))
                    .unwrap();
                (None, Type::Void)
            }
            ExprKind::IfElse {
                condition,
                then_block,
                else_block,
            } => self.emit_if(condition, then_block, else_block),
            ExprKind::Binary(op, left, right) => {
                if matches!(op, BinaryOp::Or | BinaryOp::And) {
                    self.emit_logical(op, left, right)
                } else {
                    self.emit_binary(op, left, right, &ty)
                }
            }
            ExprKind::Identifier(name) => {
                let value = self
                    .current_scope
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(name).cloned())
                    .expect("identifier non résolu");
                (Some(value), ty)
            }
            ExprKind::IntLiteral(value) => {
                (Some(self.context.i64_type().const_int(*value as u64, true).as_basic_value_enum()), Type::I64)
            }
            ExprKind::FloatLiteral(value) => (
                Some(self.context.f64_type().const_float(*value).as_basic_value_enum()),
                Type::F64,
            ),
            ExprKind::BoolLiteral(value) => (
                Some(self.context.bool_type().const_int(*value as u64, false).as_basic_value_enum()),
                Type::Bool,
            ),
            ExprKind::Call { name, args } => {
                let function = self
                    .module
                    .get_function(name)
                    .unwrap_or_else(|| panic!("function '{}' non déclarée", name));

                let args = args
                    .iter()
                    .map(|arg| {
                        let value = self.emit_expr(arg).0.expect("call arg without value");
                        BasicMetadataValueEnum::from(value)
                    })
                    .collect::<Vec<_>>();
                let call = self
                    .builder
                    .build_call(function, &args, "call")
                    .unwrap();
                if ty == Type::Void {
                    (None, Type::Void)
                } else {
                    (Some(call.try_as_basic_value().unwrap_basic()), ty)
                }
            }
            ExprKind::Alloc(size) => {
                let (size, _) = self.emit_expr(size);
                let malloc = self
                    .module
                    .get_function("malloc")
                    .expect("malloc manquant");
                let size = size.expect("sizeof alloc invalide");
                let ptr = self
                    .builder
                    .build_call(malloc, &[size.into()], "malloc_call")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .expect("malloc returned no value");
                (Some(ptr), Type::Pointer(Box::new(Type::I8)))
            }
            ExprKind::Free(ptr) => {
                let ptr = self.emit_expr(ptr);
                let ptr = ptr.0.expect("free needs pointer");
                let ptr = ptr.into_pointer_value();
                let free_fn = self
                    .module
                    .get_function("free")
                    .expect("free manquant");
                let ptr = if ptr.get_type() == self.i8_ptr_type() {
                    ptr
                } else {
                    self.builder
                        .build_pointer_cast(ptr, self.i8_ptr_type(), "free_cast")
                        .unwrap()
                };
                self.builder
                    .build_call(free_fn, &[ptr.into()], "free_call")
                    .unwrap();
                (None, Type::Void)
            }
            ExprKind::Load(ptr) => {
                let (ptr, ptr_ty) = self.emit_expr(ptr);
                let ptr = ptr.expect("load needs pointer").into_pointer_value();
                let pointee = match ptr_ty {
                    Type::Pointer(inner) => *inner,
                    _ => Type::Void,
                };
                let pointee_type = self.llvm_type(&pointee).expect("invalid pointee type");
                let loaded = self
                    .builder
                    .build_load(pointee_type, ptr, "load")
                    .unwrap();
                (Some(loaded), pointee)
            }
            ExprKind::SizeOf(ty) => (
                Some(
                    self.context
                        .i64_type()
                        .const_int(self.size_of_type(ty) as u64, false)
                        .as_basic_value_enum(),
                ),
                Type::I64,
            ),
            ExprKind::Block(block) => self.emit_block(block),
        }
    }

    fn emit_binary(
        &mut self,
        op: &BinaryOp,
        left: &Box<Expr>,
        right: &Box<Expr>,
        ty: &Type,
    ) -> ValueWithType<'static> {
        let lhs = self.emit_expr(left).0.expect("binary lhs without value");
        let rhs = self.emit_expr(right).0.expect("binary rhs without value");

        if ty.is_float() {
            let lhs = lhs.into_float_value();
            let rhs = rhs.into_float_value();
            let value = match op {
                BinaryOp::Eq => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, lhs, rhs, "fcmp_eq")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::NotEq => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, lhs, rhs, "fcmp_ne")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::Lt => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, lhs, rhs, "fcmp_lt")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::LtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, lhs, rhs, "fcmp_lte")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::Gt => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, lhs, rhs, "fcmp_gt")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::GtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, lhs, rhs, "fcmp_gte")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::Add => self
                    .builder
                    .build_float_add(lhs, rhs, "fadd")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::Sub => self
                    .builder
                    .build_float_sub(lhs, rhs, "fsub")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::Mul => self
                    .builder
                    .build_float_mul(lhs, rhs, "fmul")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::Div => self
                    .builder
                    .build_float_div(lhs, rhs, "fdiv")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::Mod => self
                    .builder
                    .build_float_rem(lhs, rhs, "frem")
                    .unwrap()
                    .as_basic_value_enum(),
                _ => unreachable!(),
            };

            return (
                Some(value),
                if matches!(
                    op,
                    BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq
                ) {
                    Type::Bool
                } else {
                    *ty
                },
            );
        }

        if let Type::Pointer(_) = ty {
            let lhs = lhs.into_pointer_value();
            let rhs = rhs.into_pointer_value();
            let lhs = self
                .builder
                .build_ptr_to_int(lhs, self.context.i64_type(), "cmp_lhs")
                .unwrap();
            let rhs = self
                .builder
                .build_ptr_to_int(rhs, self.context.i64_type(), "cmp_rhs")
                .unwrap();
            let predicate = match op {
                BinaryOp::Eq => IntPredicate::EQ,
                BinaryOp::NotEq => IntPredicate::NE,
                BinaryOp::Lt => IntPredicate::ULT,
                BinaryOp::LtEq => IntPredicate::ULE,
                BinaryOp::Gt => IntPredicate::UGT,
                BinaryOp::GtEq => IntPredicate::UGE,
                _ => unreachable!(),
            };
            (
                Some(
                    self.builder
                        .build_int_compare(predicate, lhs, rhs, "icmp_ptr")
                        .unwrap()
                        .as_basic_value_enum(),
                ),
                Type::Bool,
            )
        } else {
            let lhs = lhs.into_int_value();
            let rhs = rhs.into_int_value();
            let compare = matches!(op, BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq);
            if compare {
                let predicate = match op {
                    BinaryOp::Eq => IntPredicate::EQ,
                    BinaryOp::NotEq => IntPredicate::NE,
                    BinaryOp::Lt => {
                        if ty.is_signed_integer() {
                            IntPredicate::SLT
                        } else {
                            IntPredicate::ULT
                        }
                    }
                    BinaryOp::LtEq => {
                        if ty.is_signed_integer() {
                            IntPredicate::SLE
                        } else {
                            IntPredicate::ULE
                        }
                    }
                    BinaryOp::Gt => {
                        if ty.is_signed_integer() {
                            IntPredicate::SGT
                        } else {
                            IntPredicate::UGT
                        }
                    }
                    BinaryOp::GtEq => {
                        if ty.is_signed_integer() {
                            IntPredicate::SGE
                        } else {
                            IntPredicate::UGE
                        }
                    }
                    _ => unreachable!(),
                };
                return (
                    Some(
                        self.builder
                            .build_int_compare(predicate, lhs, rhs, "icmp")
                            .unwrap()
                            .as_basic_value_enum(),
                    ),
                    Type::Bool,
                );
            }

            let value = match op {
                BinaryOp::Add => self
                    .builder
                    .build_int_add(lhs, rhs, "add")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::Sub => self
                    .builder
                    .build_int_sub(lhs, rhs, "sub")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::Mul => self
                    .builder
                    .build_int_mul(lhs, rhs, "mul")
                    .unwrap()
                    .as_basic_value_enum(),
                BinaryOp::Div => {
                    if ty.is_signed_integer() {
                        self.builder
                            .build_int_signed_div(lhs, rhs, "sdiv")
                            .unwrap()
                            .as_basic_value_enum()
                    } else {
                        self.builder
                            .build_int_unsigned_div(lhs, rhs, "udiv")
                            .unwrap()
                            .as_basic_value_enum()
                    }
                }
                BinaryOp::Mod => {
                    if ty.is_signed_integer() {
                        self.builder
                            .build_int_signed_rem(lhs, rhs, "srem")
                            .unwrap()
                            .as_basic_value_enum()
                    } else {
                        self.builder
                            .build_int_unsigned_rem(lhs, rhs, "urem")
                            .unwrap()
                            .as_basic_value_enum()
                    }
                }
                BinaryOp::Or | BinaryOp::And | BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                    unreachable!()
                }
            };
            (Some(value), *ty)
        }
    }

    fn emit_if(
        &mut self,
        condition: &Expr,
        then_block: &Block,
        else_block: &Block,
    ) -> ValueWithType<'static> {
        let condition = self.emit_expr(condition);
        let cond_value = condition.0.expect("if condition without value");

        let current = self
            .builder
            .get_insert_block()
            .expect("builder position required");
        let parent = current.get_parent().expect("basic block without parent");

        let then_bb = self.context.append_basic_block(parent, &self.next_label("then"));
        let else_bb = self.context.append_basic_block(parent, &self.next_label("else"));
        let merge_bb = self.context.append_basic_block(parent, &self.next_label("merge"));
        self.builder
            .build_conditional_branch(cond_value.into_int_value(), then_bb, else_bb)
            .unwrap();

        self.builder.position_at_end(then_bb);
        let (then_value, then_type) = self.emit_block(then_block);
        self.builder.build_unconditional_branch(merge_bb).unwrap();

        self.builder.position_at_end(else_bb);
        let (else_value, else_type) = self.emit_block(else_block);
        self.builder.build_unconditional_branch(merge_bb).unwrap();

        self.builder.position_at_end(merge_bb);
        if then_type != Type::Void && then_type == else_type {
            let phi_type = self
                .llvm_type(&then_type)
                .expect("if value requires phi type");
            let phi = self
                .builder
                .build_phi(phi_type, "if_phi")
                .unwrap();
            let then_value =
                then_value.expect("then block should return a value");
            let else_value =
                else_value.expect("else block should return a value");
            phi.add_incoming(&[( &then_value, then_bb), (&else_value, else_bb)]);
            (Some(phi.as_basic_value()), then_type)
        } else {
            (None, Type::Void)
        }
    }

    fn emit_logical(
        &mut self,
        op: &BinaryOp,
        left: &Expr,
        right: &Expr,
    ) -> ValueWithType<'static> {
        let left = self.emit_expr(left);
        let left_val = left.0.expect("logical lhs without value").into_int_value();
        let current = self
            .builder
            .get_insert_block()
            .expect("builder position required");
        let parent = current.get_parent().expect("basic block without parent");

        let rhs_bb = self.context.append_basic_block(parent, &self.next_label("logic_rhs"));
        let short_bb = self.context.append_basic_block(parent, &self.next_label("logic_short"));
        let merge_bb = self.context.append_basic_block(parent, &self.next_label("logic_merge"));

        match op {
            BinaryOp::Or => {
                self.builder
                    .build_conditional_branch(left_val, short_bb, rhs_bb)
                    .unwrap();
            }
            BinaryOp::And => {
                self.builder
                    .build_conditional_branch(left_val, rhs_bb, short_bb)
                    .unwrap();
            }
            _ => unreachable!(),
        }

        self.builder.position_at_end(rhs_bb);
        let rhs = self.emit_expr(right);
        let rhs_value = rhs.0.expect("rhs of logical missing value");
        self.builder.build_unconditional_branch(merge_bb).unwrap();

        self.builder.position_at_end(short_bb);
        let short_value = match op {
            BinaryOp::Or => self.context.bool_type().const_int(1, false).as_basic_value_enum(),
            BinaryOp::And => self.context.bool_type().const_int(0, false).as_basic_value_enum(),
            _ => unreachable!(),
        };
        self.builder.build_unconditional_branch(merge_bb).unwrap();

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.context.bool_type().as_basic_type_enum(), "logical_phi")
            .unwrap();
        phi.add_incoming(&[(&short_value, short_bb), (&rhs_value, rhs_bb)]);
        (Some(phi.as_basic_value()), Type::Bool)
    }

    fn size_of_type(&self, ty: &Type) -> i64 {
        match ty {
            Type::Void => 0,
            Type::Bool => 1,
            Type::I8 | Type::U8 => 1,
            Type::I16 | Type::U16 => 2,
            Type::I32 | Type::U32 | Type::F32 => 4,
            Type::I64 | Type::U64 | Type::F64 => 8,
            Type::Pointer(_) => 8,
        }
    }

    fn numeric_zero_value(&self, ty: &Type) -> Option<BasicValueEnum<'static>> {
        match ty {
            Type::Bool => Some(self.context.bool_type().const_zero().as_basic_value_enum()),
            Type::I8 => Some(self.context.i8_type().const_zero().as_basic_value_enum()),
            Type::I16 => Some(self.context.i16_type().const_zero().as_basic_value_enum()),
            Type::I32 => Some(self.context.i32_type().const_zero().as_basic_value_enum()),
            Type::I64 => Some(self.context.i64_type().const_zero().as_basic_value_enum()),
            Type::U8 => Some(self.context.i8_type().const_zero().as_basic_value_enum()),
            Type::U16 => Some(self.context.i16_type().const_zero().as_basic_value_enum()),
            Type::U32 => Some(self.context.i32_type().const_zero().as_basic_value_enum()),
            Type::U64 => Some(self.context.i64_type().const_zero().as_basic_value_enum()),
            Type::F32 => Some(self.context.f32_type().const_zero().as_basic_value_enum()),
            Type::F64 => Some(self.context.f64_type().const_zero().as_basic_value_enum()),
            Type::Pointer(_) => Some(self.i8_ptr_type().const_zero().as_basic_value_enum()),
            Type::Void => None,
        }
    }
}

pub fn generate(program: &Program, types: &HashMap<usize, Type>) -> String {
    Generator::new(types).generate(program)
}

pub fn init_target_machine() -> String {
    Target::initialize_all(&InitializationConfig::default());
    let triple = TargetMachine::get_default_triple();
    triple.as_str().to_string()
}
