use std::collections::{HashSet, HashMap};
//typeid
use std::any::TypeId;
use std::hash::{Hash, Hasher};

pub trait GCObject {
    fn free(&mut self); // free the object
    fn get_traceable(&mut self) -> &mut GCTraceable; // get the traceable object
}

#[derive(Debug, Clone)]
pub struct GCRef {
    pub(self) reference: *mut dyn GCObject, // reference to the object
    pub(self) type_id: TypeId,              // type id of the object
}
impl PartialEq for GCRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self.reference, other.reference)
    }
}

impl Eq for GCRef {}

impl Hash for GCRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.reference as *const () as usize).hash(state);
        self.type_id.hash(state);
    }
}

impl GCRef {
    pub fn get_reference(&self) -> *mut dyn GCObject {
        self.reference
    }
    pub fn get_traceable(&self) -> &mut GCTraceable {
        unsafe {
            let obj = self.reference;
            (*obj).get_traceable()
        }
    }

    pub(self) fn free(&self) {
        unsafe {
            let obj = self.reference as *mut dyn GCObject;
            (*obj).free();
            let _ = Box::from_raw(self.reference);
        }
    }

    pub fn offline(&self) {
        unsafe {
            let obj = self.reference as *mut dyn GCObject;
            (*obj).get_traceable().offline();
        }
    }

    pub fn as_type<T>(&self) -> &mut T
    where
        T: GCObject + 'static,
    {
        if !self.isinstance::<T>() {
            panic!("Type mismatch! Expected type: {:?}", TypeId::of::<T>());
        }
        unsafe {
            let obj = self.reference as *mut T;
            &mut *obj
        }
    }

    pub fn as_const_type<T>(&self) -> &T
    where
        T: GCObject + 'static,
    {
        if !self.isinstance::<T>() {
            panic!("Type mismatch! Expected type: {:?}", TypeId::of::<T>());
        }
        unsafe {
            let obj = self.reference as *const T;
            &*obj
        }
    }

    pub fn isinstance<T: GCObject + 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    pub fn wrap<T: GCObject + 'static>(obj: &T) -> GCRef {
        GCRef {
            reference: obj as *const T as *mut T as *mut dyn GCObject,
            type_id: TypeId::of::<T>(),
        }
    }
}

#[derive(Debug)]
pub struct GCTraceable {
    pub ref_count: usize,
    pub should_free: bool,
    pub online: bool,
    // 替换HashSet为HashMap，以便追踪引用计数
    pub references: HashMap<GCRef, usize>,
}

impl GCTraceable {
    pub fn new(references: Option<Vec<GCRef>>) -> GCTraceable {
        let mut refs_map = HashMap::new();
        
        if let Some(refs) = references {
            for ref_obj in refs {
                *refs_map.entry(ref_obj.clone()).or_insert(0) += 1;
            }
        }
        
        let obj = GCTraceable {
            ref_count: 0,
            should_free: false,
            online: true,
            references: refs_map,
        };
        
        // 更新每个引用对象的引用计数
        for (ref_obj, count) in &obj.references {
            unsafe {
                (*ref_obj.reference).get_traceable().ref_count += count; // 增加对象的引用计数
            }
        }
        obj
    }

    pub fn offline(&mut self) {
        // 设置对象离线，以便它可以被回收
        self.online = false;
    }

    pub fn add_reference(&mut self, obj: &mut GCRef) {
        // 增加引用计数
        *self.references.entry(obj.clone()).or_insert(0) += 1;
        unsafe {
            (*obj.reference).get_traceable().ref_count += 1; // 增加被引用对象的引用计数
        }
    }

    pub fn remove_reference(&mut self, obj: &GCRef) {
        let count = self.references.get(obj).cloned();
        
        match count {
            None => panic!("Reference does not exist! existing references: {:?}, but got {:?}", self.references, obj),
            Some(1) => {
                // 最后一个引用，从HashMap中移除
                self.references.remove(obj);
            },
            Some(c) => {
                // 减少引用计数
                self.references.insert(obj.clone(), c - 1);
            }
        }
        
        unsafe {
            if (*obj.reference).get_traceable().ref_count == 0 {
                panic!("Reference count is already zero!");
            }
            (*obj.reference).get_traceable().ref_count -= 1; // 减少被引用对象的引用计数
        }
    }
}

impl Drop for GCTraceable {
    fn drop(&mut self) {
        let total_refs: usize = self.references.values().sum();
        if total_refs > 0 {
            eprintln!("[GC警告] 对象被销毁时仍有非零引用! 引用计数: {}, 引用详情: {:?}", 
                     total_refs, self.references);
            
            // 自动清理引用
            for (ref_obj, count) in std::mem::take(&mut self.references) {
                unsafe {
                    // 安全检查
                    if !ref_obj.reference.is_null() {
                        let traceable = (*ref_obj.reference).get_traceable();
                        if traceable.ref_count >= count {
                            traceable.ref_count -= count;
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct GCSystem {
    objects: Vec<GCRef>,
}

impl GCSystem {
    pub fn new() -> GCSystem {
        GCSystem {
            objects: Vec::new(),
        }
    }

    pub fn new_object<T: GCObject + 'static>(&mut self, object: T) -> GCRef {
        let obj_ref = Box::leak(Box::new(object)) as *mut dyn GCObject;
        let gc_ref = GCRef {
            reference: obj_ref,
            type_id: TypeId::of::<T>(),
        };
        self.objects.push(gc_ref.clone()); // add the object to the list of objects
        gc_ref
    }

    fn mark(&mut self) {
        let mut alive = Vec::<bool>::new();
        alive.resize(self.objects.len(), false);

        // 第一步：标记所有在线对象为活跃
        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            if gc_ref.get_traceable().should_free {
                let obj = gc_ref.get_traceable();
                panic!("Never set should_free to true! Use offline() instead! Object: {:?}", obj);
            } else if gc_ref.get_traceable().online {
                alive[i] = true;
            }
        }

        // 创建索引映射
        let mut idx_map = std::collections::HashMap::new();
        for (i, obj_ptr) in self.objects.iter().enumerate() {
            idx_map.insert(obj_ptr.reference, i);
        }

        // 第二步：构建引用图
        let mut ref_graph: Vec<Vec<usize>> = vec![Vec::new(); self.objects.len()];
        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            for (ref_obj,_) in &gc_ref.get_traceable().references {
                if let Some(&ref_idx) = idx_map.get(&ref_obj.reference) {
                    ref_graph[i].push(ref_idx);
                }
            }
        }

        // 第三步：从活跃对象出发，标记所有可达对象
        let mut worklist: Vec<usize> = Vec::new();

        // 初始化工作列表为所有活跃对象
        for i in 0..self.objects.len() {
            if alive[i] {
                worklist.push(i);
            }
        }

        // 标记从活跃对象可达的所有对象
        while let Some(idx) = worklist.pop() {
            // 遍历引用当前对象的所有对象
            for &ref_idx in &ref_graph[idx] {
                if !alive[ref_idx] {
                    alive[ref_idx] = true;
                    worklist.push(ref_idx);
                }
            }
        }

        // 第四步：更新should_free标志
        for i in 0..self.objects.len() {
            if !alive[i] {
                let gc_ref = &self.objects[i];
                gc_ref.get_traceable().should_free = true;
            }
        }
    }

    fn sweep(&mut self) {
        let mut alive = Vec::<bool>::new(); // 可达对象
        alive.resize(self.objects.len(), false);

        // 确定哪些对象是活跃的
        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            if gc_ref.get_traceable().online || !gc_ref.get_traceable().should_free {
                alive[i] = true;
            }
        }

        // 释放不再活跃的对象
        for i in 0..self.objects.len() {
            if !alive[i] {
                // 释放对象资源
                let obj_ptr = &self.objects[i];
                obj_ptr.free();
            }
        }

        let mut new_objects = Vec::new();

        for i in 0..self.objects.len() {
            if alive[i] {
                new_objects.push(self.objects[i].clone());
            }
        }

        self.objects.clear();
        self.objects.extend(new_objects);

        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            if gc_ref.get_traceable().should_free {
                panic!("Object should be freed! But it is still in the list! {:?}", gc_ref);
            }
        }
    }

    pub fn collect(&mut self) {
        self.mark();
        self.sweep();
    }

    pub fn debug_print(&self) {
        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            println!("Object {}: {:?}", i, gc_ref.get_traceable());
        }
    }

    pub fn drop_all(&mut self) {
        for gc_ref in &self.objects {
            gc_ref.offline();
        }
        self.collect();
    }

    pub fn is_available(&self, gc_ref: &GCRef) -> bool {
        for obj in &self.objects {
            if obj == gc_ref {
                return true;
            }
        }
        false
    }

    pub fn count(&self) -> usize {
        self.objects.len()
    }

}

impl Drop for GCSystem {
    fn drop(&mut self) {
        for gc_ref in &self.objects {
            gc_ref.offline();
        }
        self.collect();
        if self.objects.len() > 0 {
            panic!("Memory leak detected! {} objects are not freed!", self.objects.len());
        }
    }
}
