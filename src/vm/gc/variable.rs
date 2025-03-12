use super::gc::{GCObject, GCTraceable};
use std::collections::HashMap;

// 已有的整数类型
#[derive(Debug)]
pub struct GCInteger {
    pub value: i64,
    traceable: GCTraceable,
}

impl GCInteger {
    pub fn new(value: i64) -> GCInteger {
        GCInteger {
            value,
            traceable: GCTraceable::new(None),
        }
    }
}

impl GCObject for GCInteger {
    fn free(&mut self) {
        // Free the object
        self.traceable.offline();
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }
}

// 浮点数类型
#[derive(Debug)]
pub struct GCFloat {
    pub value: f64,
    traceable: GCTraceable,
}

impl GCFloat {
    pub fn new(value: f64) -> GCFloat {
        GCFloat {
            value,
            traceable: GCTraceable::new(None),
        }
    }
}

impl GCObject for GCFloat {
    fn free(&mut self) {
        self.traceable.offline();
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }
}

// 字符串类型
#[derive(Debug)]
pub struct GCString {
    pub value: String,
    traceable: GCTraceable,
}

impl GCString {
    pub fn new(value: String) -> GCString {
        GCString {
            value,
            traceable: GCTraceable::new(None),
        }
    }
    
    pub fn from_str(value: &str) -> GCString {
        GCString::new(value.to_string())
    }
}

impl GCObject for GCString {
    fn free(&mut self) {
        // 字符串的内存会在结构体被删除时自动释放
        self.traceable.offline();
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }
}

// 布尔类型
#[derive(Debug)]
pub struct GCBool {
    pub value: bool,
    traceable: GCTraceable,
}

impl GCBool {
    pub fn new(value: bool) -> GCBool {
        GCBool {
            value,
            traceable: GCTraceable::new(None),
        }
    }
}

impl GCObject for GCBool {
    fn free(&mut self) {
        self.traceable.offline();
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }
}

// 数组类型，存储对其他GC对象的引用
#[derive(Debug)]
pub struct GCArray {
    pub elements: Vec<super::gc::GCRef>,
    traceable: GCTraceable,
}

impl GCArray {
    pub fn new() -> GCArray {
        GCArray {
            elements: Vec::new(),
            traceable: GCTraceable::new(None),
        }
    }
    
    pub fn with_capacity(capacity: usize) -> GCArray {
        GCArray {
            elements: Vec::with_capacity(capacity),
            traceable: GCTraceable::new(None),
        }
    }
    
    pub fn push(&mut self, element: super::gc::GCRef) {
        // 添加对元素的引用
        self.traceable.add_reference(&mut element.clone());
        self.elements.push(element);
    }
    
    pub fn pop(&mut self) -> Option<super::gc::GCRef> {
        if let Some(element) = self.elements.pop() {
            // 移除对元素的引用
            self.traceable.remove_reference(&element);
            Some(element)
        } else {
            None
        }
    }
    
    pub fn len(&self) -> usize {
        self.elements.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
    
    pub fn get(&self, index: usize) -> Option<&super::gc::GCRef> {
        self.elements.get(index)
    }
}

impl GCObject for GCArray {
    fn free(&mut self) {
        // 释放对所有元素的引用
        for element in &self.elements {
            self.traceable.remove_reference(element);
        }
        self.elements.clear();
        self.traceable.offline();
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }
}

// 字典类型，存储键值对
#[derive(Debug)]
pub struct GCDictionary {
    pub entries: HashMap<String, super::gc::GCRef>,
    traceable: GCTraceable,
}

impl GCDictionary {
    pub fn new() -> GCDictionary {
        GCDictionary {
            entries: HashMap::new(),
            traceable: GCTraceable::new(None),
        }
    }
    
    pub fn insert(&mut self, key: String, value: super::gc::GCRef) {
        // 如果已存在键，先移除对旧值的引用
        if let Some(old_value) = self.entries.get(&key) {
            self.traceable.remove_reference(old_value);
        }
        
        // 添加对新值的引用
        self.traceable.add_reference(&mut value.clone());
        self.entries.insert(key, value);
    }
    
    pub fn remove(&mut self, key: &str) -> Option<super::gc::GCRef> {
        if let Some(value) = self.entries.remove(key) {
            // 移除对值的引用
            self.traceable.remove_reference(&value);
            Some(value)
        } else {
            None
        }
    }
    
    pub fn get(&self, key: &str) -> Option<&super::gc::GCRef> {
        self.entries.get(key)
    }
    
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
    
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl GCObject for GCDictionary {
    fn free(&mut self) {
        // 释放对所有值的引用
        for (_, value) in &self.entries {
            self.traceable.remove_reference(value);
        }
        self.entries.clear();
        self.traceable.offline();
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }
}




// Null类型，表示空值
#[derive(Debug)]
pub struct GCNull {
    traceable: GCTraceable,
}

impl GCNull {
    pub fn new() -> Self {
        GCNull {
            traceable: GCTraceable::new(None),
        }
    }
}

impl super::gc::GCObject for GCNull {
    fn free(&mut self) {
        self.traceable.offline();
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }
}

#[derive(Debug)]
pub struct GCIndexOfWrapper {
    pub index: usize,
    traceable: GCTraceable,
}

impl GCIndexOfWrapper {
    pub fn new() -> Self {
        GCIndexOfWrapper {
            index: 0,
            traceable: GCTraceable::new(None),
        }
    }
}