pub mod gc;

use std::collections::HashSet;

pub trait GCObject{
    fn free(&mut self); // free the object
    fn get_traceable(&mut self) -> &mut GCTraceable; // get the traceable object
}

pub struct GCTraceable {
    pub ref_count: usize,
    pub should_free: bool,
    pub online: bool,
    pub references: HashSet<*mut dyn GCObject>,
    pub is_marked: bool,
}

impl GCTraceable{
    fn new(references: Option<HashSet<*mut dyn GCObject>>) -> GCTraceable{
        let mut refs = HashSet::new();
        if let Some(references) = references{
            for reference in references.iter(){
                unsafe{
                    (*(*reference)).get_traceable().ref_count += 1;
                }
            }
            refs = references;
        }
        GCTraceable{
            ref_count: 0,
            should_free: false,
            online: true,
            references: refs,
            is_marked: false,
        }
    }

    fn offline(&mut self){ // set the object offline, so that it can be collected
        self.online = false;
    }

    fn add_reference(&mut self, reference: *mut dyn GCObject){
        if self.references.contains(&reference){
            return;
        }
        self.references.insert(reference);
        unsafe{
            (*reference).get_traceable().ref_count += 1;
        }
    }

    fn remove_reference(&mut self, reference: *mut dyn GCObject){
        if !self.references.contains(&reference){
            panic!("Reference not found: {:?}", reference);
        }
        self.references.remove(&reference);
        unsafe{
            (*reference).get_traceable().ref_count -= 1;
        }
    }
}


pub struct GCSystem{
    pub objects: Vec<*mut dyn GCObject>,
}

impl GCSystem{
    pub fn new() -> GCSystem{
        GCSystem{
            objects: Vec::new(),
        }
    }

    pub fn new_object<T: GCObject + 'static>(&mut self, object: T) -> *mut dyn GCObject{
        let boxed = Box::new(object) as Box<dyn GCObject>;
        let raw_ptr = Box::into_raw(boxed);
        self.objects.push(raw_ptr);
        raw_ptr
    }

    pub fn mark(&mut self){
        // 重置所有对象的标记状态
        for &object in self.objects.iter(){
            unsafe{
                (*object).get_traceable().is_marked = false;
            }
        }

        // 从在线对象开始标记
        let mut stack = Vec::new();
        for &object in self.objects.iter() {
            unsafe {
                if (*object).get_traceable().online {
                    stack.push(object);
                    (*object).get_traceable().is_marked = true;
                }
            }
        }

        // 标记从在线对象可达的所有对象
        while let Some(object) = stack.pop() {
            unsafe {
                let references = (*object).get_traceable().references.clone();
                for &reference in references.iter() {
                    if !(*reference).get_traceable().is_marked {
                        (*reference).get_traceable().is_marked = true;
                        stack.push(reference);
                    }
                }
            }
        }
        
        // 检查非在线对象是否应该被释放
        for &object in self.objects.iter() {
            unsafe {
                let traceable = (*object).get_traceable();
                
                // 如果是非online对象 且 (从online对象不可达 或 引用计数为0)
                if !traceable.online && (!traceable.is_marked || traceable.ref_count == 0) {
                    traceable.should_free = true;
                } else {
                    traceable.should_free = false;
                }
            }
        }
    }

    pub fn sweep(&mut self){
        let mut i = 0;
        while i < self.objects.len(){
            let object = self.objects[i];
            unsafe{
                if (*object).get_traceable().should_free {
                    (*object).free();
                    // 安全地释放Box
                    let _ = Box::from_raw(object);
                    self.objects.remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }
    
    pub fn collect(&mut self) {
        self.mark();
        self.sweep();
    }
}