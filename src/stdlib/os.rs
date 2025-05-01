use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use rustc_hash::FxHashMap;

use xlang_vm_core::{
    executor::variable::{VMBoolean, VMInt, VMNull, VMString, VMTuple, VMVariableError},
    gc::{GCRef, GCSystem},
};

use super::{check_if_tuple, build_dict};

// 获取当前工作目录
fn getcwd(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    
    match env::current_dir() {
        Ok(path) => {
            match path.to_str() {
                Some(path_str) => Ok(gc_system.new_object(VMString::new(path_str))),
                None => Err(VMVariableError::DetailedError(
                    "Current directory contains invalid UTF-8 characters".to_string()
                ))
            }
        },
        Err(e) => Err(VMVariableError::DetailedError(
            format!("Failed to get current directory: {}", e)
        ))
    }
}

// 更改当前工作目录
fn chdir(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            "chdir() takes exactly 1 argument".to_string()
        ));
    }
    
    if !tuple_obj.values[0].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[0].clone_ref(),
            "chdir() argument must be a string".to_string()
        ));
    }
    
    let new_dir = tuple_obj.values[0].as_const_type::<VMString>().value.clone();
    match env::set_current_dir(new_dir.clone()) {
        Ok(_) => Ok(gc_system.new_object(VMNull::new())),
        Err(e) => Err(VMVariableError::DetailedError(
            format!("Failed to change directory to '{}': {}", new_dir, e)
        ))
    }
}

// 获取环境变量
fn getenv(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            "getenv() takes exactly 1 argument".to_string()
        ));
    }
    
    if !tuple_obj.values[0].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[0].clone_ref(),
            "getenv() argument must be a string".to_string()
        ));
    }
    
    let var_name = tuple_obj.values[0].as_const_type::<VMString>().value.clone();
    match env::var(var_name.clone()) {
        Ok(value) => Ok(gc_system.new_object(VMString::new(&value))),
        Err(_) => Ok(gc_system.new_object(VMString::new(""))) // 返回空字符串而非错误
    }
}

// 设置环境变量
fn setenv(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    
    if tuple_obj.values.len() != 2 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            "setenv() takes exactly 2 arguments".to_string()
        ));
    }
    
    if !tuple_obj.values[0].isinstance::<VMString>() || !tuple_obj.values[1].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            "setenv() arguments must be strings".to_string()
        ));
    }
    
    let var_name = tuple_obj.values[0].as_const_type::<VMString>().value.clone();
    let var_value = tuple_obj.values[1].as_const_type::<VMString>().value.clone();
    
    env::set_var(var_name, var_value);
    Ok(gc_system.new_object(VMNull::new()))
}

// 获取所有环境变量
fn environ(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    
    let env_vars: Vec<(String, String)> = env::vars().collect();
    let mut env_dict = FxHashMap::default();
    
    for (key, val) in env_vars {
        let mut val_obj = gc_system.new_object(VMString::new(&val));
        env_dict.insert(key.clone(), val_obj.clone_ref());
        val_obj.drop_ref();
    }
    
    Ok(super::build_dict_using_string(&mut env_dict, gc_system))
}

// 获取路径分隔符
fn path_separator(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    
    #[cfg(windows)]
    let separator = "\\";
    #[cfg(not(windows))]
    let separator = "/";
    
    Ok(gc_system.new_object(VMString::new(separator)))
}

// 获取系统名称
fn system_name(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    
    Ok(gc_system.new_object(VMString::new(std::env::consts::OS)))
}

// 执行系统命令
fn system(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            "system() takes exactly 1 argument".to_string()
        ));
    }
    
    if !tuple_obj.values[0].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[0].clone_ref(),
            "system() argument must be a string".to_string()
        ));
    }
    
    let command_str = tuple_obj.values[0].as_const_type::<VMString>().value.clone();
    
    #[cfg(windows)]
    let command_result = Command::new("cmd").arg("/c").arg(&command_str).status();
    
    #[cfg(not(windows))]
    let command_result = Command::new("sh").arg("-c").arg(&command_str).status();
    
    match command_result {
        Ok(status) => {
            let exit_code = status.code().unwrap_or(-1);
            Ok(gc_system.new_object(VMInt::new(exit_code as i64)))
        },
        Err(e) => Err(VMVariableError::DetailedError(
            format!("Failed to execute command: {}", e)
        ))
    }
}

// 连接路径
fn join_path(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    
    if tuple_obj.values.len() < 2 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            "join_path() requires at least 2 arguments".to_string()
        ));
    }
    
    let mut result_path = PathBuf::new();
    
    for arg in &mut tuple_obj.values {
        if !arg.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                arg.clone_ref(),
                "join_path() arguments must be strings".to_string()
            ));
        }
        
        let path_part = arg.as_const_type::<VMString>().value.clone();
        result_path.push(path_part);
    }
    
    match result_path.to_str() {
        Some(path_str) => Ok(gc_system.new_object(VMString::new(path_str))),
        None => Err(VMVariableError::DetailedError(
            "Path contains invalid UTF-8 characters".to_string()
        ))
    }
}

// 获取父目录路径
fn dirname(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            "dirname() takes exactly 1 argument".to_string()
        ));
    }
    
    if !tuple_obj.values[0].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[0].clone_ref(),
            "dirname() argument must be a string".to_string()
        ));
    }
    
    let path_str = tuple_obj.values[0].as_const_type::<VMString>().value.clone();
    let path = Path::new(&path_str);
    
    match path.parent() {
        Some(parent) => match parent.to_str() {
            Some(parent_str) => Ok(gc_system.new_object(VMString::new(parent_str))),
            None => Err(VMVariableError::DetailedError(
                "Parent path contains invalid UTF-8 characters".to_string()
            ))
        },
        None => Ok(gc_system.new_object(VMString::new(""))) // 返回空字符串表示没有父目录
    }
}

// 获取文件名部分
fn basename(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            "basename() takes exactly 1 argument".to_string()
        ));
    }
    
    if !tuple_obj.values[0].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[0].clone_ref(),
            "basename() argument must be a string".to_string()
        ));
    }
    
    let path_str = tuple_obj.values[0].as_const_type::<VMString>().value.clone();
    let path = Path::new(&path_str);
    
    match path.file_name() {
        Some(name) => match name.to_str() {
            Some(name_str) => Ok(gc_system.new_object(VMString::new(name_str))),
            None => Err(VMVariableError::DetailedError(
                "File name contains invalid UTF-8 characters".to_string()
            ))
        },
        None => Ok(gc_system.new_object(VMString::new(""))) // 没有文件名部分时返回空字符串
    }
}

// 获取绝对路径
fn abspath(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            "abspath() takes exactly 1 argument".to_string()
        ));
    }
    
    if !tuple_obj.values[0].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[0].clone_ref(),
            "abspath() argument must be a string".to_string()
        ));
    }
    
    let path_str = tuple_obj.values[0].as_const_type::<VMString>().value.clone();
    let path = Path::new(&path_str);
    
    match path.canonicalize() {
        Ok(abs_path) => match abs_path.to_str() {
            Some(abs_path_str) => {
                // 在Windows上，canonicalize会返回带有前缀的路径，需要处理
                #[cfg(windows)]
                let abs_path_str = abs_path_str.trim_start_matches(r"\\?\");
                
                Ok(gc_system.new_object(VMString::new(abs_path_str)))
            },
            None => Err(VMVariableError::DetailedError(
                "Absolute path contains invalid UTF-8 characters".to_string()
            ))
        },
        Err(e) => Err(VMVariableError::DetailedError(
            format!("Failed to get absolute path: {}", e)
        ))
    }
}

// 获取当前进程ID
fn getpid(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    
    Ok(gc_system.new_object(VMInt::new(std::process::id() as i64)))
}

// 获取CPU核心数
fn cpu_count(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    
    match std::thread::available_parallelism() {
        Ok(count) => Ok(gc_system.new_object(VMInt::new(count.get() as i64))),
        Err(_) => Ok(gc_system.new_object(VMInt::new(1))) // 默认至少有1个核心
    }
}

// 检查路径是否存在
fn path_exists(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            "path_exists() takes exactly 1 argument".to_string()
        ));
    }
    
    if !tuple_obj.values[0].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[0].clone_ref(),
            "path_exists() argument must be a string".to_string()
        ));
    }
    
    let path_str = tuple_obj.values[0].as_const_type::<VMString>().value.clone();
    let path = Path::new(&path_str);
    
    Ok(gc_system.new_object(VMBoolean::new(path.exists())))
}

// 获取系统信息
fn system_info(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    
    let mut info = FxHashMap::default();
    
    // 操作系统类型
    let mut os_type = gc_system.new_object(VMString::new(std::env::consts::OS));
    info.insert("os", os_type.clone_ref());
    os_type.drop_ref();
    
    // 架构
    let mut arch = gc_system.new_object(VMString::new(std::env::consts::ARCH));
    info.insert("arch", arch.clone_ref());
    arch.drop_ref();
    
    // 操作系统家族
    let mut family = gc_system.new_object(VMString::new(std::env::consts::FAMILY));
    info.insert("family", family.clone_ref());
    family.drop_ref();
    
    // 获取当前时间戳
    if let Ok(time) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let mut timestamp = gc_system.new_object(VMInt::new(time.as_secs() as i64));
        info.insert("timestamp", timestamp.clone_ref());
        timestamp.drop_ref();
    }
    
    // 返回字典
    Ok(build_dict(&mut info, gc_system))
}

// 获取命令行参数
fn args(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    
    let args: Vec<String> = env::args().collect();
    let mut args_tuple = Vec::with_capacity(args.len());
    
    for arg in args {
        let mut arg_obj = gc_system.new_object(VMString::new(&arg));
        args_tuple.push(arg_obj.clone_ref());
        arg_obj.drop_ref();
    }
    
    Ok(gc_system.new_object(VMTuple::new(&mut args_tuple.iter_mut().collect())))
}

// 导出函数列表
pub fn get_os_functions() -> Vec<(
    &'static str,
    fn(&mut GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
)> {
    vec![
        ("getcwd", getcwd),
        ("chdir", chdir),
        ("getenv", getenv),
        ("setenv", setenv),
        ("environ", environ),
        ("path_separator", path_separator),
        ("system_name", system_name),
        ("system", system),
        ("join_path", join_path),
        ("dirname", dirname),
        ("basename", basename),
        ("abspath", abspath),
        ("getpid", getpid),
        ("cpu_count", cpu_count),
        ("path_exists", path_exists),
        ("system_info", system_info),
        ("args", args),
    ]
}