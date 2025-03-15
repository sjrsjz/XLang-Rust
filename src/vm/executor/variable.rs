use std::{collections::HashMap, f64::consts::E, fmt::Debug};

use crate::vm::ir::IR;

use super::super::gc::gc::{GCObject, GCRef, GCSystem, GCTraceable};

#[derive(Debug, Clone)]
pub enum VMStackObject {
    LastIP(usize, bool),
    VMObject(GCRef),
}

#[derive(Debug)]
pub enum VMVariableError {
    TypeError(GCRef, String),
    ValueError(GCRef, String),
    KeyNotFound(GCRef, GCRef),   // 键未找到
    ValueNotFound(GCRef, GCRef), // 值未找到
    IndexNotFound(GCRef, GCRef), // 索引未找到
    CopyError(GCRef, String),
    AssignError(GCRef, String),
    ReferenceError(GCRef, String),
}

impl VMVariableError {
    pub fn to_string(&self) -> String {
        match self {
            VMVariableError::TypeError(gc_ref, msg) => format!(
                "TypeError: {}: {}",
                try_repr_vmobject(gc_ref.clone()).unwrap_or(format!("{:?}", gc_ref)),
                msg
            ),
            VMVariableError::ValueError(gc_ref, msg) => format!(
                "ValueError: {}: {}",
                try_repr_vmobject(gc_ref.clone()).unwrap_or(format!("{:?}", gc_ref)),
                msg
            ),

            VMVariableError::KeyNotFound(key, gc_ref) => format!(
                "KeyNotFound: {} in {}",
                try_repr_vmobject(key.clone()).unwrap_or(format!("{:?}", key)),
                try_repr_vmobject(gc_ref.clone()).unwrap_or(format!("{:?}", gc_ref))
            ),
            VMVariableError::ValueNotFound(value, gc_ref) => format!(
                "ValueNotFound: {} in {}",
                try_repr_vmobject(value.clone()).unwrap_or(format!("{:?}", value)),
                try_repr_vmobject(gc_ref.clone()).unwrap_or(format!("{:?}", gc_ref))
            ),
            VMVariableError::IndexNotFound(index, gc_ref) => format!(
                "IndexNotFound: {} in {}",
                try_repr_vmobject(index.clone()).unwrap_or(format!("{:?}", index)),
                try_repr_vmobject(gc_ref.clone()).unwrap_or(format!("{:?}", gc_ref))
            ),
            VMVariableError::CopyError(gc_ref, msg) => format!(
                "CopyError: {}: {}",
                try_repr_vmobject(gc_ref.clone()).unwrap_or(format!("{:?}", gc_ref)),
                msg
            ),
            VMVariableError::AssignError(gc_ref, msg) => format!(
                "AssignError: {}: {}",
                try_repr_vmobject(gc_ref.clone()).unwrap_or(format!("{:?}", gc_ref)),
                msg
            ),
            VMVariableError::ReferenceError(gc_ref, msg) => format!(
                "ReferenceError: {}: {}",
                try_repr_vmobject(gc_ref.clone()).unwrap_or(format!("{:?}", gc_ref)),
                msg
            ),
        }
    }
}

pub fn try_repr_vmobject(value: GCRef) -> Result<String, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return Ok(int.value.to_string());
    } else if value.isinstance::<VMString>() {
        let string = value.as_const_type::<VMString>();
        return Ok(string.value.clone());
    } else if value.isinstance::<VMFloat>() {
        let float = value.as_const_type::<VMFloat>();
        return Ok(float.value.to_string());
    } else if value.isinstance::<VMBoolean>() {
        let boolean = value.as_const_type::<VMBoolean>();
        return Ok(boolean.value.to_string());
    } else if value.isinstance::<VMNull>() {
        return Ok("null".to_string());
    } else if value.isinstance::<VMKeyVal>() {
        let kv = value.as_const_type::<VMKeyVal>();
        let key = try_repr_vmobject(kv.get_key())?;
        let value = try_repr_vmobject(kv.get_value())?;
        return Ok(format!("{}: {}", key, value));
    } else if value.isinstance::<VMNamed>() {
        let named = value.as_const_type::<VMNamed>();
        let key = try_repr_vmobject(named.get_key())?;
        let value = try_repr_vmobject(named.get_value())?;
        return Ok(format!("{} => {}", key, value));
    } else if value.isinstance::<VMTuple>() {
        let tuple = value.as_const_type::<VMTuple>();
        let mut repr = String::new();
        for (i, val) in tuple.values.iter().enumerate() {
            if i > 0 {
                repr.push_str(", ");
            }
            repr.push_str(&try_repr_vmobject(val.clone())?);
        }
        return Ok(format!("({})", repr));
    } else if value.isinstance::<VMLambda>() {
        let lambda = value.as_const_type::<VMLambda>();
        return Ok(format!("VMLambda({})", lambda.signature));
    } else if value.isinstance::<VMInstructions>() {
        return Ok("VMInstructions".to_string());
    } else if value.isinstance::<VMVariableWrapper>() {
        let wrapper = value.as_const_type::<VMVariableWrapper>();
        return Ok(format!(
            "VMVariableWrapper({})",
            try_repr_vmobject(wrapper.value_ref.clone())?
        ));
    } else if value.isinstance::<VMNativeFunction>() {
        //let native_func = value.as_const_type::<VMNativeFunction>();
        return Ok(format!("VMNativeFunction()"));
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot represent a non-representable type".to_string(),
    ))
}

pub fn debug_print_repr(value: GCRef) {
    match try_repr_vmobject(value) {
        Ok(repr) => println!("Repr: {}", repr),
        Err(err) => println!("Cannot repr: {:?}", err),
    }
}

pub fn try_add_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.add(other, gc_system);
    } else if value.isinstance::<VMString>() {
        let string = value.as_const_type::<VMString>();
        return string.add(other, gc_system);
    } else if value.isinstance::<VMFloat>() {
        let float = value.as_const_type::<VMFloat>();
        return float.add(other, gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot add a value of non-addable type".to_string(),
    ))
}

pub fn try_sub_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.sub(other, gc_system);
    } else if value.isinstance::<VMFloat>() {
        let float = value.as_const_type::<VMFloat>();
        return float.sub(other, gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot subtract a value of non-subtractable type".to_string(),
    ))
}

pub fn try_mul_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.mul(other, gc_system);
    } else if value.isinstance::<VMFloat>() {
        let float = value.as_const_type::<VMFloat>();
        return float.mul(other, gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot multiply a value of non-multiplicable type".to_string(),
    ))
}

pub fn try_div_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.div(other, gc_system);
    } else if value.isinstance::<VMFloat>() {
        let float = value.as_const_type::<VMFloat>();
        return float.div(other, gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot divide a value of non-divisible type".to_string(),
    ))
}

pub fn try_mod_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.mod_op(other, gc_system);
    } else if value.isinstance::<VMFloat>() {
        let float = value.as_const_type::<VMFloat>();
        return float.mod_op(other, gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot mod a value of non-modable type".to_string(),
    ))
}

pub fn try_bitwise_and_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.bitwise_and(other, gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot bitwise and a value of non-bitwise-andable type".to_string(),
    ))
}

pub fn try_bitwise_or_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.bitwise_or(other, gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot bitwise or a value of non-bitwise-orable type".to_string(),
    ))
}

pub fn try_bitwise_xor_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.bitwise_xor(other, gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot bitwise xor a value of non-bitwise-xorable type".to_string(),
    ))
}

pub fn try_bitwise_not_as_vmobject(
    value: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.bitwise_not(gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot bitwise not a value of non-bitwise-notable type".to_string(),
    ))
}

pub fn try_shift_left_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.shift_left(other, gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot shift left a value of non-shift-leftable type".to_string(),
    ))
}

pub fn try_shift_right_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.shift_right(other, gc_system);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot shift right a value of non-shift-rightable type".to_string(),
    ))
}

pub fn try_less_than_as_vmobject(value: GCRef, other: GCRef) -> Result<bool, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.less_than(other);
    } else if value.isinstance::<VMFloat>() {
        let float = value.as_const_type::<VMFloat>();
        return float.less_than(other);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot less than a value of non-less-thanable type".to_string(),
    ))
}

pub fn try_greater_than_as_vmobject(value: GCRef, other: GCRef) -> Result<bool, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.greater_than(other);
    } else if value.isinstance::<VMFloat>() {
        let float = value.as_const_type::<VMFloat>();
        return float.greater_than(other);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot greater than a value of non-greater-thanable type".to_string(),
    ))
}

pub fn try_and_as_vmobject(value: GCRef, other: GCRef) -> Result<bool, VMVariableError> {
    if value.isinstance::<VMBoolean>() {
        let boolean = value.as_const_type::<VMBoolean>();
        return boolean.and(other);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot and a value of non-andable type".to_string(),
    ))
}

pub fn try_or_as_vmobject(value: GCRef, other: GCRef) -> Result<bool, VMVariableError> {
    if value.isinstance::<VMBoolean>() {
        let boolean = value.as_const_type::<VMBoolean>();
        return boolean.or(other);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot or a value of non-orable type".to_string(),
    ))
}

pub fn try_not_as_vmobject(value: GCRef) -> Result<bool, VMVariableError> {
    if value.isinstance::<VMBoolean>() {
        let boolean = value.as_const_type::<VMBoolean>();
        return boolean.not();
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot not a value of non-notable type".to_string(),
    ))
}

pub fn try_get_attr_as_vmobject(value: GCRef, attr: GCRef) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMNamed>() {
        let named = value.as_const_type::<VMNamed>();
        if named.check_key(attr.clone()) {
            return Ok(named.get_value());
        }
    } else if value.isinstance::<VMKeyVal>() {
        let kv = value.as_const_type::<VMKeyVal>();
        if kv.check_key(attr.clone()) {
            return Ok(kv.get_value());
        }
    } else if value.isinstance::<VMTuple>() {
        let tuple = value.as_const_type::<VMTuple>();
        return tuple.get_member(attr);
    }
    Err(VMVariableError::KeyNotFound(attr, value))
}

pub fn try_index_of_as_vmobject(
    value: GCRef,
    index: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMTuple>() {
        let tuple = value.as_const_type::<VMTuple>();
        return tuple.index_of(index, gc_system);
    }
    if value.isinstance::<VMString>() {
        let string = value.as_const_type::<VMString>();
        return string.index_of(index, gc_system);
    }
    Err(VMVariableError::IndexNotFound(index, value))
}

pub fn try_key_of_as_vmobject(value: GCRef) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMKeyVal>() {
        let kv = value.as_const_type::<VMKeyVal>();
        return Ok(kv.get_key());
    } else if value.isinstance::<VMNamed>() {
        let named = value.as_const_type::<VMNamed>();
        return Ok(named.get_key());
    }
    Err(VMVariableError::KeyNotFound(value.clone(), value))
}

pub fn try_value_of_as_vmobject(value: GCRef) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMKeyVal>() {
        let kv = value.as_const_type::<VMKeyVal>();
        return Ok(kv.get_value());
    } else if value.isinstance::<VMNamed>() {
        let named = value.as_const_type::<VMNamed>();
        return Ok(named.get_value());
    }
    Err(VMVariableError::ValueNotFound(value.clone(), value))
}

#[macro_export]
macro_rules! try_copy_as_type {
    ($value:expr, $gc_system:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_const_type::<$t>().copy($gc_system);
            }
        )+
    };
}

pub fn try_copy_as_vmobject(
    value: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    try_copy_as_type!(value, gc_system; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMLambda, VMInstructions, VMVariableWrapper, VMNativeFunction);
    Err(VMVariableError::CopyError(
        value.clone(),
        "Cannot copy a value of non-copyable type".to_string(),
    ))
}

#[macro_export]
macro_rules! try_assgin_as_type {
    ($value:expr, $other:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_type::<$t>().assgin($other);
            }
        )+
    };
}

pub fn try_assgin_as_vmobject(value: GCRef, other: GCRef) -> Result<GCRef, VMVariableError> {
    try_assgin_as_type!(value, other; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMLambda, VMInstructions, VMVariableWrapper, VMNativeFunction);
    Err(VMVariableError::AssignError(
        value.clone(),
        "Cannot assign a value of non-assignable type".to_string(),
    ))
}

#[macro_export]
macro_rules! try_ref_as_type {
    ($value:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_const_type::<$t>().object_ref();
            }
        )+
    };
}

pub fn try_ref_as_vmobject(value: GCRef) -> Result<GCRef, VMVariableError> {
    try_ref_as_type!(value; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMLambda, VMInstructions, VMVariableWrapper, VMNativeFunction);
    Err(VMVariableError::ReferenceError(
        value.clone(),
        "Cannot get reference of a non-referenceable type".to_string(),
    ))
}

#[macro_export]
macro_rules! try_binary_op_as_type {
    ($value:expr, $op:ident, $other:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_const_type::<$t>().$op($other);
            }
        )+
    };
}

pub fn try_eq_as_vmobject(value: GCRef, other: GCRef) -> bool {
    try_binary_op_as_type!(value, eq, other; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed);
    false
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
        try_copy_as_vmobject(self.value_ref.clone(), gc_system)
    }

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
        self.traceable.remove_reference(&self.value_ref);
        self.value_ref = value;
        self.traceable.add_reference(&mut self.value_ref);
        Ok(self.value_ref.clone())
    }

    fn object_ref(&self) -> Result<GCRef, VMVariableError> {
        return try_ref_as_vmobject(self.value_ref.clone());
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

    pub fn add(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMInt::new(self.value + other_int.value)));
        } else if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value as f64 + other_float.value)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot add a value of non-integer type".to_string(),
        ))
    }

    pub fn sub(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMInt::new(self.value - other_int.value)));
        } else if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value as f64 - other_float.value)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot subtract a value of non-integer type".to_string(),
        ))
    }

    pub fn mul(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMInt::new(self.value * other_int.value)));
        } else if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value as f64 * other_float.value)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot multiply a value of non-integer type".to_string(),
        ))
    }

    pub fn div(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(
                gc_system.new_object(VMFloat::new(self.value as f64 / other_int.value as f64))
            );
        } else if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value as f64 / other_float.value)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot divide a value of non-integer type".to_string(),
        ))
    }

    pub fn mod_op(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMInt::new(self.value % other_int.value)));
        } else if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value as f64 % other_float.value)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot mod a value of non-integer type".to_string(),
        ))
    }

    pub fn bitwise_and(
        &self,
        other: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMInt::new(self.value & other_int.value)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot bitwise and a value of non-integer type".to_string(),
        ))
    }

    pub fn bitwise_or(
        &self,
        other: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMInt::new(self.value | other_int.value)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot bitwise or a value of non-integer type".to_string(),
        ))
    }

    pub fn bitwise_xor(
        &self,
        other: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMInt::new(self.value ^ other_int.value)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot bitwise xor a value of non-integer type".to_string(),
        ))
    }

    pub fn bitwise_not(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        return Ok(gc_system.new_object(VMInt::new(!self.value)));
    }

    pub fn shift_left(
        &self,
        other: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMInt::new(self.value << other_int.value)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot shift left a value of non-integer type".to_string(),
        ))
    }

    pub fn shift_right(
        &self,
        other: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMInt::new(self.value >> other_int.value)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot shift right a value of non-integer type".to_string(),
        ))
    }

    pub fn less_than(&self, other: GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(self.value < other_int.value);
        } else if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok((self.value as f64) < other_float.value);
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot compare a value of non-integer type".to_string(),
        ))
    }

    pub fn greater_than(&self, other: GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(self.value > other_int.value);
        } else if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok((self.value as f64) > other_float.value);
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot compare a value of non-integer type".to_string(),
        ))
    }

    pub fn to_float(&self) -> Result<f64, VMVariableError> {
        return Ok(self.value as f64);
    }

    pub fn to_string(&self) -> Result<String, VMVariableError> {
        return Ok(self.value.to_string());
    }
    pub fn to_bool(&self) -> Result<bool, VMVariableError> {
        return Ok(self.value != 0);
    }

    pub fn to_int(&self) -> Result<i64, VMVariableError> {
        return Ok(self.value);
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

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
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

    pub fn to_string(&self) -> Result<String, VMVariableError> {
        return Ok(self.value.clone());
    }

    pub fn to_bool(&self) -> Result<bool, VMVariableError> {
        return Ok(!self.value.is_empty());
    }

    pub fn to_int(&self) -> Result<i64, VMVariableError> {
        return self.value.parse::<i64>().map_err(|_| {
            VMVariableError::ValueError(
                GCRef::wrap(self),
                "Cannot convert string to int".to_string(),
            )
        });
    }

    pub fn to_float(&self) -> Result<f64, VMVariableError> {
        return self.value.parse::<f64>().map_err(|_| {
            VMVariableError::ValueError(
                GCRef::wrap(self),
                "Cannot convert string to float".to_string(),
            )
        });
    }

    pub fn add(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMString>() {
            let other_string = other.as_const_type::<VMString>();
            return Ok(gc_system.new_object(VMString::new(format!(
                "{}{}",
                self.value, other_string.value
            ))));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot add a value of non-string type".to_string(),
        ))
    }

    pub fn index_of(
        &self,
        index: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMVariableError> {
        if index.isinstance::<VMInt>() {
            let index_int = index.as_const_type::<VMInt>();
            if index_int.value < 0 || index_int.value >= self.value.len() as i64 {
                return Err(VMVariableError::IndexNotFound(
                    index.clone(),
                    GCRef::wrap(self),
                ));
            }
            let char = self.value.chars().nth(index_int.value as usize).unwrap();

            return Ok(gc_system.new_object(VMString::new(char.to_string())));
        }
        Err(VMVariableError::TypeError(
            index.clone(),
            "Cannot index a string with a non-integer type".to_string(),
        ))
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

    pub fn add(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value + other_float.value)));
        } else if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMFloat::new(self.value + other_int.value as f64)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot add a value of non-float type".to_string(),
        ))
    }

    pub fn sub(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value - other_float.value)));
        } else if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMFloat::new(self.value - other_int.value as f64)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot subtract a value of non-float type".to_string(),
        ))
    }

    pub fn mul(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value * other_float.value)));
        } else if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMFloat::new(self.value * other_int.value as f64)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot multiply a value of non-float type".to_string(),
        ))
    }

    pub fn div(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value / other_float.value)));
        } else if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMFloat::new(self.value / other_int.value as f64)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot divide a value of non-float type".to_string(),
        ))
    }

    pub fn mod_op(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value % other_float.value)));
        } else if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMFloat::new(self.value % other_int.value as f64)));
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot mod a value of non-float type".to_string(),
        ))
    }

    pub fn less_than(&self, other: GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(self.value < other_float.value);
        } else if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(self.value < other_int.value as f64);
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot compare a value of non-float type".to_string(),
        ))
    }

    pub fn greater_than(&self, other: GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(self.value > other_float.value);
        } else if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(self.value > other_int.value as f64);
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot compare a value of non-float type".to_string(),
        ))
    }

    pub fn to_int(&self) -> Result<i64, VMVariableError> {
        return Ok(self.value as i64);
    }
    pub fn to_string(&self) -> Result<String, VMVariableError> {
        return Ok(self.value.to_string());
    }
    pub fn to_boolean(&self) -> Result<bool, VMVariableError> {
        return Ok(self.value != 0.0);
    }
    pub fn to_float(&self) -> Result<f64, VMVariableError> {
        return Ok(self.value);
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

    pub fn and(&self, other: GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMBoolean>() {
            let other_bool = other.as_const_type::<VMBoolean>();
            return Ok(self.value && other_bool.value);
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot perform logical AND on non-boolean type".to_string(),
        ))
    }

    pub fn or(&self, other: GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMBoolean>() {
            let other_bool = other.as_const_type::<VMBoolean>();
            return Ok(self.value || other_bool.value);
        }
        Err(VMVariableError::TypeError(
            other.clone(),
            "Cannot perform logical OR on non-boolean type".to_string(),
        ))
    }

    pub fn not(&self) -> Result<bool, VMVariableError> {
        return Ok(!self.value);
    }

    pub fn to_int(&self) -> Result<i64, VMVariableError> {
        return Ok(if self.value { 1 } else { 0 });
    }
    pub fn to_float(&self) -> Result<f64, VMVariableError> {
        return Ok(if self.value { 1.0 } else { 0.0 });
    }
    pub fn to_string(&self) -> Result<String, VMVariableError> {
        return Ok(self.value.to_string());
    }
    pub fn to_boolean(&self) -> Result<bool, VMVariableError> {
        return Ok(self.value);
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
        try_eq_as_vmobject(self.value.clone(), other)
    }

    pub fn eq(&self, other: GCRef) -> bool {
        if other.isinstance::<VMKeyVal>() {
            let other_kv = other.as_const_type::<VMKeyVal>();
            let key_eq = try_eq_as_vmobject(self.key.clone(), other_kv.key.clone());
            let value_eq = try_eq_as_vmobject(self.value.clone(), other_kv.value.clone());
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

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
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
        try_eq_as_vmobject(self.key.clone(), other)
    }

    pub fn eq(&self, other: GCRef) -> bool {
        if other.isinstance::<VMNamed>() {
            let other_kv = other.as_const_type::<VMNamed>();
            let key_eq = try_eq_as_vmobject(self.key.clone(), other_kv.key.clone());
            let value_eq = try_eq_as_vmobject(self.value.clone(), other_kv.value.clone());
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

    fn assgin(&mut self, value: GCRef) -> Result<GCRef, VMVariableError> {
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
                let eq = try_eq_as_vmobject(val.clone(), other_val.clone());

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

    pub fn index_of(
        &self,
        index: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMVariableError> {
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
    /// 将另一个元组的成员赋值给当前元组
    /// 先尝试将所有 VMNamed 对象按照键进行赋值
    /// 剩下的值按照顺序赋值到非命名位置
    pub fn assgin_members(&mut self, other: GCRef) -> Result<GCRef, VMVariableError> {
        // 确保参数是元组
        if !other.isinstance::<VMTuple>() {
            return Err(VMVariableError::TypeError(
                other.clone(),
                "Expected a tuple".to_string(),
            ));
        }

        let other_tuple = other.as_const_type::<VMTuple>();

        // 分离命名参数和普通值
        let mut key_values = Vec::new();
        let mut normal_values = Vec::new();
        let mut assgined = vec![false; self.values.len()];

        for item in &other_tuple.values {
            if item.isinstance::<VMNamed>() {
                key_values.push(item.clone());
            } else {
                normal_values.push(item.clone());
            }
        }

        // 处理所有命名参数
        for kv in key_values {
            let mut found: bool = false;
            // 在当前元组中查找匹配的键
            for i in 0..self.values.len() {
                if self.values[i].isinstance::<VMNamed>() {
                    let self_named = self.values[i].as_const_type::<VMNamed>();
                    let kv_named = kv.as_const_type::<VMNamed>();

                    // 检查键是否匹配
                    if try_eq_as_vmobject(self_named.get_key(), kv_named.get_key()) {
                        // 找到匹配的键，进行赋值
                        let value_ref = self.values[i].clone();
                        try_assgin_as_vmobject(value_ref, kv_named.get_value())?;
                        assgined[i] = true;
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                // 如果没有找到匹配的键，添加新的键值对
                self.values.push(kv);
                self.traceable
                    .add_reference(&mut self.values.last().unwrap().clone());
            }
        }

        // 按顺序处理普通值
        let mut normal_index = 0;
        for value in normal_values {
            // 寻找一个非命名且未赋值的位置
            while normal_index < assgined.len()
                && (self.values[normal_index].isinstance::<VMNamed>() && assgined[normal_index])
            {
                normal_index += 1;
            }

            if normal_index < self.values.len() {
                // 找到位置，进行赋值
                let value_ref = self.values[normal_index].clone();
                try_assgin_as_vmobject(value_ref, value)?;
                normal_index += 1;
            } else {
                // 没有更多位置，追加到末尾
                self.values.push(value.clone());
                self.traceable
                    .add_reference(&mut self.values.last().unwrap().clone());
            }
        }

        Ok(GCRef::wrap(self))
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
            let copied_value = (|| {
                return try_copy_as_vmobject(value.clone(), gc_system);
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
            default_args_tuple: default_args_tuple.clone(),
            self_object,
            lambda_instructions: lambda_instructions.clone(),
            traceable: GCTraceable::new(Some(vec![default_args_tuple, lambda_instructions])),
        }
    }

    pub fn set_self_object(&mut self, self_object: GCRef) {
        if !self.self_object.is_none() {
            self.traceable
                .remove_reference(&self.self_object.clone().unwrap());
        }
        self.self_object = Some(self_object);
        self.traceable
            .add_reference(&mut self.self_object.clone().unwrap());
    }
}

impl GCObject for VMLambda {
    fn free(&mut self) {
        if !self.self_object.is_none() {
            self.traceable
                .remove_reference(&self.self_object.clone().unwrap());
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
        let new_default_args_tuple = self
            .default_args_tuple
            .as_const_type::<VMTuple>()
            .copy(gc_system)?;

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
pub struct VMNativeFunction {
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
