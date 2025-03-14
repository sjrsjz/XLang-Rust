use std::{collections::HashMap, f64::consts::E};

use crate::vm::ir::IR;

use super::super::gc::gc::{GCObject, GCRef, GCSystem, GCTraceable};

#[derive(Debug)]
pub enum VMStackObject{
    LastIP(usize, bool),
    VMObject(GCRef),
}



#[derive(Debug)]
pub enum VMVariableError {
    TypeError(GCRef, String),
    ValueError(GCRef, String),
    KeyNotFound(GCRef, GCRef), // 键未找到
    CopyError(GCRef, String),
    AssignError(GCRef, String),
    ReferenceError(GCRef, String),
}

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
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError>;
    fn object_ref(&self) -> Result<GCRef, VMVariableError>;
    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError>;
}

#[derive(Debug)]
pub struct VMVariableWrapper {
    pub value_ref: GCRef,
    traceable: GCTraceable,
}

impl VMVariableWrapper {
    pub fn new(value: GCRef) -> Self {
        if value.isinstance::<VMVariableWrapper>() {
            panic!("Cannot wrap a variable as a variable");
        }

        VMVariableWrapper {
            value_ref: value.clone(),
            traceable: GCTraceable::new(Some(vec![value.clone()])),
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
    fn copy<'t>(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        try_copy_as_type!(self.value_ref, gc_system; VMInt, VMString, VMFloat, VMBoolean);

        if self.value_ref.isinstance::<VMVariableWrapper>() {
            return Err(VMVariableError::TypeError(
                self.value_ref.clone(),
                "Cannot copy a variable wrapper".to_string(),
            ));
        } else {
            return Err(VMVariableError::TypeError(
                self.value_ref.clone(),
                "Cannot copy a variable wrapper".to_string(),
            ));
        }
    }

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
        self.traceable.remove_reference(&self.value_ref);
        self.value_ref = value;
        self.traceable.add_reference(&mut self.value_ref);
        Ok(self.value_ref.clone())
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        try_ref_as_type!(self.value_ref, VMInt);
        
        Err(VMVariableError::TypeError(
            self.value_ref.clone(),
            "Cannot get reference of a variable wrapper".to_string(),
        ))
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
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMInt::new(self.value)))
    }

    fn assgin(&mut self, value: GCRef) ->Result<GCRef, VMVariableError> {
        if value.isinstance::<VMInt>() {
            self.value = value.as_const_type::<VMInt>().value;
        } else if value.isinstance::<VMFloat>() {
            self.value = value.as_const_type::<VMFloat>().value as i64;
        } else {
            panic!("Cannot assign a value of {:?}", value);
        }
        Ok(GCRef::wrap(self))
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
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
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMString::new(self.value.clone())))
    }

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
        if value.isinstance::<VMString>() {
            self.value = value.as_const_type::<VMString>().value.clone();
            Ok(GCRef::wrap(self))
        } else {
            Err(VMVariableError::TypeError(
                value.clone(),
                "Cannot assign a value of non-string type".to_string(),
            ))
        }
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
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
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMFloat::new(self.value)))
    }

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
        if value.isinstance::<VMFloat>() {
            self.value = value.as_const_type::<VMFloat>().value;
            Ok(GCRef::wrap(self))
        } else if value.isinstance::<VMInt>() {
            self.value = value.as_const_type::<VMInt>().value as f64;
            Ok(GCRef::wrap(self))
        } else {
            return Err(VMVariableError::TypeError(
                value.clone(),
                "Cannot assign a value of non-float type".to_string(),
            ));
        }
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
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
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMBoolean::new(self.value)))
    }

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
        if value.isinstance::<VMBoolean>() {
            self.value = value.as_const_type::<VMBoolean>().value;
            Ok(GCRef::wrap(self))
        } else if value.isinstance::<VMInt>() {
            self.value = value.as_const_type::<VMInt>().value != 0;
            Ok(GCRef::wrap(self))
        } else {
            return Err(VMVariableError::TypeError(
                value.clone(),
                "Cannot assign a value of non-boolean type".to_string(),
            ));
        }
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
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
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMNull::new()))
    }

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
        if value.isinstance::<VMNull>() {
            return Ok(GCRef::wrap(self));
        } else {
            return Err(VMVariableError::TypeError(
                value.clone(),
                "Cannot assign a value of non-null type".to_string(),
            ));
        }
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
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
            traceable: GCTraceable::new(Some(vec![key, value])),
        }
    }

    pub fn get_key(&self) -> GCRef {
        self.key.clone()
    }

    pub fn get_value(&self) -> GCRef {
        self.value.clone()
    }

    pub fn check_key(&self, other: GCRef) -> bool {
        try_binary_op_as_type!(self.key, eq, other; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple);
        false
    }

    pub fn eq(&self, other: GCRef) -> bool {
        if other.isinstance::<VMKeyVal>() {
            let other_kv = other.as_const_type::<VMKeyVal>();
            let key_eq = (||{
                try_binary_op_as_type!(self.key, eq, other_kv.key.clone(); VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple);
                false
            })();
            let value_eq = (||{
                try_binary_op_as_type!(self.value, eq, other_kv.value.clone(); VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple);
                false
            })();
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
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMKeyVal::new(self.key.clone(), self.value.clone())))
    }

    fn assgin(&mut self, value: GCRef)  -> Result<GCRef, VMVariableError> {
        self.traceable.remove_reference(&self.value);
        self.value = value.clone();
        self.traceable.add_reference(&mut self.value);
        Ok(value.clone())
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }
}


#[derive(Debug)]
pub struct VMNamed {
    pub key: GCRef,
    pub value: GCRef,
    traceable: GCTraceable,
}

impl VMNamed {
    pub fn new(key: GCRef, value: GCRef) -> Self {
        VMNamed {
            key: key.clone(),
            value: value.clone(),
            traceable: GCTraceable::new(Some(vec![key, value])),
        }
    }

    pub fn get_key(&self) -> GCRef {
        self.key.clone()
    }

    pub fn get_value(&self) -> GCRef {
        self.value.clone()
    }

    pub fn check_key(&self, other: GCRef) -> bool {
        try_binary_op_as_type!(self.key, eq, other; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed);
        false
    }

    pub fn eq(&self, other: GCRef) -> bool {
        if other.isinstance::<VMNamed>() {
            let other_kv = other.as_const_type::<VMNamed>();
            let key_eq = (||{
                try_binary_op_as_type!(self.key, eq, other_kv.key.clone(); VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed);
                false
            })();
            let value_eq = (||{
                try_binary_op_as_type!(self.value, eq, other_kv.value.clone(); VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed);
                false
            })();
            return key_eq && value_eq;
        } else {
            false
        }
    }
}

impl GCObject for VMNamed {
    fn free(&mut self) {
        self.traceable.remove_reference(&self.key);
        self.traceable.remove_reference(&self.value);
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMNamed {
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMNamed::new(self.key.clone(), self.value.clone())))
    }

    fn assgin(&mut self, value: GCRef)  -> Result<GCRef, VMVariableError> {
        self.traceable.remove_reference(&self.value);
        self.value = value.clone();
        self.traceable.add_reference(&mut self.value);
        Ok(value.clone())
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }
}


#[derive(Debug)]
pub struct VMTuple {
    pub values: Vec<GCRef>,
    traceable: GCTraceable,
}

impl VMTuple {
    pub fn new(values: Vec<GCRef>) -> Self {
        // 创建对象并设置引用跟踪
        VMTuple {
            values: values.clone(),
            traceable: GCTraceable::new(Some(values)),
        }
    }

    pub fn get(&self, index: usize) -> Option<GCRef> {
        if index < self.values.len() {
            Some(self.values[index].clone())
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn eq(&self, other: GCRef) -> bool {
        if other.isinstance::<VMTuple>() {
            let other_tuple = other.as_const_type::<VMTuple>();
            
            if self.values.len() != other_tuple.values.len() {
                return false;
            }
            
            // 比较每个元素
            for (i, val) in self.values.iter().enumerate() {
                let other_val = &other_tuple.values[i];
                
                // 使用元素的eq方法进行比较
                let eq = (|| {
                    try_binary_op_as_type!(val.clone(), eq, other_val.clone(); 
                        VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed);
                    false
                })();
                
                if !eq {
                    return false;
                }
            }
            
            return true;
        } else {
            false
        }
    }

    pub fn get_member(&self, key: GCRef) -> Result<GCRef, VMVariableError> {
        for val in &self.values {
            if val.isinstance::<VMKeyVal>() {
                let kv = val.as_const_type::<VMKeyVal>();
                if kv.check_key(key.clone()) {
                    return Ok(kv.get_value());
                }
            }
        }
        Err(VMVariableError::KeyNotFound(
            key.clone(),
            self.object_ref()?,
        ))
    }

    pub fn index_of(&self, index:GCRef) -> Result<GCRef, VMVariableError> {
        if !index.isinstance::<VMInt>() {
            return Err(VMVariableError::TypeError(
                index.clone(),
                "Index must be an integer".to_string(),
            ));
        }
        let idx = index.as_const_type::<VMInt>().value;
        if idx < 0 {
            return Err(VMVariableError::ValueError(
                index.clone(),
                "Index must be a non-negative integer".to_string(),
            ));
        }
        let idx = idx as usize;
        if idx >= self.values.len() {
            return Err(VMVariableError::ValueError(
                index.clone(),
                "Index out of bounds".to_string(),
            ));
        }
        Ok(self.values[idx].clone())
    }
        

}

impl GCObject for VMTuple {
    fn free(&mut self) {
        // 移除对所有元素的引用
        for value in &self.values {
            self.traceable.remove_reference(value);
        }
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMTuple {
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        // 深拷贝元组中的每个元素
        let mut new_values = Vec::new();
        for value in &self.values {
            let copied_value = (||{
                try_copy_as_type!(value.clone(), gc_system; 
                    VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed);
                Err(VMVariableError::CopyError(
                    value.clone(),
                    "Cannot copy a value of non-copyable type".to_string(),
                ))
            })()?;
            new_values.push(copied_value);
        }
        // 创建新的元组对象
        let new_tuple = gc_system.new_object(VMTuple::new(new_values));
        Ok(new_tuple)
    }

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
        if value.isinstance::<VMTuple>() {
            // 移除对当前所有元素的引用
            for val in &self.values {
                self.traceable.remove_reference(val);
            }
            
            // 复制新元组的元素引用
            let other_tuple = value.as_const_type::<VMTuple>();
            self.values = other_tuple.values.clone();
            
            // 添加对新元素的引用
            for val in &self.values {
                self.traceable.add_reference(&mut val.clone());
            }
            Ok(GCRef::wrap(self))
        } else {
            Err(VMVariableError::TypeError(
                value.clone(),
                "Cannot assign a value of non-tuple type".to_string(),
            ))
        }
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }
}


#[derive(Debug)]
pub struct VMInstructions {
    pub instructions: Vec<IR>,
    pub func_ips: HashMap<String, usize>,
    traceable: GCTraceable,
}

impl VMInstructions {
    pub fn new(instructions: Vec<IR>, func_ips: HashMap<String, usize>) -> Self {
        VMInstructions {
            instructions,
            func_ips,
            traceable: GCTraceable::new(None),
        }
    }

}

impl GCObject for VMInstructions {
    fn free(&mut self) {
        // 不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}
impl VMObject for VMInstructions {
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMInstructions::new(
            self.instructions.clone(),
            self.func_ips.clone(),
        )))
    }

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
        panic!("Cannot assign a value to VMInstructions");
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }
}

#[derive(Debug)]
pub struct VMLambda {
    pub code_position: usize,
    pub signature: String,
    pub default_args_tuple: GCRef,
    pub self_object: Option<GCRef>,
    pub lambda_instructions: GCRef,
    traceable: GCTraceable,
}

impl VMLambda {
    pub fn new(
        code_position: usize,
        signature: String,
        default_args_tuple: GCRef,
        self_object: Option<GCRef>,
        lambda_instructions: GCRef,
    ) -> Self {
        if !lambda_instructions.isinstance::<VMInstructions>() {
            panic!("lambda_instructions must be a VMInstructions");
        }
        if !default_args_tuple.isinstance::<VMTuple>() {
            panic!("default_args_tuple must be a VMTuple");
        }
        VMLambda {
            code_position,
            signature,
            default_args_tuple,
            self_object,
            lambda_instructions,
            traceable: GCTraceable::new(None),
        }
    }

    pub fn set_self_object(&mut self, self_object: GCRef) {
        if !self.self_object.is_none() {
            self.traceable.remove_reference(&self.self_object.clone().unwrap());
        }
        self.self_object = Some(self_object);
        self.traceable.add_reference(&mut self.self_object.clone().unwrap());
    }
}

impl GCObject for VMLambda {
    fn free(&mut self) {
        if !self.self_object.is_none() {
            self.traceable.remove_reference(&self.self_object.clone().unwrap());
        }
        self.traceable.remove_reference(&self.default_args_tuple);
        self.traceable.remove_reference(&self.lambda_instructions);
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMLambda {
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        
        let new_default_args_tuple = self.default_args_tuple.as_const_type::<VMTuple>().copy(gc_system)?;
        
        Ok(gc_system.new_object(VMLambda::new(
            self.code_position,
            self.signature.clone(),
            new_default_args_tuple,
            self.self_object.clone(),
            self.lambda_instructions.clone(),
        )))
    }

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
        panic!("Cannot assign a value to VMLambda");
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }
}

#[derive(Debug)]
pub struct VMNativeFunction{
    // 包装rust函数， 函数定义为 fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>
    pub function: fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
    traceable: GCTraceable,
}

impl VMNativeFunction {
    pub fn new(function: fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>) -> Self {
        VMNativeFunction {
            function,
            traceable: GCTraceable::new(None),
        }
    }

    pub fn call(&self, args: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        (self.function)(args, gc_system)
    }
}
impl GCObject for VMNativeFunction {
    fn free(&mut self) {
        // 不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        return &mut self.traceable;
    }
}

impl VMObject for VMNativeFunction {
    fn copy(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMNativeFunction::new(self.function)))
    }

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
        panic!("Cannot assign a value to VMNativeFunction");
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }
}