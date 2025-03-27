use rustc_hash::FxHashMap as HashMap;
//typeid
use std::any::TypeId;
use std::hash::{Hash, Hasher};


pub trait GCObject {
    fn free(&mut self); // free the object
    fn get_traceable(&mut self) -> &mut GCTraceable; // get the traceable object
    fn get_const_traceable(&self) -> &GCTraceable; // get the traceable object
}

#[derive(Debug, Clone)]
pub struct GCRef {
    pub(self) reference: *mut dyn GCObject, // reference to the object
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
    }
}

impl std::fmt::Display for GCRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GCRef({:?})", self.reference)
    }
}

impl GCRef {
    pub fn new(reference: *mut dyn GCObject) -> Self {
        if reference.is_null() {
            panic!("Null pointer exception!");
        }
        GCRef { reference }
    }

    pub fn get_reference(&self) -> *mut dyn GCObject {
        self.reference
    }

    pub fn get_const_reference(&self) -> *const dyn GCObject {
        self.reference
    }

    pub fn get_traceable(&mut self) -> &mut GCTraceable {
        unsafe {
            let obj = self.reference;
            (*obj).get_traceable()
        }
    }

    pub fn get_const_traceable(&self) -> &GCTraceable {
        unsafe {
            let obj = self.reference;
            (*obj).get_const_traceable()
        }
    }

    pub(self) fn free(&self) {
        unsafe {
            let obj = self.reference as *mut dyn GCObject;
            (*obj).free();
        }
    }

    pub(self) fn delete(&self) {
        unsafe {
            let _ = Box::from_raw(self.reference);
        }
    }


    pub fn is_online(&self) -> bool {
        self.get_const_traceable().native_gcref_object_count > 0
    }

    pub fn as_type<T>(&mut self) -> &mut T
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
        self.get_const_traceable().isinstance::<T>()
    }

    pub fn wrap<T: GCObject + 'static>(obj: &T) -> GCRef {
        let obj = obj as *const T as *mut T as *mut dyn GCObject;
        if obj.is_null() {
            panic!("Failed to wrap object!");
        }

        GCRef {
            reference: obj,
        }
    }

    pub fn wrap_mut<T: GCObject + 'static>(obj: &mut T) -> GCRef {
        let obj = obj as *mut T as *mut dyn GCObject;
        if obj.is_null() {
            panic!("Failed to wrap object!");
        }
        GCRef {
            reference: obj,
        }
    }

    pub fn lock(&self) {
        unsafe {
            let obj = self.reference as *mut dyn GCObject;
            (*obj).get_traceable().lock = true;
        }
    }

    pub fn unlock(&self) {
        unsafe {
            let obj = self.reference as *mut dyn GCObject;
            (*obj).get_traceable().lock = false;
        }
    }

    pub fn is_locked(&self) -> bool {
        unsafe {
            let obj = self.reference as *const dyn GCObject;
            (*obj).get_const_traceable().lock
        }
    }

    pub fn clone_ref(&mut self) -> Self {
        if !self.is_online() && self.get_const_traceable().ref_count == 0 {
            panic!("Cannot clone an offline object!");
        }
        unsafe {
            let obj = self.reference as *mut dyn GCObject;
            (*obj).get_traceable().native_gcref_object_count += 1; // 增加原生引用计数
        }
        GCRef {
            reference: self.reference,
        }
    }

    pub fn drop_ref(&mut self) {
        // println!("{}",
        //     format!("[GC] Dropping reference: {:?}", self).to_string().red().to_string()
        // );
        let traceable = self.get_traceable();
        if traceable.native_gcref_object_count == 0 {
            panic!("Reference count is already zero!");
        }
        traceable.native_gcref_object_count -= 1;
    }
}

#[derive(Debug)]
pub struct GCTraceable {
    pub native_gcref_object_count: usize, // 原生对象数量, 当GCRef被创建时增加
    pub ref_count: usize,
    pub should_free: bool,
    pub lock: bool, // 是否锁定对象禁止回收
    pub references: HashMap<GCRef, usize>,
    pub type_id: TypeId,              // type id of the object
}

impl GCTraceable {
    pub fn new<T:GCObject + 'static>(references: Option<&mut Vec<&mut GCRef>>) -> GCTraceable {
        let mut refs_map = HashMap::default();

        if let Some(refs) = references {
            for i in 0..refs.len() {
                *refs_map.entry((*refs[i]).clone()).or_insert(0) += 1;
                (*refs[i]).get_traceable().ref_count += 1; // 增加对象的引用计数
            }
        }

        GCTraceable {
            native_gcref_object_count: 0,
            ref_count: 0,
            should_free: false,
            lock: false,
            references: refs_map,
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn add_reference(&mut self, obj: &GCRef) {
        if obj.reference.is_null() {
            panic!("Reference is null! {}", obj);
        }
        // 增加引用计数
        *self.references.entry(obj.clone()).or_insert(0) += 1;
        unsafe {
            (*obj.reference).get_traceable().ref_count += 1; // 增加被引用对象的引用计数
        }
    }

    pub fn remove_reference(&mut self, obj: &mut GCRef) {
        if obj.reference.is_null() {
            panic!("Reference is null! {}", obj);
        }
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
                return; //panic!("Reference count is already zero! {}", obj);
            }
            (*obj.reference).get_traceable().ref_count -= 1; // 减少被引用对象的引用计数
        }
    }

    pub fn isinstance<T: GCObject + 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
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
        let trigger = trigger.unwrap_or((100, 4096 * 1024));
        let maximum_new_objects_count = trigger.0;
        let maximum_allocation_size = trigger.1;
        GCSystem {
            objects: Vec::new(),
            new_objects_count: 0,
            new_objects_sum_size: 0,
            maximum_allocation_size,
            _maximum_new_objects_count: maximum_new_objects_count,
        }
    }

    pub fn new_object<T: GCObject + 'static>(&mut self, object: T) -> GCRef {
        self.new_objects_sum_size += std::mem::size_of::<T>();
        self.new_objects_count += 1;

        let trigger_threshold = self.objects.len() / 2; // 20%的增长率触发GC

        if self.new_objects_sum_size > self.maximum_allocation_size
            && self.new_objects_count > trigger_threshold
        {
            self.collect();
            self.new_objects_sum_size = 0;
            self.new_objects_count = 0;
        }
        let obj_ref = Box::into_raw(Box::new(object)) as *mut dyn GCObject;
        if obj_ref.is_null() {
            panic!("Failed to allocate memory for object!");
        }
        let mut gc_ref = GCRef { reference: obj_ref };
        gc_ref.get_traceable().native_gcref_object_count = 1; // 设置原生引用计数为1
        self.objects.push(gc_ref.clone()); // add the object to the list of objects
        gc_ref
    }

    fn mark(&mut self) {
        let mut alive = vec![false; self.objects.len()]; // 可达对象

        // 第一步：标记所有在线对象为活跃
        for i in 0..self.objects.len() {
            let gc_ref = &mut self.objects[i];
            if gc_ref.get_traceable().should_free {
                let obj = gc_ref.get_traceable();
                panic!(
                    "Never set should_free to true! Use offline() instead! Object: {:?}",
                    obj
                );
            } else if gc_ref.is_online() || gc_ref.is_locked() {
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
            let gc_ref = &mut self.objects[i];
            let type_id = gc_ref.get_const_traceable().type_id;
            for ref_obj in gc_ref.get_traceable().references.keys() {
                let ref_usize = ref_obj.reference as *const () as usize;
                match idx_map.get(&ref_usize) {
                    Some(ref_idx) => ref_graph[i].push(*ref_idx),
                    None => {
                        // Build comprehensive diagnostics message
                        let mut error_msg = String::new();

                        error_msg.push_str(
                            "\n===== FATAL ERROR: INVALID REFERENCE DETECTED =====\n",
                        );
                        error_msg.push_str(&format!(
                            "Object #{} (Type: {:?}) references an object not managed by the GC\n",
                            i, type_id
                        ));
                        error_msg.push_str(&format!("Reference target: {:?}\n", ref_obj));
                        error_msg.push_str(&format!("Reference address: 0x{:x}\n\n", ref_usize));
                        error_msg.push_str("All References:\n");
                        for (ref_obj, count) in &gc_ref.get_traceable().references {
                            error_msg.push_str(&format!(
                                "  - Object {:?} (Type: {:?}): {} references\n",
                                ref_obj.reference, ref_obj.get_const_traceable().type_id, count
                            ));
                        }

                        // Include index mapping for diagnostics
                        error_msg.push_str("Current GC object map:\n");
                        for (&addr, &idx) in &idx_map {
                            error_msg.push_str(&format!("  0x{:x} -> Object #{}\n", addr, idx));
                        }

                        // Include partial reference graph for context
                        error_msg.push_str("\nCurrent reference graph (partial):\n");
                        for (idx, refs) in ref_graph.iter().enumerate().take(i) {
                            if !refs.is_empty() {
                                error_msg.push_str(&format!(
                                    "  Object #{} references -> {:?}\n",
                                    idx, refs
                                ));
                            }
                        }

                        error_msg.push_str("\n======= PROGRAM TERMINATING =======\n");

                        // Panic with the complete error message
                        panic!("{}", error_msg);
                    }
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
                let gc_ref = &mut self.objects[i];
                gc_ref.get_traceable().should_free = true;
            }
        }
    }

    fn sweep(&mut self) {
        let mut alive = vec![false; self.objects.len()];

        // 标记存活对象
        for i in 0..self.objects.len() {
            let gc_ref = &mut self.objects[i];
            if gc_ref.get_traceable().native_gcref_object_count > 0 || !gc_ref.get_traceable().should_free {
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
        for obj in &dead_objects {
            obj.free();
        }
        for obj in dead_objects {
            obj.delete();
        }
    }

    pub fn immediate_collect(&mut self) {
        let mut alive = vec![false; self.objects.len()];

        // 标记活对象
        for i in 0..self.objects.len() {
            let gc_ref = &mut self.objects[i];
            alive[i] = !(gc_ref.get_traceable().ref_count == 0
                && !gc_ref.is_online()
                && gc_ref.get_traceable().references.is_empty()
                && !gc_ref.is_locked()); // 检查孤岛对象
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
        for obj in &dead_objects {
            obj.free();
        }
        for obj in dead_objects {
            obj.delete();
        }
    }
    pub fn collect(&mut self) {
        self.immediate_collect();
        self.mark();
        self.sweep();
    }

    pub fn debug_print(&self) {
        for i in 0..self.objects.len() {
            let gc_ref = &self.objects[i];
            println!("Object {}: {:?}", i, gc_ref.get_const_traceable());
        }
    }

    pub fn print_reference_graph(&self) {
        println!("\n=== GC Reference Graph ===");

        // 创建对象索引映射，方便查找
        let mut obj_index_map = HashMap::default();
        for (i, obj) in self.objects.iter().enumerate() {
            obj_index_map.insert(obj.reference, i);
        }

        // 打印每个对象的信息和引用关系
        for (i, obj) in self.objects.iter().enumerate() {
            let traceable = obj.get_const_traceable();

            // 打印对象基本信息
            println!(
                "Object #{}: {:?} (RefCount: {}, NativeCount: {}, ShouldFree: {})",
                i,
                obj.get_const_traceable().type_id,
                traceable.ref_count,
                traceable.native_gcref_object_count,
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
                            ref_idx, ref_obj.get_const_traceable().type_id, count
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
        for gc_ref in &mut self.objects {
            gc_ref.get_traceable().native_gcref_object_count = 0; // 清除原生引用计数
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

    pub fn get_all_objects(&self) -> &Vec<GCRef> {
        &self.objects
    }
}

impl Drop for GCSystem {
    fn drop(&mut self) {
        self.drop_all();
        if !self.objects.is_empty() {
            panic!(
                "Memory leak detected! {} objects are not freed!",
                self.objects.len()
            );
        }
    }
}
