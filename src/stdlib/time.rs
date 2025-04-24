use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use xlang_vm_core::{
    executor::variable::{
        VMFloat, VMInt, VMLambda, VMLambdaBody, VMNull, VMNativeGeneratorFunction,
        VMVariableError,
    },
    gc::{GCRef, GCSystem},
};

use super::check_if_tuple;

#[derive(Debug, Clone)]
struct SleepGenerator {
    start_time: Option<Instant>,
    duration: Duration,
    done: bool,
}

impl SleepGenerator {
    fn new() -> Self {
        SleepGenerator {
            start_time: None,
            duration: Duration::from_secs(0), // Default duration
            done: false,
        }
    }
}

impl VMNativeGeneratorFunction for SleepGenerator {
    fn init(&mut self, _arg: &mut GCRef, _gc_system: &mut GCSystem) -> Result<(), VMVariableError> {
        self.start_time = Some(Instant::now());
        self.done = false;
        Ok(())
    }

    fn step(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if self.done {
            // Should ideally not be called again after done, but return Null if it is.
             return Ok(gc_system.new_object(VMNull::new()));
        }

        if let Some(start) = self.start_time {
            if start.elapsed() >= self.duration {
                self.done = true;
                // Signal completion by returning Null (or could return a specific completion value)
                 Ok(gc_system.new_object(VMNull::new()))
            } else {
                // Not done yet, yield Null to indicate suspension
                 Ok(gc_system.new_object(VMNull::new()))
            }
        } else {
            // Should not happen if init was called correctly
            Err(VMVariableError::DetailedError(
                "SleepGenerator not initialized".to_string(),
            ))
        }
    }

    fn get_result(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        // The result of sleep is typically Null
        Ok(gc_system.new_object(VMNull::new()))
    }

    fn is_done(&self) -> bool {
        self.done
    }

    fn clone_generator(&self) -> Arc<Box<dyn VMNativeGeneratorFunction>> {
        Arc::new(Box::new(self.clone()))
    }
}

// Entry function exposed to the VM
pub fn sleep_entry(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple.clone())?;
    let tuple_obj = tuple.as_const_type::<xlang_vm_core::executor::variable::VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone(),
            format!("sleep expected 1 argument (duration), got {}", tuple_obj.values.len()),
        ));
    }
    let duration_arg = tuple_obj.values[0].clone();

    // Create the generator state
    let mut generator = SleepGenerator::new();
    // Initialize the generator with the duration argument
    if duration_arg.isinstance::<VMInt>() {
        let duration = duration_arg.as_const_type::<VMInt>().to_int()?;
        generator.duration = Duration::from_secs(duration as u64);
    } else if duration_arg.isinstance::<VMFloat>() {
        let duration = duration_arg.as_const_type::<VMFloat>().to_float()?;
        generator.duration = Duration::from_secs_f64(duration);
    } else {
        return Err(VMVariableError::TypeError(
            duration_arg.clone(),
            "sleep argument must be an int or float".to_string(),
        ));
    }

    
    // Create an empty tuple for default args
    let mut params = gc_system.new_object(xlang_vm_core::executor::variable::VMTuple::new(&mut vec![]));
    let mut result = gc_system.new_object(VMNull::new()); // Default result placeholder

    // Wrap the generator in a VMLambda
    let lambda = gc_system.new_object(VMLambda::new(
        0,                               // code_position, 0 for native generator
        "<builtins>::sleep_generator".to_string(), // signature
        &mut params,
        None, // capture
        None, // self_object
        &mut VMLambdaBody::VMNativeGeneratorFunction(Arc::new(Box::new(generator))),
        &mut result,
    ));

    // Drop refs owned by the lambda now
    params.drop_ref();
    result.drop_ref();

    Ok(lambda)
}


pub fn timestamp(
    _value: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError>{
    // return the current time in seconds since the epoch
    let now = SystemTime::now();
    let duration = now.duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| VMVariableError::DetailedError(format!("System time error: {}", e)))?;
    let seconds = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000.0;
    let result = gc_system.new_object(VMFloat::new(seconds));
    return Ok(result);
}

// Helper to provide the entry function for registration
pub fn get_time_function() -> Vec<(
    &'static str,
    fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
)> {
    vec![("sleep", sleep_entry), ("timestamp", timestamp)]
}