use serde::{Deserialize, Serialize};

use super::ir::DebugInfo;

/// 虚拟机指令集
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VMInstruction {
    // 栈操作
    LoadNull = 0,
    LoadInt32 = 1,
    LoadInt64 = 2,
    LoadFloat32 = 3,
    LoadFloat64 = 4,
    LoadString = 5,
    LoadBytes = 6,
    LoadBool = 7,
    LoadLambda = 8,
    Pop = 9,

    // 数据结构构建
    BuildTuple = 10,
    BuildKeyValue = 11,
    BuildNamed = 12,
    BuildRange = 13,
    BuildSet = 14,

    // 二元操作符
    BinaryAdd = 20,    // +
    BinarySub = 21,    // -
    BinaryMul = 22,    // *
    BinaryDiv = 23,    // /
    BinaryMod = 24,    // %
    BinaryPow = 25,    // **
    BinaryBitAnd = 26, // &
    BinaryBitOr = 27,  // |
    BinaryBitXor = 28, // ^
    BinaryShl = 29,    // <<
    BinaryShr = 30,    // >>
    BinaryEq = 31,     // ==
    BinaryNe = 32,     // !=
    BinaryGt = 33,     // >
    BinaryLt = 34,     // <
    BinaryGe = 35,     // >=
    BinaryLe = 36,     // <=
    BinaryIn = 37,     // in
    BinaryIs = 38,     // is

    // 一元操作
    UnaryBitNot = 40, // ~
    UnaryAbs = 41,    // abs
    UnaryNeg = 42,    // -

    // 变量与引用
    StoreVar = 50,    // 存储变量
    LoadVar = 51,     // 加载变量
    SetValue = 52,    // 设置值
    WrapObj = 53,     // 包装对象
    GetAttr = 54,     // 获取属性
    IndexOf = 55,     // 获取索引
    KeyOf = 56,       // 获取键
    ValueOf = 57,     // 获取值
    SelfOf = 58,      // 获取self
    TypeOf = 59,      // 获取类型
    DeepCopy = 60,    // 深拷贝
    ShallowCopy = 61, // 浅拷贝
    MakeRef = 62,     // 创建引用
    Deref = 63,       // 解引用
    Swap = 64,        // 交换栈两个值
    ResetIter = 65,   // 重置迭代器
    NextOrJump = 66,  // 下一个或跳转
    ForkStackObjectRef = 67, // 分叉栈对象引用
    PushValueIntoTuple = 68, // 将值推入元组
    LengthOf = 69, // 获取对象长度

    // 控制流
    Call = 70,        // 调用函数
    AsyncCall = 71,   // 异步调用
    Return = 72,      // 返回
    Raise = 73,       // 抛出异常
    Jump = 74,        // 跳转
    JumpIfFalse = 75, // 条件跳转

    // 帧操作
    NewFrame = 80,         // 新建帧
    NewBoundaryFrame = 81, // 新建边界帧
    PopFrame = 82,         // 弹出帧
    PopBoundaryFrame = 83, // 弹出边界帧
    ResetStack = 84,       // 重置栈

    // 模块操作
    Import = 90, // 导入模块

    // 特殊操作
    Fork = 100,       // 分叉指令
    BindSelf = 101,   // 绑定self
    Assert = 102,     // 断言
    Emit = 103,       // 发射事件
    IsFinished = 104, // 检查是否完成

    // 别名操作
    Alias = 110,     // 设置别名
    WipeAlias = 111, // 清除别名
    AliasOf = 112,   // 获取别名

    CaptureOf = 120, // 获取捕获的值

    // 其他
    Nop = 255, // 空操作
}

impl VMInstruction {
    /// 根据操作码获取指令
    pub fn from_opcode(opcode: u8) -> Option<Self> {
        match opcode {
            0 => Some(Self::LoadNull),
            1 => Some(Self::LoadInt32),
            2 => Some(Self::LoadInt64),
            3 => Some(Self::LoadFloat32),
            4 => Some(Self::LoadFloat64),
            5 => Some(Self::LoadString),
            6 => Some(Self::LoadBytes),
            7 => Some(Self::LoadBool),
            8 => Some(Self::LoadLambda),
            9 => Some(Self::Pop),

            10 => Some(Self::BuildTuple),
            11 => Some(Self::BuildKeyValue),
            12 => Some(Self::BuildNamed),
            13 => Some(Self::BuildRange),
            14 => Some(Self::BuildSet),

            20 => Some(Self::BinaryAdd),
            21 => Some(Self::BinarySub),
            22 => Some(Self::BinaryMul),
            23 => Some(Self::BinaryDiv),
            24 => Some(Self::BinaryMod),
            25 => Some(Self::BinaryPow),
            26 => Some(Self::BinaryBitAnd),
            27 => Some(Self::BinaryBitOr),
            28 => Some(Self::BinaryBitXor),
            29 => Some(Self::BinaryShl),
            30 => Some(Self::BinaryShr),
            31 => Some(Self::BinaryEq),
            32 => Some(Self::BinaryNe),
            33 => Some(Self::BinaryGt),
            34 => Some(Self::BinaryLt),
            35 => Some(Self::BinaryGe),
            36 => Some(Self::BinaryLe),
            37 => Some(Self::BinaryIn),
            38 => Some(Self::BinaryIs),

            40 => Some(Self::UnaryBitNot),
            41 => Some(Self::UnaryAbs),
            42 => Some(Self::UnaryNeg),

            50 => Some(Self::StoreVar),
            51 => Some(Self::LoadVar),
            52 => Some(Self::SetValue),
            53 => Some(Self::WrapObj),
            54 => Some(Self::GetAttr),
            55 => Some(Self::IndexOf),
            56 => Some(Self::KeyOf),
            57 => Some(Self::ValueOf),
            58 => Some(Self::SelfOf),
            59 => Some(Self::TypeOf),
            60 => Some(Self::DeepCopy),
            61 => Some(Self::ShallowCopy),
            62 => Some(Self::MakeRef),
            63 => Some(Self::Deref),
            64 => Some(Self::Swap),
            65 => Some(Self::ResetIter),
            66 => Some(Self::NextOrJump),
            67 => Some(Self::ForkStackObjectRef),
            68 => Some(Self::PushValueIntoTuple),
            69 => Some(Self::LengthOf),

            70 => Some(Self::Call),
            71 => Some(Self::AsyncCall),
            72 => Some(Self::Return),
            73 => Some(Self::Raise),
            74 => Some(Self::Jump),
            75 => Some(Self::JumpIfFalse),

            80 => Some(Self::NewFrame),
            81 => Some(Self::NewBoundaryFrame),
            82 => Some(Self::PopFrame),
            83 => Some(Self::PopBoundaryFrame),
            84 => Some(Self::ResetStack),

            90 => Some(Self::Import),

            100 => Some(Self::Fork),
            101 => Some(Self::BindSelf),
            102 => Some(Self::Assert),
            103 => Some(Self::Emit),
            104 => Some(Self::IsFinished),

            110 => Some(Self::Alias),
            111 => Some(Self::WipeAlias),
            112 => Some(Self::AliasOf),

            120 => Some(Self::CaptureOf),

            255 => Some(Self::Nop),

            _ => None,
        }
    }
}

use rustc_hash::FxHashMap as HashMap;
use std::fs;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMInstructionPackage {
    function_ips: HashMap<String, usize>, // 签名定位表
    code: Vec<u32>,
    string_pool: Vec<String>,
    bytes_pool: Vec<Vec<u8>>,
    debug_infos: HashMap<usize, DebugInfo>,
    source: Option<String>,
}

impl VMInstructionPackage {
    pub fn new(
        function_ips: HashMap<String, usize>,
        code: Vec<u32>,
        string_pool: Vec<String>,
        bytes_pool: Vec<Vec<u8>>,
        debug_infos: HashMap<usize, DebugInfo>,
        source: Option<String>,
    ) -> Self {
        VMInstructionPackage {
            function_ips,
            code,
            string_pool,
            bytes_pool,
            debug_infos,
            source,
        }
    }
    pub fn get_table(&self) -> &HashMap<String, usize> {
        &self.function_ips
    }
    pub fn get_code(&self) -> &Vec<u32> {
        &self.code
    }
    pub fn get_string_pool(&self) -> &Vec<String> {
        &self.string_pool
    }
    pub fn get_bytes_pool(&self) -> &Vec<Vec<u8>> {
        &self.bytes_pool
    }
    pub fn get_source(&self) -> &Option<String> {
        &self.source
    }
    pub fn get_debug_info(&self) -> &HashMap<usize, DebugInfo> {
        &self.debug_infos
    }

    pub fn write_to_file(&self, path: &str) -> Result<(), std::io::Error> {
        let serialized = bincode::serialize(self)
            .map_err(|e| std::io::Error::other(format!("Serialization error: {}", e)))?;

        fs::write(path, serialized)
    }

    pub fn read_from_file(path: &str) -> Result<Self, std::io::Error> {
        let bytes = fs::read(path)?;
        bincode::deserialize(&bytes)
            .map_err(|e| std::io::Error::other(format!("Deserialization error: {}", e)))
    }

}
