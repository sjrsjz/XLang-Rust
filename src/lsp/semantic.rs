use crate::parser::ast::{ASTNode, ASTNodeType};

/// 语义着色器

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenTypes {
    Null,        // Null
    String,      // String
    Boolean,     // Boolean
    Number,      // Number (Integer, Float)
    Base64,      // Base64
    Variable,    // Variable
    Let,         // x := expression
    Body,        // {...}
    Boundary,    // boundary {...}
    Assign,      // x = expression
    LambdaDef,   // tuple -> body or tuple -> dyn expression
    Expressions, // expression1; expression2; ...
    LambdaCall,  // x (tuple)
    Operation,   // x + y, x - y, x * y, x / y ...
    Tuple,       // x, y, z, ...
    AssumeTuple, // ...value
    KeyValue,    // x: y
    IndexOf,     // x[y]
    GetAttr,     // x.y
    Return,      // return expression
    Raise,       // raise expression
    If,          // if expression truecondition || if expression truecondition else falsecondition
    While,       // while expression body
    Modifier,    // modifier expression
    NamedTo,     // x => y (x is name of y)
    Break,       // break
    Continue,    // continue
    Range,       // x..y
    In,
    Yield,
    AsyncLambdaCall,
    Alias, // Type::Value
    Set,   // collection | filter
    Map,   // collection |> map
    Comment,
}

pub fn do_semantic(code: &str, ast: ASTNode) -> Result<Vec<SemanticTokenTypes>, String> {
    let mut semantic_tokens = vec![SemanticTokenTypes::Comment; code.len()];

    // 递归处理AST节点并标记语义类型
    process_node(&ast, &mut semantic_tokens, code)?;

    Ok(semantic_tokens)
}
fn process_node(
    node: &ASTNode,
    tokens: &mut Vec<SemanticTokenTypes>,
    code: &str,
) -> Result<(), String> {
    // 处理当前节点，计算它的管辖范围
    // 计算当前节点管辖的整个范围（包括子节点）
    let (range_start, range_end) = calculate_node_range(node);

    if range_start < range_end && range_start < tokens.len() && range_end <= tokens.len() {
        let token_type = match &node.node_type {
            ASTNodeType::Null => SemanticTokenTypes::Null,
            ASTNodeType::String(_) => SemanticTokenTypes::String,
            ASTNodeType::Boolean(_) => SemanticTokenTypes::Boolean,
            ASTNodeType::Number(_) => SemanticTokenTypes::Number,
            ASTNodeType::Base64(_) => SemanticTokenTypes::Base64,
            ASTNodeType::Variable(_) => SemanticTokenTypes::Variable,
            ASTNodeType::Let(_) => SemanticTokenTypes::Let,
            ASTNodeType::Body => SemanticTokenTypes::Body,
            ASTNodeType::Boundary => SemanticTokenTypes::Boundary,
            ASTNodeType::Assign => SemanticTokenTypes::Assign,
            ASTNodeType::LambdaDef(_) => SemanticTokenTypes::LambdaDef,
            ASTNodeType::Expressions => SemanticTokenTypes::Expressions,
            ASTNodeType::LambdaCall => SemanticTokenTypes::LambdaCall,
            ASTNodeType::AsyncLambdaCall => SemanticTokenTypes::AsyncLambdaCall,
            ASTNodeType::Operation(_) => SemanticTokenTypes::Operation,
            ASTNodeType::Tuple => SemanticTokenTypes::Tuple,
            ASTNodeType::AssumeTuple => SemanticTokenTypes::AssumeTuple,
            ASTNodeType::KeyValue => SemanticTokenTypes::KeyValue,
            ASTNodeType::IndexOf => SemanticTokenTypes::IndexOf,
            ASTNodeType::GetAttr => SemanticTokenTypes::GetAttr,
            ASTNodeType::Return => SemanticTokenTypes::Return,
            ASTNodeType::Raise => SemanticTokenTypes::Raise,
            ASTNodeType::If => SemanticTokenTypes::If,
            ASTNodeType::While => SemanticTokenTypes::While,
            ASTNodeType::Modifier(_) => SemanticTokenTypes::Modifier,
            ASTNodeType::NamedTo => SemanticTokenTypes::NamedTo,
            ASTNodeType::Break => SemanticTokenTypes::Break,
            ASTNodeType::Continue => SemanticTokenTypes::Continue,
            ASTNodeType::Range => SemanticTokenTypes::Range,
            ASTNodeType::In => SemanticTokenTypes::In,
            ASTNodeType::Yield => SemanticTokenTypes::Yield,
            ASTNodeType::Alias(_) => SemanticTokenTypes::Alias,
            ASTNodeType::Set => SemanticTokenTypes::Set,
            ASTNodeType::Map => SemanticTokenTypes::Map,
            _ => SemanticTokenTypes::Variable, // 默认情况
        };

        // 标记节点的token，但跳过空白字符
        for i in range_start..range_end {
            if i < tokens.len() {
                // 检查当前字符是否为空白字符
                if i < code.len() {
                    let ch = code.as_bytes()[i] as char;
                    if !ch.is_whitespace() {
                        tokens[i] = token_type;
                    }
                } else {
                    tokens[i] = token_type;
                }
            }
        }
    }

    for child in &node.children {
        process_node(child, tokens, code)?;
    }

    Ok(())
}

// 计算节点管辖的范围
fn calculate_node_range(node: &ASTNode) -> (usize, usize) {
    let mut min_pos = usize::MAX;
    let mut max_pos = 0;

    // 考虑当前节点自身的token
    if let Some(token) = &node.token {
        min_pos = min_pos.min(token.position);
        max_pos = max_pos.max(token.position + token.origin_token.len());
    }

    // 递归考虑所有子节点的范围
    for child in &node.children {
        let (child_min, child_max) = calculate_node_range(child);
        min_pos = min_pos.min(child_min);
        max_pos = max_pos.max(child_max);
    }

    // 如果没有token，返回默认范围
    if min_pos == usize::MAX {
        return (usize::MAX, 0);
    }

    (min_pos, max_pos)
}



/// 将语义标记编码为LSP协议所需的格式 (Semantic Tokens) - Corrected Version
///
/// LSP 语义标记数据格式是一个 `Vec<u32>`，其中每 5 个 u32 代表一个语义标记：
/// 1. delta_line: 相对于前一个标记的行号增量。
/// 2. delta_start_char: 相对于前一个标记起始字符的增量 (UTF-16)。新行则为绝对起始位置。
/// 3. length: 标记的长度 (UTF-16)。
/// 4. token_type: 标记类型的索引 (由 legend 决定)。
/// 5. token_modifiers_bitmask: 标记修饰符 (默认为 0)。
///
/// # Arguments
///
/// * `tokens`: 一个 `SemanticTokenTypes` 的 slice，长度等于 `text` 的字节数。
/// * `text`: 需要进行语义着色的原始文本。
///
/// # Returns
///
/// 符合 LSP 规范的 `Vec<u32>` 编码数据。
pub(super) fn encode_semantic_tokens(tokens: &[SemanticTokenTypes], text: &str) -> Vec<u32> {
    assert_eq!(
        tokens.len(),
        text.as_bytes().len(),
        "Tokens length must match text byte length"
    );

    // 将自定义语义标记类型映射到LSP语义标记类型索引
    let get_token_type_index = |token_type: &SemanticTokenTypes| -> Option<u32> {
        match token_type {
            SemanticTokenTypes::Null => Some(22),
            SemanticTokenTypes::String => Some(18),
            SemanticTokenTypes::Boolean => Some(23),
            SemanticTokenTypes::Number => Some(19),
            SemanticTokenTypes::Base64 => Some(24),
            SemanticTokenTypes::Variable => Some(8),
            SemanticTokenTypes::Let => Some(25),
            SemanticTokenTypes::Body => Some(26),
            SemanticTokenTypes::Boundary => Some(27),
            SemanticTokenTypes::Assign => Some(28),
            SemanticTokenTypes::LambdaDef => Some(29),
            SemanticTokenTypes::Expressions => Some(17), // 如果表达式没有子节点，使用默认类型
            SemanticTokenTypes::LambdaCall => Some(31),
            SemanticTokenTypes::AsyncLambdaCall => Some(32),
            SemanticTokenTypes::Operation => Some(33),
            SemanticTokenTypes::Tuple => Some(34),
            SemanticTokenTypes::AssumeTuple => Some(35),
            SemanticTokenTypes::KeyValue => Some(36),
            SemanticTokenTypes::IndexOf => Some(37),
            SemanticTokenTypes::GetAttr => Some(38),
            SemanticTokenTypes::Return => Some(39),
            SemanticTokenTypes::Raise => Some(40),
            SemanticTokenTypes::If => Some(41),
            SemanticTokenTypes::While => Some(42),
            SemanticTokenTypes::Modifier => Some(16),
            SemanticTokenTypes::NamedTo => Some(43),
            SemanticTokenTypes::Break => Some(44),
            SemanticTokenTypes::Continue => Some(45),
            SemanticTokenTypes::Range => Some(46),
            SemanticTokenTypes::In => Some(47),
            SemanticTokenTypes::Yield => Some(48),
            SemanticTokenTypes::Alias => Some(49),
            SemanticTokenTypes::Set => Some(50),
            SemanticTokenTypes::Map => Some(51),
            SemanticTokenTypes::Comment => Some(17),
        }
    };
    const MOD_NONE: u32 = 0; // 无修饰符
    const MOD_DECLARATION: u32 = 1 << 0; // 声明
    const MOD_DEFINITION: u32 = 1 << 1; // 定义
    const MOD_READONLY: u32 = 1 << 2; // 只读属性
    const MOD_STATIC: u32 = 1 << 3; // 静态成员
    const _MOD_DEPRECATED: u32 = 1 << 4; // 已弃用
    const _MOD_ABSTRACT: u32 = 1 << 5; // 抽象
    const MOD_ASYNC: u32 = 1 << 6; // 异步
    const MOD_MODIFICATION: u32 = 1 << 7; // 修改操作
    const _MOD_DOCUMENTATION: u32 = 1 << 8; // 文档
    const _MOD_DEFAULT: u32 = 1 << 9; // 默认值
    let get_token_modifiers = |token_type: &SemanticTokenTypes| -> u32 {
        match token_type {
            // 基本类型 - 无修饰符
            SemanticTokenTypes::Null
            | SemanticTokenTypes::String
            | SemanticTokenTypes::Boolean
            | SemanticTokenTypes::Number
            | SemanticTokenTypes::Base64 => MOD_NONE,

            // 对象属性相关
            SemanticTokenTypes::KeyValue => MOD_DEFINITION,
            SemanticTokenTypes::IndexOf => MOD_READONLY,
            SemanticTokenTypes::GetAttr => MOD_READONLY,

            // 变量相关
            SemanticTokenTypes::Variable => MOD_NONE,
            SemanticTokenTypes::Let => MOD_DECLARATION | MOD_DEFINITION,
            SemanticTokenTypes::Modifier => MOD_STATIC | MOD_MODIFICATION,
            SemanticTokenTypes::NamedTo => MOD_DEFINITION,

            // 函数/lambda相关
            SemanticTokenTypes::LambdaDef => MOD_DECLARATION | MOD_DEFINITION,
            SemanticTokenTypes::LambdaCall => MOD_NONE,
            SemanticTokenTypes::AsyncLambdaCall => MOD_ASYNC,

            // 控制流相关
            SemanticTokenTypes::Range => MOD_READONLY,
            SemanticTokenTypes::In => MOD_READONLY,
            SemanticTokenTypes::Yield => MOD_ASYNC,

            // 其他操作
            SemanticTokenTypes::Assign => MOD_MODIFICATION,
            SemanticTokenTypes::Operation => MOD_MODIFICATION,

            // 默认无修饰符
            _ => MOD_NONE,
        }
    };
    let mut result = Vec::new();
    let mut previous_line = 0;
    let mut previous_char_utf16 = 0;

    let mut current_line = 0;
    let mut current_char_utf16 = 0;

    // --- State for the currently accumulating token ---
    // Stores the ORIGINAL type and start position
    let mut current_original_token_type: Option<SemanticTokenTypes> = None; // Store the actual type
    let mut token_start_line = 0;
    let mut token_start_char_utf16 = 0;
    let mut current_token_len_utf16 = 0;

    for (byte_idx, ch) in text.char_indices() {
        let char_len_utf16 = ch.len_utf16() as u32;
        // Get the *original* semantic type for the start byte of this char
        let byte_semantic_type = tokens
            .get(byte_idx)
            .copied()
            .unwrap_or(SemanticTokenTypes::Null); // Default to Null if out of bounds (shouldn't happen with assert)

        // --- Check if the ORIGINAL token type changes OR if the current char is Null ---
        // A new token starts/ends if the type is different from the accumulating one,
        // or if the current type is Null (which always breaks segments).
        let type_changed = match current_original_token_type {
            Some(current_type) => current_type != byte_semantic_type,
            None => byte_semantic_type != SemanticTokenTypes::Null, // If nothing is accumulating, change only if the new type isn't Null
        };

        if type_changed {
            // --- Finalize the previous token (if one exists and is not Null) ---
            if let Some(prev_original_type) = current_original_token_type {
                // Only generate LSP token if the original type maps to an index
                if let Some(lsp_token_index) = get_token_type_index(&prev_original_type) {
                    let delta_line = token_start_line - previous_line;
                    let delta_start_char = if delta_line == 0 {
                        token_start_char_utf16 - previous_char_utf16
                    } else {
                        token_start_char_utf16
                    };
                    let token_modifiers = 0;

                    result.extend_from_slice(&[
                        delta_line,
                        delta_start_char,
                        current_token_len_utf16,
                        lsp_token_index,
                        token_modifiers,
                    ]);

                    previous_line = token_start_line;
                    previous_char_utf16 = token_start_char_utf16;
                }
            }

            // --- Start a new token (only if the current character's type is NOT Null) ---

            // Check again if this type maps to an LSP index before starting
            if get_token_type_index(&byte_semantic_type).is_some() {
                current_original_token_type = Some(byte_semantic_type);
                token_start_line = current_line;
                token_start_char_utf16 = current_char_utf16;
                current_token_len_utf16 = char_len_utf16;
            } else {
                // This type (e.g., a custom type not in legend) maps to None, treat as Null
                current_original_token_type = None;
                current_token_len_utf16 = 0;
            }
        } else {
            // --- Continue the current token (if one is active) ---
            // Type is the same as the previous character, just extend the length
            // This also handles consecutive Null characters correctly (does nothing)
            if current_original_token_type.is_some() {
                current_token_len_utf16 += char_len_utf16;
            }
        }

        // --- Advance the current position tracking ---
        if ch == '\n' {
            current_line += 1;
            current_char_utf16 = 0;
        } else {
            current_char_utf16 += char_len_utf16;
        }
    }

    // --- Finalize the very last token after the loop finishes ---
    if let Some(last_original_type) = current_original_token_type {
        if let Some(lsp_token_index) = get_token_type_index(&last_original_type) {
            let delta_line = token_start_line - previous_line;
            let delta_start_char = if delta_line == 0 {
                token_start_char_utf16 - previous_char_utf16
            } else {
                token_start_char_utf16
            };
            let token_modifiers = get_token_modifiers(&last_original_type);

            result.extend_from_slice(&[
                delta_line,
                delta_start_char,
                current_token_len_utf16,
                lsp_token_index,
                token_modifiers,
            ]);
        }
    }

    result
}