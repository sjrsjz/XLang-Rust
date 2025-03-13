use super::variable::{VMVariableWrapper, VMInt, VMString, VMFloat, VMBoolean};
use super::super::gc::gc::{GCObject, GCRef, GCSystem};
use super::variable::VMObject;

#[test]
fn test_vm_int_operations() {
    let mut gc_system = GCSystem::new();
    
    // 创建整数对象
    let int_obj = gc_system.new_object(VMInt::new(42));
    
    // 测试直接访问值
    assert_eq!(int_obj.as_const_type::<VMInt>().value, 42);
    
    // 测试复制
    let int_copy = int_obj.as_const_type::<VMInt>().copy(&mut gc_system);
    assert_eq!(int_copy.as_const_type::<VMInt>().value, 42);
    
    // 测试赋值
    let mut mutable_int = VMInt::new(10);
    mutable_int.assgin(int_obj);
    assert_eq!(mutable_int.value, 42);
    
    // 测试从浮点数赋值
    let float_obj = gc_system.new_object(VMFloat::new(3.14));
    mutable_int.assgin(float_obj);
    assert_eq!(mutable_int.value, 3);
}

#[test]
fn test_vm_variable_wrapper() {
    let mut gc_system = GCSystem::new();
    
    // 创建基础值
    let int_obj = gc_system.new_object(VMInt::new(42));
    
    // 创建变量包装器
    let var_wrapper = gc_system.new_object(VMVariableWrapper::new(int_obj.clone()));
    
    // 检查变量包装器持有正确的值引用
    assert!(var_wrapper.as_const_type::<VMVariableWrapper>().value_ref.isinstance::<VMInt>());
    assert_eq!(
        var_wrapper.as_const_type::<VMVariableWrapper>().value_ref.as_const_type::<VMInt>().value,
        42
    );
    
    // 测试变量赋值
    let string_obj = gc_system.new_object(VMString::new("Hello".to_string()));
    {
        let mut var = var_wrapper.as_type::<VMVariableWrapper>();
        var.assgin(string_obj.clone());
    }
    
    // 验证变量现在持有的是字符串
    assert!(var_wrapper.as_const_type::<VMVariableWrapper>().value_ref.isinstance::<VMString>());
    assert_eq!(
        var_wrapper.as_const_type::<VMVariableWrapper>().value_ref.as_const_type::<VMString>().value,
        "Hello"
    );
}

#[test]
fn test_vm_types_conversion() {
    let mut gc_system = GCSystem::new();
    
    // 测试布尔值从整数转换
    let int_obj = gc_system.new_object(VMInt::new(1));
    let mut bool_obj = VMBoolean::new(false);
    bool_obj.assgin(int_obj);
    assert_eq!(bool_obj.value, true);
    
    let int_zero = gc_system.new_object(VMInt::new(0));
    bool_obj.assgin(int_zero);
    assert_eq!(bool_obj.value, false);
    
    // 测试整数从浮点数转换
    let float_obj = gc_system.new_object(VMFloat::new(3.99));
    let mut int_val = VMInt::new(0);
    int_val.assgin(float_obj);
    assert_eq!(int_val.value, 3); // 应该截断小数部分
    
    // 测试浮点数从整数转换
    let int_obj = gc_system.new_object(VMInt::new(5));
    let mut float_val = VMFloat::new(0.0);
    float_val.assgin(int_obj);
    assert_eq!(float_val.value, 5.0);
}

#[test]
fn test_complex_operations() {
    let mut gc_system = GCSystem::new();
    
    // 创建各种类型的值
    let int_obj = gc_system.new_object(VMInt::new(42));
    let float_obj = gc_system.new_object(VMFloat::new(3.14));
    let string_obj = gc_system.new_object(VMString::new("Hello".to_string()));
    let bool_obj = gc_system.new_object(VMBoolean::new(true));
    
    // 创建一个变量，并测试变量的赋值
    let var = gc_system.new_object(VMVariableWrapper::new(int_obj.clone()));
    
    {
        let mut var_mut = var.as_type::<VMVariableWrapper>();
        var_mut.assgin(float_obj.clone());
        assert!(var_mut.value_ref.isinstance::<VMFloat>());
        
        var_mut.assgin(string_obj.clone());
        assert!(var_mut.value_ref.isinstance::<VMString>());
        
        var_mut.assgin(bool_obj.clone());
        assert!(var_mut.value_ref.isinstance::<VMBoolean>());
    }
    
    // 测试复制操作
    let bool_copy = bool_obj.as_const_type::<VMBoolean>().copy(&mut gc_system);
    assert_eq!(bool_copy.as_const_type::<VMBoolean>().value, true);
    
    let string_copy = string_obj.as_const_type::<VMString>().copy(&mut gc_system);
    assert_eq!(string_copy.as_const_type::<VMString>().value, "Hello");
    
    // 通过变量包装器复制
    {
        var.as_type::<VMVariableWrapper>().assgin(int_obj.clone());
        
        // 当包装器包含整数时，应该能从变量正确地复制整数值
        let copy_result = var.as_const_type::<VMVariableWrapper>().copy(&mut gc_system);
        assert!(copy_result.isinstance::<VMInt>());
        assert_eq!(copy_result.as_const_type::<VMInt>().value, 42);
    }
}

#[test]
#[should_panic(expected = "Cannot wrap a variable as a variable")]
fn test_variable_wrapping_restriction() {
    let mut gc_system = GCSystem::new();
    
    // 创建一个变量
    let int_obj = gc_system.new_object(VMInt::new(42));
    let var = gc_system.new_object(VMVariableWrapper::new(int_obj));
    
    // 尝试用另一个变量包装这个变量，应该会失败
    let _double_wrapped = VMVariableWrapper::new(var);
}