use super::protocol::*;

/// 初始化LSP服务器能力
pub fn initialize_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncOptions {
            open_close: Some(true),
            change: Some(TextDocumentSyncKind::Full),
            will_save: Some(false),
            will_save_wait_until: Some(false),
            save: Some(SaveOptions {
                include_text: Some(false),
            }),
        }),
        hover_provider: Some(false), // 暂不支持悬停
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![
                ".".to_string(),
                ":".to_string(),
                "(".to_string(),
                ",".to_string(),
            ]),
        }),
        signature_help_provider: None, // 暂不支持函数签名帮助
        definition_provider: Some(false), // 暂不支持跳转到定义
        type_definition_provider: Some(false), // 暂不支持类型定义
        implementation_provider: Some(false), // 暂不支持实现查找
        references_provider: Some(false), // 暂不支持引用查找
        document_highlight_provider: Some(false), // 暂不支持文档高亮
        document_symbol_provider: Some(false), // 暂不支持文档符号
        workspace_symbol_provider: Some(false), // 暂不支持工作区符号
        code_action_provider: Some(false), // 暂不支持代码操作
        document_formatting_provider: Some(false), // 暂不支持文档格式化
        document_range_formatting_provider: Some(false), // 暂不支持范围格式化
        rename_provider: Some(false), // 暂不支持重命名
        other: std::collections::HashMap::new(),
    }
}