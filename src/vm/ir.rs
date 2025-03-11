#[derive(Debug)]
pub enum IROperation {
    Add,          // +
    Subtract,     // -
    Multiply,     // *
    Divide,       // /
    Modulus,      // %
    Power,        // ^
    BitwiseAnd,   // &
    BitwiseOr,    // |
    BitwiseXor,   // ^
    ShiftLeft,    // <<
    ShiftRight,   // >>
    And,          // and
    Or,           // or
    Not,          // not
    Equal,        // ==
    NotEqual,     // !=
    Greater,      // >
    Less,         // <
    GreaterEqual, // >=
    LessEqual,    // <=
}

#[derive(Debug)]
pub struct DebugInfo{
    pub code_position: usize,
}


#[derive(Debug)]
pub enum IRType{
    LoadNull, // load null to stack 
    LoadInt(i32), // load integer to stack
    LoadFloat(f64), // load float to stack
    LoadString(String), // load string to stack
    LoadBool(bool), // load bool to stack
    LoadLambda(String, usize), // signature, code position
    BuildTuple(usize), // number of elements
    BuildKeyValue, // pop key and value from stack and build key value pair
    BuildNamed, // pop key and value from stack and build named argument
    BinaryOp(IROperation), // pop two values from stack and perform binary operation
    UnaryOp(IROperation), // pop one value from stack and perform unary operation
    Let(String), // pop value from stack and store it in variable
    Get(String), // get value from context and push the reference to stack
    Set, // pop value and reference from stack and set value
    GetAttr, // pop object and attribute from stack and push the reference to attribute to stack
    IndexOf, // pop object and index from stack and push the reference to index to stack
    KeyOf, // pop object and get the key of the object
    SelfOf, // pop object and get the self of the object
    CallLambda, // pop lambda and arguments from stack and call lambda
    Return, // pop value from stack and return it
    NewFrame, // create new frame
    PopFrame, // pop frame
    JumpOffset(i32), // jump to offset
    JumpIfFalseOffset(i32), // jump to offset if false
    ResetStack, // reset stack
    CopyValue, // copy value
    RefValue, // get reference value
    DerefValue, // get dereference value
    Assert, // assert value
    DebugInfo(DebugInfo), // debug info
    Import, // import module from file
    RedirectJump(String), // redirect ir, not for vm just for ir generation
    RedirectJumpIfFalse(String), 
    RedirectLabel(String),
}