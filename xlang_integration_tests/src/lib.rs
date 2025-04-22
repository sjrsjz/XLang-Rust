#[cfg(test)]
mod tests {
    use xlang_frontend::compile::build_code;
    use xlang_vm_core::executor::variable::{
        VMInstructions, VMLambda, VMLambdaBody, VMNull, VMTuple, try_repr_vmobject,
    };
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
    }
}
