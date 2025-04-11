use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};

use log::{debug, error, info, warn};
use serde_json::Value;

use crate::lsp::semantic::encode_semantic_tokens;
use crate::parser::lexer;

use super::capabilities::initialize_capabilities;
use super::document::TextDocument;
use super::protocol::*;
use super::semantic::SemanticTokenTypes;

/// LSP 服务器状态
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ServerState {
    Uninitialized,
    Initializing,
    Initialized,
    ShutDown,
}

/// LSP 服务器数据结构
pub struct LspServer {
    /// 服务器当前状态
    state: ServerState,
    /// 打开的文档映射
    documents: HashMap<String, TextDocument>,
    /// 客户端能力
    client_capabilities: Option<ClientCapabilities>,
    /// 工作空间根目录
    root_uri: Option<String>,
}

impl LspServer {
    /// 创建新的LSP服务器实例
    pub fn new() -> Self {
        Self {
            state: ServerState::Uninitialized,
            documents: HashMap::new(),
            client_capabilities: None,
            root_uri: None,
        }
    }

    /// 处理初始化请求
    pub fn initialize(
        &mut self,
        params: InitializeParams,
    ) -> Result<InitializeResult, ResponseError> {
        if self.state == ServerState::Initialized {
            return Err(ResponseError {
                code: error_codes::INVALID_REQUEST,
                message: "Server is already initialized".to_string(),
                data: None,
            });
        }

        self.state = ServerState::Initializing;
        self.client_capabilities = params.capabilities;
        self.root_uri = params.root_uri;

        Ok(InitializeResult {
            capabilities: initialize_capabilities(),
            server_info: Some(ServerInfo {
                name: "XLang Language Server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    /// 处理初始化完成通知
    pub fn initialized(&mut self) {
        self.state = ServerState::Initialized;
    }

    /// 处理关闭请求
    pub fn shutdown(&mut self) -> Result<Value, ResponseError> {
        self.state = ServerState::ShutDown;
        Ok(Value::Null)
    }

    /// 处理退出通知
    pub fn exit(&mut self) -> bool {
        self.state == ServerState::ShutDown
    }

    /// 处理文档打开通知
    pub fn did_open(&mut self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let document = TextDocument::new(
            params.text_document.uri,
            params.text_document.language_id,
            params.text_document.version,
            params.text_document.text,
        );
        self.documents.insert(uri.clone(), document);

        // 触发文档验证
        self.validate_document(&uri);
        info!("Document opened: {}", uri);
    }

    /// 处理文档变更通知
    pub fn did_change(&mut self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        info!("文档变更: {} (版本 {})", uri, params.text_document.version);

        // 获取或创建文档
        let document = if let Some(doc) = self.documents.get_mut(&uri) {
            info!("更新现有文档");
            doc
        } else {
            info!("文档不存在，创建新文档: {}", uri);
            // 确定初始内容
            let initial_content = if !params.content_changes.is_empty() {
                match &params.content_changes[0] {
                    TextDocumentContentChangeEvent::Full { text } => text.clone(),
                    TextDocumentContentChangeEvent::Incremental { .. } => String::new(),
                }
            } else {
                String::new()
            };

            let doc = TextDocument::new(
                uri.clone(),
                "xlang".to_string(), // 假设语言为 xlang
                params.text_document.version,
                initial_content,
            );

            self.documents.insert(uri.clone(), doc);
            self.documents.get_mut(&uri).unwrap()
        };

        // 应用所有变更
        for (i, change) in params.content_changes.iter().enumerate() {
            match change {
                TextDocumentContentChangeEvent::Full { text } => {
                    info!("应用全量变更 #{}: 内容长度 {} 字节", i, text.len());
                    document.update_content(text.clone());
                }
                TextDocumentContentChangeEvent::Incremental { range, text } => {
                    info!(
                        "应用增量变更 #{}: 范围 [{},{}]-[{},{}], 文本长度 {} 字节",
                        i,
                        range.start.line,
                        range.start.character,
                        range.end.line,
                        range.end.character,
                        text.len()
                    );
                    document.apply_incremental_change(range.clone(), text.clone());
                }
            }
        }

        // 更新文档版本
        let old_version = document.version;
        document.version = params.text_document.version;
        info!("文档版本从 {} 更新到 {}", old_version, document.version);

        // 打印当前文档内容 (调试用)
        debug!(
            "更新后的文档内容 (前100字符): {}",
            &document.content.chars().take(100).collect::<String>()
        );
    }
    /// 处理文档关闭通知
    pub fn did_close(&mut self, params: DidCloseTextDocumentParams) {
        info!("Document closed: {}", params.text_document.uri);
        let uri = params.text_document.uri.clone();
        if self.documents.remove(&uri).is_none() {
            warn!("Tried to close non-existent document: {}", uri);
        }
    }

    // 验证文档并发送诊断消息
    fn validate_document(
        &self,
        uri: &str,
    ) -> Option<(Vec<Diagnostic>, Option<Vec<SemanticTokenTypes>>)> {
        info!("验证文档: {} 开始", uri);
        if let Some(document) = self.documents.get(uri) {
            info!(
                "找到文档，版本 {}, 内容长度 {} 字节",
                document.version,
                document.content.len()
            );

            // 生成诊断信息和语义着色
            match std::panic::catch_unwind(|| super::diagnostics::validate_document(document)) {
                Ok((diagnostics, semantic_tokens)) => {
                    info!("诊断完成: 生成了 {} 个诊断信息", diagnostics.len());
                    if let Some(tokens) = &semantic_tokens {
                        info!("语义着色: 生成了 {} 个标记", tokens.len());
                    } else {
                        info!("语义着色: 未能生成标记");
                    }
                    Some((diagnostics, semantic_tokens))
                }
                Err(e) => {
                    error!("诊断过程中发生崩溃: {:?}", e);
                    Some((vec![], None)) // 返回空诊断列表和无语义标记
                }
            }
        } else {
            warn!("找不到要验证的文档: {}", uri);
            None
        }
    }
    /// 处理自动完成请求
    pub fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<CompletionResponse, ResponseError> {
        let uri = params.text_document.uri.clone();

        if let Some(document) = self.documents.get(&uri) {
            // 从文档内容和位置计算补全项
            let items = self.calculate_completions(document, params.position);

            Ok(CompletionResponse::List(items))
        } else {
            Err(ResponseError {
                code: error_codes::INVALID_PARAMS,
                message: format!("Document not found: {}", uri),
                data: None,
            })
        }
    }

    /// 计算给定位置的补全项
    fn calculate_completions(
        &self,
        document: &TextDocument,
        _position: Position,
    ) -> Vec<CompletionItem> {
        // 基于XLang语法，提供关键字、运算符等的完成项
        let mut items = Vec::new();

        // 添加XLang关键字
        let keywords = vec![
            "if", "else", "while", "return", "break", "continue", "and", "or", "not", "null",
            "true", "false", "in", "async", "await", "yield",
        ];

        for keyword in keywords {
            items.push(CompletionItem {
                label: keyword.to_string(),
                kind: Some(CompletionItemKind::Keyword),
                detail: Some("XLang keyword".to_string()),
                documentation: None,
                insert_text: Some(keyword.to_string()),
                other: HashMap::new(),
            });
        }

        // 添加XLang内置函数
        let functions = vec![
            ("print", "Print a value to the console"),
            ("input", "Read input from the console"),
            ("len", "Get the length of a collection"),
        ];

        for (func, desc) in functions {
            items.push(CompletionItem {
                label: func.to_string(),
                kind: Some(CompletionItemKind::Function),
                detail: Some("Built-in function".to_string()),
                documentation: Some(desc.to_string()),
                insert_text: Some(format!("{}(", func)),
                other: HashMap::new(),
            });
        }

        // 尝试从当前文档中提取变量名
        if let Some(extracted_vars) = self.extract_variables_from_document(document) {
            for var in extracted_vars {
                items.push(CompletionItem {
                    label: var.clone(),
                    kind: Some(CompletionItemKind::Variable),
                    detail: Some("Local variable".to_string()),
                    documentation: None,
                    insert_text: Some(var),
                    other: HashMap::new(),
                });
            }
        }

        items
    }

    /// 从文档中提取变量名
    fn extract_variables_from_document(&self, document: &TextDocument) -> Option<Vec<String>> {
        let tokens = lexer::lexer::tokenize(&document.content);
        let tokens = lexer::lexer::reject_comment(&tokens);
        let mut variables = Vec::new();

        // 简单的变量提取: 查找 := 定义的变量
        for i in 0..tokens.len().saturating_sub(1) {
            if tokens[i].token_type == lexer::TokenType::IDENTIFIER
                && i + 1 < tokens.len()
                && tokens[i + 1].token_type == lexer::TokenType::SYMBOL
                && tokens[i + 1].token == ":="
            {
                variables.push(tokens[i].token.to_string());
            }
        }

        Some(variables)
    }
}

/// 运行LSP服务器
pub fn run_lsp_server<R: BufRead, W: Write>(
    server: Arc<Mutex<LspServer>>,
    mut reader: R,
    mut writer: W,
) -> Result<(), String> {
    loop {
        // 读取消息头
        let mut header = String::new();
        let mut content_length = 0;

        loop {
            header.clear();
            if reader.read_line(&mut header).map_err(|e| e.to_string())? == 0 {
                debug!("Connection closed by client");
                return Ok(()); // 连接已关闭
            }

            header = header.trim().to_string();
            if header.is_empty() {
                break; // 头部结束
            }

            // 解析Content-Length头
            if header.to_lowercase().starts_with("content-length: ") {
                content_length = header[16..]
                    .parse::<usize>()
                    .map_err(|e| format!("Invalid Content-Length: {}", e))?;
            }
        }

        if content_length == 0 {
            return Err("Missing Content-Length header".to_string());
        }

        // 读取消息体
        let mut buffer = vec![0; content_length];
        reader
            .read_exact(&mut buffer)
            .map_err(|e| format!("Failed to read message: {}", e))?;

        let message =
            String::from_utf8(buffer).map_err(|e| format!("Invalid UTF-8 in message: {}", e))?;

        // 解析消息类型并处理
        if message.contains("\"id\":") {
            // 请求
            match serde_json::from_str::<RequestMessage>(&message) {
                Ok(request) => {
                    let response = handle_request(server.clone(), request.clone());
                    send_message(&mut writer, &response)?;

                    // 如果是退出请求并且服务器状态是关闭，则退出循环
                    if request.method == "exit" {
                        let server = server.lock().unwrap();
                        if server.state == ServerState::ShutDown {
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse request: {}", e);
                    let error_response = ResponseMessage {
                        jsonrpc: "2.0".to_string(),
                        id: RequestId::Number(-1),
                        result: None,
                        error: Some(ResponseError {
                            code: error_codes::PARSE_ERROR,
                            message: format!("Failed to parse request: {}", e),
                            data: None,
                        }),
                    };
                    send_message(&mut writer, &error_response)?;
                }
            }
        } else {
            // 通知
            match serde_json::from_str::<NotificationMessage>(&message) {
                Ok(notification) => {
                    handle_notification(server.clone(), notification, &mut writer)?;
                }
                Err(e) => {
                    error!("Failed to parse notification: {}", e);
                }
            }
        }
    }
}

/// 处理LSP请求
fn handle_request(server: Arc<Mutex<LspServer>>, request: RequestMessage) -> ResponseMessage {
    let mut response = ResponseMessage {
        jsonrpc: "2.0".to_string(),
        id: request.id.clone(),
        result: None,
        error: None,
    };

    match request.method.as_str() {
        "initialize" => match serde_json::from_value::<InitializeParams>(request.params) {
            Ok(params) => {
                let mut server = server.lock().unwrap();
                match server.initialize(params) {
                    Ok(result) => {
                        response.result = Some(serde_json::to_value(result).unwrap());
                    }
                    Err(err) => {
                        response.error = Some(err);
                    }
                }
            }
            Err(e) => {
                response.error = Some(ResponseError {
                    code: error_codes::INVALID_PARAMS,
                    message: format!("Invalid initialize params: {}", e),
                    data: None,
                });
            }
        },
        "shutdown" => {
            let mut server = server.lock().unwrap();
            match server.shutdown() {
                Ok(result) => {
                    response.result = Some(result);
                }
                Err(err) => {
                    response.error = Some(err);
                }
            }
        }
        "textDocument/completion" => {
            match serde_json::from_value::<CompletionParams>(request.params) {
                Ok(params) => {
                    let server = server.lock().unwrap();
                    match server.completion(params) {
                        Ok(result) => {
                            response.result = Some(serde_json::to_value(result).unwrap());
                        }
                        Err(err) => {
                            response.error = Some(err);
                        }
                    }
                }
                Err(e) => {
                    response.error = Some(ResponseError {
                        code: error_codes::INVALID_PARAMS,
                        message: format!("Invalid completion params: {}", e),
                        data: None,
                    });
                }
            }
        }
        "textDocument/semanticTokens/full" => {
            match serde_json::from_value::<TextDocumentIdentifier>(
                request
                    .params
                    .get("textDocument")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            ) {
                Ok(text_doc) => {
                    let uri = text_doc.uri.clone();
                    let server = server.lock().unwrap();

                    if let Some(document) = server.documents.get(&uri) {
                        // 生成语义标记
                        if let Some((_, semantic_tokens)) = server.validate_document(&uri) {
                            if let Some(tokens) = semantic_tokens {
                                // 编码标记
                                let encoded_tokens =
                                    encode_semantic_tokens(&tokens, &document.content);

                                // 创建结果
                                let result = serde_json::json!({
                                    "data": encoded_tokens
                                });

                                response.result = Some(result);
                            } else {
                                // 如果没有标记，返回空数组
                                response.result = Some(serde_json::json!({
                                    "data": []
                                }));
                            }
                        } else {
                            // 如果没有验证结果，返回空数组
                            response.result = Some(serde_json::json!({
                                "data": []
                            }));
                        }
                    } else {
                        response.error = Some(ResponseError {
                            code: error_codes::INVALID_PARAMS,
                            message: format!("Document not found: {}", uri),
                            data: None,
                        });
                    }
                }
                Err(e) => {
                    response.error = Some(ResponseError {
                        code: error_codes::INVALID_PARAMS,
                        message: format!("Invalid semantic tokens params: {}", e),
                        data: None,
                    });
                }
            }
        }
        _ => {
            response.error = Some(ResponseError {
                code: error_codes::METHOD_NOT_FOUND,
                message: format!("Method not found: {}", request.method),
                data: None,
            });
        }
    }

    response
}

/// 处理LSP通知
fn handle_notification<W: Write>(
    server: Arc<Mutex<LspServer>>,
    notification: NotificationMessage,
    writer: &mut W,
) -> Result<(), String> {
    debug!("Notification: {}", notification.method);

    match notification.method.as_str() {
        "initialized" => {
            let mut server = server.lock().unwrap();
            server.initialized();
        }
        "exit" => {
            let mut server = server.lock().unwrap();
            if server.exit() {
                return Ok(());
            }
        }
        "textDocument/didOpen" => {
            match serde_json::from_value::<DidOpenTextDocumentParams>(notification.params) {
                Ok(params) => {
                    let uri = params.text_document.uri.clone();
                    let mut server_locked = server.lock().unwrap();
                    server_locked.did_open(params);

                    // 发送诊断通知和语义着色信息
                    if let Some((diagnostics, semantic_tokens)) =
                        server_locked.validate_document(&uri)
                    {
                        // 获取文档内容
                        let content = if let Some(doc) = server_locked.documents.get(&uri) {
                            doc.content.clone()
                        } else {
                            String::new()
                        };

                        drop(server_locked); // 释放锁

                        // 发送诊断信息
                        send_diagnostics(writer, &uri, diagnostics)?;

                        // 如果有语义着色信息，则发送语义着色通知
                        if let Some(tokens) = semantic_tokens {
                            // 编码标记并发送
                            let encoded_tokens = encode_semantic_tokens(&tokens, &content);
                            send_semantic_tokens_encoded(writer, &uri, encoded_tokens)?;
                        }
                    }
                }
                Err(e) => {
                    error!("Invalid didOpen params: {}", e);
                }
            }
        }
        "textDocument/didChange" => {
            match serde_json::from_value::<DidChangeTextDocumentParams>(notification.params) {
                Ok(params) => {
                    let uri = params.text_document.uri.clone();
                    let mut server = server.lock().unwrap();
                    server.did_change(params);

                    // 发送诊断通知和语义着色信息
                    if let Some((diagnostics, _)) = server.validate_document(&uri) {
                        // 获取文档内容

                        drop(server); // 释放锁

                        // 发送诊断信息
                        send_diagnostics(writer, &uri, diagnostics)?;

                    }
                }
                Err(e) => {
                    error!("Invalid didChange params: {}", e);
                }
            }
        }
        "textDocument/didClose" => {
            match serde_json::from_value::<DidCloseTextDocumentParams>(notification.params) {
                Ok(params) => {
                    let mut server = server.lock().unwrap();
                    server.did_close(params);
                }
                Err(e) => {
                    error!("Invalid didClose params: {}", e);
                }
            }
        }
        // 添加对 setTrace 通知的处理
        "$/setTrace" => {
            // 仅记录日志，不需要其他操作
            debug!("Trace level set to: {}", notification.params);
        }
        // 添加对 cancelRequest 通知的处理
        "$/cancelRequest" => {
            debug!("Request cancellation received: {}", notification.params);
            // 未实现请求取消逻辑，因为当前实现是同步的
        }
        // 添加对 progress 通知的处理
        "$/progress" => {
            debug!("Progress notification: {}", notification.params);
        }
        // 添加对 logTrace 通知的处理
        "$/logTrace" => {
            debug!("Log trace notification: {}", notification.params);
        }
        // 添加对 telemetry/event 通知的处理
        "telemetry/event" => {
            debug!("Telemetry event: {}", notification.params);
        }
        // 添加对 workspace/didChangeConfiguration 通知的处理
        "workspace/didChangeConfiguration" => {
            debug!("Configuration changed: {}", notification.params);
        }
        // 添加对 workspace/didChangeWatchedFiles 通知的处理
        "workspace/didChangeWatchedFiles" => {
            debug!("Watched files changed: {}", notification.params);
        }
        _ => {
            // 改为 info 级别，减少错误日志
            info!("Unhandled notification: {}", notification.method);
        }
    }

    Ok(())
}

/// 发送诊断通知
fn send_diagnostics<W: Write>(
    writer: &mut W,
    uri: &str,
    diagnostics: Vec<Diagnostic>,
) -> Result<(), String> {
    let params = PublishDiagnosticsParams {
        uri: uri.to_string(),
        diagnostics,
    };

    let notification = NotificationMessage {
        jsonrpc: "2.0".to_string(),
        method: "textDocument/publishDiagnostics".to_string(),
        params: serde_json::to_value(params).unwrap(),
    };

    send_message(writer, &notification)
}

/// 发送已编码的语义着色通知
fn send_semantic_tokens_encoded<W: Write>(
    writer: &mut W,
    uri: &str,
    encoded_tokens: Vec<u32>,
) -> Result<(), String> {
    let params = serde_json::json!({
        "textDocument": {
            "uri": uri
        },
        "tokens": encoded_tokens
    });

    let notification = NotificationMessage {
        jsonrpc: "2.0".to_string(),
        method: "textDocument/semanticTokens/full".to_string(),
        params,
    };
    send_message(writer, &notification)
}

/// 发送LSP消息
fn send_message<T: serde::Serialize, W: Write>(writer: &mut W, message: &T) -> Result<(), String> {
    let json = serde_json::to_string(message).map_err(|e| e.to_string())?;
    let content = json.as_bytes();

    debug!("Sending: {}", json);

    write!(writer, "Content-Length: {}\r\n\r\n", content.len())
        .map_err(|e| format!("Failed to write header: {}", e))?;

    writer
        .write_all(content)
        .map_err(|e| format!("Failed to write content: {}", e))?;

    writer
        .flush()
        .map_err(|e| format!("Failed to flush: {}", e))?;

    Ok(())
}
