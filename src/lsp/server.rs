use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};

use log::{debug, error, info, warn};
use serde_json::Value;

use crate::parser::lexer;

use super::capabilities::initialize_capabilities;
use super::diagnostics::validate_document;
use super::document::TextDocument;
use super::protocol::*;

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
                code: ErrorCodes::INVALID_REQUEST,
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
    }

    /// 处理文档变更通知
    pub fn did_change(&mut self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        if let Some(document) = self.documents.get_mut(&uri) {
            for change in params.content_changes {
                match change {
                    TextDocumentContentChangeEvent::Full { text } => {
                        document.update_content(text);
                    }
                    TextDocumentContentChangeEvent::Incremental { range, text } => {
                        document.apply_incremental_change(range, text);
                    }
                }
            }
            document.version = params.text_document.version;

            // 触发文档验证
            self.validate_document(&uri);
        } else {
            warn!("Received change for unopened document: {}", uri);
        }
    }

    /// 处理文档关闭通知
    pub fn did_close(&mut self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if self.documents.remove(&uri).is_none() {
            warn!("Tried to close non-existent document: {}", uri);
        }
    }

    /// 验证文档并发送诊断消息
    fn validate_document(&self, uri: &str) -> Option<Vec<Diagnostic>> {
        if let Some(document) = self.documents.get(uri) {
            let diagnostics = validate_document(document);
            Some(diagnostics)
        } else {
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
                code: ErrorCodes::INVALID_PARAMS,
                message: format!("Document not found: {}", uri),
                data: None,
            })
        }
    }

    /// 计算给定位置的补全项
    fn calculate_completions(
        &self,
        document: &TextDocument,
        position: Position,
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
        let tokens = lexer::lexer::reject_comment(lexer::lexer::tokenize(&document.content));
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

        debug!("Received: {}", message);

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
                            code: ErrorCodes::PARSE_ERROR,
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
    debug!("Request: {} (id: {:?})", request.method, request.id);

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
                    code: ErrorCodes::INVALID_PARAMS,
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
                        code: ErrorCodes::INVALID_PARAMS,
                        message: format!("Invalid completion params: {}", e),
                        data: None,
                    });
                }
            }
        }
        _ => {
            response.error = Some(ResponseError {
                code: ErrorCodes::METHOD_NOT_FOUND,
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

                    // 发送诊断通知 - 使用传入参数中的 URI
                    if let Some(diagnostics) = server_locked.validate_document(&uri) {
                        drop(server_locked); // 释放锁
                        send_diagnostics(writer, &uri, diagnostics)?;
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
                    let mut server_locked = server.lock().unwrap();
                    server_locked.did_change(params);

                    // 发送诊断通知
                    if let Some(diagnostics) = server_locked.validate_document(&uri) {
                        drop(server_locked); // 释放锁
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
