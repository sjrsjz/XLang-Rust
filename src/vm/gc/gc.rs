
use std::collections::HashSet;
//typeid
use std::any::{Any, TypeId};
use std::hash::{Hash, Hasher};
use std::path;

pub trait GCObject{
    fn free(&mut self); // free the object
    fn get_traceable(&mut self) -> &mut GCTraceable; // get the traceable object
}

#[derive(Debug, Clone)]
pub struct GCRef{
    pub reference: *mut dyn GCObject, // reference to the object
    pub type_id: TypeId, // type id of the object
}
impl PartialEq for GCRef {
    fn eq(&self, other: &Self) -> bool {
        self.reference == other.reference
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
    pub fn new(reference: *mut dyn GCObject) -> GCRef {
        GCRef {
            reference,
            type_id: TypeId::of::<dyn GCObject>(),
        }
    }

    pub fn get_type_id(&self) -> TypeId {
        self.type_id
    }
    
    pub fn get_traceable(&self) -> &mut GCTraceable {
        unsafe {
            let obj = self.reference as *mut dyn GCObject;
            (*obj).get_traceable()
        }
    }
}

#[derive(Debug)]
pub struct GCTraceable {
    pub ref_count: usize,
    pub should_free: bool,
    pub online: bool,
    pub references: HashSet<GCRef>,
}

impl GCTraceable{
    pub fn new(references: Option<HashSet<GCRef>>) -> GCTraceable{
        GCTraceable{
            ref_count: 0,
            should_free: false,
            online: true,
            references: references.unwrap_or(HashSet::new()),
        }
    }

    pub fn offline(&mut self){ // set the object offline, so that it can be collected
        self.online = false;
    }

    pub fn add_reference(&mut self, obj: &mut GCRef){
        if self.references.contains(obj) {
            panic!("Reference already exists!");
        }
        self.references.insert(obj.clone());
        unsafe {
            (*obj.reference).get_traceable().ref_count += 1; // increase the reference count of the object
        }
    }

    pub fn remove_reference(&mut self, obj: &GCRef){
        if !self.references.contains(obj) {
            panic!("Reference does not exist!");
        }
        self.references.remove(obj);
        unsafe {
            (*obj.reference).get_traceable().ref_count -= 1; // decrease the reference count of the object
        }
    }
}


pub struct GCSystem{
    pub objects: Vec<*mut GCRef>,
}

impl GCSystem{
    pub fn new() -> GCSystem{
        GCSystem{
            objects: Vec::new(),
        }
    }

    pub fn new_object<T: GCObject + 'static>(&mut self, object: T) -> GCRef {
        let mut obj = Box::new(object);
        let obj_ref = &mut *obj as *mut dyn GCObject;
        let gc_ref = GCRef {
            reference: obj_ref,
            type_id: TypeId::of::<T>(),
        };
        self.objects.push(obj_ref as *mut GCRef); // add the object to the list of objects
        gc_ref
    }

    pub fn as_type<T>(obj: &GCRef) -> &T where T: GCObject + 'static {
        unsafe {
            let obj = obj.reference as *const T;
            &*obj
        }
    }

    pub fn check_type<T:GCObject + 'static>(obj: & GCRef) -> bool {
        obj.type_id == TypeId::of::<T>()
    }

    pub fn mark(&mut self){
        let mut alive = Vec::<bool>::new(); // reachable objects
        let mut accessed = Vec::<bool>::new(); // accessed objects
        alive.resize(self.objects.len(), false); // initialize the vector to false
        accessed.resize(self.objects.len(), false); // initialize the vector to false
        // 重置所有对象的标记状态
        for i in 0..self.objects.len() {
            unsafe {
                let gc_ref = &*self.objects[i];
                if gc_ref.get_traceable().online {
                    alive[i] = true; // mark the object as alive
                }
                else if gc_ref.get_traceable().ref_count == 0 {
                    gc_ref.get_traceable().should_free = true; // mark the object as should free
                    alive[i] = false; // mark the object as dead
                }
            }
        }

        // 标记所有引用的对象
        /*
        从所有非alive对象开始，使用深度优先搜索算法遍历所有其祖先对象，如果存在alive的祖先对象，将该对象到alive对象的路径上的所有对象标记为alive。并立即返回
        考虑到递归有可能导致栈溢出，使用栈来实现深度优先搜索算法。
         */

        let mut path = Vec::<(usize, usize)>::new(); // path for dfs, (idx, ref_idx)

        // map pointer to index
        let mut idx_map = std::collections::HashMap::new();
        for (i, &obj_ptr) in self.objects.iter().enumerate() {
            unsafe {
                idx_map.insert((*obj_ptr).reference, i);
            }
        }

        for i in 0..self.objects.len() {
            unsafe {
                let gc_ref = &*self.objects[i];
                if !alive[i] && !gc_ref.get_traceable().should_free { // if the object is not alive and should not free? then check its references
                    path.push((i, 0)); // push the object to the path
                    while !path.is_empty() {
                        let (idx, ref_idx) = path.pop().unwrap(); // pop the object from the path
                        accessed[idx] = true; // mark the object as accessed
                        let gc_ref = &*self.objects[idx]; // get the object
                        let len = gc_ref.get_traceable().references.len(); // get the number of references
                        for j in ref_idx..len { // for each reference
                            let ref_obj = &gc_ref.get_traceable().references.iter().nth(j).unwrap(); // get the reference
                            let ref_idx = idx_map.get(&ref_obj.reference); // get the index of the reference
                            if ref_idx.is_none() { // if the reference is not in the map, then panic
                                panic!("Cannot find reference in map! Is the GCSystem crashing?");
                            }
                            let ref_idx = *ref_idx.unwrap(); // get the index of the reference
                            if alive[ref_idx] { // if the reference is alive, then mark all objects in the path as alive
                                for k in 0..path.len() {
                                    let (path_idx, _) = path[k]; // get the index of the object in the path
                                    alive[path_idx] = true; // mark the object as alive
                                }
                                alive[idx] = true; // mark the object as alive
                                path.clear(); // clear the path
                                break; // break the loop
                            } else if !accessed[ref_idx] { // if the reference is not accessed, then mark it as accessed and push it to the path
                                path.push((ref_idx, 0)); // push the object to the path
                                break;
                            } else { // if the reference is accessed, then continue to the next reference
                                continue; // continue to the next reference                                
                            }
                        }
                    }
                }
            }
        }        

    }

    pub fn sweep(&mut self){
    }
    
    pub fn collect(&mut self) {
        self.mark();
        self.sweep();
    }
}