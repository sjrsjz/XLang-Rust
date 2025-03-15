mod parser;
pub mod vm;
use vm::executor::variable::VMInstructions;
use vm::executor::variable::VMLambda;
use vm::executor::variable::VMTuple;
use vm::ir::IR;

use self::parser::ast::build_ast;
use self::parser::ast::ast_token_stream;
use self::parser::lexer::{lexer, Token, TokenType};

use self::vm::gc::gc::GCSystem;

use self::vm::ir_generator::ir_generator;
use self::vm::ir::Functions;
use self::vm::executor::vm::*;
use self::vm::executor::variable::*;


fn main() {

    let code = r#"
lazy := (computation => null) -> {
    result := null;
    evaluated := false;
    
    return (evaluated => evaluated,
            result => wrap result, // wrap because we don't know the type of result
            computation => computation) -> {
        if (evaluated == false) {
            result = computation();
            evaluated = true;
        };
        return valueof result;
    };
};

expensiveComputation := lazy(() -> {
    print("Computing...");
    return 42;
});

print(expensiveComputation());

print(expensiveComputation()); 


"#;
    let code = r#"

    /*createCounter := (start => 0) -> {
        return (count => start) -> {
            count = count + 1;
            return count;
        };
    };
    counter := createCounter(0);
    print(counter());
    print(counter());
    print(counter());

    arr := (1, 2, 3, 4, 5);
    print(arr);
    arr[0] = 100;
    print(arr);

    kv:= "key" : arr;
    print(kv);
    print(keyof kv);
    print(valueof kv);



    mutistr := (str => "", n => 0) -> {
        result := "";

        i := 0; while (i = i + 1; i <= n) {
            result = result + str;
        };

        return result;
    };
    print(mutistr("a", 5)); 


    loop := (func => (n => 0) -> {return false}) -> {
        return (n => 0, func => func) -> {
            while (func(n)) {
                n = n + 1;
            };
        };
    };

    loop_func := loop((n => 0) -> {
        print(n);
        return n < 50;
    });

    loop_func();

    iter := (container => ('T' : null), n => 0) -> {
        n = n + 1;
        E := valueof container;
        T := keyof container;
        if (n <= len(T)) {
            valueof E = T[n - 1];
            return true;
        } else {
            return false;
        };
    };

    arr := (1, 2, 3, 4, 5);
    elem := 0;
    while (iter(arr: wrap(elem))) {
        print(elem);
    };
    

    lambda := (A => 1, B => 2) -> print(A+B);
    lambda();*/

    classA := (
        "value": 0,
        inc => ()->{
            self.value = self.value + 1;
        }
    );
    classA.inc();
    print(classA.value);

    
"#;
    let tokens = lexer::reject_comment(lexer::tokenize(code));
    for token in &tokens {
        print!("{:?} ", token.to_string());
    }

    let gathered = ast_token_stream::from_stream(&tokens);
    let ast = build_ast(&gathered);
    if ast.is_err() {
        println!("");
        let err_token = ast.err().unwrap();
        println!("Error token: {:?}", err_token.format(&tokens));
        return;
    }
    println!("\n\nAST:\n");

    ast.as_ref().unwrap().formatted_print(0);

    let namespace = ir_generator::NameSpace::new("Main".to_string(), None);
    let mut functions = Functions::new();
    let mut ir_generator = ir_generator::IRGenerator::new(&mut functions, namespace);

    let ir = ir_generator.generate(&ast.unwrap());
    if ir.is_err() {
        println!("Error: {:?}", ir.err().unwrap());
        return;
    }
    let mut ir = ir.unwrap();
    ir.push(IR::Return);
    functions.append("__main__".to_string(), ir);
    
    let (built_ins, ips) = functions.build_instructions();
    println!("\n\nBuilt Ins:\n");
    for ir in &built_ins {
        println!("{:?}", ir);
    }
    println!("\n\nFunction IPs:\n");
    for (name, ip) in &ips {
        println!("{}: {:?}", name, ip);
    }

    let mut executor = IRExecutor::new(Some(code.to_string()));
    let mut gc_system = GCSystem::new(None);

    let default_args_tuple = gc_system.new_object(VMTuple::new(vec![]));
    let lambda_instructions = gc_system.new_object(VMInstructions::new(built_ins, ips));
    
    let main_lambda = gc_system.new_object(VMLambda::new(
        0,
        "__main__".to_string(),
        default_args_tuple.clone(),
        None,
        lambda_instructions.clone(),
    ));
    default_args_tuple.offline();
    lambda_instructions.offline();

    println!("\n[Running main lambda...]\n");

    let result = executor.execute(main_lambda, &mut gc_system);

    println!("\n[Finished running main lambda]\n");

    if result.is_err() {
        println!("Error: {}", result.err().unwrap().to_string());
        return;
    }
    let result = result.unwrap();
    let repr = try_repr_vmobject(result.clone());
    if repr.is_ok(){
        println!("Result: {}", repr.unwrap());
    } else {
        println!("Error: {}", repr.err().unwrap().to_string());
    }
    print!("\n\nResult GCRef: {:?}\n", result);
    result.offline();
    gc_system.collect();
    println!("Existing GCRef: {:?}", gc_system.count());
    gc_system.print_reference_graph();


}
