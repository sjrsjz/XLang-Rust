use super::gc::*;
use super::variable::*;

#[test]
fn test_gc_basic_allocation() {
    // 创建GC系统
    let mut gc = GCSystem::new();
    
    // 分配不同类型的对象
    let int_ref = gc.new_object(GCInteger::new(42));
    let float_ref = gc.new_object(GCFloat::new(3.14));
    let string_ref = gc.new_object(GCString::from_str("Hello, GC!"));
    let bool_ref = gc.new_object(GCBool::new(true));
    
    // 验证对象的值
    assert_eq!(int_ref.as_type::<GCInteger>().value, 42);
    assert_eq!(float_ref.as_type::<GCFloat>().value, 3.14);
    assert_eq!(bool_ref.as_type::<GCBool>().value, true);
    assert_eq!(string_ref.as_type::<GCString>().value, "Hello, GC!");
    
    // 对象应该处于在线状态，不应该被回收
    assert!(int_ref.get_traceable().online);
    assert!(float_ref.get_traceable().online);
    assert!(bool_ref.get_traceable().online);
    assert!(string_ref.get_traceable().online);
}

#[test]
fn test_gc_collection() {
    // 创建GC系统
    let mut gc = GCSystem::new();
    
    // 分配一些对象
    let int_ref = gc.new_object(GCInteger::new(42));
    let float_ref = gc.new_object(GCFloat::new(3.14));
    
    // 将对象设为离线状态
    int_ref.offline();
    
    // 执行垃圾回收
    gc.collect();
    
    // int_ref 应该已被回收，float_ref 仍然存在
    assert_eq!(gc.objects.len(), 1);
    assert_eq!(float_ref.as_type::<GCFloat>().value, 3.14);
}

#[test]
fn test_gc_references() {
    // 创建GC系统
    let mut gc = GCSystem::new();
    
    // 创建一个数组，其中包含一些对象
    let array = GCArray::new();
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
    
    // 执行垃圾回收
    gc.collect();
    
    // 验证对象仍然存在，因为它们被数组引用
    assert_eq!(gc.objects.len(), 3);
    
    // 使数组离线，这应该导致所有对象都被回收
    array_ref.offline();
    gc.collect();
    
    // 验证所有对象都被回收
    assert_eq!(gc.objects.len(), 0);
}

#[test]
fn test_gc_cyclic_references() {
    // 创建GC系统
    let mut gc = GCSystem::new();
    
    // 创建两个字典对象，它们相互引用
    let dict1 = GCDictionary::new();
    let dict2 = GCDictionary::new();
    
    let dict1_ref = gc.new_object(dict1);
    let dict2_ref = gc.new_object(dict2);
    
    // 建立循环引用
    dict1_ref.as_type::<GCDictionary>().insert("ref".to_string(), dict2_ref.clone());
    dict2_ref.as_type::<GCDictionary>().insert("ref".to_string(), dict1_ref.clone());
    
    // 验证引用
    assert_eq!(dict1_ref.get_traceable().references.len(), 1);
    assert_eq!(dict2_ref.get_traceable().references.len(), 1);
    
    // 将两个字典都设置为离线
    dict1_ref.offline();
    dict2_ref.offline();
    
    // 执行垃圾回收
    gc.collect();
    
    // 验证循环引用的对象都被回收
    assert_eq!(gc.objects.len(), 0);
}

#[test]
fn test_gc_complex_structure() {
    // 创建GC系统
    let mut gc = GCSystem::new();
    
    // 创建一个包含多种类型对象的复杂结构
    let array_ref = gc.new_object(GCArray::new());
    let dict_ref = gc.new_object(GCDictionary::new());
    let int_ref = gc.new_object(GCInteger::new(42));
    let string_ref = gc.new_object(GCString::from_str("Hello, Complex GC!"));
    
    // 构建对象关系
    array_ref.as_type::<GCArray>().push(int_ref.clone());
    array_ref.as_type::<GCArray>().push(string_ref.clone());
    dict_ref.as_type::<GCDictionary>().insert("array".to_string(), array_ref.clone());
    dict_ref.as_type::<GCDictionary>().insert("int".to_string(), int_ref.clone());
    gc.debug_print();
    // 验证引用关系
    assert_eq!(array_ref.get_traceable().references.len(), 2);
    assert_eq!(dict_ref.get_traceable().references.len(), 2);
    
    // 将字典设置为离线状态
    dict_ref.offline();

    int_ref.offline();
    string_ref.offline();
    
    // 执行垃圾回收
    gc.collect();
    
    // 字典引用了数组和整数，而数组引用了整数和字符串，整数和字符串都被离线了
    // 因此整数和字符串不应该被回收
    assert_eq!(gc.objects.len(), 3);
    
    // 将数组也设置为离线状态
    array_ref.offline();
    
    // 再次执行垃圾回收
    gc.collect();
    
    // 验证所有对象都被收集
    assert_eq!(gc.objects.len(), 0);
}



// 测试内存泄漏情况
#[test]
fn test_gc_memory_leak_prevention() {
    // 创建GC系统
    let mut gc = GCSystem::new();
    
    // 创建一个复杂的对象网络，包含循环引用
    let array1_ref = gc.new_object(GCArray::new());
    let array2_ref = gc.new_object(GCArray::new());
    let dict_ref = gc.new_object(GCDictionary::new());
    
    // 建立循环引用：array1 -> array2 -> dict -> array1
    array1_ref.as_type::<GCArray>().push(array2_ref.clone());
    array2_ref.as_type::<GCArray>().push(dict_ref.clone());
    dict_ref.as_type::<GCDictionary>().insert("array".to_string(), array1_ref.clone());
    
    // 将所有对象设置为离线
    array1_ref.offline();
    array2_ref.offline();
    dict_ref.offline();
    
    // 执行垃圾回收
    gc.collect();
    
    // 验证所有对象都被回收，即使它们互相引用
    assert_eq!(gc.objects.len(), 0);
}