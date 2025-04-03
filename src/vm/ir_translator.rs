use std::result::Result;

use super::instruction_set::*;
use super::ir::DebugInfo;
use super::ir::IROperation;
use super::ir::IRPackage;
use super::ir::IR;
use super::opcode::*;
use rustc_hash::FxHashMap as HashMap;

#[derive(Debug)]
pub enum IRTranslatorError {
    InvalidInstruction(IR),
}

#[derive(Debug)]
pub struct IRTranslator {
    ir_package: IRPackage,
    function_ips: HashMap<String, usize>, // 签名定位表
    ir_to_ip: Vec<usize>,                 // ir -> 指令集地址映射
    code: Vec<u32>,
    string_pool: Vec<String>,
    bytes_pool: Vec<Vec<u8>>,
    debug_infos: HashMap<usize, DebugInfo>,
}

impl IRTranslator {
    pub fn new(ir_package: &IRPackage) -> Self {
        IRTranslator {
            ir_package: ir_package.clone(),
            function_ips: HashMap::default(),
            ir_to_ip: vec![],
            code: vec![],
            string_pool: vec![],
            bytes_pool: vec![],
            debug_infos: HashMap::default(),
        }
    }
    pub fn alloc_string(&mut self, value: String) -> usize {
        let index = self.string_pool.len();
        self.string_pool.push(value);
        index
    }
    pub fn alloc_bytes(&mut self, value: Vec<u8>) -> usize {
        let index = self.bytes_pool.len();
        self.bytes_pool.push(value);
        index
    }
}

impl IRTranslator {
    pub fn translate(&mut self) -> Result<(), IRTranslatorError> {
        let mut redirect_table = Vec::<(usize/*偏移计算位置*/, usize/*填充位置*/, usize/*跳转的ir*/, bool)>::new(); // bool 表示是 i64 填充
        let cloned = self.ir_package.instructions.clone();
        for idx in 0..cloned.len() {
            let (debug_info, ir) = cloned[idx].clone();
            self.ir_to_ip.push(self.code.len());
            self.debug_infos.insert(self.code.len(), debug_info);
            match ir {
                IR::LoadInt(value) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::LoadInt64 as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    self.code.push(Opcode32::lower32(value as u64));
                    self.code.push(Opcode32::upper32(value as u64));
                }
                IR::LoadFloat(value) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::LoadFloat64 as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64 | OperandFlag::ShiftType,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    self.code.push(Opcode32::f64lower32(value));
                    self.code.push(Opcode32::f64upper32(value));
                }
                IR::LoadNull => {
                    self.code.push(
                        Opcode32::build_opcode(VMInstruction::LoadNull as u8, 0, 0, 0).get_opcode(),
                    );
                }
                IR::LoadString(value) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::LoadString as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64 | OperandFlag::UseConstPool,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    let index = self.alloc_string(value);
                    self.code.push(Opcode32::lower32(index as u64));
                    self.code.push(Opcode32::upper32(index as u64));
                }
                IR::LoadBytes(value) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::LoadBytes as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64 | OperandFlag::UseConstPool | OperandFlag::ShiftType,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    let index = self.alloc_bytes(value);
                    self.code.push(Opcode32::lower32(index as u64));
                    self.code.push(Opcode32::upper32(index as u64));
                }
                IR::LoadBool(value) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::LoadBool as u8,
                            OperandFlag::Valid as u8,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    self.code.push(value as u32);
                }
                IR::LoadLambda(signature, code_position) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::LoadLambda as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64 | OperandFlag::UseConstPool,
                            OperandFlag::Valid | OperandFlag::ArgSize64,
                            0,
                        )
                        .get_opcode(),
                    );
                    let index = self.alloc_string(signature);
                    self.code.push(Opcode32::lower32(index as u64));
                    self.code.push(Opcode32::upper32(index as u64));
                    self.code.push(Opcode32::lower32(code_position as u64));
                    self.code.push(Opcode32::upper32(code_position as u64));
                }
                IR::ForkInstruction => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Fork as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::BuildTuple(size) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::BuildTuple as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    self.code.push(Opcode32::lower32(size as u64));
                    self.code.push(Opcode32::upper32(size as u64));
                }
                IR::BuildKeyValue => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::BuildKeyValue as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::BuildNamed => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::BuildNamed as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::BuildRange => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::BuildRange as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::BindSelf => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::BindSelf as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::BinaryOp(op) => {
                    let opcode = match op {
                        IROperation::Add => VMInstruction::BinaryAdd,
                        IROperation::Subtract => VMInstruction::BinarySub,
                        IROperation::Multiply => VMInstruction::BinaryMul,
                        IROperation::Divide => VMInstruction::BinaryDiv,
                        IROperation::Modulus => VMInstruction::BinaryMod,
                        IROperation::Power => VMInstruction::BinaryPow,
                        IROperation::BitwiseAnd => VMInstruction::BinaryBitAnd,
                        IROperation::BitwiseOr => VMInstruction::BinaryBitOr,
                        IROperation::BitwiseXor => VMInstruction::BinaryBitXor,
                        IROperation::ShiftLeft => VMInstruction::BinaryShl,
                        IROperation::ShiftRight => VMInstruction::BinaryShr,
                        IROperation::And => VMInstruction::BinaryAnd,
                        IROperation::Or => VMInstruction::BinaryOr,
                        IROperation::Equal => VMInstruction::BinaryEq,
                        IROperation::NotEqual => VMInstruction::BinaryNe,
                        IROperation::Greater => VMInstruction::BinaryGt,
                        IROperation::Less => VMInstruction::BinaryLt,
                        IROperation::GreaterEqual => VMInstruction::BinaryGe,
                        IROperation::LessEqual => VMInstruction::BinaryLe,
                        _ => {
                            return Err(IRTranslatorError::InvalidInstruction(IR::BinaryOp(
                                op.clone()
                            )))
                        }
                    };
                    self.code.push(
                        Opcode32::build_opcode(
                            opcode as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::UnaryOp(op) => {
                    let opcode = match op {
                        IROperation::Not => VMInstruction::UnaryNot,
                        IROperation::BitwiseNot => VMInstruction::UnaryBitNot,
                        IROperation::Add => VMInstruction::UnaryAbs,
                        IROperation::Subtract => VMInstruction::UnaryNeg,
                        _ => {
                            return Err(IRTranslatorError::InvalidInstruction(IR::UnaryOp(
                                op.clone(),
                            )))
                        }
                    };
                    self.code.push(
                        Opcode32::build_opcode(
                            opcode as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::Let(name) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::StoreVar as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    let index = self.alloc_string(name);
                    self.code.push(Opcode32::lower32(index as u64));
                    self.code.push(Opcode32::upper32(index as u64));
                }
                IR::Get(name) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::LoadVar as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    let index = self.alloc_string(name);
                    self.code.push(Opcode32::lower32(index as u64));
                    self.code.push(Opcode32::upper32(index as u64));
                }
                IR::Set => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::SetValue as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::Wrap => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::WrapObj as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::GetAttr => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::GetAttr as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::IndexOf => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::IndexOf as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::KeyOf => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::KeyOf as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::ValueOf => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::ValueOf as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::SelfOf => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::SelfOf as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::TypeOf => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::TypeOf as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::CallLambda => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Call as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::Return => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Return as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::Raise => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Raise as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::NewFrame => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::NewFrame as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::NewBoundaryFrame(ir_offset) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::NewBoundaryFrame as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    self.code.push(0u32);
                    self.code.push(0u32);
                    redirect_table.push((self.code.len(), self.code.len() - 2, (idx as isize + ir_offset + 1) as usize, true));
                }
                IR::PopFrame => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::PopFrame as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::PopBoundaryFrame => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::PopBoundaryFrame as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::Pop => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Pop as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::JumpOffset(offset) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Jump as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    self.code.push(0u32);
                    self.code.push(0u32);
                    redirect_table.push((self.code.len(), self.code.len() - 2, (idx as isize + offset + 1) as usize, true));
                }
                IR::JumpIfFalseOffset(offset) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::JumpIfFalse as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    self.code.push(0u32);
                    self.code.push(0u32);
                    redirect_table.push((self.code.len(), self.code.len() - 2, (idx as isize + offset + 1) as usize, true));
                }
                IR::ResetStack => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::ResetStack as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::DeepCopyValue => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::DeepCopy as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::CopyValue => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::ShallowCopy as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::RefValue => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::MakeRef as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::DerefValue => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Deref as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::Assert => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Assert as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::Import => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Import as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::Alias(name) => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Alias as u8,
                            OperandFlag::Valid | OperandFlag::ArgSize64,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                    let index = self.alloc_string(name);
                    self.code.push(Opcode32::lower32(index as u64));
                    self.code.push(Opcode32::upper32(index as u64));
                }
                IR::WipeAlias => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::WipeAlias as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::In => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::BinaryIn as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::AliasOf => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::AliasOf as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::Emit => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::Emit as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::AsyncCallLambda => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::AsyncCall as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                IR::IsFinished => {
                    self.code.push(
                        Opcode32::build_opcode(
                            VMInstruction::IsFinished as u8,
                            0,
                            0,
                            0,
                        )
                        .get_opcode(),
                    );
                }
                _ => {
                    return Err(IRTranslatorError::InvalidInstruction(ir.clone()));
                }
            }
        }

        // 处理跳转指令
        // 偏移是相对于操作数而言的，而非相对于指令起始地址
        for (ip, write_ip, ir_ip, is_i64) in redirect_table {
            let calced_offset = *self.ir_to_ip.get(ir_ip).ok_or(
                IRTranslatorError::InvalidInstruction(IR::JumpOffset(ir_ip as isize)),
            )? as isize
                - ip as isize;
            if is_i64 {
                self.code[write_ip] = Opcode32::lower32(calced_offset as u64);
                self.code[write_ip + 1] = Opcode32::upper32(calced_offset as u64);
            } else {
                self.code[write_ip] = calced_offset as u32;
            }
        }

        // 填充签名跳转表
        for (name, ip) in self.ir_package.function_ips.iter() {
            self.function_ips.insert(name.clone(), self.ir_to_ip[*ip]);
        }

        Ok(())
    }

    pub fn get_result(&self) -> VMInstructionPackage {
        VMInstructionPackage::new(self.function_ips.clone(), self.code.clone(), self.string_pool.clone(), self.bytes_pool.clone(), self.debug_infos.clone(), self.ir_package.source.clone())
    }
}
