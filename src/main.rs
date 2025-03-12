mod parser;
pub mod vm;
use self::parser::ast::build_ast;
use self::parser::ast::ASTTokenStream;
use self::parser::lexer::{lexer, Token, TokenType};

use self::vm::gc::gc::{GCObject, GCSystem, GCTraceable};
use self::vm::gc::variable::{
    GCArray, GCBool, GCDictionary, GCFloat, GCFunction, GCInteger, GCNull, GCString,
};

fn test_gc_references() {
    // 创建GC系统
    let mut gc = GCSystem::new();

    // 创建一个数组，其中包含一些对象
    let mut array = GCArray::new();
    let array_ref = gc.new_object(array);

    // 创建一些对象并将其添加到数组中
    let int_ref = gc.new_object(GCInteger::new(42));
    let string_ref = gc.new_object(GCString::from_str("Hello, GC!"));

    // 添加对象到数组
    array_ref.as_type::<GCArray>().push(int_ref.clone());
    array_ref.as_type::<GCArray>().push(string_ref.clone());

    // 验证引用关系
    assert_eq!(array_ref.get_traceable().references.len(), 2);
    assert!(array_ref.get_traceable().references.contains(&int_ref));
    assert!(array_ref.get_traceable().references.contains(&string_ref));

    // 验证引用计数
    assert_eq!(int_ref.get_traceable().ref_count, 1);
    assert_eq!(string_ref.get_traceable().ref_count, 1);

    // 将原始引用设置为离线状态，但它们仍然被数组引用
    int_ref.offline();
    string_ref.offline();
    gc.debug_print();
    // 执行垃圾回收
    gc.collect();
    gc.debug_print();
    // 验证对象仍然存在，因为它们被数组引用
    assert_eq!(gc.objects.len(), 3);

    // 使数组离线，这应该导致所有对象都被回收
    array_ref.offline();
    gc.collect();

    // 验证所有对象都被回收
    assert_eq!(gc.objects.len(), 0);
}

fn gc_test() {
    let mut gc = GCSystem::new();
    let mut A_i32 = gc.new_object(GCInteger::new(10));
    let mut B_i32 = gc.new_object(GCInteger::new(20));
    let mut C_i32 = gc.new_object(GCInteger::new(30));

    println!("Before offline");
    gc.debug_print();

    A_i32.offline();

    println!("After offline");
    gc.debug_print();

    gc.collect();

    println!("After collect");
    gc.debug_print();

    println!("B: {:?}", B_i32.as_type::<GCInteger>().value);
    println!("C: {:?}", C_i32.as_type::<GCInteger>().value);

    B_i32.offline();
    C_i32.offline();

    println!("After offline All");
    gc.debug_print();

    gc.collect();

    println!("After collect All");
    gc.debug_print();
}

fn main() {
    test_gc_references();
    return;

    let code = r#"


factorial := Z((f => null) -> {
    return (n => 0, f => f) -> {
        if (n <= 1) {
            return 1;
        } else {
            return n * f(n - 1);
        };
    };
});

"#;
    let tokens = lexer::reject_comment(lexer::tokenize(code));
    for token in &tokens {
        print!("{:?} ", token.to_string());
    }

    let gathered = ASTTokenStream::from_stream(&tokens);
    let ast = build_ast(&gathered);
    if ast.is_err() {
        println!("");
        let err_token = ast.err().unwrap();
        println!("Error token: {:?}", err_token.format(&tokens));
        return;
    }
    println!("\n\nAST:\n");

    ast.unwrap().formatted_print(0);
}
