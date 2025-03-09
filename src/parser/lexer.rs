
#[derive(Debug, Clone)]
pub enum TokenType {
    NUMBER,
    STRING,
    IDENTIFIER,
    SYMBOL,
    COMMENT,
    BASE64,
}
impl TokenType {
    pub fn to_string(&self) -> String {
        match self {
            TokenType::NUMBER => "NUMBER".to_string(),
            TokenType::STRING => "STRING".to_string(),
            TokenType::IDENTIFIER => "IDENTIFIER".to_string(),
            TokenType::SYMBOL => "SYMBOL".to_string(),
            TokenType::COMMENT => "COMMENT".to_string(),
            TokenType::BASE64 => "BASE64".to_string(),
        }
    }
}


impl PartialEq for TokenType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TokenType::NUMBER, TokenType::NUMBER) => true,
            (TokenType::STRING, TokenType::STRING) => true,
            (TokenType::IDENTIFIER, TokenType::IDENTIFIER) => true,
            (TokenType::SYMBOL, TokenType::SYMBOL) => true,
            (TokenType::COMMENT, TokenType::COMMENT) => true,
            (TokenType::BASE64, TokenType::BASE64) => true,
            _ => false,
        }
    }
    
}


#[derive(Debug, Clone)]
pub struct Token<'a> {
    pub token: &'a str,         // The token string
    pub token_type: TokenType, // The type of the token
    pub origin_token: String,  // The original token string
    pub position: usize,       // The position of the token in the input string
}

impl<'a> Token<'a> {
    pub fn new(
        token: String,
        token_type: TokenType,
        origin_token: String,
        position: usize,
    ) -> Token<'a> {
        let token_str = Box::leak(token.into_boxed_str());
        Token {
            token: token_str,
            token_type,
            origin_token,
            position,
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "{} <{}, Token: {}, Type: {}>",
            self.origin_token,
            self.position,
            self.token,
            self.token_type.to_string()
        )
    }
}




pub mod lexer {
    use std::cell::RefCell;

    pub fn is_operator(symbol: &str) -> bool {
        let operators = vec![
            "+", "-", "*", "/", "\\", "%", "&", "!", "^", "~", "=", "==", ">", "<", "<=", ">=",
            "!=", "?=", "|", "?", ":>", "#", "&&", ",", ".", "\n", ":", "->", "<<", ">>", "/*",
            "*/", ";", " ", ":=", "|>", "<|", "::", "=>", "++", "||", ">>", "<<", "\"\"\"", "'''",
            "(", ")", "[", "]", "{", "}",
        ];
        operators.contains(&symbol)
    }

    // Tokenize the input code
    pub fn tokenize(code: &str) -> Vec<super::Token> {
        let tokens = RefCell::new(Vec::<super::Token>::new());
        let curr_pos = RefCell::new(0usize);

        // Skip whitespace
        let skip_space = || {
            let mut curr_pos = curr_pos.borrow_mut();
            while *curr_pos < code.len() && code.chars().nth(*curr_pos).unwrap().is_whitespace() {
                *curr_pos += 1;
            }
        };

        // 测试字符串匹配的闭包
        let test_string = |test_str: &str, pos| -> bool {
            if pos + test_str.len() > code.len() {
                return false;
            }
            String::from(&code[pos..pos + test_str.len()]) == test_str
        };

        // 测试数字模式的闭包
        let test_number = |pos| -> usize {
            let number_pattern = r"^\d*\.?\d+([eE][-+]?\d+)?";
            let re = regex::Regex::new(number_pattern).unwrap();
            if let Some(matched) = re.find(&code[pos..]) {
                matched.end()
            } else {
                0
            }
        };

        // 读取数字的闭包，同时可以更新 curr_pos
        let read_number = || -> Option<(String, String)> {
            let mut pos = curr_pos.borrow_mut();
            let len = test_number(*pos);
            if len == 0 {
                return None;
            }
            let token = String::from(&code[*pos..*pos + len]);
            *pos += len;
            return Some((token.clone(), token)); // token, original token
        };

        let read_base64 = || -> Option<(String, String)> {
            let mut curr_pos = curr_pos.borrow_mut();
            let mut current_token = String::new();
            if test_string("$\"", *curr_pos) {
                *curr_pos += 2;
                while *curr_pos < code.len() {
                    if code.chars().nth(*curr_pos).unwrap() == '\\' {
                        *curr_pos += 1;
                        if *curr_pos < code.len() {
                            let escape_char = code.chars().nth(*curr_pos).unwrap();
                            match escape_char {
                                'n' => current_token.push('\n'),
                                'r' => current_token.push('\r'),
                                't' => current_token.push('\t'),
                                'b' => current_token.push('\x08'),
                                'f' => current_token.push('\x0C'),
                                'v' => current_token.push('\x0B'),
                                'a' => current_token.push('\x07'),
                                '"' | '\\' => current_token.push(escape_char),
                                'u' => {
                                    *curr_pos += 1;
                                    if *curr_pos + 4 < code.len() {
                                        let unicode_str = &code[*curr_pos..*curr_pos + 4];
                                        if let Ok(unicode_char) =
                                            u32::from_str_radix(unicode_str, 16)
                                        {
                                            current_token
                                                .push(std::char::from_u32(unicode_char).unwrap());
                                            *curr_pos += 3;
                                        }
                                    }
                                }

                                _ => {
                                    current_token.push('\\');
                                    current_token.push(escape_char);
                                }
                            }
                            *curr_pos += 1;
                        } else {
                            return None;
                        }
                    } else if code.chars().nth(*curr_pos).unwrap() == '"' {
                        *curr_pos += 1;
                        return Some((
                            current_token.clone(),
                            String::from("$\"") + &current_token + "\"",
                        ));
                    } else {
                        current_token.push(code.chars().nth(*curr_pos).unwrap());
                        *curr_pos += 1;
                    }
                }
            }
            return None;
        };

        let read_string = || -> Option<(String, String)> {
            let mut curr_pos = curr_pos.borrow_mut();
            let mut current_token = String::new();
            let mut original_token = String::new();

            // 处理 R"..." 原始字符串
            if test_string("R\"", *curr_pos) {
                *curr_pos += 2;
                let mut divider = String::new();

                // 读取分隔符
                while *curr_pos < code.len() && code.chars().nth(*curr_pos).unwrap() != '(' {
                    divider.push(code.chars().nth(*curr_pos).unwrap());
                    *curr_pos += 1;
                }

                if *curr_pos < code.len() {
                    *curr_pos += 1; // 跳过 '('
                    let end_divider = format!(")){}", divider);

                    while *curr_pos < code.len()
                        && !test_string(&(end_divider.clone() + "\""), *curr_pos)
                    {
                        current_token.push(code.chars().nth(*curr_pos).unwrap());
                        *curr_pos += 1;
                    }

                    if *curr_pos < code.len() {
                        *curr_pos += end_divider.len() + 1; // +1 for the closing quote
                        original_token =
                            format!("R\"{}({}){}", divider, current_token, end_divider);
                        return Some((current_token, original_token));
                    }
                }
                return None;
            }

            // 处理 """...""" 三引号字符串
            if test_string("\"\"\"", *curr_pos) {
                *curr_pos += 3;
                original_token.push_str("\"\"\"");

                while *curr_pos < code.len() {
                    if test_string("\"\"\"", *curr_pos) {
                        *curr_pos += 3;
                        original_token.push_str("\"\"\"");
                        return Some((current_token, original_token));
                    }

                    if code.chars().nth(*curr_pos).unwrap() == '\\' {
                        original_token.push('\\');
                        *curr_pos += 1;
                        if *curr_pos < code.len() {
                            let escape_char = code.chars().nth(*curr_pos).unwrap();
                            original_token.push(escape_char);

                            match escape_char {
                                'n' => current_token.push('\n'),
                                't' => current_token.push('\t'),
                                '"' | '\\' => current_token.push(escape_char),
                                'u' => {
                                    *curr_pos += 1;
                                    if *curr_pos + 4 <= code.len() {
                                        let hex_str = &code[*curr_pos..*curr_pos + 4];
                                        if let Ok(unicode_value) = u32::from_str_radix(hex_str, 16)
                                        {
                                            if let Some(unicode_char) =
                                                std::char::from_u32(unicode_value)
                                            {
                                                current_token.push(unicode_char);
                                                original_token.push_str(hex_str);
                                                *curr_pos += 3;
                                            } else {
                                                return None;
                                            }
                                        } else {
                                            return None;
                                        }
                                    } else {
                                        return None;
                                    }
                                }
                                _ => {
                                    current_token.push('\\');
                                    current_token.push(escape_char);
                                }
                            }
                            *curr_pos += 1;
                        } else {
                            return None;
                        }
                    } else {
                        let c = code.chars().nth(*curr_pos).unwrap();
                        current_token.push(c);
                        original_token.push(c);
                        *curr_pos += 1;
                    }
                }
                return None;
            }

            // 处理普通引号字符串
            let quote_pairs: std::collections::HashMap<char, char> =
                [('"', '"'), ('\'', '\''), ('"', '"')]
                    .iter()
                    .cloned()
                    .collect();

            if let Some(start_char) = code.chars().nth(*curr_pos) {
                if let Some(&end_char) = quote_pairs.get(&start_char) {
                    *curr_pos += 1;
                    original_token.push(start_char);

                    while *curr_pos < code.len() {
                        let current_char = code.chars().nth(*curr_pos).unwrap();

                        if current_char == '\\' {
                            original_token.push('\\');
                            *curr_pos += 1;
                            if *curr_pos < code.len() {
                                let escape_char = code.chars().nth(*curr_pos).unwrap();
                                original_token.push(escape_char);

                                match escape_char {
                                    'n' => current_token.push('\n'),
                                    't' => current_token.push('\t'),
                                    '"' | '\'' | '\\' => current_token.push(escape_char),
                                    'u' => {
                                        *curr_pos += 1;
                                        if *curr_pos + 4 <= code.len() {
                                            let hex_str = &code[*curr_pos..*curr_pos + 4];
                                            if let Ok(unicode_value) =
                                                u32::from_str_radix(hex_str, 16)
                                            {
                                                if let Some(unicode_char) =
                                                    std::char::from_u32(unicode_value)
                                                {
                                                    current_token.push(unicode_char);
                                                    original_token.push_str(hex_str);
                                                    *curr_pos += 3;
                                                } else {
                                                    return None;
                                                }
                                            } else {
                                                return None;
                                            }
                                        } else {
                                            return None;
                                        }
                                    }
                                    _ => {
                                        current_token.push('\\');
                                        current_token.push(escape_char);
                                    }
                                }
                                *curr_pos += 1;
                            } else {
                                return None;
                            }
                        } else if current_char == end_char {
                            *curr_pos += 1;
                            original_token.push(end_char);
                            return Some((current_token, original_token));
                        } else {
                            current_token.push(current_char);
                            original_token.push(current_char);
                            *curr_pos += 1;
                        }
                    }
                }
            }

            None
        };

        let read_token = || -> Option<(String, String)> {
            let mut curr_pos = curr_pos.borrow_mut();
            let mut curr_token = String::new();
            while *curr_pos < code.len() {
                if code.chars().nth(*curr_pos).unwrap().is_whitespace()
                    || vec!['\'', '"'].contains(&code.chars().nth(*curr_pos).unwrap())
                {
                    *curr_pos += 1;
                    break;
                }
                if *curr_pos < code.len() - 2 {
                    let three_chars = &code[*curr_pos..*curr_pos + 3];
                    if is_operator(three_chars) {
                        break;
                    }
                }
                if *curr_pos < code.len() - 1 {
                    let two_chars = &code[*curr_pos..*curr_pos + 2];
                    if is_operator(two_chars) {
                        break;
                    }
                }
                if is_operator(&code[*curr_pos..*curr_pos + 1]) {
                    break;
                }
                curr_token.push(code.chars().nth(*curr_pos).unwrap());
                *curr_pos += 1;
            }
            return Some((curr_token.clone(), curr_token));
        };

        let read_operator = || -> Option<(String, String)> {
            let mut curr_pos = curr_pos.borrow_mut();
            if *curr_pos < code.len() - 2 {
                let three_chars = &code[*curr_pos..*curr_pos + 3];
                if is_operator(three_chars) {
                    *curr_pos += 3;
                    return Some((three_chars.to_string(), three_chars.to_string()));
                }
            }
            if *curr_pos < code.len() - 1 {
                let two_chars = &code[*curr_pos..*curr_pos + 2];
                if is_operator(two_chars) {
                    *curr_pos += 2;
                    return Some((two_chars.to_string(), two_chars.to_string()));
                }
            }
            if is_operator(&code[*curr_pos..*curr_pos + 1]) {
                *curr_pos += 1;
                return Some((
                    code[*curr_pos - 1..*curr_pos].to_string(),
                    code[*curr_pos - 1..*curr_pos].to_string(),
                ));
            }
            return None;
        };

        let read_comment = || -> Option<(String, String)> {
            let mut curr_pos = curr_pos.borrow_mut();
            if test_string("//", *curr_pos) {
                *curr_pos += 2;
                let mut current_token = String::new();
                while *curr_pos < code.len()
                    && !vec!['\n', '\r'].contains(&code.chars().nth(*curr_pos).unwrap())
                {
                    current_token.push(code.chars().nth(*curr_pos).unwrap());
                    *curr_pos += 1;
                }
                return Some((current_token.clone(), format!("//{}", current_token)));
            }
            if test_string("/*", *curr_pos) {
                *curr_pos += 2;
                let mut current_token = String::new();
                while *curr_pos < code.len() && !test_string("*/", *curr_pos) {
                    current_token.push(code.chars().nth(*curr_pos).unwrap());
                    *curr_pos += 1;
                }
                if *curr_pos < code.len() {
                    *curr_pos += 2;
                    return Some((current_token.clone(), format!("/*{}*/", current_token)));
                }
            }
            return None;
        };

        loop {
            skip_space();
            let curr_pos = *curr_pos.borrow_mut();
            if curr_pos >= code.len() {
                break;
            }

            if let Some((token, origin_token)) = read_number() {
                let mut tokens = tokens.borrow_mut();
                tokens.push(super::Token::new(
                    token,
                    super::TokenType::NUMBER,
                    origin_token,
                    curr_pos,
                ));
                continue;
            }

            if let Some((token, origin_token)) = read_base64() {
                let mut tokens = tokens.borrow_mut();
                tokens.push(super::Token::new(
                    token,
                    super::TokenType::BASE64,
                    origin_token,
                    curr_pos,
                ));
                continue;
            }

            if let Some((token, origin_token)) = read_string() {
                let mut tokens = tokens.borrow_mut();
                tokens.push(super::Token::new(
                    token,
                    super::TokenType::STRING,
                    origin_token,
                    curr_pos,
                ));
                continue;
            }

            if let Some((token, origin_token)) = read_comment() {
                let mut tokens = tokens.borrow_mut();
                tokens.push(super::Token::new(
                    token,
                    super::TokenType::COMMENT,
                    origin_token,
                    curr_pos,
                ));
                continue;
            }

            if let Some((token, origin_token)) = read_operator() {
                let mut tokens = tokens.borrow_mut();
                tokens.push(super::Token::new(
                    token,
                    super::TokenType::SYMBOL,
                    origin_token,
                    curr_pos,
                ));
                continue;
            }

            if let Some((token, origin_token)) = read_token() {
                let mut tokens = tokens.borrow_mut();
                tokens.push(super::Token::new(
                    token.clone(),
                    super::TokenType::IDENTIFIER,
                    origin_token.clone(),
                    curr_pos,
                ));
                continue;
            }
        }

        return tokens.into_inner();
    }


    // Reject comments from the token list
    pub fn reject_comment(tokens: Vec<super::Token>) -> Vec<super::Token> {
        let mut result = Vec::new();
        for token in tokens {
            if token.token_type != super::TokenType::COMMENT {
                result.push(token);
            }
        }
        return result;
    }


}
