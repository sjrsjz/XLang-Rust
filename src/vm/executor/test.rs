use crate::vm::gc;

use super::variable::{VMVariableWrapper, VMInt, VMString, VMFloat, VMBoolean, VMKeyVal, VMNull};
use super::super::gc::gc::{GCObject, GCRef, GCSystem};
use super::variable::VMObject;

#[test]
fn test_vm_int_operations() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建整数对象
    let int_obj = gc_system.new_object(VMInt::new(42));
    
    // 测试直接访问值
    assert_eq!(int_obj.as_const_type::<VMInt>().value, 42);
    
    // 测试复制
    let int_copy = int_obj.as_const_type::<VMInt>().copy(&mut gc_system).unwrap();
    assert_eq!(int_copy.as_const_type::<VMInt>().value, 42);
    
    // 测试赋值
    let mut mutable_int = VMInt::new(10);
    mutable_int.assign(int_obj);
    assert_eq!(mutable_int.value, 42);
    
    // 测试从浮点数赋值
    let float_obj = gc_system.new_object(VMFloat::new(3.14));
    mutable_int.assign(float_obj);
    assert_eq!(mutable_int.value, 3);
}



#[test]
fn test_vm_types_conversion() {
    let mut gc_system = GCSystem::new(None);
    
    // 测试布尔值从整数转换
    let int_obj = gc_system.new_object(VMInt::new(1));
    let mut bool_obj = VMBoolean::new(false);
    bool_obj.assign(int_obj);
    assert_eq!(bool_obj.value, true);
    
    let int_zero = gc_system.new_object(VMInt::new(0));
    bool_obj.assign(int_zero);
    assert_eq!(bool_obj.value, false);
    
    // 测试整数从浮点数转换
    let float_obj = gc_system.new_object(VMFloat::new(3.99));
    let mut int_val = VMInt::new(0);
    int_val.assign(float_obj);
    assert_eq!(int_val.value, 3); // 应该截断小数部分
    
    // 测试浮点数从整数转换
    let int_obj = gc_system.new_object(VMInt::new(5));
    let mut float_val = VMFloat::new(0.0);
    float_val.assign(int_obj);
    assert_eq!(float_val.value, 5.0);
}


#[test]
#[should_panic(expected = "Cannot wrap a variable as a variable")]
fn test_variable_wrapping_restriction() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建一个变量
    let int_obj = gc_system.new_object(VMInt::new(42));
    let var = gc_system.new_object(VMVariableWrapper::new(int_obj));
    
    // 尝试用另一个变量包装这个变量，应该会失败
    let _double_wrapped = VMVariableWrapper::new(var);
}


#[test]
fn test_keyval_creation_and_access() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建键值对元素
    let key = gc_system.new_object(VMString::new("name".to_string()));
    let value = gc_system.new_object(VMString::new("Alice".to_string()));
    
    let keyval = gc_system.new_object(VMKeyVal::new(key.clone(), value.clone()));
    
    // 测试访问键和值
    let returned_key = keyval.as_const_type::<VMKeyVal>().get_key();
    let returned_value = keyval.as_const_type::<VMKeyVal>().get_value();
    
    assert!(returned_key.isinstance::<VMString>());
    assert_eq!(
        returned_key.as_const_type::<VMString>().value,
        "name"
    );
    
    assert!(returned_value.isinstance::<VMString>());
    assert_eq!(
        returned_value.as_const_type::<VMString>().value,
        "Alice"
    );
}

#[test]
fn test_keyval_check_key() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建不同类型的键值对
    let int_key = gc_system.new_object(VMInt::new(42));
    let str_value = gc_system.new_object(VMString::new("test".to_string()));
    
    let keyval = gc_system.new_object(VMKeyVal::new(int_key.clone(), str_value));
    
    // 测试键匹配
    let matching_key = gc_system.new_object(VMInt::new(42));
    assert!(keyval.as_const_type::<VMKeyVal>().check_key(matching_key));
    
    // 测试键不匹配
    let non_matching_key = gc_system.new_object(VMInt::new(24));
    assert!(!keyval.as_const_type::<VMKeyVal>().check_key(non_matching_key));
    
    // 测试不同类型的键
    let string_key = gc_system.new_object(VMString::new("42".to_string()));
    assert!(!keyval.as_const_type::<VMKeyVal>().check_key(string_key));
}

#[test]
fn test_keyval_equality() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建两个相同的键值对
    let key1 = gc_system.new_object(VMString::new("id".to_string()));
    let value1 = gc_system.new_object(VMInt::new(1001));
    let keyval1 = gc_system.new_object(VMKeyVal::new(key1, value1));
    
    let key2 = gc_system.new_object(VMString::new("id".to_string()));
    let value2 = gc_system.new_object(VMInt::new(1001));
    let keyval2 = gc_system.new_object(VMKeyVal::new(key2, value2));
    
    // 测试相等性
    assert!(keyval1.as_const_type::<VMKeyVal>().eq(keyval2));
    
    // 创建一个键相同值不同的键值对
    let key3 = gc_system.new_object(VMString::new("id".to_string()));
    let value3 = gc_system.new_object(VMInt::new(1002)); // 不同的值
    let keyval3 = gc_system.new_object(VMKeyVal::new(key3, value3));
    
    // 测试不相等
    assert!(!keyval1.as_const_type::<VMKeyVal>().eq(keyval3));
    
    // 创建一个键不同值相同的键值对
    let key4 = gc_system.new_object(VMString::new("uuid".to_string())); // 不同的键
    let value4 = gc_system.new_object(VMInt::new(1001));
    let keyval4 = gc_system.new_object(VMKeyVal::new(key4, value4));
    
    // 测试不相等
    assert!(!keyval1.as_const_type::<VMKeyVal>().eq(keyval4));
}

#[test]
fn test_keyval_copy_and_assign() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建键值对
    let key = gc_system.new_object(VMString::new("key".to_string()));
    let value = gc_system.new_object(VMFloat::new(3.14));
    let keyval = gc_system.new_object(VMKeyVal::new(key, value));
    
    // 测试复制
    let copied = keyval.as_const_type::<VMKeyVal>().copy(&mut gc_system).unwrap();
    assert!(copied.isinstance::<VMKeyVal>());
    
    // 验证复制的键值对内容
    let copied_kv = copied.as_const_type::<VMKeyVal>();
    assert!(copied_kv.get_key().as_const_type::<VMString>().value == "key");
    assert!(copied_kv.get_value().as_const_type::<VMFloat>().value == 3.14);
    
    // 测试赋值
    let new_key = gc_system.new_object(VMString::new("updated".to_string()));
    let new_value = gc_system.new_object(VMBoolean::new(true));
    let new_keyval = gc_system.new_object(VMKeyVal::new(new_key, new_value));
    
    {
        let mutable_keyval = keyval.as_type::<VMKeyVal>();
        mutable_keyval.assign(new_keyval.as_const_type::<VMKeyVal>().get_value().clone());
        
        // 赋值只修改 value，不修改 key
        assert!(mutable_keyval.get_key().as_const_type::<VMString>().value == "key");
        assert!(mutable_keyval.get_value().isinstance::<VMBoolean>());
        assert!(mutable_keyval.get_value().as_const_type::<VMBoolean>().value == true);
    }
    gc_system.debug_print();
}

#[test]
fn test_keyval_with_nested_keyval() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建嵌套的键值对
    let inner_key = gc_system.new_object(VMString::new("inner_key".to_string()));
    let inner_value = gc_system.new_object(VMInt::new(42));
    let inner_keyval = gc_system.new_object(VMKeyVal::new(inner_key, inner_value));
    
    let outer_key = gc_system.new_object(VMString::new("outer_key".to_string()));
    let outer_keyval = gc_system.new_object(VMKeyVal::new(outer_key, inner_keyval.clone()));
    
    // 测试嵌套访问
    let retrieved_inner = outer_keyval.as_const_type::<VMKeyVal>().get_value();
    assert!(retrieved_inner.isinstance::<VMKeyVal>());
    
    let inner_value_ref = retrieved_inner.as_const_type::<VMKeyVal>().get_value();
    assert!(inner_value_ref.isinstance::<VMInt>());
    assert_eq!(inner_value_ref.as_const_type::<VMInt>().value, 42);
}

#[test]
fn test_keyval_with_null() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建带有 null 值的键值对
    let key = gc_system.new_object(VMString::new("maybe".to_string()));
    let null_value = gc_system.new_object(VMNull::new());
    let keyval = gc_system.new_object(VMKeyVal::new(key, null_value.clone()));
    
    // 测试值是否为 null
    let retrieved_value = keyval.as_const_type::<VMKeyVal>().get_value();
    assert!(retrieved_value.isinstance::<VMNull>());
    
    // 测试与另一个 null 值相等
    let another_null = gc_system.new_object(VMNull::new());
    assert!(retrieved_value.as_const_type::<VMNull>().eq(another_null));
}

#[test]
fn panic_because_hashset_ref(){
    let mut gc_system = GCSystem::new(None);
    let key = gc_system.new_object(VMString::new("key".to_string()));
    let keyval = gc_system.new_object(VMKeyVal::new(key.clone(), key.clone()));
    key.offline();
    gc_system.collect();
    keyval.offline();
    gc_system.collect(); // panic! if use HashSet<GCRef> in GCSystem
    println!("Pass");
}

#[test]
fn test_self_referential_objects() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建一个键值对
    let key = gc_system.new_object(VMString::new("self".to_string()));
    let value = gc_system.new_object(VMInt::new(42));
    let keyval = gc_system.new_object(VMKeyVal::new(key, value.clone()));
    
    // 创建自循环引用 - keyval引用自身作为值
    {
        let mutable_keyval = keyval.as_type::<VMKeyVal>();
        mutable_keyval.assign(keyval.clone());
    }
    
    // 验证现在keyval的value是它自己
    assert!(keyval.as_const_type::<VMKeyVal>().get_value().isinstance::<VMKeyVal>());

    
    // 强制GC
    keyval.offline();
    gc_system.collect();
    
    // 检查是否成功回收，不应该panic
    println!("自循环引用对象成功回收");
}

#[test]
fn test_cyclic_references() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建三个键值对，形成循环引用：A -> B -> C -> A
    let key_a = gc_system.new_object(VMString::new("a".to_string()));
    let key_b = gc_system.new_object(VMString::new("b".to_string()));
    let key_c = gc_system.new_object(VMString::new("c".to_string()));
    
    let keyval_a = gc_system.new_object(VMKeyVal::new(key_a, key_b.clone()));
    let keyval_b = gc_system.new_object(VMKeyVal::new(key_b, key_c.clone()));
    let keyval_c = gc_system.new_object(VMKeyVal::new(key_c, keyval_a.clone()));
    
    // 检查引用关系
    assert!(keyval_c.as_const_type::<VMKeyVal>().get_value().isinstance::<VMKeyVal>());
    
    // 使所有对象离线
    keyval_a.offline();
    keyval_b.offline();
    keyval_c.offline();
    
    // 尝试回收，不应该panic
    gc_system.collect();
    
    println!("循环引用链成功回收");
}

#[test]
fn test_same_reference_as_key_and_value() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建一个对象同时用作键和值
    let obj = gc_system.new_object(VMString::new("dual_purpose".to_string()));
    let keyval = gc_system.new_object(VMKeyVal::new(obj.clone(), obj.clone()));
        
    // 回收共享引用
    obj.offline();
    gc_system.collect();
    
    // 回收键值对
    keyval.offline();
    gc_system.collect();
    
    println!("同一引用作为键和值的对象成功回收");
}

#[test]
fn test_complex_reference_graph() {
    let mut gc_system = GCSystem::new(None);
    
    // 创建一些基础对象
    let str1 = gc_system.new_object(VMString::new("node1".to_string()));
    let str2 = gc_system.new_object(VMString::new("node2".to_string()));
    let num1 = gc_system.new_object(VMInt::new(1));
    let num2 = gc_system.new_object(VMInt::new(2));
    
    // 创建复杂引用图
    // kv1 -> (str1, num1)
    let kv1 = gc_system.new_object(VMKeyVal::new(str1.clone(), num1.clone()));
    
    // kv2 -> (str2, kv1)
    let kv2 = gc_system.new_object(VMKeyVal::new(str2.clone(), kv1.clone()));
    
    // kv3 -> (kv1, kv2) - 引用了前两个键值对
    let kv3 = gc_system.new_object(VMKeyVal::new(kv1.clone(), kv2.clone()));
    
    // 修改kv1使其引用kv3，创建循环: kv1 -> kv3 -> kv1
    {
        let mut mutable_kv1 = kv1.as_type::<VMKeyVal>();
        mutable_kv1.assign(kv3.clone());
    }
    
    // 验证我们创建了循环引用
    assert!(kv1.as_const_type::<VMKeyVal>().get_value().isinstance::<VMKeyVal>());
    assert!(kv1.as_const_type::<VMKeyVal>().get_value().as_const_type::<VMKeyVal>().get_key().isinstance::<VMKeyVal>());
    
    // 使所有对象离线
    str1.offline();
    str2.offline();
    num1.offline();
    num2.offline();
    kv1.offline();
    kv2.offline();
    kv3.offline();
    
    // 执行回收
    gc_system.collect();
    
    println!("复杂引用图成功回收");
}


#[test]
fn test_gc_stress_test() {
    use std::time::{Duration, Instant};
    use rand::{Rng, thread_rng, seq::SliceRandom};
    
    // 配置测试参数
    const TEST_DURATION_SECS: u64 = 10; // 测试持续时间(秒)
    const MAX_OBJECTS: usize = 5000;   // 最大对象数量
    const GC_INTERVAL: usize = 500;    // 每创建多少对象执行一次GC
    const OFFLINE_RATIO: f64 = 0.7;    // 每轮使多少比例的对象离线
    
    println!("开始GC压力测试，持续{}秒...", TEST_DURATION_SECS);
    
    let mut gc_system = GCSystem::new(None);
    let mut rng = thread_rng();
    let mut cycle_count = 0;
    let mut objects_created = 0;
    let mut objects_collected = 0;
    
    let start_time = Instant::now();
    
    while start_time.elapsed() < Duration::from_secs(TEST_DURATION_SECS) {
        cycle_count += 1;
        let mut all_objects: Vec<GCRef> = Vec::with_capacity(MAX_OBJECTS);        
        // 阶段1: 创建对象并建立引用关系
        for _ in 0..GC_INTERVAL {
            if all_objects.len() >= MAX_OBJECTS {
                break;
            }
            
            // 随机创建不同类型的对象
            let obj: GCRef = match rng.gen_range(0..5) {
                0 => gc_system.new_object(VMInt::new(rng.gen())),
                1 => gc_system.new_object(VMString::new(format!("str_{}", rng.gen::<u32>()))),
                2 => gc_system.new_object(VMFloat::new(rng.gen())),
                3 => gc_system.new_object(VMBoolean::new(rng.gen())),
                _ => {
                    if false {
                        // 创建KeyVal并引用已有对象
                        let idx1 = rng.gen_range(0..all_objects.len());
                        let idx2 = rng.gen_range(0..all_objects.len());
                        gc_system.new_object(VMKeyVal::new(
                            all_objects[idx1].clone(), 
                            all_objects[idx2].clone()
                        ))
                    } else {
                        // 创建KeyVal引用新对象
                        let key = gc_system.new_object(VMInt::new(rng.gen()));
                        let val = gc_system.new_object(VMString::new(format!("val_{}", rng.gen::<u32>())));
                        all_objects.push(key.clone());
                        all_objects.push(val.clone());
                        objects_created += 2;
                        gc_system.new_object(VMKeyVal::new(key, val))
                    }
                }
            };
            
            all_objects.push(obj);
            objects_created += 1;
            
            // // 随机创建循环引用
            // if rng.gen_bool(0.05) && all_objects.len() >= 3 {
            //     // 选择一对KeyVal对象创建循环
            //     let mut keyval_indices = Vec::new();
            //     for (i, obj) in all_objects.iter().enumerate() {
            //         if obj.isinstance::<VMKeyVal>() {
            //             keyval_indices.push(i);
            //             if keyval_indices.len() >= 2 {
            //                 break;
            //             }
            //         }
            //     }
                
            //     if keyval_indices.len() >= 2 {
            //         let idx1 = keyval_indices[0];
            //         let idx2 = keyval_indices[1];
                    
            //         // 创建循环: kv1 -> kv2 -> kv1
            //         if rng.gen_bool(0.5) {
            //             let kv1 = all_objects[idx1].clone();
            //             let kv2 = all_objects[idx2].clone();
                        
                        {
                            let mut mutable_kv1 = kv1.as_type::<VMKeyVal>();
                            mutable_kv1.assign(kv2.clone());
                        }
                        {
                            let mut mutable_kv2 = kv2.as_type::<VMKeyVal>();
                            mutable_kv2.assign(kv1.clone());
                        }
                    }
                }
            }
        }
        
        // 阶段2: 使一部分对象离线
        let offline_count = (all_objects.len() as f64 * OFFLINE_RATIO) as usize;
        let mut offline_indices: Vec<usize> = (0..all_objects.len()).collect();
        offline_indices.shuffle(&mut rng);
        
        for &idx in offline_indices.iter().take(offline_count) {
            all_objects[idx].offline();
        }
        
        // 阶段3: 执行垃圾回收
        //gc_system.collect();
        
        // 更新统计信息并清理列表
        objects_collected += offline_count;
        all_objects.retain(|obj| {
            unsafe {
                (*obj.get_reference()).get_traceable().online
            }
        });
        
        // 状态报告
        if cycle_count % 10 == 0 {
            println!("循环 {}: 创建了 {} 个对象，回收了 {} 个对象，当前存活 {} 个",
                     cycle_count, objects_created, objects_collected, gc_system.count());
        }
    }
    
    gc_system.drop_all();
    let elapsed = start_time.elapsed();
    println!("GC压力测试完成! 耗时: {:.2}秒", elapsed.as_secs_f64());
    println!("总计创建对象: {}", objects_created);
    println!("总计垃圾回收循环: {}", cycle_count);
    println!("每秒创建对象: {:.2}", objects_created as f64 / elapsed.as_secs_f64());
}

#[test]
fn test_gc_circular_reference_stress() {
    use std::time::{Duration, Instant};
    use rand::{Rng, thread_rng};
    
    // 专门测试循环引用
    const CHAIN_LENGTH: usize = 100;   // 循环引用链长度
    const CHAINS_COUNT: usize = 500;    // 创建多少个循环引用链
    const ITERATIONS: usize = 5;       // 重复测试次数
    
    println!("开始循环引用压力测试...");
    println!("每次迭代创建 {} 个长度为 {} 的循环引用链", CHAINS_COUNT, CHAIN_LENGTH);
    
    let start_time = Instant::now();
    let mut rng = thread_rng();
    
    for iter in 0..ITERATIONS {
        let mut gc_system = GCSystem::new(None);
        
        // 创建多个循环引用链
        for chain in 0..CHAINS_COUNT {
            let mut chain_objects:Vec<GCRef> = Vec::with_capacity(CHAIN_LENGTH);
            
            // 为链创建键值对对象
            for i in 0..CHAIN_LENGTH {
                let key = gc_system.new_object(VMString::new(format!("chain{}_node{}", chain, i)));
                let value = if i == 0 {
                    // 第一个节点暂时使用简单值
                    gc_system.new_object(VMInt::new(i as i64))
                } else {
                    // 其他节点引用前一个节点
                    chain_objects[i-1].clone()
                };
                
                let keyval = gc_system.new_object(VMKeyVal::new(key, value));
                chain_objects.push(keyval);
            }
            
            // 闭合循环: 让第一个节点引用最后一个节点
            {
                let first = chain_objects[0].clone();
                let last = chain_objects[CHAIN_LENGTH-1].clone();
                let mut mutable_first = first.as_type::<VMKeyVal>();
                mutable_first.assign(last.clone());
            }
            
            // 随机中断一些链以测试部分回收
            if rng.gen_bool(0.3) {
                let break_point = rng.gen_range(0..CHAIN_LENGTH);
                chain_objects[break_point].offline();
            }
        }
        
        // 执行垃圾回收
        gc_system.collect();
        println!("迭代 {}/{}: 成功创建并回收循环引用链", iter+1, ITERATIONS);
    }
    
    let elapsed = start_time.elapsed();
    println!("循环引用压力测试完成! 耗时: {:.2}秒", elapsed.as_secs_f64());
    println!("总共处理了 {} 个循环引用对象", CHAINS_COUNT * CHAIN_LENGTH * ITERATIONS);
}