#[derive(Debug)]
pub struct Opcode32 {
    opcode: u32,
}

pub enum OperandFlag {
    Valid = 0b00000001,
    UseConstPool = 0b00000010,
    ArgSize64 = 0b000000100,
    ConstType = 0b00001000, // string/byte array or int/float, determine by UseConstPool
}

pub struct DecodedOpcode {
    instruction: u8,
    operand1: u8, // const pool idx/constant (1bit)
    // argsize (1bit, 32/64)
    // valid (1bit)
    operand2: u8,
    operand3: u8,
}

#[derive(Debug, Clone)]
pub enum OpcodeArgument {
    None,
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(u64),
    ByteArray(u64),
}
#[derive(Debug, Clone)]
pub struct ProcessedOpcode {
    instruction: u8,
    operand1: OpcodeArgument,
    operand2: OpcodeArgument,
    operand3: OpcodeArgument,
}

#[derive(Debug)]
pub struct Instruction32 {
    bytes: Vec<u32>, // 4 bytes
    pointer: usize,
}

impl Instruction32 {
    pub fn new(bytes: Vec<u32>) -> Self {
        Instruction32 { bytes, pointer: 0 }
    }

    pub fn get_next(&mut self) -> Option<u32> {
        if self.pointer < self.bytes.len() {
            let byte = self.bytes[self.pointer];
            self.pointer += 1;
            Some(byte)
        } else {
            None
        }
    }

    pub fn get_processed_opcode(&mut self) -> Option<ProcessedOpcode> {
        let opcode = self.get_next()?;
        let decoded_opcode = self.decode_opcode(opcode);
        let operand1_arg = self.get_operand_arg(decoded_opcode.operand1)?;
        let operand2_arg = self.get_operand_arg(decoded_opcode.operand2)?;
        let operand3_arg = self.get_operand_arg(decoded_opcode.operand3)?;

        // 构建操作数
        let operand1 = self.build_operand_argument(decoded_opcode.operand1, operand1_arg);
        let operand2 = self.build_operand_argument(decoded_opcode.operand2, operand2_arg);
        let operand3 = self.build_operand_argument(decoded_opcode.operand3, operand3_arg);

        Some(ProcessedOpcode {
            instruction: decoded_opcode.instruction,
            operand1,
            operand2,
            operand3,
        })
    }

    // 根据操作数标志和原始值构建具体的操作数类型
    fn build_operand_argument(&self, operand_flags: u8, arg_value: u64) -> OpcodeArgument {
        // 如果操作数无效，返回None
        if (operand_flags & OperandFlag::Valid as u8) == 0 {
            return OpcodeArgument::None;
        }

        // 处理常量池引用
        if (operand_flags & OperandFlag::UseConstPool as u8) != 0 {
            // 根据常量类型决定返回字符串引用或字节数组引用
            return if (operand_flags & OperandFlag::ConstType as u8) != 0 {
                OpcodeArgument::String(arg_value)
            } else {
                OpcodeArgument::ByteArray(arg_value)
            };
        }

        // 处理直接值
        if (operand_flags & OperandFlag::ConstType as u8) != 0 {
            // 浮点数值
            return if (operand_flags & OperandFlag::ArgSize64 as u8) != 0 {
                // 64位浮点数
                OpcodeArgument::Float64(f64::from_bits(arg_value))
            } else {
                // 32位浮点数
                OpcodeArgument::Float32(f32::from_bits(arg_value as u32))
            };
        }

        // 整数值
        if (operand_flags & OperandFlag::ArgSize64 as u8) != 0 {
            // 64位整数
            OpcodeArgument::Int64(arg_value as i64)
        } else {
            // 32位整数
            OpcodeArgument::Int32(arg_value as i32)
        }
    }
    fn decode_opcode(&self, opcode: u32) -> DecodedOpcode {
        let instruction = (opcode >> 24) as u8;
        let operand1 = ((opcode >> 16) & 0xFF) as u8;
        let operand2 = ((opcode >> 8) & 0xFF) as u8;
        let operand3 = (opcode & 0xFF) as u8;

        DecodedOpcode {
            instruction,
            operand1,
            operand2,
            operand3,
        }
    }
    fn get_operand_arg(&mut self, operand: u8) -> Option<u64> {
        if (operand & OperandFlag::Valid as u8) == 0 {
            return Some(0);
        }
        if (operand & OperandFlag::ArgSize64 as u8) != 0 {
            // 64 bit
            let arg_l = self.get_next()?;
            let arg_h = self.get_next()?;
            Some(((arg_h as u64) << 32) | (arg_l as u64))
        } else {
            // 32 bit
            let arg = self.get_next()?;
            Some(arg as u64)
        }
    }
}
