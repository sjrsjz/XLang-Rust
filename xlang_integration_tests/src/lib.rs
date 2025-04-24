#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use xlang_frontend::compile::build_code;
    use xlang_vm_core::{executor::variable::{
        try_repr_vmobject, VMInstructions, VMInt, VMLambda, VMLambdaBody, VMNativeGeneratorFunction, VMNull, VMTuple, VMVariableError
    }, gc::GCRef};
    #[test]
    fn test_xlang_compile_to_ir() {
        let code = r#"
        foo := (a?, b?) -> {
            return a + b
        };
        return foo(1, 2);
        "#;
        let dir_stack = xlang_frontend::dir_stack::DirStack::new(None);
        if let Err(e) = dir_stack {
            panic!("Failed to push directory: {}", e);
        }
        // Test the build_code function
        let mut dir_stack = dir_stack.unwrap();
        let ir_package = build_code(code, &mut dir_stack);
        match ir_package {
            Ok(ir_package) => {
                // Check if the IR package is not empty
                println!("IR Package: {:?}", ir_package);
            }
            Err(e) => {
                println!("Failed to build code: {}", e);
            }
        }
    }

    #[test]
    fn test_xlang_execute() {
        let code = r#"
        foo := (a?, b?) -> {
            return a + b
        };
        return foo(1, 2);
        "#;
        let dir_stack = xlang_frontend::dir_stack::DirStack::new(None);
        if let Err(e) = dir_stack {
            panic!("Failed to push directory: {}", e);
        }
        // Test the build_code function
        let mut dir_stack = dir_stack.unwrap();
        let ir_package = build_code(code, &mut dir_stack);
        match &ir_package {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to build code: {}", e);
                return;
            }
        }
        let ir_package = ir_package.unwrap();

        // transform the IR package to a runtime package
        let mut vm_instructions_package =
            xlang_vm_core::ir_translator::IRTranslator::new(&ir_package);
        match vm_instructions_package.translate() {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to translate IR package: {:?}", e);
                return;
            }
        }
        let vm_instructions_package = vm_instructions_package.get_result();

        // create a GC system
        let mut gc = xlang_vm_core::gc::GCSystem::new(None);

        // now create `() -> dyn vm_instructions` lambda and execute it
        let mut default_args_tuple = gc.new_object(
            VMTuple::new(&mut vec![]), // nothing to pass
        );
        let mut default_result = gc.new_object(VMNull::new());

        let mut lambda_body = gc.new_object(VMInstructions::new(&vm_instructions_package));

        let mut lambda = gc.new_object(VMLambda::new(
            0,
            "__main__".to_string(),
            &mut default_args_tuple,
            None,
            None,
            &mut VMLambdaBody::VMInstruction(lambda_body.clone()),
            &mut default_result,
        ));

        //drop used objects
        default_args_tuple.drop_ref();
        default_result.drop_ref();
        lambda_body.drop_ref();

        // create a coroutine pool
        let mut coroutine_pool = xlang_vm_core::executor::vm::VMCoroutinePool::new(true);

        // create a coroutine
        lambda.clone_ref(); // create a coroutine will drop the lambda, so we need to clone it before entering the coroutine
        let _ = coroutine_pool.new_coroutine(&mut lambda, &mut gc);

        // run until all coroutines are finished
        let result = coroutine_pool.run_until_finished(&mut gc);
        match result {
            Ok(_) => {
                // Check if the result is not empty
                println!(
                    "Result: {}",
                    try_repr_vmobject(lambda.as_const_type::<VMLambda>().result.clone(), None)
                        .unwrap_or_else(|e| e.to_string())
                );
            }
            Err(e) => {
                println!("Failed to execute code: {:?}", e);
            }
        }

        // drop the lambda
        lambda.drop_ref();

        // collect
        gc.collect();
    }

    #[test]
    fn test_xlang_custom_native_function() {
        let code = r#"
        @dynamic hello_from_rust("Hello from Rust!");
        "#;
        let dir_stack = xlang_frontend::dir_stack::DirStack::new(None);
        if let Err(e) = dir_stack {
            panic!("Failed to push directory: {}", e);
        }
        // Test the build_code function
        let mut dir_stack = dir_stack.unwrap();
        let ir_package = build_code(code, &mut dir_stack);
        match &ir_package {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to build code: {}", e);
                return;
            }
        }
        let ir_package = ir_package.unwrap();

        // transform the IR package to a runtime package
        let mut vm_instructions_package =
            xlang_vm_core::ir_translator::IRTranslator::new(&ir_package);
        match vm_instructions_package.translate() {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to translate IR package: {:?}", e);
                return;
            }
        }
        let vm_instructions_package = vm_instructions_package.get_result();

        // create a GC system
        let mut gc = xlang_vm_core::gc::GCSystem::new(None);

        // () -> dyn native_function
        let mut params = gc.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc.new_object(VMNull::new());
        let mut hello_from_rust = gc.new_object(VMLambda::new(
            0,
            "<builtins>::hello_from_rust".to_string(),
            &mut params,
            None,
            None,
            &mut VMLambdaBody::VMNativeFunction(|params_tuple, gc| {
                let repr = try_repr_vmobject(params_tuple.clone(), None)
                    .unwrap_or_else(|_| "<error>".to_string());
                println!("Hello from Rust! {}", repr);
                let result = gc.new_object(VMNull::new());
                return Ok(result);
            }),
            &mut result,
        ));
        params.drop_ref();
        result.drop_ref();

        // now create `() -> dyn vm_instructions` lambda and execute it
        let mut default_args_tuple = gc.new_object(
            VMTuple::new(&mut vec![]), // nothing to pass
        );
        let mut default_result = gc.new_object(VMNull::new());

        let mut lambda_body = gc.new_object(VMInstructions::new(&vm_instructions_package));

        let mut lambda = gc.new_object(VMLambda::new(
            0,
            "__main__".to_string(),
            &mut default_args_tuple,
            None,
            None,
            &mut VMLambdaBody::VMInstruction(lambda_body.clone()),
            &mut default_result,
        ));

        //drop used objects
        default_args_tuple.drop_ref();
        default_result.drop_ref();
        lambda_body.drop_ref();

        // create a coroutine pool
        let mut coroutine_pool = xlang_vm_core::executor::vm::VMCoroutinePool::new(true);

        // create a coroutine
        lambda.clone_ref(); // create a coroutine will drop the lambda, so we need to clone it before entering the coroutine
        let id = coroutine_pool.new_coroutine(&mut lambda, &mut gc);
        if id.is_err() {
            println!("Failed to create coroutine: {:?}", id.err());
            return;
        }
        let id = id.unwrap();

        // bind the native function to the coroutine
        let _ = coroutine_pool
            .get_executor_mut(id)
            .unwrap()
            .get_context_mut()
            .let_var("hello_from_rust", &mut hello_from_rust, &mut gc);
        hello_from_rust.drop_ref();

        // run until all coroutines are finished
        let result = coroutine_pool.run_until_finished(&mut gc);
        match result {
            Ok(_) => {
                // Check if the result is not empty
                println!(
                    "Result: {}",
                    try_repr_vmobject(lambda.as_const_type::<VMLambda>().result.clone(), None)
                        .unwrap_or_else(|e| e.to_string())
                );
            }
            Err(e) => {
                println!("Failed to execute code: {:?}", e);
            }
        }

        // drop the lambda
        lambda.drop_ref();

        // collect
        gc.collect();
        gc._print_reference_graph();
    }

    #[derive(Debug, Clone)]
    struct TestGenerator {
        steps_taken: i64,
        total_steps: i64,
        is_initialized: bool,
    }

    impl TestGenerator {
        fn new() -> Self {
            TestGenerator {
                steps_taken: 0,
                total_steps: 0,
                is_initialized: false,
            }
        }
    }

    impl VMNativeGeneratorFunction for TestGenerator {
        fn init(
            &mut self,
            arg: &mut xlang_vm_core::gc::GCRef,
            _gc_system: &mut xlang_vm_core::gc::GCSystem,
        ) -> Result<(), xlang_vm_core::executor::variable::VMVariableError> {
            if self.is_initialized {
                return Err(VMVariableError::DetailedError(
                    "Generator already initialized".to_string(),
                ));
            }

            if !arg.isinstance::<VMTuple>() {
                return Err(VMVariableError::TypeError(
                    arg.clone(),
                    "Expected a VMTuple argument for initialization".to_string(),
                ));
            }

            let arg_tuple = arg.as_const_type::<VMTuple>();

            if arg_tuple.values.is_empty() {
                return Err(VMVariableError::DetailedError(
                    "Initialization tuple is empty".to_string(),
                ));
            }

            let steps_arg = &arg_tuple.values[0];
            if !steps_arg.isinstance::<VMInt>() {
                return Err(VMVariableError::TypeError(
                steps_arg.clone(),
                "Expected an integer as the first element of the initialization tuple for total steps".to_string(),
            ));
            }

            let steps = steps_arg.as_const_type::<VMInt>().value;
            if steps < 0 {
                return Err(VMVariableError::DetailedError(
                    "Number of steps cannot be negative".to_string(),
                ));
            }

            self.total_steps = steps;
            self.is_initialized = true;
            Ok(())
        }

        fn step(
            &mut self,
            gc_system: &mut xlang_vm_core::gc::GCSystem,
        ) -> Result<GCRef, xlang_vm_core::executor::variable::VMVariableError> {
            if !self.is_initialized {
                return Err(VMVariableError::DetailedError(
                    "Generator not initialized".to_string(),
                ));
            }
            if self.is_done() {
                return Ok(gc_system.new_object(VMNull::new()));
            }
            println!("Executing step {}", self.steps_taken + 1);
            self.steps_taken += 1;

            Ok(gc_system.new_object(VMNull::new()))
        }

        fn is_done(&self) -> bool {
            self.is_initialized && self.steps_taken >= self.total_steps
        }

        fn clone_generator(&self) -> Arc<Box<dyn VMNativeGeneratorFunction>> {
            Arc::new(Box::new(self.clone()))
        }

        fn get_result(&mut self, gc_system: &mut xlang_vm_core::gc::GCSystem) -> Result<GCRef, VMVariableError> {
            return Ok(gc_system.new_object(VMNull::new()));
        }
    }

    #[test]
    fn test_xlang_custom_native_async_function() {
        let code = r#"
        n := 0;
        while (n = n + 1; n <= 100) {
            @dynamic async (copy custom_generator)(n * 10);
        }
        "#;
        let dir_stack = xlang_frontend::dir_stack::DirStack::new(None);
        if let Err(e) = dir_stack {
            panic!("Failed to push directory: {}", e);
        }
        // Test the build_code function
        let mut dir_stack = dir_stack.unwrap();
        let ir_package = build_code(code, &mut dir_stack);
        match &ir_package {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to build code: {}", e);
                return;
            }
        }
        let ir_package = ir_package.unwrap();

        // transform the IR package to a runtime package
        let mut vm_instructions_package =
            xlang_vm_core::ir_translator::IRTranslator::new(&ir_package);
        match vm_instructions_package.translate() {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to translate IR package: {:?}", e);
                return;
            }
        }
        let vm_instructions_package = vm_instructions_package.get_result();

        // create a GC system
        let mut gc = xlang_vm_core::gc::GCSystem::new(None);

        // () -> dyn native_function
        let mut params = gc.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc.new_object(VMNull::new());
        let mut custom_generator = gc.new_object(VMLambda::new(
            0,
            "<builtins>::custom_generator".to_string(),
            &mut params,
            None,
            None,
            &mut VMLambdaBody::VMNativeGeneratorFunction(Arc::new(Box::new(TestGenerator::new()))),
            &mut result,
        ));
        params.drop_ref();
        result.drop_ref();

        // now create `() -> dyn vm_instructions` lambda and execute it
        let mut default_args_tuple = gc.new_object(
            VMTuple::new(&mut vec![]), // nothing to pass
        );
        let mut default_result = gc.new_object(VMNull::new());

        let mut lambda_body = gc.new_object(VMInstructions::new(&vm_instructions_package));

        let mut lambda = gc.new_object(VMLambda::new(
            0,
            "__main__".to_string(),
            &mut default_args_tuple,
            None,
            None,
            &mut VMLambdaBody::VMInstruction(lambda_body.clone()),
            &mut default_result,
        ));

        //drop used objects
        default_args_tuple.drop_ref();
        default_result.drop_ref();
        lambda_body.drop_ref();

        // create a coroutine pool
        let mut coroutine_pool = xlang_vm_core::executor::vm::VMCoroutinePool::new(true);

        // create a coroutine
        lambda.clone_ref(); // create a coroutine will drop the lambda, so we need to clone it before entering the coroutine
        let id = coroutine_pool.new_coroutine(&mut lambda, &mut gc);
        if id.is_err() {
            println!("Failed to create coroutine: {:?}", id.err());
            return;
        }
        let id = id.unwrap();

        // bind the native function to the coroutine
        let _ = coroutine_pool
            .get_executor_mut(id)
            .unwrap()
            .get_context_mut()
            .let_var("custom_generator", &mut custom_generator, &mut gc);
        custom_generator.drop_ref();

        // run until all coroutines are finished
        let result = coroutine_pool.run_until_finished(&mut gc);
        match result {
            Ok(_) => {
                // Check if the result is not empty
                println!(
                    "Result: {}",
                    try_repr_vmobject(lambda.as_const_type::<VMLambda>().result.clone(), None)
                        .unwrap_or_else(|e| e.to_string())
                );
            }
            Err(e) => {
                println!("Failed to execute code: {}", e.to_string());
            }
        }

        // drop the lambda
        lambda.drop_ref();

        // collect
        gc.collect();

        gc._print_reference_graph();
    }
}
