use std::collections::HashMap;

use crate::ast::{Block, BinaryOp, Expr, ExprKind, Function, Program, Type};
use inkwell::AddressSpace;
use inkwell::FloatPredicate;
use inkwell::IntPredicate;
use inkwell::builder::{Builder, BuilderError};
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{InitializationConfig, Target, TargetMachine};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum};

type ValueWithType = (Option<BasicValueEnum<'static>>, Type);

pub struct Generator {
    types: HashMap<usize, Type>,
    current_scope: Vec<HashMap<String, BasicValueEnum<'static>>>,
    context: *const Context,
    module: *const Module<'static>,
    builder: *const Builder<'static>,
    next_label: usize,
}

impl Generator {
    pub fn new(types: &HashMap<usize, Type>) -> Generator {
        let context = Box::new(Context::create());
        let context: &'static Context = Box::leak(context);
        let module = Box::new(context.create_module("funC-module"));
        let module: &'static Module<'static> = Box::leak(module);
        let builder = Box::new(context.create_builder());
        let builder: &'static Builder<'static> = Box::leak(builder);

        Self {
            types: types.clone(),
            current_scope: vec![HashMap::new()],
            context: context as *const _,
            module: module as *const _,
            builder: builder as *const _,
            next_label: 0,
        }
    }

    pub fn generate(mut self, program: &Program) -> String {
        let triple = init_target_machine();
        self.module_ref().set_triple(&triple);
        declare_runtime(self.context, self.module);

        for function in &program.functions {
            self.emit_function(function);
        }

        self.module_ref().print_to_string().to_string()
    }

    #[inline]
    fn context_ref(&self) -> &'static Context {
        unsafe { std::mem::transmute::<*const Context, &'static Context>(self.context) }
    }

    #[inline]
    fn module_ref(&self) -> &'static Module<'static> {
        unsafe { std::mem::transmute::<*const Module<'static>, &'static Module<'static>>(self.module) }
    }

    #[inline]
    fn builder_ref(&self) -> &'static Builder<'static> {
        unsafe { std::mem::transmute::<*const Builder<'static>, &'static Builder<'static>>(self.builder) }
    }

    fn expect<T>(&self, value: Result<T, BuilderError>, operation: &str) -> T {
        value.expect(operation)
    }

    fn i8_ptr_type(&self) -> inkwell::types::PointerType<'static> {
        unsafe { std::mem::transmute(self.context_ref().ptr_type(AddressSpace::default())) }
    }

    fn llvm_type(&self, ty: &Type) -> Option<BasicTypeEnum<'static>> {
        let context = self.context_ref();
        let value = match ty {
            Type::Void => return None,
            Type::Bool => context.bool_type().as_basic_type_enum(),
            Type::I8 => context.i8_type().as_basic_type_enum(),
            Type::I16 => context.i16_type().as_basic_type_enum(),
            Type::I32 => context.i32_type().as_basic_type_enum(),
            Type::I64 => context.i64_type().as_basic_type_enum(),
            Type::U8 => context.i8_type().as_basic_type_enum(),
            Type::U16 => context.i16_type().as_basic_type_enum(),
            Type::U32 => context.i32_type().as_basic_type_enum(),
            Type::U64 => context.i64_type().as_basic_type_enum(),
            Type::F32 => context.f32_type().as_basic_type_enum(),
            Type::F64 => context.f64_type().as_basic_type_enum(),
            Type::Pointer(_) => self.i8_ptr_type().as_basic_type_enum(),
        };
        Some(unsafe {
            std::mem::transmute::<
                BasicTypeEnum<'_>,
                BasicTypeEnum<'static>,
            >(value)
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
            self.context_ref()
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

        let compiled = self.module_ref().add_function(&function.name, fn_type, None);
        let entry = self.context_ref().append_basic_block(compiled, "entry");
        self.builder_ref().position_at_end(entry);

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
            let _ = self.builder_ref().build_return(None);
        } else if let Some(value) = ret_value {
            let _ = self.builder_ref().build_return(Some(&value));
        } else {
            let fallback = self
                .numeric_zero_value(&function.return_type)
                .expect("cannot build fallback zero");
            let _ = self.builder_ref().build_return(Some(&fallback));
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

    fn emit_expr(&mut self, expr: &Expr) -> ValueWithType {
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
                let _ = self.builder_ref()
                    .build_store(ptr, rhs.0.expect("store without RHS"));
                (None, Type::Void)
            }
            ExprKind::IfElse {
                condition,
                then_block,
                else_block,
            } => self.emit_if(condition, then_block, else_block),
            ExprKind::Not(expr) => {
                let expr = self.emit_expr(expr);
                let expr = expr.0.expect("unary ! without value").into_int_value();
                let value = self
                    .expect(self.builder_ref().build_not(expr, "not"), "not")
                    .as_basic_value_enum();
                (Some(value), Type::Bool)
            }
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
                (Some(self.context_ref().i64_type().const_int(*value as u64, true).as_basic_value_enum()), Type::I64)
            }
            ExprKind::FloatLiteral(value) => (
                Some(self.context_ref().f64_type().const_float(*value).as_basic_value_enum()),
                Type::F64,
            ),
            ExprKind::BoolLiteral(value) => (
                Some(self.context_ref().bool_type().const_int(*value as u64, false).as_basic_value_enum()),
                Type::Bool,
            ),
            ExprKind::Call { name, args } => {
                let function = self
                    .module_ref()
                    .get_function(name)
                    .unwrap_or_else(|| panic!("function '{}' non déclarée", name));

                let args = args
                    .iter()
                    .map(|arg| {
                        let value = self.emit_expr(arg).0.expect("call arg without value");
                        BasicMetadataValueEnum::from(value)
                    })
                    .collect::<Vec<_>>();
                let call = self.expect(
                    self.builder_ref().build_call(function, &args, "call"),
                    "call",
                );
                if ty == Type::Void {
                    (None, Type::Void)
                } else {
                    (Some(
                        call.try_as_basic_value()
                            .expect_basic("call should return a value"),
                    ), ty)
                }
            }
            ExprKind::Alloc(size) => {
                let (size, _) = self.emit_expr(size);
                let malloc = self
                    .module_ref()
                    .get_function("malloc")
                    .expect("malloc manquant");
                let size = size.expect("sizeof alloc invalide");
                let ptr = self
                    .expect(
                        self.builder_ref()
                            .build_call(malloc, &[size.into()], "malloc_call"),
                        "malloc",
                    )
                    .try_as_basic_value()
                    .expect_basic("malloc returned no value");
                (Some(ptr), Type::Pointer(Box::new(Type::I8)))
            }
            ExprKind::Free(ptr) => {
                let ptr = self.emit_expr(ptr);
                let ptr = ptr.0.expect("free needs pointer");
                let ptr = ptr.into_pointer_value();
                let free_fn = self
                    .module_ref()
                    .get_function("free")
                    .expect("free manquant");
                let ptr = if ptr.get_type() == self.i8_ptr_type() {
                    ptr
                } else {
                    self.expect(
                        self.builder_ref()
                            .build_pointer_cast(ptr, self.i8_ptr_type(), "free_cast"),
                        "free_cast",
                    )
                };
                self.expect(
                    self.builder_ref().build_call(free_fn, &[ptr.into()], "free_call"),
                    "free",
                );
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
                let loaded = self.expect(
                    self.builder_ref().build_load(pointee_type, ptr, "load"),
                    "load",
                );
                (Some(loaded), pointee)
            }
            ExprKind::SizeOf(ty) => (
                Some(
                    self.context_ref()
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
    ) -> ValueWithType {
        let lhs = self.emit_expr(left).0.expect("binary lhs without value");
        let rhs = self.emit_expr(right).0.expect("binary rhs without value");

        if ty.is_float() {
            let lhs = lhs.into_float_value();
            let rhs = rhs.into_float_value();
            let value = match op {
                BinaryOp::Eq => self
                    .expect(
                        self.builder_ref()
                            .build_float_compare(FloatPredicate::OEQ, lhs, rhs, "fcmp_eq"),
                        "fcmp_eq",
                    )
                    .as_basic_value_enum(),
                BinaryOp::NotEq => self
                    .expect(
                        self.builder_ref()
                            .build_float_compare(FloatPredicate::ONE, lhs, rhs, "fcmp_ne"),
                        "fcmp_ne",
                    )
                    .as_basic_value_enum(),
                BinaryOp::Lt => self
                    .expect(
                        self.builder_ref()
                            .build_float_compare(FloatPredicate::OLT, lhs, rhs, "fcmp_lt"),
                        "fcmp_lt",
                    )
                    .as_basic_value_enum(),
                BinaryOp::LtEq => self
                    .expect(
                        self.builder_ref()
                            .build_float_compare(FloatPredicate::OLE, lhs, rhs, "fcmp_lte"),
                        "fcmp_lte",
                    )
                    .as_basic_value_enum(),
                BinaryOp::Gt => self
                    .expect(
                        self.builder_ref()
                            .build_float_compare(FloatPredicate::OGT, lhs, rhs, "fcmp_gt"),
                        "fcmp_gt",
                    )
                    .as_basic_value_enum(),
                BinaryOp::GtEq => self
                    .expect(
                        self.builder_ref()
                            .build_float_compare(FloatPredicate::OGE, lhs, rhs, "fcmp_gte"),
                        "fcmp_gte",
                    )
                    .as_basic_value_enum(),
                BinaryOp::Add => self
                    .expect(self.builder_ref().build_float_add(lhs, rhs, "fadd"), "fadd")
                    .as_basic_value_enum(),
                BinaryOp::Sub => self
                    .expect(self.builder_ref().build_float_sub(lhs, rhs, "fsub"), "fsub")
                    .as_basic_value_enum(),
                BinaryOp::Mul => self
                    .expect(self.builder_ref().build_float_mul(lhs, rhs, "fmul"), "fmul")
                    .as_basic_value_enum(),
                BinaryOp::Div => self
                    .expect(self.builder_ref().build_float_div(lhs, rhs, "fdiv"), "fdiv")
                    .as_basic_value_enum(),
                BinaryOp::Mod => self
                    .expect(self.builder_ref().build_float_rem(lhs, rhs, "frem"), "frem")
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
                    ty.clone()
                },
            );
        }

        if let Type::Pointer(_) = ty {
            let lhs = lhs.into_pointer_value();
            let rhs = rhs.into_pointer_value();
            let lhs = self
                .expect(
                    self.builder_ref()
                        .build_ptr_to_int(lhs, self.context_ref().i64_type(), "cmp_lhs"),
                    "ptr_to_int lhs",
                );
            let rhs = self
                .expect(
                    self.builder_ref()
                        .build_ptr_to_int(rhs, self.context_ref().i64_type(), "cmp_rhs"),
                    "ptr_to_int rhs",
                );
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
                        self.expect(
                            self.builder_ref().build_int_compare(predicate, lhs, rhs, "icmp_ptr"),
                            "icmp_ptr",
                        )
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
                        self.expect(
                            self.builder_ref().build_int_compare(predicate, lhs, rhs, "icmp"),
                            "icmp",
                        )
                            .as_basic_value_enum(),
                    ),
                    Type::Bool,
                );
            }

            let value = match op {
                BinaryOp::Add => self
                    .expect(self.builder_ref().build_int_add(lhs, rhs, "add"), "add")
                    .as_basic_value_enum(),
                BinaryOp::Sub => self
                    .expect(self.builder_ref().build_int_sub(lhs, rhs, "sub"), "sub")
                    .as_basic_value_enum(),
                BinaryOp::Mul => self
                    .expect(self.builder_ref().build_int_mul(lhs, rhs, "mul"), "mul")
                    .as_basic_value_enum(),
                BinaryOp::Div => {
                    if ty.is_signed_integer() {
                        self.expect(
                            self.builder_ref().build_int_signed_div(lhs, rhs, "sdiv"),
                            "sdiv",
                        )
                            .as_basic_value_enum()
                    } else {
                        self.expect(
                            self.builder_ref().build_int_unsigned_div(lhs, rhs, "udiv"),
                            "udiv",
                        )
                            .as_basic_value_enum()
                    }
                }
                BinaryOp::Mod => {
                    if ty.is_signed_integer() {
                        self.expect(
                            self.builder_ref().build_int_signed_rem(lhs, rhs, "srem"),
                            "srem",
                        )
                            .as_basic_value_enum()
                    } else {
                        self.expect(
                            self.builder_ref().build_int_unsigned_rem(lhs, rhs, "urem"),
                            "urem",
                        )
                            .as_basic_value_enum()
                    }
                }
                BinaryOp::Or | BinaryOp::And | BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                    unreachable!()
                }
            };
            (Some(value), ty.clone())
        }
    }

    fn emit_if(
        &mut self,
        condition: &Expr,
        then_block: &Block,
        else_block: &Block,
    ) -> ValueWithType {
        let condition = self.emit_expr(condition);
        let cond_value = condition.0.expect("if condition without value");

        let current = self
            .builder_ref()
            .get_insert_block()
            .expect("builder position required");
        let parent = current.get_parent().expect("basic block without parent");

        let then_bb = self.context_ref().append_basic_block(parent, &self.next_label("then"));
        let else_bb = self.context_ref().append_basic_block(parent, &self.next_label("else"));
        let merge_bb = self.context_ref().append_basic_block(parent, &self.next_label("merge"));
        let _ = self
            .builder_ref()
            .build_conditional_branch(cond_value.into_int_value(), then_bb, else_bb);

        self.builder_ref().position_at_end(then_bb);
        let (then_value, then_type) = self.emit_block(then_block);
        let _ = self.builder_ref().build_unconditional_branch(merge_bb);

        self.builder_ref().position_at_end(else_bb);
        let (else_value, else_type) = self.emit_block(else_block);
        let _ = self.builder_ref().build_unconditional_branch(merge_bb);

        self.builder_ref().position_at_end(merge_bb);
            if then_type != Type::Void && then_type == else_type {
                let phi_type = self
                    .llvm_type(&then_type)
                    .expect("if value requires phi type");
            let phi = self.expect(self.builder_ref().build_phi(phi_type, "if_phi"), "if_phi");
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
    ) -> ValueWithType {
        let left = self.emit_expr(left);
        let left_val = left.0.expect("logical lhs without value").into_int_value();
        let current = self
            .builder_ref()
            .get_insert_block()
            .expect("builder position required");
        let parent = current.get_parent().expect("basic block without parent");

        let rhs_bb = self.context_ref().append_basic_block(parent, &self.next_label("logic_rhs"));
        let short_bb = self.context_ref().append_basic_block(parent, &self.next_label("logic_short"));
        let merge_bb = self.context_ref().append_basic_block(parent, &self.next_label("logic_merge"));

        match op {
            BinaryOp::Or => {
                let _ = self.builder_ref()
                    .build_conditional_branch(left_val, short_bb, rhs_bb);
            }
            BinaryOp::And => {
                let _ = self.builder_ref()
                    .build_conditional_branch(left_val, rhs_bb, short_bb);
            }
            _ => unreachable!(),
        }

        self.builder_ref().position_at_end(rhs_bb);
        let rhs = self.emit_expr(right);
        let rhs_value = rhs.0.expect("rhs of logical missing value");
        let _ = self.builder_ref().build_unconditional_branch(merge_bb);

        self.builder_ref().position_at_end(short_bb);
        let short_value = match op {
            BinaryOp::Or => self.context_ref().bool_type().const_int(1, false).as_basic_value_enum(),
            BinaryOp::And => self.context_ref().bool_type().const_int(0, false).as_basic_value_enum(),
            _ => unreachable!(),
        };
        let _ = self.builder_ref().build_unconditional_branch(merge_bb);

        self.builder_ref().position_at_end(merge_bb);
        let phi = self
            .builder_ref()
            .build_phi(self.context_ref().bool_type().as_basic_type_enum(), "logical_phi")
            ;
        let phi = self.expect(phi, "logical_phi");
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
        let context = self.context_ref();
        let value = match ty {
            Type::Bool => Some(context.bool_type().const_zero().as_basic_value_enum()),
            Type::I8 => Some(context.i8_type().const_zero().as_basic_value_enum()),
            Type::I16 => Some(context.i16_type().const_zero().as_basic_value_enum()),
            Type::I32 => Some(context.i32_type().const_zero().as_basic_value_enum()),
            Type::I64 => Some(context.i64_type().const_zero().as_basic_value_enum()),
            Type::U8 => Some(context.i8_type().const_zero().as_basic_value_enum()),
            Type::U16 => Some(context.i16_type().const_zero().as_basic_value_enum()),
            Type::U32 => Some(context.i32_type().const_zero().as_basic_value_enum()),
            Type::U64 => Some(context.i64_type().const_zero().as_basic_value_enum()),
            Type::F32 => Some(context.f32_type().const_zero().as_basic_value_enum()),
            Type::F64 => Some(context.f64_type().const_zero().as_basic_value_enum()),
            Type::Pointer(_) => Some(self.i8_ptr_type().const_zero().as_basic_value_enum()),
            Type::Void => None,
        };
        value
            .map(|value| unsafe {
                std::mem::transmute::<BasicValueEnum<'_>, BasicValueEnum<'static>>(value)
            })
    }
}

pub fn generate(program: &Program, types: &HashMap<usize, Type>) -> String {
    Generator::new(types).generate(program)
}

fn declare_runtime(context: *const Context, module: *const Module<'static>) {
    let context = unsafe { &*context };
    let module = unsafe { &*module };
    let malloc_type = context
        .ptr_type(AddressSpace::default())
        .fn_type(&[context.i64_type().into()], false);
    let _ = module.add_function("malloc", malloc_type, None);

    let free_type = context
        .void_type()
        .fn_type(&[context.ptr_type(AddressSpace::default()).into()], false);
    let _ = module.add_function("free", free_type, None);
}

pub fn init_target_machine() -> inkwell::targets::TargetTriple {
    Target::initialize_all(&InitializationConfig::default());
    let triple = TargetMachine::get_default_triple();
    triple
}
