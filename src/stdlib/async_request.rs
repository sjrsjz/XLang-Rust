use std::{
    fmt::{self, Debug},
    sync::{Arc, Mutex},
};
use tokio::runtime::Runtime;
use xlang_vm_core::{
    executor::variable::{
        try_to_string_vmobject, VMBytes, VMInt, VMKeyVal, VMLambda, VMLambdaBody, VMNamed,
        VMNativeGeneratorFunction, VMNull, VMString, VMTuple, VMVariableError,
    },
    gc::{GCRef, GCSystem},
};
use super::check_if_tuple;
use once_cell::sync::Lazy;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Body, Client, Method, Url,
}; // Import necessary reqwest types

static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all() // 启用所有 Tokio 功能 (IO, time)
        .build()
        .expect("Failed to create Tokio runtime")
});

// 重用 Reqwest Client 以利用连接池等优化
static CLIENT: Lazy<Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .build()
        .expect("Failed to create Reqwest client")
});

// --- Request Generator 状态 ---
#[derive(Clone)]
enum RequestState {
    /// 初始状态，尚未开始
    Idle,
    /// 请求已发送，正在等待响应
    Pending,
    /// 请求已完成（成功或失败）
    Done(Result<RequestResult, String>),
}

// 封装成功请求的结果
#[derive(Clone)]
struct RequestResult {
    status_code: i64,
    body_bytes: Vec<u8>,
    // 可以添加 headers 等
}

// Debug 手动实现，因为 reqwest::Response 不支持 Debug 或 Clone
impl Debug for RequestState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestState::Idle => write!(f, "Idle"),
            RequestState::Pending => write!(f, "Pending"),
            RequestState::Done(Ok(result)) => f
                .debug_tuple("Done")
                .field(&format!(
                    "Ok(status={}, body_len={})",
                    result.status_code,
                    result.body_bytes.len()
                ))
                .finish(),
            RequestState::Done(Err(e)) => f.debug_tuple("Done").field(&format!("Err({})", e)).finish(),
        }
    }
}

// --- Request Generator 实现 ---
#[derive(Debug, Clone)]
struct RequestGenerator {
    url: Url,
    method: Method,
    headers: HeaderMap, // Store parsed headers
    body: Option<Vec<u8>>, // Store body bytes
    state: Arc<Mutex<RequestState>>,
}

impl RequestGenerator {
    fn new(url: Url, method: Method, headers: HeaderMap, body: Option<Vec<u8>>) -> Self {
        RequestGenerator {
            url,
            method,
            headers,
            body,
            state: Arc::new(Mutex::new(RequestState::Idle)),
        }
    }

    // 异步执行 HTTP 请求的内部函数
    async fn perform_request(
        client: &'static Client,
        method: Method,
        url: Url,
        headers: HeaderMap, // Pass headers
        body: Option<Vec<u8>>, // Pass body
        state: Arc<Mutex<RequestState>>,
    ) {
        log::debug!("Starting request: {} {}", method, url); // 使用 log crate

        let mut request_builder = client.request(method.clone(), url.clone());

        // Add headers
        request_builder = request_builder.headers(headers);

        // Add body if present
        if let Some(body_bytes) = body {
            request_builder = request_builder.body(Body::from(body_bytes));
        }

        let result = match request_builder.send().await {
            Ok(response) => {
                log::debug!("Request finished: {}", response.status());
                let status = response.status().as_u16() as i64;
                match response.bytes().await {
                    Ok(body_bytes) => Ok(RequestResult {
                        status_code: status,
                        body_bytes: body_bytes.to_vec(),
                    }),
                    Err(e) => {
                        log::error!("Failed to read response body for {}: {}", url, e);
                        Err(format!("Failed to read response body: {}", e))
                    }
                }
            }
            Err(e) => {
                log::error!("Request failed for {}: {}", url, e);
                Err(format!("Request failed: {}", e))
            }
        };

        // 更新共享状态
        let mut state_lock = state.lock().unwrap();
        *state_lock = RequestState::Done(result);
        log::debug!("State updated for {}", url);
    }
}

impl VMNativeGeneratorFunction for RequestGenerator {
    fn init(&mut self, _arg: &mut GCRef, _gc_system: &mut GCSystem) -> Result<(), VMVariableError> {
        let mut current_state = self.state.lock().unwrap();
        // 确保只初始化一次
        if matches!(*current_state, RequestState::Idle) {
            *current_state = RequestState::Pending;
            let state_clone = self.state.clone();
            let url_clone = self.url.clone();
            let method_clone = self.method.clone();
            let headers_clone = self.headers.clone(); // Clone headers
            let body_clone = self.body.clone(); // Clone body

            // 在 Tokio Runtime 中异步执行请求
            RUNTIME.spawn(Self::perform_request(
                &CLIENT, // 使用全局 Client
                method_clone,
                url_clone,
                headers_clone, // Pass headers
                body_clone, // Pass body
                state_clone,
            ));
            log::trace!("Request generator initialized and task spawned.");
        } else {
            log::warn!("Request generator init called on non-idle state.");
            // 或者返回错误？取决于 VM 期望的行为
            // return Err(VMVariableError::DetailedError("Generator already initialized".to_string()));
        }
        Ok(())
    }

    fn step(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        // step 仅用于轮询，检查是否完成。
        // 如果 VM 支持更高级的异步集成（例如暂停执行直到 Future 完成），
        // 则可以采用不同的方法。
        // 对于基于轮询的生成器，返回 Null 表示“尚未完成，请稍后重试”。
        let state = self.state.lock().unwrap();
        match *state {
            RequestState::Pending => Ok(gc_system.new_object(VMNull::new())),
            // 如果已完成或处于空闲状态（理论上不应在 step 中发生），也返回 Null。
            // get_result 将处理最终结果。
            _ => Ok(gc_system.new_object(VMNull::new())),
        }
    }

    fn get_result(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        let state = self.state.lock().unwrap();
        match &*state {
            RequestState::Done(Ok(result)) => {
                // 成功：返回 (status_code: int, body: bytes)
                let mut status_obj = gc_system.new_object(VMInt::new(result.status_code));
                let mut body_obj = gc_system.new_object(VMBytes::new(&result.body_bytes)); // 使用引用创建
                let mut tuple_elements = vec![&mut status_obj, &mut body_obj];
                let result_tuple = gc_system.new_object(VMTuple::new(&mut tuple_elements));
                // 释放临时 GCRef
                status_obj.drop_ref();
                body_obj.drop_ref();
                log::trace!("Request generator returning successful result.");
                Ok(result_tuple)
            }
            RequestState::Done(Err(e)) => {
                // 失败：返回 (null, error_message: string)
                let mut null_obj = gc_system.new_object(VMNull::new());
                let mut err_str_obj = gc_system.new_object(VMString::new(e));
                let mut tuple_elements = vec![&mut null_obj, &mut err_str_obj];
                let result_tuple = gc_system.new_object(VMTuple::new(&mut tuple_elements));
                // 释放临时 GCRef
                null_obj.drop_ref();
                err_str_obj.drop_ref();
                log::trace!("Request generator returning error result: {}", e);
                Ok(result_tuple)
                // 或者，如果 VM 应该因错误而停止：
                // Err(VMVariableError::DetailedError(format!("Request failed: {}", e)))
            }
            RequestState::Idle | RequestState::Pending => {
                // 在生成器完成之前调用了 get_result
                log::warn!("get_result called before generator completed.");
                Err(VMVariableError::DetailedError(
                    "Generator result requested before completion".to_string(),
                ))
            }
        }
    }

    fn is_done(&self) -> bool {
        let state = self.state.lock().unwrap();
        matches!(*state, RequestState::Done(_))
    }

    fn clone_generator(&self) -> Arc<Box<dyn VMNativeGeneratorFunction>> {
        // 克隆生成器会创建一个新的、处于 Idle 状态的实例，
        // 使用相同的初始参数（URL, method, headers, body）。
        // 它不会复制正在进行的请求的状态。
        log::trace!("Cloning request generator.");
        Arc::new(Box::new(RequestGenerator::new(
            self.url.clone(),
            self.method.clone(),
            self.headers.clone(), // Clone headers
            self.body.clone(), // Clone body
        )))
    }
}

// --- VM 可调用函数 ---

/// 创建一个异步 HTTP 请求生成器。
/// 接受一个元组参数，包含命名参数：
/// - url: string (必需)
/// - method: string (可选, 默认 "GET")
/// - header: tuple (可选, 元素为 key:value 或 key=>value, key/value 需为 string)
/// - body: bytes | string (可选)
pub fn request(mut tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple.clone())?;
    let tuple_obj = tuple.as_type::<VMTuple>(); // Get mutable reference

    // --- 解析 URL (必需) ---
    let url_ref = tuple_obj
        .get_member_by_string("url", gc_system)
        .map_err(|_| {
            VMVariableError::DetailedError("Missing required named argument 'url'".to_string())
        })?;
    if !url_ref.isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            url_ref.clone(),
            "Named argument 'url' must be a string".to_string(),
        ));
    }
    let url_str = &url_ref.as_const_type::<VMString>().value;
    let url = Url::parse(url_str).map_err(|e| {
        VMVariableError::DetailedError(format!("Invalid URL '{}': {}", url_str, e))
    })?;
    let url_str_for_sig = url_str.clone(); // For lambda signature

    // --- 解析 Method (可选) ---
    let method = match tuple_obj.get_member_by_string("method", gc_system) {
        Ok(method_ref) => {
            if !method_ref.isinstance::<VMString>() {
                return Err(VMVariableError::TypeError(
                    method_ref.clone(),
                    "Named argument 'method' must be a string".to_string(),
                ));
            }
            let method_str = &method_ref.as_const_type::<VMString>().value;
            let parsed_method =
                Method::from_bytes(method_str.to_uppercase().as_bytes()).map_err(|e| {
                    VMVariableError::DetailedError(format!(
                        "Invalid or unsupported HTTP method '{}': {}",
                        method_str, e
                    ))
                })?;
            parsed_method
        }
        Err(_) => Method::GET, // Default to GET if not found
    };

    // --- 解析 Headers (可选) ---
    let mut headers = HeaderMap::new();
    if let Ok(header_ref) = tuple_obj.get_member_by_string("header", gc_system) {
        if !header_ref.isinstance::<VMTuple>() {
            return Err(VMVariableError::TypeError(
                header_ref.clone(),
                "Named argument 'header' must be a tuple".to_string(),
            ));
        }
        let header_tuple = header_ref.as_const_type::<VMTuple>();
        for header_item in &header_tuple.values {
            let (h_key_ref, h_val_ref) = if header_item.isinstance::<VMKeyVal>() {
                let kv = header_item.as_const_type::<VMKeyVal>();
                (kv.get_const_key(), kv.get_const_value())
            } else if header_item.isinstance::<VMNamed>() {
                let named_h = header_item.as_const_type::<VMNamed>();
                (named_h.get_const_key(), named_h.get_const_value())
            } else {
                return Err(VMVariableError::TypeError(
                    header_item.clone(),
                    "Header items must be key:value or key=>value pairs".to_string(),
                ));
            };

            let h_key_str = try_to_string_vmobject(h_key_ref.clone(), None)?;
            let h_val_str = try_to_string_vmobject(h_val_ref.clone(), None)?;

            let header_name = HeaderName::from_bytes(h_key_str.as_bytes()).map_err(|e| {
                VMVariableError::DetailedError(format!("Invalid header name '{}': {}", h_key_str, e))
            })?;
            let header_value = HeaderValue::from_bytes(h_val_str.as_bytes()).map_err(|e| {
                VMVariableError::DetailedError(format!(
                    "Invalid header value for '{}': {}",
                    h_key_str, e
                ))
            })?;
            headers.append(header_name, header_value);
        }
    }

    // --- 解析 Body (可选) ---
    let mut body_opt: Option<Vec<u8>> = None;
    if let Ok(body_ref) = tuple_obj.get_member_by_string("body", gc_system) {
        if body_ref.isinstance::<VMBytes>() {
            body_opt = Some(body_ref.as_const_type::<VMBytes>().value.clone());
        } else if body_ref.isinstance::<VMString>() {
            body_opt = Some(
                body_ref
                    .as_const_type::<VMString>()
                    .value
                    .as_bytes()
                    .to_vec(),
            );
        } else if body_ref.isinstance::<VMNull>() {
            body_opt = None; // Explicitly allow null to clear body
        } else {
            return Err(VMVariableError::TypeError(
                body_ref.clone(),
                "Named argument 'body' must be bytes, string, or null".to_string(),
            ));
        }
    }

    // --- 创建 Generator ---
    log::debug!(
        "Creating RequestGenerator for: {} {} with headers: {:?}, body: {:?}",
        method,
        url,
        headers,
        body_opt.is_some()
    );
    let generator = RequestGenerator::new(url.clone(), method.clone(), headers, body_opt); // Pass headers and body
    let generator_arc: Arc<Box<dyn VMNativeGeneratorFunction>> = Arc::new(Box::new(generator));

    // --- 创建 Lambda 返回给 VM ---
    let mut lambda_body = VMLambdaBody::VMNativeGeneratorFunction(generator_arc);
    // 生成器 lambda 通常没有默认参数或捕获（除非需要传递环境）
    let mut default_args = gc_system.new_object(VMTuple::new(&mut vec![]));
    let mut result_placeholder = gc_system.new_object(VMNull::new()); // 结果占位符，通常不用于生成器

    let lambda = gc_system.new_object(VMLambda::new(
        0, // code_position 不适用
        format!("request_generator(url='{}', ...)", url_str_for_sig), // Use stored url string
        &mut default_args,
        None, // capture
        None, // self_object
        &mut lambda_body,
        &mut result_placeholder,
    ));

    // 释放临时 GCRef
    default_args.drop_ref();
    result_placeholder.drop_ref();

    Ok(lambda)
}

// --- 注册函数 ---
/// 返回包含 `request` 函数的向量，以便注册到 VM。
pub fn get_request_functions() -> Vec<(
    &'static str,
    fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
)> {
    vec![("request", request)]
}
