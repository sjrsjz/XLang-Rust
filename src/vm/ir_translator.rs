use crate::vm::ir::{IR, IROperation, DebugInfo};
use crate::vm::opcode::{Opcode32, OpcodeArgument, ProcessedOpcode, Instruction32};
use crate::vm::instruction_set::VMInstruction;
use std::collections::HashMap;

pub struct IRTranslator {
    // 存储标签和对应位置的映射
    label_positions: HashMap<String, usize>,
    // 存储需要回填的跳转指令位置
    pending_jumps: Vec<(usize, String)>,
    // 生成的指令序列
    instructions: Vec<u32>,
    // 字符串常量池
    string_pool: HashMap<String, u64>,
    // 字节数组常量池
    byte_array_pool: HashMap<Vec<u8>, u64>,
    // 下一个常量的ID
    next_const_id: u64,
}

impl IRTranslator {
    pub fn new() -> Self {
        IRTranslator {
            label_positions: HashMap::new(),
            pending_jumps: Vec::new(),
            instructions: Vec::new(),
            string_pool: HashMap::new(),
            byte_array_pool: HashMap::new(),
            next_const_id: 0,
        }
    }
    
    // 获取或添加字符串到常量池
    fn get_or_add_string(&mut self, s: &str) -> u64 {
        if let Some(&id) = self.string_pool.get(s) {
            return id;
        }
        let id = self.next_const_id;
        self.next_const_id += 1;
        self.string_pool.insert(s.to_string(), id);
        id
    }
    
    // 获取或添加字节数组到常量池
    fn get_or_add_bytes(&mut self, bytes: &[u8]) -> u64 {
        let bytes_vec = bytes.to_vec();
        if let Some(&id) = self.byte_array_pool.get(&bytes_vec) {
            return id;
        }
        let id = self.next_const_id;
        self.next_const_id += 1;
        self.byte_array_pool.insert(bytes_vec, id);
        id
    }
    
    // 生成操作码
    fn emit_opcode(&mut self, instruction: VMInstruction, op1: u8, op2: u8, op3: u8) -> usize {
        let instr_byte = instruction as u8;
        let opcode = (instr_byte as u32) << 24 | (op1 as u32) << 16 | (op2 as u32) << 8 | (op3 as u32);
        let pos = self.instructions.len();
        self.instructions.push(opcode);
        pos
    }
    
    // 添加32位整数参数
    fn add_int32_arg(&mut self, val: i32) {
        self.instructions.push(val as u32);
    }
    
    // 添加64位整数参数
    fn add_int64_arg(&mut self, val: i64) {
        self.instructions.push((val & 0xFFFFFFFF) as u32); // 低32位
        self.instructions.push(((val >> 32) & 0xFFFFFFFF) as u32); // 高32位
    }
    
    // 添加32位浮点数参数
    fn add_float32_arg(&mut self, val: f32) {
        self.instructions.push(val.to_bits());
    }
    
    // 添加64位浮点数参数
    fn add_float64_arg(&mut self, val: f64) {
        let bits = val.to_bits();
        self.instructions.push((bits & 0xFFFFFFFF) as u32); // 低32位
        self.instructions.push(((bits >> 32) & 0xFFFFFFFF) as u32); // 高32位
    }
    
    // 添加字符串常量引用
    fn add_string_arg(&mut self, id: u64) {
        if id <= 0xFFFFFFFF {
            self.instructions.push(id as u32);
        } else {
            self.add_int64_arg(id as i64);
        }
    }
    
    // 翻译二元运算操作
    fn translate_binary_op(&mut self, op: &IROperation) -> VMInstruction {
        match op {
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
            _ => panic!("不支持的二元操作: {:?}", op),
        }
    }
    
    // 翻译一元运算操作
    fn translate_unary_op(&mut self, op: &IROperation) -> VMInstruction {
        match op {
            IROperation::Not => VMInstruction::UnaryNot,
            IROperation::BitwiseNot => VMInstruction::UnaryBitNot,
            _ => panic!("不支持的一元操作: {:?}", op),
        }
    }
    
    // 翻译单条IR指令
    pub fn translate_ir(&mut self, ir: &IR) {
        match ir {
            IR::LoadNull => {
                self.emit_opcode(VMInstruction::LoadNull, 0, 0, 0);
            },
            IR::LoadInt(value) => {
                if *value >= i32::MIN as i64 && *value <= i32::MAX as i64 {
                    // 32位整数
                    let pos = self.emit_opcode(VMInstruction::LoadInt32, 0b00000001, 0, 0);
                    self.add_int32_arg(*value as i32);
                } else {
                    // 64位整数
                    let pos = self.emit_opcode(VMInstruction::LoadInt64, 0b00000101, 0, 0);
                    self.add_int64_arg(*value);
                }
            },
            IR::LoadFloat(value) => {
                if *value >= f32::MIN as f64 && *value <= f32::MAX as f64 {
                    // 32位浮点数
                    let pos = self.emit_opcode(VMInstruction::LoadFloat32, 0b00001001, 0, 0);
                    self.add_float32_arg(*value as f32);
                } else {
                    // 64位浮点数
                    let pos = self.emit_opcode(VMInstruction::LoadFloat64, 0b00001101, 0, 0);
                    self.add_float64_arg(*value);
                }
            },
            IR::LoadString(value) => {
                let string_id = self.get_or_add_string(value);
                let pos = self.emit_opcode(VMInstruction::LoadString, 0b00001011, 0, 0);
                self.add_string_arg(string_id);
            },
            IR::LoadBytes(bytes) => {
                let bytes_id = self.get_or_add_bytes(bytes);
                let pos = self.emit_opcode(VMInstruction::LoadBytes, 0b00000011, 0, 0);
                self.add_string_arg(bytes_id);
            },
            IR::LoadBool(value) => {
                self.emit_opcode(VMInstruction::LoadBool, 0b00000001, 0, if *value { 1 } else { 0 });
            },
            IR::LoadLambda(signature, position) => {
                let sig_id = self.get_or_add_string(signature);
                let pos = self.emit_opcode(VMInstruction::LoadLambda, 0b00001011, 0, 0);
                self.add_string_arg(sig_id);
                self.add_int32_arg(*position as i32);
            },
            IR::ForkInstruction => {
                self.emit_opcode(VMInstruction::Fork, 0, 0, 0);
            },
            IR::BuildTuple(size) => {
                let pos = self.emit_opcode(VMInstruction::BuildTuple, 0b00000001, 0, 0);
                self.add_int32_arg(*size as i32);
            },
            IR::BuildKeyValue => {
                self.emit_opcode(VMInstruction::BuildKeyValue, 0, 0, 0);
            },
            IR::BuildNamed => {
                self.emit_opcode(VMInstruction::BuildNamed, 0, 0, 0);
            },
            IR::BuildRange => {
                self.emit_opcode(VMInstruction::BuildRange, 0, 0, 0);
            },
            IR::BindSelf => {
                self.emit_opcode(VMInstruction::BindSelf, 0, 0, 0);
            },
            IR::BinaryOp(operation) => {
                let vm_op = self.translate_binary_op(operation);
                self.emit_opcode(vm_op, 0, 0, 0);
            },
            IR::UnaryOp(operation) => {
                let vm_op = self.translate_unary_op(operation);
                self.emit_opcode(vm_op, 0, 0, 0);
            },
            IR::Let(var_name) => {
                let name_id = self.get_or_add_string(var_name);
                let pos = self.emit_opcode(VMInstruction::StoreVar, 0b00001011, 0, 0);
                self.add_string_arg(name_id);
            },
            IR::Get(var_name) => {
                let name_id = self.get_or_add_string(var_name);
                let pos = self.emit_opcode(VMInstruction::LoadVar, 0b00001011, 0, 0);
                self.add_string_arg(name_id);
            },
            IR::Set => {
                self.emit_opcode(VMInstruction::SetValue, 0, 0, 0);
            },
            IR::Wrap => {
                self.emit_opcode(VMInstruction::WrapObj, 0, 0, 0);
            },
            IR::GetAttr => {
                self.emit_opcode(VMInstruction::GetAttr, 0, 0, 0);
            },
            IR::IndexOf => {
                self.emit_opcode(VMInstruction::IndexOf, 0, 0, 0);
            },
            IR::KeyOf => {
                self.emit_opcode(VMInstruction::KeyOf, 0, 0, 0);
            },
            IR::ValueOf => {
                self.emit_opcode(VMInstruction::ValueOf, 0, 0, 0);
            },
            IR::SelfOf => {
                self.emit_opcode(VMInstruction::SelfOf, 0, 0, 0);
            },
            IR::TypeOf => {
                self.emit_opcode(VMInstruction::TypeOf, 0, 0, 0);
            },
            IR::CallLambda => {
                self.emit_opcode(VMInstruction::Call, 0, 0, 0);
            },
            IR::AsyncCallLambda => {
                self.emit_opcode(VMInstruction::AsyncCall, 0, 0, 0);
            },
            IR::Return => {
                self.emit_opcode(VMInstruction::Return, 0, 0, 0);
            },
            IR::Raise => {
                self.emit_opcode(VMInstruction::Raise, 0, 0, 0);
            },
            IR::NewFrame => {
                self.emit_opcode(VMInstruction::NewFrame, 0, 0, 0);
            },
            IR::NewBoundaryFrame(level) => {
                let pos = self.emit_opcode(VMInstruction::NewBoundaryFrame, 0b00000001, 0, 0);
                self.add_int32_arg(*level as i32);
            },
            IR::PopFrame => {
                self.emit_opcode(VMInstruction::PopFrame, 0, 0, 0);
            },
            IR::PopBoundaryFrame => {
                self.emit_opcode(VMInstruction::PopBoundaryFrame, 0, 0, 0);
            },
            IR::Pop => {
                self.emit_opcode(VMInstruction::Pop, 0, 0, 0);
            },
            IR::JumpOffset(offset) => {
                let pos = self.emit_opcode(VMInstruction::Jump, 0b00000001, 0, 0);
                self.add_int32_arg(*offset as i32);
            },
            IR::JumpIfFalseOffset(offset) => {
                let pos = self.emit_opcode(VMInstruction::JumpIfFalse, 0b00000001, 0, 0);
                self.add_int32_arg(*offset as i32);
            },
            IR::ResetStack => {
                self.emit_opcode(VMInstruction::ResetStack, 0, 0, 0);
            },
            IR::DeepCopyValue => {
                self.emit_opcode(VMInstruction::DeepCopy, 0, 0, 0);
            },
            IR::CopyValue => {
                self.emit_opcode(VMInstruction::ShallowCopy, 0, 0, 0);
            },
            IR::RefValue => {
                self.emit_opcode(VMInstruction::MakeRef, 0, 0, 0);
            },
            IR::DerefValue => {
                self.emit_opcode(VMInstruction::Deref, 0, 0, 0);
            },
            IR::Assert => {
                self.emit_opcode(VMInstruction::Assert, 0, 0, 0);
            },
            IR::Import => {
                self.emit_opcode(VMInstruction::Import, 0, 0, 0);
            },
            IR::Alias(name) => {
                let name_id = self.get_or_add_string(name);
                let pos = self.emit_opcode(VMInstruction::Alias, 0b00001011, 0, 0);
                self.add_string_arg(name_id);
            },
            IR::WipeAlias => {
                self.emit_opcode(VMInstruction::WipeAlias, 0, 0, 0);
            },
            IR::AliasOf => {
                self.emit_opcode(VMInstruction::AliasOf, 0, 0, 0);
            },
            IR::In => {
                self.emit_opcode(VMInstruction::BinaryIn, 0, 0, 0);
            },
            IR::Emit => {
                self.emit_opcode(VMInstruction::Emit, 0, 0, 0);
            },
            IR::IsFinished => {
                self.emit_opcode(VMInstruction::IsFinished, 0, 0, 0);
            },
            IR::RedirectLabel(label) => {
                // 记录标签位置
                self.label_positions.insert(label.clone(), self.instructions.len());
            },
            IR::RedirectJump(label) => {
                // 创建待回填的跳转
                let pos = self.emit_opcode(VMInstruction::Jump, 0b00000001, 0, 0);
                self.pending_jumps.push((pos + 1, label.clone())); // +1 是为了跳过指令本身，指向参数位置
                self.add_int32_arg(0); // 占位
            },
            IR::RedirectJumpIfFalse(label) => {
                // 创建待回填的条件跳转
                let pos = self.emit_opcode(VMInstruction::JumpIfFalse, 0b00000001, 0, 0);
                self.pending_jumps.push((pos + 1, label.clone()));
                self.add_int32_arg(0); // 占位
            },
            IR::RedirectNewBoundaryFrame(label) => {
                let pos = self.emit_opcode(VMInstruction::NewBoundaryFrame, 0b00000001, 0, 0);
                self.pending_jumps.push((pos + 1, label.clone()));
                self.add_int32_arg(0); // 占位
            },
            IR::DebugInfo(_) => {
                // DebugInfo在翻译过程中可以忽略或单独处理
            },
        }
    }
    
    // 翻译IR序列
    pub fn translate(&mut self, ir_list: &[IR]) -> Instruction32 {
        for ir in ir_list {
            self.translate_ir(ir);
        }
        
        // 解决所有待处理的跳转
        self.resolve_jumps();
        
        // 创建指令序列
        Instruction32::new(self.instructions.clone())
    }
    
    // 解决待回填的跳转
    fn resolve_jumps(&mut self) {
        for (jump_pos, label) in &self.pending_jumps {
            if let Some(&target_pos) = self.label_positions.get(label) {
                // 计算从跳转指令到目标位置的偏移量
                let offset = target_pos as isize - (*jump_pos as isize + 1); // +1 跳过参数本身
                self.instructions[*jump_pos] = offset as i32 as u32;
            } else {
                panic!("未定义的标签: {}", label);
            }
        }
    }
    
    // 导出常量池
    pub fn get_string_pool(&self) -> &HashMap<String, u64> {
        &self.string_pool
    }
    
    pub fn get_byte_array_pool(&self) -> &HashMap<Vec<u8>, u64> {
        &self.byte_array_pool
    }
}




#[test]
fn translate_example() {
    // 示例IR代码
    let ir_code = vec![
        IR::LoadInt(42),
        IR::Let("x".to_string()),
        IR::Get("x".to_string()),
        IR::LoadInt(100),
        IR::BinaryOp(IROperation::Add),
        IR::Return,
    ];
    
    // 创建翻译器并翻译IR
    let mut translator = IRTranslator::new();
    let instructions = translator.translate(&ir_code);
    
    // 获取常量池供VM使用
    let string_pool = translator.get_string_pool();
    let byte_array_pool = translator.get_byte_array_pool();

    // 打印翻译后的指令
    println!("翻译后的指令: {:?}", instructions);
    println!("字符串常量池: {:?}", string_pool);
    println!("字节数组常量池: {:?}", byte_array_pool);
    
    // 下一步可以将instructions和常量池传递给VM执行...
}