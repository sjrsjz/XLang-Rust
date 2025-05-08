use super::build_dict;
use super::check_if_tuple;
use once_cell::sync::Lazy;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Body, Client, Method, Url,
};
use rustc_hash::FxHashMap;
use std::{
    fmt::{self, Debug},
    sync::{Arc, Mutex},
    time::Duration, // Import Duration
};
use tokio::runtime::Runtime;
use xlang_vm_core::{
    executor::variable::{
        try_to_string_vmobject, VMBytes, VMInt, VMKeyVal, VMLambda, VMLambdaBody, VMNamed,
        VMNativeGeneratorFunction, VMNull, VMString, VMTuple, VMVariableError,
    },
    gc::{GCRef, GCSystem},
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
    headers: HeaderMap, // Store response headers
    body_bytes: Vec<u8>,
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
                    "Ok(status={}, headers={:?}, body_len={})", // Include headers in debug
                    result.status_code,
                    result.headers, // Debug print headers
                    result.body_bytes.len()
                ))
                .finish(),
            RequestState::Done(Err(e)) => {
                f.debug_tuple("Done").field(&format!("Err({})", e)).finish()
            }
        }
    }
}

// --- Request Generator 实现 ---
#[derive(Debug, Clone)]
struct RequestGenerator {
    url: Url,
    method: Method,
    headers: HeaderMap, // Request headers
    body: Option<Vec<u8>>,
    timeout: Option<Duration>, // Optional request timeout
    state: Arc<Mutex<RequestState>>,
}

impl RequestGenerator {
    fn new(
        url: Url,
        method: Method,
        headers: HeaderMap,
        body: Option<Vec<u8>>,
        timeout: Option<Duration>, // Add timeout parameter
    ) -> Self {
        RequestGenerator {
            url,
            method,
            headers,
            body,
            timeout, // Store timeout
            state: Arc::new(Mutex::new(RequestState::Idle)),
        }
    }

    // 异步执行 HTTP 请求的内部函数
    async fn perform_request(
        client: &'static Client,
        method: Method,
        url: Url,
        headers: HeaderMap,
        body: Option<Vec<u8>>,
        timeout: Option<Duration>, // Pass timeout
        state: Arc<Mutex<RequestState>>,
    ) {
        log::debug!(
            "Starting request: {} {} with timeout {:?}",
            method,
            url,
            timeout
        ); // Log timeout

        let mut request_builder = client.request(method.clone(), url.clone());

        // Add headers
        request_builder = request_builder.headers(headers);

        // Add body if present
        if let Some(body_bytes) = body {
            request_builder = request_builder.body(Body::from(body_bytes));
        }

        // Add timeout if present
        if let Some(duration) = timeout {
            request_builder = request_builder.timeout(duration);
        }

        let result = match request_builder.send().await {
            Ok(response) => {
                log::debug!("Request finished: {}", response.status());
                let status = response.status().as_u16() as i64;
                let response_headers = response.headers().clone(); // Clone response headers
                match response.bytes().await {
                    Ok(body_bytes) => Ok(RequestResult {
                        status_code: status,
                        headers: response_headers, // Store response headers
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
                // Check if the error is a timeout
                if e.is_timeout() {
                    Err("Request timed out".to_string())
                } else {
                    Err(format!("Request failed: {}", e))
                }
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
            let headers_clone = self.headers.clone();
            let body_clone = self.body.clone();
            let timeout_clone = self.timeout; // Clone timeout

            // 在 Tokio Runtime 中异步执行请求
            RUNTIME.spawn(Self::perform_request(
                &CLIENT,
                method_clone,
                url_clone,
                headers_clone,
                body_clone,
                timeout_clone, // Pass timeout
                state_clone,
            ));
            log::trace!("Request generator initialized and task spawned.");
        } else {
            log::warn!("Request generator init called on non-idle state.");
        }
        Ok(())
    }

    fn step(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        let state = self.state.lock().unwrap();
        match *state {
            RequestState::Pending => Ok(gc_system.new_object(VMNull::new())),
            _ => Ok(gc_system.new_object(VMNull::new())),
        }
    }

    fn get_result(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        let state = self.state.lock().unwrap();
        match &*state {
            RequestState::Done(Ok(result)) => {
                // 成功：返回 {status_code: int, headers: {key:str, ...}, body: bytes, error_message: null}
                let mut status_obj = gc_system.new_object(VMInt::new(result.status_code));
                let mut body_obj = gc_system.new_object(VMBytes::new(&result.body_bytes));
                let mut error_message = gc_system.new_object(VMNull::new());

                // Convert HeaderMap to VM Dictionary
                let mut headers_map = FxHashMap::default();
                for (name, value) in &result.headers {
                    // Header values might not be valid UTF-8, handle potential errors
                    let value_str = match value.to_str() {
                        Ok(s) => s.to_string(),
                        Err(_) => {
                            // Option 1: Skip non-UTF8 headers
                            // log::warn!("Skipping non-UTF8 header value for key: {}", name.as_str());
                            // continue;
                            // Option 2: Use lossy conversion (might be unexpected for user)
                            // value.to_str().unwrap_or_else(|_| String::from_utf8_lossy(value.as_bytes()).into_owned())
                            // Option 3: Return bytes (requires VMBytes support in dict values)
                            // For simplicity, let's use lossy conversion for now
                            String::from_utf8_lossy(value.as_bytes()).into_owned()
                        }
                    };
                    let mut key_gc = gc_system.new_object(VMString::new(name.as_str()));
                    let mut val_gc = gc_system.new_object(VMString::new(&value_str));
                    headers_map.insert(name.as_str(), val_gc.clone()); // Use String key for map
                    key_gc.drop_ref(); // Drop temporary GCRefs
                    val_gc.drop_ref();
                }
                let mut headers_dict = build_dict(&mut headers_map, gc_system);

                let mut result_dict_map = FxHashMap::from_iter(vec![
                    ("status_code", status_obj.clone()),
                    ("headers", headers_dict.clone()), // Add headers dict
                    ("body", body_obj.clone()),
                    ("error_message", error_message.clone()),
                ]);
                let dict = build_dict(&mut result_dict_map, gc_system);

                // 释放临时 GCRef
                status_obj.drop_ref();
                headers_dict.drop_ref(); // Drop headers dict ref
                body_obj.drop_ref();
                error_message.drop_ref();
                log::trace!("Request generator returning successful result.");
                Ok(dict)
            }
            RequestState::Done(Err(e)) => {
                // 失败：返回 {status_code: null, headers: null, body: null, error_message: string}
                let mut null_obj = gc_system.new_object(VMNull::new());
                let mut error_message = gc_system.new_object(VMString::new(e));
                let mut result_dict_map = FxHashMap::from_iter(vec![
                    ("status_code", null_obj.clone()),
                    ("headers", null_obj.clone()), // Headers are null on error
                    ("body", null_obj.clone()),    // Body is null on error
                    ("error_message", error_message.clone()),
                ]);
                let result_dict = build_dict(&mut result_dict_map, gc_system);

                // 释放临时 GCRef
                null_obj.drop_ref();
                error_message.drop_ref();
                log::trace!("Request generator returning error result: {}", e);
                Ok(result_dict)
            }
            RequestState::Idle | RequestState::Pending => {
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
        log::trace!("Cloning request generator.");
        Arc::new(Box::new(RequestGenerator::new(
            self.url.clone(),
            self.method.clone(),
            self.headers.clone(),
            self.body.clone(),
            self.timeout, // Clone timeout
        )))
    }
}

// --- VM 可调用函数 ---

/// 创建一个异步 HTTP 请求生成器。
/// 接受一个元组参数，包含命名参数：
/// - url: string (必需)
/// - method: string (可选, 默认 "GET")
/// - header: tuple (可选, 元素为 key:value 或 key=>value, key/value 需为 string)
/// - body: bytes | string | null (可选)
/// - timeout_ms: int (可选, 超时毫秒数)
pub fn request(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    tuple: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>(); // Get mutable reference

    // --- 解析 URL (必需) ---
    let mut url_ref = tuple_obj
        .get_member_by_string("url", gc_system)
        .map_err(|_| {
            VMVariableError::DetailedError("Missing required named argument 'url'".to_string())
        })?;
    if !url_ref.isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            url_ref.clone_ref(),
            "Named argument 'url' must be a string".to_string(),
        ));
    }
    let url_str = &url_ref.as_const_type::<VMString>().value;
    let url = Url::parse(url_str)
        .map_err(|e| VMVariableError::DetailedError(format!("Invalid URL '{}': {}", url_str, e)))?;
    let url_str_for_sig = url_str.clone(); // For lambda signature

    // --- 解析 Method (可选) ---
    let method = match tuple_obj.get_member_by_string("method", gc_system) {
        Ok(mut method_ref) => {
            if !method_ref.isinstance::<VMString>() {
                return Err(VMVariableError::TypeError(
                    method_ref.clone_ref(),
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
    if let Ok(mut header_ref) = tuple_obj.get_member_by_string("header", gc_system) {
        if !header_ref.isinstance::<VMTuple>() {
            return Err(VMVariableError::TypeError(
                header_ref.clone_ref(),
                "Named argument 'header' must be a tuple".to_string(),
            ));
        }
        let header_tuple = header_ref.as_type::<VMTuple>();
        for header_item in &mut header_tuple.values {
            let (h_key_str, h_val_str) = if header_item.isinstance::<VMKeyVal>() {
                let kv = header_item.as_type::<VMKeyVal>();
                (
                    try_to_string_vmobject(kv.get_key(), None)?,
                    try_to_string_vmobject(kv.get_value(), None)?,
                )
            } else if header_item.isinstance::<VMNamed>() {
                let named_h = header_item.as_type::<VMNamed>();
                (
                    try_to_string_vmobject(named_h.get_key(), None)?,
                    try_to_string_vmobject(named_h.get_value(), None)?,
                )
            } else {
                return Err(VMVariableError::TypeError(
                    header_item.clone_ref(),
                    "Header items must be key:value or key=>value pairs".to_string(),
                ));
            };

            let header_name = HeaderName::from_bytes(h_key_str.as_bytes()).map_err(|e| {
                VMVariableError::DetailedError(format!(
                    "Invalid header name '{}': {}",
                    h_key_str, e
                ))
            })?;
            // Use from_str for HeaderValue as it handles validation better for common cases
            let header_value = HeaderValue::from_str(&h_val_str).map_err(|e| {
                VMVariableError::DetailedError(format!(
                    "Invalid header value for '{}': {}",
                    h_key_str, e
                ))
            })?;
            // HeaderMap::append allows multiple values for the same header name
            headers.append(header_name, header_value);
        }
    }

    // --- 解析 Body (可选) ---
    let mut body_opt: Option<Vec<u8>> = None;
    if let Ok(mut body_ref) = tuple_obj.get_member_by_string("body", gc_system) {
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
                body_ref.clone_ref(),
                "Named argument 'body' must be bytes, string, or null".to_string(),
            ));
        }
    }

    // --- 解析 Timeout (可选) ---
    let mut timeout_opt: Option<Duration> = None;
    if let Ok(mut timeout_ref) = tuple_obj.get_member_by_string("timeout_ms", gc_system) {
        if timeout_ref.isinstance::<VMInt>() {
            let ms = timeout_ref.as_const_type::<VMInt>().value;
            if ms > 0 {
                timeout_opt = Some(Duration::from_millis(ms as u64));
            } else if ms == 0 {
                // Treat 0 as no timeout (or default Reqwest timeout)
                timeout_opt = None;
            } else {
                return Err(VMVariableError::DetailedError(
                    "Named argument 'timeout_ms' must be a non-negative integer".to_string(),
                ));
            }
        } else if !timeout_ref.isinstance::<VMNull>() {
            // Allow null to explicitly mean no timeout
            return Err(VMVariableError::TypeError(
                timeout_ref.clone_ref(),
                "Named argument 'timeout_ms' must be an integer or null".to_string(),
            ));
        }
    }

    // --- 创建 Generator ---
    log::debug!(
        "Creating RequestGenerator for: {} {} with headers: {:?}, body: {:?}, timeout: {:?}",
        method,
        url,
        headers,
        body_opt.is_some(),
        timeout_opt // Log timeout
    );
    let generator = RequestGenerator::new(
        url.clone(),
        method.clone(),
        headers,
        body_opt,
        timeout_opt, // Pass timeout
    );
    let generator_arc: Arc<Box<dyn VMNativeGeneratorFunction>> = Arc::new(Box::new(generator));

    // --- 创建 Lambda 返回给 VM ---
    let mut lambda_body = VMLambdaBody::VMNativeGeneratorFunction(generator_arc);
    let mut default_args = gc_system.new_object(VMTuple::new(&mut vec![]));
    let mut result_placeholder = gc_system.new_object(VMNull::new());

    let lambda = gc_system.new_object(VMLambda::new(
        0,
        format!("request_generator(url='{}', ...)", url_str_for_sig),
        &mut default_args,
        None,
        None,
        &mut lambda_body,
        &mut result_placeholder,
        false,
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
    fn(
        Option<&mut GCRef>,
        Option<&mut GCRef>,
        &mut GCRef,
        &mut GCSystem,
    ) -> Result<GCRef, VMVariableError>,
)> {
    vec![("request", request)]
}
