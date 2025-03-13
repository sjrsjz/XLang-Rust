use super::super::gc::gc::{GCObject, GCRef, GCTraceable, GCSystem};
use std::collections::HashSet;


trait VMObject{
    fn copy(&mut self)->GCRef;
    fn object_ref(&self)->GCRef;
    fn assgin(&self, value:GCRef);
}




#[derive(Debug)]
pub struct VMVariableWrapper<'t> {
    gc_system: &'t mut GCSystem,
    pub value_ref: GCRef,
    traceable: GCTraceable,
}

impl<'t> VMVariableWrapper<'t> {
    pub fn new(gc_system: &'t mut GCSystem, value: GCRef) -> Self {
        if value.isinstance::<VMVariableWrapper>(){
            panic!("Cannot wrap a variable as a variable")
        }
        VMVariableWrapper {
            gc_system,
            value_ref: value.clone(),
            traceable: GCTraceable::new(Some(HashSet::from([value]))),
        }
    }
}

impl GCObject for VMVariableWrapper<'_> {
    fn free(&mut self) {
        self.traceable.remove_reference(&self.value_ref);
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable
    }
}

impl VMObject for VMVariableWrapper<'_> {
    fn copy<'t>(&mut self) -> GCRef{
        if self.value_ref.isinstance::<VMInt>(){
            self.value_ref.as_type::<VMInt>().copy()
        }
        else if self.value_ref.isinstance::<VMVariableWrapper>(){
            panic!("Cannot copy a variable of {:?}", self.value_ref);
        }
        else{
            panic!("Cannot copy a variable of {:?}", self.value_ref);
        }
    }

    fn assgin(&self, value:GCRef) {
        
    }

    fn object_ref(&self)->GCRef{
    }
}

#[derive(Debug)]
pub struct VMInt<'t> {
    gc_system: &'t mut GCSystem,
    pub value: i64,
    traceable: GCTraceable,
}

impl<'t> VMInt<'t> {
    pub fn new(gc_system:&'t mut GCSystem, value: i64) -> Self {
        VMInt {
            gc_system,
            value,
            traceable: GCTraceable::new(None),
        }
    }
}

impl GCObject for VMInt<'_> {
    fn free(&mut self) {
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable
    }
}

impl VMObject for VMInt<'_> {
    fn copy(&mut self) -> GCRef{
        self.gc_system.new_object(VMInt::new(self.gc_system, self.value))
    }

    fn assgin(&self, value:GCRef) {
        
    }

    fn object_ref(&self)->GCRef {
        
    }
}