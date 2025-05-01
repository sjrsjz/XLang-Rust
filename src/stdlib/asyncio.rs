use super::check_if_tuple;
use xlang_vm_core::{
    executor::variable::{VMBoolean, VMCoroutineStatus, VMLambda, VMTuple, VMVariableError},
    gc::{GCRef, GCSystem},
};

pub fn pause(tuple: &mut GCRef, _gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!(
                "pause expected 1 arguments, got {}",
                tuple.as_const_type::<VMTuple>().values.len()
            ),
        ));
    }
    let arg = &mut tuple_obj.values[0];
    if !arg.isinstance::<VMLambda>() {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Argument to pause must be a VMLambda".to_string(),
        ));
    }
    let lambda = arg.as_type::<VMLambda>();
    if lambda.coroutine_status == VMCoroutineStatus::Crashed {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Coroutine is in a crashed state".to_string(),
        ));
    }
    if lambda.coroutine_status == VMCoroutineStatus::Finished {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Coroutine is in a finished state".to_string(),
        ));
    }
    lambda.coroutine_status = VMCoroutineStatus::Pending;
    let obj = arg.clone_ref();
    Ok(obj)
}

pub fn resume(tuple: &mut GCRef, _gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!(
                "resume expected 1 arguments, got {}",
                tuple.as_const_type::<VMTuple>().values.len()
            ),
        ));
    }
    let arg = &mut tuple_obj.values[0];
    if !arg.isinstance::<VMLambda>() {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Argument to resume must be a VMLambda".to_string(),
        ));
    }
    let lambda = arg.as_type::<VMLambda>();
    if lambda.coroutine_status == VMCoroutineStatus::Crashed {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Coroutine is in a crashed state".to_string(),
        ));
    }
    if lambda.coroutine_status == VMCoroutineStatus::Finished {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Coroutine is in a finished state".to_string(),
        ));
    }
    lambda.coroutine_status = VMCoroutineStatus::Running;
    let obj = arg.clone_ref();
    Ok(obj)
}

pub fn kill(tuple: &mut GCRef, _gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!(
                "kill expected 1 arguments, got {}",
                tuple.as_const_type::<VMTuple>().values.len()
            ),
        ));
    }
    let arg = &mut tuple_obj.values[0];
    if !arg.isinstance::<VMLambda>() {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Argument to kill must be a VMLambda".to_string(),
        ));
    }
    let lambda = arg.as_type::<VMLambda>();
    if lambda.coroutine_status == VMCoroutineStatus::Crashed {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Coroutine is in a crashed state".to_string(),
        ));
    }
    if lambda.coroutine_status == VMCoroutineStatus::Finished {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Coroutine is in a finished state".to_string(),
        ));
    }
    lambda.coroutine_status = VMCoroutineStatus::Finished;
    let obj = arg.clone_ref();
    Ok(obj)
}

pub fn is_running(tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!(
                "is_running expected 1 arguments, got {}",
                tuple.as_const_type::<VMTuple>().values.len()
            ),
        ));
    }
    let arg = &mut tuple_obj.values[0];
    if !arg.isinstance::<VMLambda>() {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Argument to is_running must be a VMLambda".to_string(),
        ));
    }
    let lambda = arg.as_type::<VMLambda>();
    let obj = gc_system.new_object(VMBoolean::new(
        lambda.coroutine_status == VMCoroutineStatus::Running,
    ));
    Ok(obj)
}

pub fn is_pending(tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!(
                "is_pending expected 1 arguments, got {}",
                tuple.as_const_type::<VMTuple>().values.len()
            ),
        ));
    }
    let arg = &mut tuple_obj.values[0];
    if !arg.isinstance::<VMLambda>() {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Argument to is_pending must be a VMLambda".to_string(),
        ));
    }
    let lambda = arg.as_type::<VMLambda>();
    let obj = gc_system.new_object(VMBoolean::new(
        lambda.coroutine_status == VMCoroutineStatus::Pending,
    ));
    Ok(obj)
}

pub fn is_crashed(tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!(
                "is_crashed expected 1 arguments, got {}",
                tuple.as_const_type::<VMTuple>().values.len()
            ),
        ));
    }
    let arg = &mut tuple_obj.values[0];
    if !arg.isinstance::<VMLambda>() {
        return Err(VMVariableError::TypeError(
            arg.clone_ref(),
            "Argument to is_crashed must be a VMLambda".to_string(),
        ));
    }
    let lambda = arg.as_type::<VMLambda>();
    let obj = gc_system.new_object(VMBoolean::new(
        lambda.coroutine_status == VMCoroutineStatus::Crashed,
    ));
    Ok(obj)
}
// Helper to provide functions for registration
pub fn get_asyncio_functions() -> Vec<(
    &'static str,
    fn(&mut GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
)> {
    vec![
        ("pause", pause),
        ("resume", resume),
        ("kill", kill),
        ("is_running", is_running),
        ("is_pending", is_pending),
        ("is_crashed", is_crashed),
    ]
}
