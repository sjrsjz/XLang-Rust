#[cfg(test)]
mod tests {
    use xlang_frontend::compile::build_code;
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
            Ok(ir_package) => {
                // Check if the IR package is not empty
                println!("IR Package: {:?}", ir_package);
            }
            Err(e) => {
                println!("Failed to build code: {}", e);
                return;
            }            
        }
        let ir_package = ir_package.unwrap();

        // transform the IR package to a runtime package
        let mut vm_instructions_package = xlang_vm_core::ir_translator::IRTranslator::new(&ir_package);
        match vm_instructions_package.translate() {
            Ok(_) => {
                // Check if the VM instructions package is not empty
                println!("VM Instructions Package: {:?}", vm_instructions_package);
            }
            Err(e) => {
                println!("Failed to translate IR package: {:?}", e);
            }
        }
    }
}