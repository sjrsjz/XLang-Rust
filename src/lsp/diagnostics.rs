use log::debug;
use log::info;

use super::document::TextDocument;
use super::protocol::*;
use crate::parser::ast::ast_token_stream;
use crate::parser::ast::build_ast;
use crate::parser::lexer::lexer;

/// 验证文档并生成诊断信息
pub fn validate_document(document: &TextDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    info!("Lexing document: {}", document.uri);    
    // 进行词法分析
    let tokens = lexer::tokenize(&document.content);
    let filtered_tokens = lexer::reject_comment(tokens);
    // 尝试解析AST
    let gathered = ast_token_stream::from_stream(&filtered_tokens);
    match build_ast(gathered) {
        Ok(_) => {
            // 解析成功，没有错误
            info!("Document parsed successfully: {}", document.uri);
        },
        Err(parse_error) => {
            // 解析失败，创建诊断信息
            let error_message = parse_error.format(&filtered_tokens, document.content.clone());
            
            // 尝试定位错误位置
            // 简单实现 - 实际情况应该从parse_error中获取更准确的位置
            if let Some(token_with_error) = filtered_tokens.first() {
                let position = token_with_error.position;
                let line_col = get_line_col(&document.content, position);
                
                let range = Range {
                    start: Position {
                        line: line_col.0 as u32,
                        character: line_col.1 as u32,
                    },
                    end: Position {
                        line: line_col.0 as u32,
                        character: (line_col.1 + token_with_error.token.len()) as u32,
                    },
                };
                
                diagnostics.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Error),
                    code: None,
                    source: Some("xlang-lsp".to_string()),
                    message: error_message,
                    related_information: None,
                });
            }
            info!("Document parsing failed: {}", document.uri);
        }
    }
    
    diagnostics
}

/// 获取字节位置对应的行列号
fn get_line_col(text: &str, byte_pos: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    let mut current_pos = 0;
    
    for c in text.chars() {
        if current_pos >= byte_pos {
            break;
        }
        
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        
        current_pos += c.len_utf8();
    }
    
    (line, col)
}