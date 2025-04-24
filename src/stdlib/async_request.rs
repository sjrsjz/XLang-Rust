use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
    thread,
};
use tokio::runtime::Runtime;
use xlang_vm_core::{
    executor::variable::{
        try_to_string_vmobject, VMBytes, VMInt, VMLambda, VMLambdaBody, VMNativeGeneratorFunction,
        VMNull, VMString, VMTuple, VMVariableError,
    },
    gc::{GCRef, GCSystem},
};
use super::check_if_tuple;
use once_cell::sync::Lazy; // 用于全局 Runtime

// --- 全局 Tokio Runtime ---
// 使用 Lazy 确保 Runtime 只被初始化一次
static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime")
});

// --- Request Generator 实现 ---

#[derive(Clone)]
enum RequestState {
    Idle,
    Pending,
    Done(Result<(i64, Vec<u8>), String>), // (status_code, body_bytes) or error message
}

// Debug 手动实现，因为 reqwest::Response 不支持 Debug 或 Clone
impl Debug for RequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestState::Idle => write!(f, "Idle"),
            RequestState::Pending => write!(f, "Pending"),
            RequestState::Done(Ok((status, body))) => f
                .debug_tuple("Done")
                .field(&format!("Ok(status={}, body_len={})", status, body.len()))
                .finish(),
            RequestState::Done(Err(e)) => f.debug_tuple("Done").field(&format!("Err({})", e)).finish(),
        }
    }
}

#[derive(Debug, Clone)]
struct RequestGenerator {
    url: String,
    method: String, // "GET", "POST", etc.
    // Add fields for headers, body etc. if needed
    state: Arc<Mutex<RequestState>>,
}

impl RequestGenerator {
    fn new(url: String, method: String) -> Self {
        RequestGenerator {
            url,
            method,
            state: Arc::new(Mutex::new(RequestState::Idle)),
        }
    }
}

impl VMNativeGeneratorFunction for RequestGenerator {
    fn init(&mut self, _arg: &mut GCRef, _gc_system: &mut GCSystem) -> Result<(), VMVariableError> {
        let state_clone = self.state.clone();
        let url_clone = self.url.clone();
        let method_clone = self.method.clone(); // Assuming GET for simplicity now

        // 在 Tokio Runtime 中异步执行请求
        RUNTIME.spawn(async move {
            println!("Starting request to {}", url_clone); // Debug
            let client = reqwest::Client::new();
            let res = match method_clone.to_uppercase().as_str() {
                 "GET" => client.get(&url_clone).send().await,
                 // Add other methods like POST here
                 _ => {
                     let mut state = state_clone.lock().unwrap();
                     *state = RequestState::Done(Err(format!("Unsupported method: {}", method_clone)));
                     return;
                 }
            };

            let final_state = match res {
                Ok(response) => {
                    println!("Request finished: {}", response.status()); // Debug
                    let status = response.status().as_u16() as i64;
                    match response.bytes().await {
                        Ok(body_bytes) => RequestState::Done(Ok((status, body_bytes.to_vec()))),
                        Err(e) => RequestState::Done(Err(format!("Failed to read response body: {}", e))),
                    }
                }
                Err(e) => {
                    println!("Request failed: {}", e); // Debug
                    RequestState::Done(Err(format!("Request failed: {}", e)))
                }
            };

            // 更新共享状态
            let mut state = state_clone.lock().unwrap();
            *state = final_state;
            println!("State updated"); // Debug
        });

        // 将状态设置为 Pending
        let mut state = self.state.lock().unwrap();
        *state = RequestState::Pending;
        Ok(())
    }

    fn step(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        // Step 对于这种后台线程模型来说，只是检查状态
        // 如果仍在 Pending，返回 Null 让 VM 继续轮询
        let state = self.state.lock().unwrap();
        match *state {
            RequestState::Pending => Ok(gc_system.new_object(VMNull::new())),
            _ => Ok(gc_system.new_object(VMNull::new())), // 或者返回一个特殊值表示完成
        }
    }

    fn get_result(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        let state = self.state.lock().unwrap();
        match &*state {
            RequestState::Done(Ok((status, body_bytes))) => {
                // 创建包含状态码和响应体的元组
                let mut status_obj = gc_system.new_object(VMInt::new(*status));
                let mut body_obj = gc_system.new_object(VMBytes::new(body_bytes));
                let mut tuple_elements = vec![&mut status_obj, &mut body_obj];
                let result_tuple = gc_system.new_object(VMTuple::new(&mut tuple_elements));
                status_obj.drop_ref();
                body_obj.drop_ref();
                Ok(result_tuple)
            }
            RequestState::Done(Err(e)) => {
                 // 返回一个表示错误的元组或引发 VM 错误
                 // 这里返回一个包含 null 状态码和错误字符串的元组
                let mut null_obj = gc_system.new_object(VMNull::new());
                let mut err_str_obj = gc_system.new_object(VMString::new(e));
                let mut tuple_elements = vec![&mut null_obj, &mut err_str_obj];
                let result_tuple = gc_system.new_object(VMTuple::new(&mut tuple_elements));
                null_obj.drop_ref();
                err_str_obj.drop_ref();
                Ok(result_tuple)
                // 或者直接返回 VMVariableError::DetailedError(e.clone())
            }
            _ => Err(VMVariableError::DetailedError(
                "Generator result requested before completion".to_string(),
            )),
        }
    }

    fn is_done(&self) -> bool {
        let state = self.state.lock().unwrap();
        !matches!(*state, RequestState::Idle | RequestState::Pending)
    }

    fn clone_generator(&self) -> Arc<Box<dyn VMNativeGeneratorFunction>> {
        // 创建一个新的 Generator 实例，状态重置为 Idle
        // 注意：这不会复制正在进行的请求的状态
        Arc::new(Box::new(RequestGenerator::new(
            self.url.clone(),
            self.method.clone(),
        )))
    }
}

// --- VM 可调用函数 ---

pub fn request(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple.clone())?;
    let tuple_obj = tuple.as_const_type::<VMTuple>();

    if tuple_obj.values.len() < 1 || tuple_obj.values.len() > 2 {
        return Err(VMVariableError::TypeError(
            tuple.clone(),
            format!(
                "request expected 1 or 2 arguments (url, [method='GET']), got {}",
                tuple_obj.values.len()
            ),
        ));
    }

    // 获取 URL
    let url_obj = &tuple_obj.values[0];
    if !url_obj.isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            url_obj.clone(),
            "Argument 1 (url) must be a string".to_string(),
        ));
    }
    let url = url_obj.as_const_type::<VMString>().value.clone();

    // 获取方法 (可选, 默认为 GET)
    let method = if tuple_obj.values.len() == 2 {
        let method_obj = &tuple_obj.values[1];
        if !method_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                method_obj.clone(),
                "Argument 2 (method) must be a string".to_string(),
            ));
        }
        method_obj.as_const_type::<VMString>().value.clone()
    } else {
        "GET".to_string()
    };

    // 创建 Generator
    let generator = RequestGenerator::new(url, method);
    let generator_arc: Arc<Box<dyn VMNativeGeneratorFunction>> = Arc::new(Box::new(generator));

    // 创建 Lambda 返回给 VM
    let mut lambda_body = VMLambdaBody::VMNativeGeneratorFunction(generator_arc);
    let mut default_args = gc_system.new_object(VMTuple::new(&mut vec![])); // 无默认参数
    let mut result_placeholder = gc_system.new_object(VMNull::new()); // 结果占位符

    let lambda = gc_system.new_object(VMLambda::new(
        0, // code_position 不适用
        "request_generator".to_string(),
        &mut default_args,
        None, // capture
        None, // self_object
        &mut lambda_body,
        &mut result_placeholder,
    ));

    default_args.drop_ref();
    result_placeholder.drop_ref();

    Ok(lambda)
}

// --- 注册函数 ---
pub fn get_request_functions() -> Vec<(
    &'static str,
    fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
)> {
    vec![("request", request)]
}
