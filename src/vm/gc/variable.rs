use super::gc::{GCObject,GCTraceable,GCSystem};


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