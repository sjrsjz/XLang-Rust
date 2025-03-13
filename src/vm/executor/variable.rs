use super::super::gc::gc::{GCObject, GCRef, GCSystem, GCTraceable};
use std::collections::HashSet;

macro_rules! try_copy_as_type {
    ($value:expr, $gc_system:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_const_type::<$t>().copy($gc_system);
            }
        )+
    };
}

macro_rules! try_ref_as_type {
    ($value:expr,$($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_const_type::<$t>().object_ref();
            }
        )+
    };
}

macro_rules! try_binary_op_as_type {
    ($value:expr, $op:ident, $other:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_const_type::<$t>().$op($other);
            }
        )+
    };
}

pub trait VMObject {
    fn copy(&self, gc_system: &mut GCSystem) -> GCRef;
    fn object_ref(&self) -> GCRef;
    fn assgin(&mut self, value: GCRef);
}

#[derive(Debug)]
pub struct VMVariableWrapper {
    pub value_ref: GCRef,
    traceable: GCTraceable,
}

impl VMVariableWrapper {
    pub fn new(value: GCRef) -> Self {
        if value.isinstance::<VMVariableWrapper>() {
            panic!("Cannot wrap a variable as a variable")
        }

        VMVariableWrapper {
            value_ref: value.clone(),
            traceable: GCTraceable::new(Some(HashSet::from([value.clone()]))),
        }
    }
}

impl GCObject for VMVariableWrapper {
    fn free(&mut self) {
        self.traceable.remove_reference(&self.value_ref);
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMVariableWrapper {
    fn copy<'t>(&self, gc_system: &mut GCSystem) -> GCRef {
        try_copy_as_type!(self.value_ref, gc_system; VMInt, VMString, VMFloat, VMBoolean);

        if self.value_ref.isinstance::<VMVariableWrapper>() {
            panic!("Cannot copy a variable of {:?}", self.value_ref);
        } else {
            panic!("Cannot copy a variable of {:?}", self.value_ref);
        }
    }

    fn assgin(&mut self, value: GCRef) {
        self.traceable.remove_reference(&self.value_ref);
        self.value_ref = value;
        self.traceable.add_reference(&mut self.value_ref);
    }

    fn object_ref(&self) -> GCRef {
        try_ref_as_type!(self.value_ref, VMInt);
        panic!("Cannot get object ref of {:?}", self.value_ref);
    }
}

#[derive(Debug)]
pub struct VMInt {
    pub value: i64,
    traceable: GCTraceable,
}

impl VMInt {
    pub fn new(value: i64) -> Self {
        VMInt {
            value,
            traceable: GCTraceable::new(None),
        }
    }

    pub fn eq(&self, other: GCRef) -> bool {
        if other.isinstance::<VMInt>() {
            return self.value == other.as_const_type::<VMInt>().value;
        } else if other.isinstance::<VMFloat>() {
            return self.value as f64 == other.as_const_type::<VMFloat>().value;
        } else {
            false
        }
    }
}

impl GCObject for VMInt {
    fn free(&mut self) {}

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMInt {
    fn copy(&self, gc_system: &mut GCSystem) -> GCRef {
        gc_system.new_object(VMInt::new(self.value))
    }

    fn assgin(&mut self, value: GCRef) {
        if value.isinstance::<VMInt>() {
            self.value = value.as_const_type::<VMInt>().value;
        } else if value.isinstance::<VMFloat>() {
            self.value = value.as_const_type::<VMFloat>().value as i64;
        } else {
            panic!("Cannot assign a value of {:?}", value);
        }
    }

    fn object_ref(&self) -> GCRef {
        GCRef::wrap(self)
    }
}
#[derive(Debug)]
pub struct VMString {
    pub value: String,
    traceable: GCTraceable,
}

impl VMString {
    pub fn new(value: String) -> Self {
        VMString {
            value,
            traceable: GCTraceable::new(None),
        }
    }

    pub fn eq(&self, other: GCRef) -> bool {
        if other.isinstance::<VMString>() {
            return self.value == other.as_const_type::<VMString>().value;
        } else {
            false
        }
    }
}

impl GCObject for VMString {
    fn free(&mut self) {
        // 字符串不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMString {
    fn copy(&self, gc_system: &mut GCSystem) -> GCRef {
        gc_system.new_object(VMString::new(self.value.clone()))
    }

    fn assgin(&mut self, value: GCRef) {
        if value.isinstance::<VMString>() {
            self.value = value.as_const_type::<VMString>().value.clone();
        } else if value.isinstance::<VMVariableWrapper>() {
            let var_wrapper = value.as_const_type::<VMVariableWrapper>();
            if var_wrapper.value_ref.isinstance::<VMString>() {
                self.value = var_wrapper
                    .value_ref
                    .as_const_type::<VMString>()
                    .value
                    .clone();
            } else {
                panic!("Cannot assign a non-string variable to string");
            }
        } else {
            panic!("Cannot assign a value of {:?} to VMString", value);
        }
    }

    fn object_ref(&self) -> GCRef {
        GCRef::wrap(self)
    }
}

#[derive(Debug)]
pub struct VMFloat {
    pub value: f64,
    traceable: GCTraceable,
}

impl VMFloat {
    pub fn new(value: f64) -> Self {
        VMFloat {
            value,
            traceable: GCTraceable::new(None),
        }
    }

    pub fn eq(&self, other: GCRef) -> bool {
        if other.isinstance::<VMFloat>() {
            return self.value == other.as_const_type::<VMFloat>().value;
        } else if other.isinstance::<VMInt>() {
            return self.value == other.as_const_type::<VMInt>().value as f64;
        } else {
            false
        }
    }
}

impl GCObject for VMFloat {
    fn free(&mut self) {
        // 浮点数不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMFloat {
    fn copy(&self, gc_system: &mut GCSystem) -> GCRef {
        gc_system.new_object(VMFloat::new(self.value))
    }

    fn assgin(&mut self, value: GCRef) {
        if value.isinstance::<VMFloat>() {
            self.value = value.as_const_type::<VMFloat>().value;
        } else if value.isinstance::<VMInt>() {
            self.value = value.as_const_type::<VMInt>().value as f64;
        } else {
            panic!("Cannot assign a value of {:?} to VMFloat", value);
        }
    }

    fn object_ref(&self) -> GCRef {
        GCRef::wrap(self)
    }
}

#[derive(Debug)]
pub struct VMBoolean {
    pub value: bool,
    traceable: GCTraceable,
}

impl VMBoolean {
    pub fn new(value: bool) -> Self {
        VMBoolean {
            value,
            traceable: GCTraceable::new(None),
        }
    }

    pub fn eq(&self, other: GCRef) -> bool {
        if other.isinstance::<VMBoolean>() {
            return self.value == other.as_const_type::<VMBoolean>().value;
        } else {
            false
        }
    }
}

impl GCObject for VMBoolean {
    fn free(&mut self) {
        // 布尔值不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMBoolean {
    fn copy(&self, gc_system: &mut GCSystem) -> GCRef {
        gc_system.new_object(VMBoolean::new(self.value))
    }

    fn assgin(&mut self, value: GCRef) {
        if value.isinstance::<VMBoolean>() {
            self.value = value.as_const_type::<VMBoolean>().value;
        } else if value.isinstance::<VMInt>() {
            self.value = value.as_const_type::<VMInt>().value != 0;
        } else {
            panic!("Cannot assign a value of {:?} to VMBoolean", value);
        }
    }

    fn object_ref(&self) -> GCRef {
        GCRef::wrap(self)
    }
}

#[derive(Debug)]
pub struct VMNull {
    traceable: GCTraceable,
}

impl VMNull {
    pub fn new() -> Self {
        VMNull {
            traceable: GCTraceable::new(None),
        }
    }

    pub fn eq(&self, other: GCRef) -> bool {
        other.isinstance::<VMNull>()
    }
}

impl GCObject for VMNull {
    fn free(&mut self) {
        // Null 不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMNull {
    fn copy(&self, gc_system: &mut GCSystem) -> GCRef {
        gc_system.new_object(VMNull::new())
    }

    fn assgin(&mut self, value: GCRef) {
        if value.isinstance::<VMNull>() {
            // Null 什么都不做
        } else {
            panic!("Cannot assign a value of {:?} to VMNull", value);
        }
    }

    fn object_ref(&self) -> GCRef {
        GCRef::wrap(self)
    }
}

#[derive(Debug)]
pub struct VMKeyVal {
    pub key: GCRef,
    pub value: GCRef,
    traceable: GCTraceable,
}

impl VMKeyVal {
    pub fn new(key: GCRef, value: GCRef) -> Self {
        VMKeyVal {
            key: key.clone(),
            value: value.clone(),
            traceable: GCTraceable::new(Some(HashSet::from([key, value]))),
        }
    }

    pub fn get_key(&self) -> GCRef {
        self.key.clone()
    }

    pub fn get_value(&self) -> GCRef {
        self.value.clone()
    }

    pub fn check_key(&self, other: GCRef) -> bool {
        try_binary_op_as_type!(self.key, eq, other; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal);
        false
    }

    pub fn eq(&self, other: GCRef) -> bool {
        if other.isinstance::<VMKeyVal>() {
            let other_kv = other.as_const_type::<VMKeyVal>();
            let key_eq = {
                try_binary_op_as_type!(self.key, eq, other_kv.key.clone(); VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal);
                false
            };
            let value_eq = {
                try_binary_op_as_type!(self.value, eq, other_kv.value.clone(); VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal);
                false
            };
            return key_eq && value_eq;
        } else {
            false
        }
    }
}

impl GCObject for VMKeyVal {
    fn free(&mut self) {
        self.traceable.remove_reference(&self.key);
        self.traceable.remove_reference(&self.value);
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMKeyVal {
    fn copy(&self, gc_system: &mut GCSystem) -> GCRef {
        gc_system.new_object(VMKeyVal::new(self.key.clone(), self.value.clone()))
    }

    fn assgin(&mut self, value: GCRef) {
        if value.isinstance::<VMKeyVal>() {
            self.value = value.as_const_type::<VMKeyVal>().value.clone();
        } else {
            panic!("Cannot assign a value of {:?} to VMKeyVal", value);
        }
    }

    fn object_ref(&self) -> GCRef {
        GCRef::wrap(self)
    }
}
