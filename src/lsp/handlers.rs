use serde_json::Value;

use super::protocol::*;
use super::server::LspServer;

/// 处理hover请求
pub fn handle_hover(server: &LspServer, params: Value) -> Result<Value, ResponseError> {
    // 简单实现 - 实际逻辑会复杂得多
    Ok(serde_json::Value::Null)
}

/// 处理跳转到定义请求
pub fn handle_definition(server: &LspServer, params: Value) -> Result<Value, ResponseError> {
    // 简单实现 - 实际逻辑会复杂得多
    Ok(serde_json::Value::Null)
}

/// 处理符号查找请求
pub fn handle_document_symbol(server: &LspServer, params: Value) -> Result<Value, ResponseError> {
    // 简单实现 - 实际逻辑会复杂得多
    Ok(serde_json::Value::Null)
}