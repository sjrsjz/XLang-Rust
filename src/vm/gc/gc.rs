use std::collections::HashMap;
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

impl std::fmt::Display for GCRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GCRef({:?}, {:?})", self.reference, self.type_id)
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

    pub fn mark_as_online(&self) {
        unsafe {
            let obj = self.reference as *mut dyn GCObject;
            (*obj).get_traceable().mark_as_online();
        }
    }

    pub fn is_online(&self) -> bool {
        unsafe {
            let obj = self.reference as *mut dyn GCObject;
            (*obj).get_traceable().online
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

    pub fn mark_as_online(&mut self) {
        // 设置对象在线，以防止被回收
        self.online = true;
    }
    pub fn add_reference(&mut self, obj: &GCRef) {
        // 增加引用计数
        *self.references.entry(obj.clone()).or_insert(0) += 1;
        unsafe {
            (*obj.reference).get_traceable().ref_count += 1; // 增加被引用对象的引用计数
        }
    }

    pub fn remove_reference(&mut self, obj: &GCRef) {
        let count = self.references.get(obj).cloned();

        match count {
            None => panic!(
                "Reference does not exist! existing references: {:?}, but got {:?}",
                self.references, obj
            ),
            Some(1) => {
                // 最后一个引用，从HashMap中移除
                self.references.remove(obj);
            }
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
            eprintln!(
                "[GC] Warning: {} references are not cleaned up! References: {:?}",
                total_refs, self.references
            );

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
    new_objects_count: usize, // 新创建的对象数量
    new_objects_sum_size: usize,
    _maximum_new_objects_count: usize, // GC触发对象数量限制
    maximum_allocation_size: usize,    // GC触发内存限制
}

impl GCSystem {
    pub fn new(trigger: Option<(usize, usize)>) -> GCSystem {
        let trigger = trigger.unwrap_or((100, 1024 * 1024));
        let maximum_new_objects_count = trigger.0;
        let maximum_allocation_size = trigger.1;
        GCSystem {
            objects: Vec::new(),
            new_objects_count: 0,
            new_objects_sum_size: 0,
            maximum_allocation_size: maximum_allocation_size,
            _maximum_new_objects_count: maximum_new_objects_count,
        }
    }

    pub fn new_object<T: GCObject + 'static>(&mut self, object: T) -> GCRef {
        self.new_objects_sum_size += std::mem::size_of::<T>();
        self.new_objects_count += 1;

        let trigger_threshold = self.objects.len() / 5; // 20%的增长率触发GC

        if self.new_objects_sum_size > self.maximum_allocation_size
            && self.new_objects_count > trigger_threshold
        {
            self.collect();
            self.new_objects_sum_size = 0;
            self.new_objects_count = 0;
        }
        let obj_ref = Box::into_raw(Box::new(object)) as *mut dyn GCObject;
        let gc_ref = GCRef {
            reference: obj_ref,
            type_id: TypeId::of::<T>(),
        };
        self.objects.push(gc_ref.clone()); // add the object to the list of objects
        gc_ref
    }

    fn mark(&mut self) {
        let mut alive = vec![false; self.objects.len()]; // 可达对象

        // 第一步：标记所有在线对象为活跃
        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            if gc_ref.get_traceable().should_free {
                let obj = gc_ref.get_traceable();
                panic!(
                    "Never set should_free to true! Use offline() instead! Object: {:?}",
                    obj
                );
            } else if gc_ref.get_traceable().online {
                alive[i] = true;
            }
        }

        // 创建索引映射
        let idx_map: HashMap<_, _> = self
            .objects
            .iter()
            .enumerate()
            .map(|(i, obj)| (obj.reference as *const () as usize, i))
            .collect();

        // 第二步：构建引用图
        let mut ref_graph: Vec<Vec<usize>> = vec![Vec::new(); self.objects.len()];
        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            for (ref_obj, _) in &gc_ref.get_traceable().references {
                let ref_usize = ref_obj.reference as *const () as usize;
                match idx_map.get(&ref_usize) {
                    Some(ref_idx) => ref_graph[i].push(*ref_idx),
                    None => {
                        // Build comprehensive diagnostics message
                        let mut error_msg = String::new();

                        error_msg.push_str(&format!(
                            "\n===== FATAL ERROR: INVALID REFERENCE DETECTED =====\n"
                        ));
                        error_msg.push_str(&format!(
                            "Object #{} (Type: {:?}) references an object not managed by the GC\n",
                            i, gc_ref.type_id
                        ));
                        error_msg.push_str(&format!("Reference target: {:?}\n", ref_obj));
                        error_msg.push_str(&format!("Reference address: 0x{:x}\n\n", ref_usize));

                        // Include index mapping for diagnostics
                        error_msg.push_str(&format!("Current GC object map:\n"));
                        for (&addr, &idx) in &idx_map {
                            error_msg.push_str(&format!("  0x{:x} -> Object #{}\n", addr, idx));
                        }

                        // Include partial reference graph for context
                        error_msg.push_str(&format!("\nCurrent reference graph (partial):\n"));
                        for (idx, refs) in ref_graph.iter().enumerate().take(i) {
                            if !refs.is_empty() {
                                error_msg.push_str(&format!(
                                    "  Object #{} references -> {:?}\n",
                                    idx, refs
                                ));
                            }
                        }

                        error_msg.push_str(&format!("\n======= PROGRAM TERMINATING =======\n"));

                        // Panic with the complete error message
                        panic!("{}", error_msg);
                    }
                }
            }
        }

        // let mut ref_graph: Vec<Vec<usize>> = vec![Vec::new(); self.objects.len()];
        // for i in 0..self.objects.len() {
        //     let gc_ref = &self.objects[i];
        //     for (ref_obj, _) in &gc_ref.get_traceable().references {
        //         // 线性搜索而非使用HashMap
        //         let mut found = false;
        //         let mut ref_idx = 0;
        //         for (j, obj) in self.objects.iter().enumerate() {
        //             if std::ptr::addr_eq(obj.reference, ref_obj.reference) {
        //                 found = true;
        //                 ref_idx = j;
        //                 break;
        //             }
        //         }

        //         if !found {
        //             println!("Mapping: {:?}", idx_map);
        //             println!("Reference graph: {:?}", ref_graph);

        //             panic!("警告：对象 #{} 引用了不在GC管理下的对象: {:?}", i, ref_obj);

        //             // 可以在这里继续处理
        //             // 如果想要严格行为，可以保留panic
        //             // panic!("Reference object not found in index map! Object: {:?}", ref_obj);
        //         } else {
        //             ref_graph[i].push(ref_idx);
        //         }
        //     }
        // }

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
        let mut alive = vec![false; self.objects.len()];

        // 标记存活对象
        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            if gc_ref.get_traceable().online || !gc_ref.get_traceable().should_free {
                alive[i] = true;
            }
        }

        // 重要变化：我们先复制存活对象到新列表，再释放死亡对象
        // 这样可以避免在释放过程中引用已经被释放的对象
        let mut new_objects = Vec::with_capacity(self.objects.len());
        for i in 0..self.objects.len() {
            if alive[i] {
                new_objects.push(self.objects[i].clone());
            }
        }

        // 收集要释放的对象
        let dead_objects: Vec<GCRef> = self
            .objects
            .iter()
            .enumerate()
            .filter(|&(i, _)| !alive[i])
            .map(|(_, obj)| obj.clone())
            .collect();

        // 替换对象列表
        self.objects = new_objects;

        // 现在安全地释放对象，因为它们已经从列表中移除
        for obj in dead_objects {
            obj.free();
        }
    }

    pub fn immediate_collect(&mut self) {
        let mut alive = vec![false; self.objects.len()];

        // 标记活对象
        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            alive[i] = !(gc_ref.get_traceable().ref_count == 0 && !gc_ref.is_online() && gc_ref.get_traceable().references.is_empty()); // 检查孤岛对象
        }

        // 先创建新的对象列表，仅包含活对象
        let mut new_objects = Vec::with_capacity(self.objects.len());
        for i in 0..self.objects.len() {
            if alive[i] {
                new_objects.push(self.objects[i].clone());
            }
        }

        // 收集要释放的对象
        let dead_objects: Vec<GCRef> = self
            .objects
            .iter()
            .enumerate()
            .filter(|&(i, _)| !alive[i])
            .map(|(_, obj)| obj.clone())
            .collect();

        // 更新对象列表
        self.objects = new_objects;

        // 最后释放死亡对象
        for obj in dead_objects {
            obj.free();
        }
    }
    pub fn collect(&mut self) {
        self.immediate_collect(); // panic! corrupt double-linked list
        self.mark();
        self.sweep();
        
    }

    pub fn debug_print(&self) {
        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            println!("Object {}: {:?}", i, gc_ref.get_traceable());
        }
    }

    pub fn print_reference_graph(&self) {
        println!("\n=== GC Reference Graph ===");

        // 创建对象索引映射，方便查找
        let mut obj_index_map = HashMap::new();
        for (i, obj) in self.objects.iter().enumerate() {
            obj_index_map.insert(obj.reference, i);
        }

        // 打印每个对象的信息和引用关系
        for (i, obj) in self.objects.iter().enumerate() {
            let traceable = obj.get_traceable();

            // 打印对象基本信息
            println!(
                "Object #{}: {:?} (RefCount: {}, Online: {}, ShouldFree: {})",
                i,
                obj.type_id, // 或者使用自定义的类型名称映射
                traceable.ref_count,
                traceable.online,
                traceable.should_free
            );

            // 打印引用关系
            if !traceable.references.is_empty() {
                println!("  References:");
                for (ref_obj, count) in &traceable.references {
                    // 尝试获取被引用对象的索引
                    if let Some(&ref_idx) = obj_index_map.get(&(ref_obj.reference)) {
                        println!(
                            "    -> Object #{} (type: {:?}): {} references",
                            ref_idx, ref_obj.type_id, count
                        );
                    } else {
                        println!(
                            "    -> External object {:?}: {} references",
                            ref_obj.reference, count
                        );
                    }
                }
            } else {
                println!("  No outgoing references");
            }
        }

        println!("=========================\n");
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
            panic!(
                "Memory leak detected! {} objects are not freed!",
                self.objects.len()
            );
        }
    }
}
