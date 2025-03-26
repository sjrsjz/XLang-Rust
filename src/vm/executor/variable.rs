use base64::{self, Engine};
use colored::Colorize;
/**
 * 约定：
 * - vmobject：虚拟机对象
 * - value_ref：获得值引用，不产生clone行为
 * - 任何new对象的行为都需要使用gc_system，并且会产生一个native_gcref_object_count，虚拟机必须在某处drop_ref直到为0
 *
 */
use std::{collections::HashMap, fmt::Debug};

use crate::vm::ir::IR;

use super::super::gc::gc::{GCObject, GCRef, GCSystem, GCTraceable};

#[derive(Debug, Clone)]
pub enum VMStackObject {
    LastIP(GCRef, usize, bool),
    VMObject(GCRef),
}

#[derive(Debug)]
pub enum VMVariableError {
    TypeError(GCRef, String),
    ValueError2Param(GCRef, GCRef, String),
    ValueError(GCRef, String),
    KeyNotFound(GCRef, GCRef),   // 键未找到
    ValueNotFound(GCRef, GCRef), // 值未找到
    IndexNotFound(GCRef, GCRef), // 索引未找到
    CopyError(GCRef, String),
    AssignError(GCRef, String),
    ReferenceError(GCRef, String),
    OverflowError(GCRef, GCRef, String),
    UnstableRef(GCRef, String),
}

impl VMVariableError {
    pub fn to_string(&self) -> String {
        match self {
            VMVariableError::TypeError(gc_ref, msg) => format!(
                "TypeError: {}: {}",
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref)),
                msg
            ),
            VMVariableError::ValueError2Param(gc_ref, other, msg) => format!(
                "ValueError: {}: {} & {}",
                msg,
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref)),
                try_repr_vmobject(other.clone(), None).unwrap_or(format!("{:?}", other)),
            ),
            VMVariableError::ValueError(gc_ref, msg) => format!(
                "ValueError: {}: {}",
                msg,
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref)),
            ),
            VMVariableError::KeyNotFound(key, gc_ref) => format!(
                "KeyNotFound: {} in {}",
                try_repr_vmobject(key.clone(), None).unwrap_or(format!("{:?}", key)),
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref))
            ),
            VMVariableError::ValueNotFound(value, gc_ref) => format!(
                "ValueNotFound: {} in {}",
                try_repr_vmobject(value.clone(), None).unwrap_or(format!("{:?}", value)),
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref))
            ),
            VMVariableError::IndexNotFound(index, gc_ref) => format!(
                "IndexNotFound: {} in {}",
                try_repr_vmobject(index.clone(), None).unwrap_or(format!("{:?}", index)),
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref))
            ),
            VMVariableError::CopyError(gc_ref, msg) => format!(
                "CopyError: {}: {}",
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref)),
                msg
            ),
            VMVariableError::AssignError(gc_ref, msg) => format!(
                "AssignError: {}: {}",
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref)),
                msg
            ),
            VMVariableError::ReferenceError(gc_ref, msg) => format!(
                "ReferenceError: {}: {}",
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref)),
                msg
            ),
            VMVariableError::OverflowError(gc_ref, other, msg) => format!(
                "OverflowError: {}: {} & {}",
                msg,
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref)),
                try_repr_vmobject(other.clone(), None).unwrap_or(format!("{:?}", other)),
            ),
            VMVariableError::UnstableRef(gc_ref, msg) => format!(
                "UnstableRef: {}: {}",
                try_repr_vmobject(gc_ref.clone(), None).unwrap_or(format!("{:?}", gc_ref)),
                msg
            ),
        }
    }
}

pub fn try_contains_as_vmobject(value: &GCRef, other: &GCRef) -> Result<bool, VMVariableError> {
    if value.isinstance::<VMString>() {
        let string = value.as_const_type::<VMString>();
        return string.contains(other);
    } else if value.isinstance::<VMTuple>() {
        let tuple = value.as_const_type::<VMTuple>();
        return tuple.contains(other);
    } else if value.isinstance::<VMRange>() {
        let range = value.as_const_type::<VMRange>();
        return range.contains(other);
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot check contains for a non-containable type".to_string(),
    ))
}

pub fn try_repr_vmobject(
    value: GCRef,
    ref_path: Option<Vec<GCRef>>,
) -> Result<String, VMVariableError> {
    // 检查循环引用
    if let Some(ref path) = ref_path {
        for prev_ref in path {
            if std::ptr::eq(prev_ref.get_const_reference(), value.get_const_reference()) {
                return Ok("<Cycled>".to_string());
            }
        }
    }

    // 创建新的引用路径，将当前对象添加到路径中
    let new_ref_path = if let Some(mut path) = ref_path {
        path.push(value.clone());
        Some(path)
    } else {
        Some(vec![value.clone()])
    };

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
        let key = try_repr_vmobject(kv.get_const_key().clone(), new_ref_path.clone())?;
        let value = try_repr_vmobject(kv.get_const_value().clone(), new_ref_path)?;
        return Ok(format!("{}: {}", key, value));
    } else if value.isinstance::<VMNamed>() {
        let named = value.as_const_type::<VMNamed>();
        let key = try_repr_vmobject(named.get_const_key().clone(), new_ref_path.clone())?;
        let value = try_repr_vmobject(named.get_const_value().clone(), new_ref_path)?;
        return Ok(format!("{} => {}", key, value));
    } else if value.isinstance::<VMTuple>() {
        let tuple = value.as_const_type::<VMTuple>();
        let mut repr = String::new();
        if tuple.values.is_empty() {
            return Ok("()".to_string());
        }
        if tuple.values.len() == 1 {
            return Ok(format!(
                "({},)",
                try_repr_vmobject(tuple.values[0].clone(), new_ref_path)?
            ));
        }
        for (i, val) in tuple.values.iter().enumerate() {
            if i > 0 {
                repr.push_str(", ");
            }
            repr.push_str(&try_repr_vmobject(val.clone(), new_ref_path.clone())?);
        }
        return Ok(format!("({})", repr));
    } else if value.isinstance::<VMLambda>() {
        let lambda = value.as_const_type::<VMLambda>();
        return Ok(format!(
            "{}::{} -> {}",
            lambda.signature,
            try_repr_vmobject(lambda.default_args_tuple.clone(), new_ref_path.clone())?,
            try_repr_vmobject(lambda.result.clone(), new_ref_path)?
        ));
    } else if value.isinstance::<VMInstructions>() {
        return Ok("VMInstructions".to_string());
    } else if value.isinstance::<VMVariableWrapper>() {
        let wrapper = value.as_const_type::<VMVariableWrapper>();
        return Ok(format!(
            "variable({})",
            try_repr_vmobject(wrapper.value_ref.clone(), new_ref_path)?
        ));
    } else if value.isinstance::<VMNativeFunction>() {
        return Ok(format!(
            "VMNativeFunction({:?})",
            value.as_const_type::<VMNativeFunction>().function
        ));
    } else if value.isinstance::<VMWrapper>() {
        let wrapper = value.as_const_type::<VMWrapper>();
        return Ok(format!(
            "wrap({})",
            try_repr_vmobject(wrapper.value_ref.clone(), new_ref_path)?
        ));
    } else if value.isinstance::<VMRange>() {
        let range = value.as_const_type::<VMRange>();
        return Ok(format!("{}..{}", range.start, range.end));
    } else if value.isinstance::<VMBytes>() {
        let bytes = value.as_const_type::<VMBytes>();
        return Ok(format!(
            "$\"{}\"",
            base64::engine::general_purpose::STANDARD
                .encode(&bytes.value)
                .chars()
                .collect::<String>()
        ));
    }
    Err(VMVariableError::TypeError(
        value.clone(),
        "Cannot represent a non-representable type".to_string(),
    ))
}

pub fn debug_print_repr(value: GCRef) {
    match try_repr_vmobject(value.clone(), None) {
        Ok(repr) => println!(
            "Repr:{}| {:?}, {:?} {}",
            value.get_const_traceable().native_gcref_object_count,
            value.get_const_reference() as *const (),
            value.get_const_traceable().references,
            repr
        ),
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
    } else if value.isinstance::<VMTuple>() {
        let tuple = value.as_const_type::<VMTuple>();
        return tuple.add(other, gc_system);
    } else if value.isinstance::<VMRange>() {
        let named = value.as_const_type::<VMRange>();
        return named.add(other, gc_system);
    } else if value.isinstance::<VMBytes>() {
        let bytes = value.as_const_type::<VMBytes>();
        return bytes.add(other, gc_system);
    }
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
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
    } else if value.isinstance::<VMRange>() {
        let named = value.as_const_type::<VMRange>();
        return named.sub(other, gc_system);
    }
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
        "Cannot divide a value of non-dividable type".to_string(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
        "Cannot mod a value of non-modable type".to_string(),
    ))
}

pub fn try_power_as_vmobject(
    value: GCRef,
    other: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMInt>() {
        let int = value.as_const_type::<VMInt>();
        return int.power(other, gc_system);
    } else if value.isinstance::<VMFloat>() {
        let float = value.as_const_type::<VMFloat>();
        return float.power(other, gc_system);
    }
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
        "Cannot power a value of non-powerable type".to_string(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
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
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
        "Cannot greater than a value of non-greater-thanable type".to_string(),
    ))
}

pub fn try_and_as_vmobject(value: GCRef, other: GCRef) -> Result<bool, VMVariableError> {
    if value.isinstance::<VMBoolean>() {
        let boolean = value.as_const_type::<VMBoolean>();
        return boolean.and(other);
    }
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
        "Cannot and a value of non-andable type".to_string(),
    ))
}

pub fn try_or_as_vmobject(value: GCRef, other: GCRef) -> Result<bool, VMVariableError> {
    if value.isinstance::<VMBoolean>() {
        let boolean = value.as_const_type::<VMBoolean>();
        return boolean.or(other);
    }
    Err(VMVariableError::ValueError2Param(
        value.clone(),
        other.clone(),
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

pub fn try_get_attr_as_vmobject<'t>(
    value: &'t mut GCRef,
    attr: &'t GCRef,
) -> Result<&'t mut GCRef, VMVariableError> {
    if value.isinstance::<VMTuple>() {
        let tuple = value.as_type::<VMTuple>();
        return tuple.get_member(attr);
    }
    Err(VMVariableError::KeyNotFound(attr.clone(), value.clone()))
}

pub fn try_index_of_as_vmobject(
    value: &mut GCRef,
    index: GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    if value.isinstance::<VMTuple>() {
        let tuple = value.as_type::<VMTuple>();
        return tuple.index_of(index, gc_system);
    }
    if value.isinstance::<VMString>() {
        let string = value.as_type::<VMString>();
        return string.index_of(index, gc_system);
    }
    if value.isinstance::<VMBytes>() {
        let range = value.as_type::<VMBytes>();
        return range.index_of(index, gc_system);
    }
    Err(VMVariableError::IndexNotFound(index.clone(), value.clone()))
}

pub fn try_key_of_as_vmobject(value: &mut GCRef) -> Result<&mut GCRef, VMVariableError> {
    if value.isinstance::<VMKeyVal>() {
        let kv = value.as_type::<VMKeyVal>();
        return Ok(kv.get_key());
    } else if value.isinstance::<VMNamed>() {
        let named = value.as_type::<VMNamed>();
        return Ok(named.get_key());
    } else if value.isinstance::<VMLambda>() {
        let wrapper = value.as_type::<VMLambda>();
        return Ok(wrapper.get_key());
    }
    Err(VMVariableError::KeyNotFound(value.clone(), value.clone()))
}

pub fn try_value_of_as_vmobject(value: &mut GCRef) -> Result<&mut GCRef, VMVariableError> {
    if value.isinstance::<VMKeyVal>() {
        let kv = value.as_type::<VMKeyVal>();
        return Ok(kv.get_value());
    } else if value.isinstance::<VMNamed>() {
        let named = value.as_type::<VMNamed>();
        return Ok(named.get_value());
    } else if value.isinstance::<VMWrapper>() {
        let wrapper = value.as_type::<VMWrapper>();
        return Ok(wrapper.get_value());
    } else if value.isinstance::<VMLambda>() {
        let wrapper = value.as_type::<VMLambda>();
        return Ok(wrapper.get_value());
    }
    Err(VMVariableError::ValueNotFound(value.clone(), value.clone()))
}

#[macro_export]
macro_rules! try_deepcopy_as_type {
    ($value:expr, $gc_system:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_type::<$t>().deepcopy($gc_system);
            }
        )+
    };
}

pub fn try_deepcopy_as_vmobject(
    value: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    try_deepcopy_as_type!(value, gc_system; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMLambda, VMInstructions, VMVariableWrapper, VMNativeFunction, VMWrapper, VMRange, VMBytes);
    Err(VMVariableError::CopyError(
        value.clone(),
        "Cannot deepcopy a value of non-copyable type".to_string(),
    ))
}

#[macro_export]
macro_rules! try_copy_as_type {
    ($value:expr, $gc_system:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_type::<$t>().copy($gc_system);
            }
        )+
    };
}

pub fn try_copy_as_vmobject(
    value: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    try_copy_as_type!(value, gc_system; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMLambda, VMInstructions, VMVariableWrapper, VMNativeFunction, VMWrapper, VMRange, VMBytes);
    Err(VMVariableError::CopyError(
        value.clone(),
        "Cannot copy a value of non-copyable type".to_string(),
    ))
}

#[macro_export]
macro_rules! try_assign_as_type {
    ($value:expr, $other:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_type::<$t>().assign($other);
            }
        )+
    };
}

pub fn try_assign_as_vmobject<'t>(
    value: &mut GCRef,
    other: &'t mut GCRef,
) -> Result<&'t mut GCRef, VMVariableError> {
    try_assign_as_type!(value, other; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMLambda, VMInstructions, VMVariableWrapper, VMNativeFunction, VMWrapper, VMRange, VMBytes);
    Err(VMVariableError::AssignError(
        value.clone(),
        "Cannot assign a value of non-assignable type".to_string(),
    ))
}

#[macro_export]
macro_rules! try_value_ref_as_type {
    ($value:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_type::<$t>().value_ref();
            }
        )+
    };
}

pub fn try_value_ref_as_vmobject(value: &mut GCRef) -> Result<GCRef, VMVariableError> {
    try_value_ref_as_type!(value; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMLambda, VMInstructions, VMVariableWrapper, VMNativeFunction, VMWrapper, VMRange, VMBytes);
    Err(VMVariableError::ReferenceError(
        value.clone(),
        "Cannot get reference of a non-referenceable type".to_string(),
    ))
}

#[macro_export]
macro_rules! try_value_const_ref_as_type {
    ($value:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return $value.as_const_type::<$t>().value_const_ref();
            }
        )+
    };
}

pub fn try_value_const_ref_as_vmobject(value: &GCRef) -> Result<GCRef, VMVariableError> {
    try_value_const_ref_as_type!(value; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMLambda, VMInstructions, VMVariableWrapper, VMNativeFunction, VMWrapper, VMRange, VMBytes);
    Err(VMVariableError::ReferenceError(
        value.clone(),
        "Cannot get reference of a non-referenceable type".to_string(),
    ))
}

#[macro_export]
macro_rules! try_const_alias_as_type {
    ($value:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return Ok($value.as_const_type::<$t>().alias_const());
            }
        )+
    };
}

pub fn try_const_alias_as_vmobject(value: &GCRef) -> Result<&Vec<String>, VMVariableError> {
    try_const_alias_as_type!(value; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMLambda, VMInstructions, VMVariableWrapper, VMNativeFunction, VMWrapper, VMRange, VMBytes);
    Err(VMVariableError::ReferenceError(
        value.clone(),
        "Cannot get reference of a non-referenceable type".to_string(),
    ))
}

#[macro_export]
macro_rules! try_alias_as_type {
    ($value:expr; $($t:ty),+) => {
        $(
            if $value.isinstance::<$t>() {
                return Ok($value.as_type::<$t>().alias());
            }
        )+
    };
}

pub fn try_alias_as_vmobject(value: &mut GCRef) -> Result<&mut Vec<String>, VMVariableError> {
    try_alias_as_type!(value; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMLambda, VMInstructions, VMVariableWrapper, VMNativeFunction, VMWrapper, VMRange, VMBytes);
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

pub fn try_eq_as_vmobject(value: &GCRef, other: &GCRef) -> bool {
    try_binary_op_as_type!(value, eq, other; VMInt, VMString, VMFloat, VMBoolean, VMNull, VMKeyVal, VMTuple, VMNamed, VMRange, VMBytes);
    false
}

pub trait VMObject {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError>;
    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError>;
    fn value_ref(&mut self) -> Result<GCRef, VMVariableError>;
    fn value_const_ref(&self) -> Result<GCRef, VMVariableError>;
    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError>;
    fn alias_const(&self) -> &Vec<String>;
    fn alias(&mut self) -> &mut Vec<String>;
}

// 变量包装器
// 在虚拟机中表示一个变量引用
// 允许修改其引用的值
// 支持变量赋值操作
#[derive(Debug)]
pub struct VMVariableWrapper {
    pub value_ref: GCRef,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMVariableWrapper {
    pub fn new(value: &mut GCRef) -> Self {
        if value.isinstance::<VMVariableWrapper>() {
            panic!("Cannot wrap a variable as a variable");
        }

        VMVariableWrapper {
            value_ref: value.clone(),
            traceable: GCTraceable::new(Some(&vec![value])),
            alias: Vec::new(),
        }
    }
}

impl GCObject for VMVariableWrapper {
    fn free(&mut self) {
        self.traceable.remove_reference(&mut self.value_ref);
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMVariableWrapper {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        try_deepcopy_as_vmobject(&mut self.value_ref, gc_system)
    }
    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        try_copy_as_vmobject(&mut self.value_ref, gc_system)
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        try_assign_as_vmobject(&mut self.value_ref, value)
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        try_value_ref_as_vmobject(&mut self.value_ref)
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        try_value_const_ref_as_vmobject(&self.value_ref)
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 值包装器
// 将任意值包装为一个可操作对象
// 用于实现值的引用和解包
// 通过valueof运算符解包
#[derive(Debug)]
pub struct VMWrapper {
    // 变体包装
    pub value_ref: GCRef,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMWrapper {
    pub fn new(value: &mut GCRef) -> Self {
        VMWrapper {
            value_ref: value.clone(),
            traceable: GCTraceable::new(Some(&vec![value])),
            alias: Vec::new(),
        }
    }

    pub fn get_value(&mut self) -> &mut GCRef {
        &mut self.value_ref
    }
}

impl GCObject for VMWrapper {
    fn free(&mut self) {
        self.traceable.remove_reference(&mut self.value_ref);
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMWrapper {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        try_deepcopy_as_vmobject(&mut self.value_ref, gc_system)
    }
    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        try_copy_as_vmobject(&mut self.value_ref, gc_system)
    }
    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        let new_value = value.clone();
        self.traceable.remove_reference(&mut self.value_ref);
        self.value_ref = new_value;
        self.traceable.add_reference(&mut self.value_ref);
        Ok(value)
    }
    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }
    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 整数类型
// 存储64位整数值
// 支持算术运算、比较运算和类型转换
#[derive(Debug)]
pub struct VMInt {
    pub value: i64,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMInt {
    pub fn new(value: i64) -> Self {
        VMInt {
            value,
            traceable: GCTraceable::new(None),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(value: i64, alias: &Vec<String>) -> Self {
        VMInt {
            value,
            traceable: GCTraceable::new(None),
            alias: alias.clone(),
        }
    }

    pub fn eq(&self, other: &GCRef) -> bool {
        if other.isinstance::<VMInt>() {
            self.value == other.as_const_type::<VMInt>().value
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot mod a value of non-integer type".to_string(),
        ))
    }

    pub fn power(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            let r = self.value.checked_pow(other_int.value as u32);
            if r.is_none() {
                return Err(VMVariableError::OverflowError(
                    GCRef::wrap(self),
                    other,
                    "Overflow when power".to_string(),
                ));
            }
            return Ok(gc_system.new_object(VMInt::new(r.unwrap())));
        } else if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(
                gc_system.new_object(VMFloat::new((self.value as f64).powf(other_float.value)))
            );
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot power a value of non-integer type".to_string(),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot bitwise xor a value of non-integer type".to_string(),
        ))
    }

    pub fn bitwise_not(&self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMInt::new(!self.value)))
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot compare a value of non-integer type".to_string(),
        ))
    }

    pub fn to_float(&self) -> Result<f64, VMVariableError> {
        Ok(self.value as f64)
    }

    pub fn to_string(&self) -> Result<String, VMVariableError> {
        Ok(self.value.to_string())
    }
    pub fn to_bool(&self) -> Result<bool, VMVariableError> {
        Ok(self.value != 0)
    }

    pub fn to_int(&self) -> Result<i64, VMVariableError> {
        Ok(self.value)
    }
}

impl GCObject for VMInt {
    fn free(&mut self) {}

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMInt {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMInt::new_with_alias(self.value, &self.alias)))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMInt::new_with_alias(self.value, &self.alias)))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        if value.isinstance::<VMInt>() {
            self.value = value.as_const_type::<VMInt>().value;
        } else if value.isinstance::<VMFloat>() {
            self.value = value.as_const_type::<VMFloat>().value as i64;
        } else {
            return Err(VMVariableError::ValueError2Param(
                GCRef::wrap_mut(self),
                value.clone(),
                "Cannot assign a value of non-integer type".to_string(),
            ));
        }
        Ok(value)
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 字符串类型
// 存储Unicode字符串
// 支持字符串连接、索引访问和类型转换
#[derive(Debug)]
pub struct VMString {
    pub value: String,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMString {
    pub fn new(value: String) -> Self {
        VMString {
            value,
            traceable: GCTraceable::new(None),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(value: String, alias: &Vec<String>) -> Self {
        VMString {
            value,
            traceable: GCTraceable::new(None),
            alias: alias.clone(),
        }
    }

    pub fn eq(&self, other: &GCRef) -> bool {
        if other.isinstance::<VMString>() {
            self.value == other.as_const_type::<VMString>().value
        } else {
            false
        }
    }

    pub fn to_string(&self) -> Result<String, VMVariableError> {
        Ok(self.value.clone())
    }

    pub fn to_bool(&self) -> Result<bool, VMVariableError> {
        Ok(!self.value.is_empty())
    }

    pub fn to_int(&self) -> Result<i64, VMVariableError> {
        self.value.parse::<i64>().map_err(|_| {
            VMVariableError::ValueError(
                GCRef::wrap(self),
                "Cannot convert string to int".to_string(),
            )
        })
    }

    pub fn to_float(&self) -> Result<f64, VMVariableError> {
        self.value.parse::<f64>().map_err(|_| {
            VMVariableError::ValueError(
                GCRef::wrap(self),
                "Cannot convert string to float".to_string(),
            )
        })
    }

    pub fn add(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMString>() {
            let other_string = other.as_const_type::<VMString>();
            return Ok(gc_system.new_object(VMString::new(format!(
                "{}{}",
                self.value, other_string.value
            ))));
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        } else if index.isinstance::<VMRange>() {
            let range = index.as_const_type::<VMRange>();
            let start = range.start;
            let end = range.end;
            if start < 0 || end > self.value.len() as i64 {
                return Err(VMVariableError::IndexNotFound(
                    index.clone(),
                    GCRef::wrap(self),
                ));
            }
            let substring = &self.value[start as usize..end as usize];
            return Ok(gc_system.new_object(VMString::new(substring.to_string())));
        }

        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            index.clone(),
            "Cannot index a string with a non-integer type".to_string(),
        ))
    }

    pub fn contains(&self, other: &GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMString>() {
            let other_string = other.as_const_type::<VMString>();
            return Ok(self.value.contains(&other_string.value));
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot check if a string contains a non-string type".to_string(),
        ))
    }
}

impl GCObject for VMString {
    fn free(&mut self) {
        // 字符串不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMString {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMString::new_with_alias(self.value.clone(), &self.alias)))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMString::new_with_alias(self.value.clone(), &self.alias)))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        if value.isinstance::<VMString>() {
            self.value = value.as_const_type::<VMString>().value.clone();
            Ok(value)
        } else {
            Err(VMVariableError::ValueError2Param(
                GCRef::wrap_mut(self),
                value.clone(),
                "Cannot assign a value of non-string type".to_string(),
            ))
        }
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 浮点数类型
// 存储64位浮点数值
// 支持算术运算、比较运算和类型转换
#[derive(Debug)]
pub struct VMFloat {
    pub value: f64,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMFloat {
    pub fn new(value: f64) -> Self {
        VMFloat {
            value,
            traceable: GCTraceable::new(None),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(value: f64, alias: &Vec<String>) -> Self {
        VMFloat {
            value,
            traceable: GCTraceable::new(None),
            alias: alias.clone(),
        }
    }

    pub fn eq(&self, other: &GCRef) -> bool {
        if other.isinstance::<VMFloat>() {
            self.value == other.as_const_type::<VMFloat>().value
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot mod a value of non-float type".to_string(),
        ))
    }

    pub fn power(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMFloat>() {
            let other_float = other.as_const_type::<VMFloat>();
            return Ok(gc_system.new_object(VMFloat::new(self.value.powf(other_float.value))));
        } else if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMFloat::new(self.value.powi(other_int.value as i32))));
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot power a value of non-float type".to_string(),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
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
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot compare a value of non-float type".to_string(),
        ))
    }

    pub fn to_int(&self) -> Result<i64, VMVariableError> {
        Ok(self.value as i64)
    }
    pub fn to_string(&self) -> Result<String, VMVariableError> {
        Ok(self.value.to_string())
    }
    pub fn to_bool(&self) -> Result<bool, VMVariableError> {
        Ok(self.value != 0.0)
    }
    pub fn to_float(&self) -> Result<f64, VMVariableError> {
        Ok(self.value)
    }
}

impl GCObject for VMFloat {
    fn free(&mut self) {
        // 浮点数不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMFloat {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMFloat::new_with_alias(self.value, &self.alias)))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMFloat::new_with_alias(self.value, &self.alias)))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        if value.isinstance::<VMFloat>() {
            self.value = value.as_const_type::<VMFloat>().value;
            Ok(value)
        } else if value.isinstance::<VMInt>() {
            self.value = value.as_const_type::<VMInt>().value as f64;
            Ok(value)
        } else {
            return Err(VMVariableError::ValueError2Param(
                GCRef::wrap_mut(self),
                value.clone(),
                "Cannot assign a value of non-float type".to_string(),
            ));
        }
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 布尔类型
// 存储true/false值
// 支持逻辑运算(and/or/not)和类型转换
#[derive(Debug)]
pub struct VMBoolean {
    pub value: bool,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMBoolean {
    pub fn new(value: bool) -> Self {
        VMBoolean {
            value,
            traceable: GCTraceable::new(None),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(value: bool, alias: &Vec<String>) -> Self {
        VMBoolean {
            value,
            traceable: GCTraceable::new(None),
            alias: alias.clone(),
        }
    }

    pub fn eq(&self, other: &GCRef) -> bool {
        if other.isinstance::<VMBoolean>() {
            self.value == other.as_const_type::<VMBoolean>().value
        } else {
            false
        }
    }

    pub fn and(&self, other: GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMBoolean>() {
            let other_bool = other.as_const_type::<VMBoolean>();
            return Ok(self.value && other_bool.value);
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot perform logical AND on non-boolean type".to_string(),
        ))
    }

    pub fn or(&self, other: GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMBoolean>() {
            let other_bool = other.as_const_type::<VMBoolean>();
            return Ok(self.value || other_bool.value);
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot perform logical OR on non-boolean type".to_string(),
        ))
    }

    pub fn not(&self) -> Result<bool, VMVariableError> {
        Ok(!self.value)
    }

    pub fn to_int(&self) -> Result<i64, VMVariableError> {
        Ok(if self.value { 1 } else { 0 })
    }
    pub fn to_float(&self) -> Result<f64, VMVariableError> {
        Ok(if self.value { 1.0 } else { 0.0 })
    }
    pub fn to_string(&self) -> Result<String, VMVariableError> {
        Ok(self.value.to_string())
    }
    pub fn to_bool(&self) -> Result<bool, VMVariableError> {
        Ok(self.value)
    }
}

impl GCObject for VMBoolean {
    fn free(&mut self) {
        // 布尔值不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMBoolean {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMBoolean::new_with_alias(self.value, &self.alias)))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMBoolean::new_with_alias(self.value, &self.alias)))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        if value.isinstance::<VMBoolean>() {
            self.value = value.as_const_type::<VMBoolean>().value;
            Ok(value)
        } else if value.isinstance::<VMInt>() {
            self.value = value.as_const_type::<VMInt>().value != 0;
            Ok(value)
        } else {
            return Err(VMVariableError::ValueError2Param(
                GCRef::wrap_mut(self),
                value.clone(),
                "Cannot assign a value of non-boolean type".to_string(),
            ));
        }
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 空值类型
// 表示"无值"或"未定义"
// 不支持大多数操作，主要用于初始化和空值检查
#[derive(Debug)]
pub struct VMNull {
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl Default for VMNull {
    fn default() -> Self {
        Self::new()
    }
}

impl VMNull {
    pub fn new() -> Self {
        VMNull {
            traceable: GCTraceable::new(None),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(alias: &Vec<String>) -> Self {
        VMNull {
            traceable: GCTraceable::new(None),
            alias: alias.clone(),
        }
    }

    pub fn eq(&self, other: &GCRef) -> bool {
        other.isinstance::<VMNull>()
    }
}

impl GCObject for VMNull {
    fn free(&mut self) {
        // Null 不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMNull {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMNull::new_with_alias(&self.alias)))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMNull::new_with_alias(&self.alias)))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        if value.isinstance::<VMNull>() {
            Ok(value)
        } else {
            Err(VMVariableError::ValueError2Param(
                GCRef::wrap_mut(self),
                value.clone(),
                "Cannot assign a value of non-null type".to_string(),
            ))
        }
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 键值对类型
// 存储单个键值对，用于字典和对象属性
// 通过键可以获取相应的值
// 由 key: value 语法创建
#[derive(Debug)]
pub struct VMKeyVal {
    pub key: GCRef,
    pub value: GCRef,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMKeyVal {
    pub fn new(key: &mut GCRef, value: &mut GCRef) -> Self {
        VMKeyVal {
            key: key.clone(),
            value: value.clone(),
            traceable: GCTraceable::new(Some(&vec![key, value])),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(key: &mut GCRef, value: &mut GCRef, alias: &Vec<String>) -> Self {
        VMKeyVal {
            key: key.clone(),
            value: value.clone(),
            traceable: GCTraceable::new(Some(&vec![key, value])),
            alias: alias.clone(),
        }
    }

    pub fn get_key(&mut self) -> &mut GCRef {
        &mut self.key
    }

    pub fn get_const_key(&self) -> &GCRef {
        &self.key
    }

    pub fn get_value(&mut self) -> &mut GCRef {
        &mut self.value
    }

    pub fn get_const_value(&self) -> &GCRef {
        &self.value
    }

    pub fn check_key(&self, other: &GCRef) -> bool {
        try_eq_as_vmobject(&self.key, other)
    }

    pub fn eq(&self, other: &GCRef) -> bool {
        if other.isinstance::<VMKeyVal>() {
            let other_kv = other.as_const_type::<VMKeyVal>();
            let key_eq = try_eq_as_vmobject(&self.key, &other_kv.key);
            let value_eq = try_eq_as_vmobject(&self.value, &other_kv.value);
            key_eq && value_eq
        } else {
            false
        }
    }
}

impl GCObject for VMKeyVal {
    fn free(&mut self) {
        self.traceable.remove_reference(&mut self.key);
        self.traceable.remove_reference(&mut self.value);
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMKeyVal {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        let mut new_key = try_deepcopy_as_vmobject(&mut self.key, gc_system)?;
        let mut new_value = try_deepcopy_as_vmobject(&mut self.value, gc_system)?;
        Ok(gc_system.new_object(VMKeyVal::new_with_alias(
            &mut new_key,
            &mut new_value,
            &self.alias,
        )))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMKeyVal::new_with_alias(
            &mut self.key,
            &mut self.value,
            &self.alias,
        )))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        let new_value = value.clone();
        self.traceable.remove_reference(&mut self.value);
        self.value = new_value;
        self.traceable.add_reference(&mut self.value);
        Ok(value)
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 命名参数类型
// 存储参数名和参数值
// 用于函数调用中的命名参数传递
// 由 name => value 语法创建
#[derive(Debug)]
pub struct VMNamed {
    pub key: GCRef,
    pub value: GCRef,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMNamed {
    pub fn new(key: &mut GCRef, value: &mut GCRef) -> Self {
        VMNamed {
            key: key.clone(),
            value: value.clone(),
            traceable: GCTraceable::new(Some(&vec![key, value])),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(key: &mut GCRef, value: &mut GCRef, alias: &Vec<String>) -> Self {
        VMNamed {
            key: key.clone(),
            value: value.clone(),
            traceable: GCTraceable::new(Some(&vec![key, value])),
            alias: alias.clone(),
        }
    }

    pub fn get_key(&mut self) -> &mut GCRef {
        &mut self.key
    }

    pub fn get_const_key(&self) -> &GCRef {
        &self.key
    }

    pub fn get_value(&mut self) -> &mut GCRef {
        &mut self.value
    }

    pub fn get_const_value(&self) -> &GCRef {
        &self.value
    }

    pub fn check_key(&self, other: &GCRef) -> bool {
        try_eq_as_vmobject(&self.key, other)
    }

    pub fn eq(&self, other: &GCRef) -> bool {
        if other.isinstance::<VMNamed>() {
            let other_kv = other.as_const_type::<VMNamed>();
            let key_eq = try_eq_as_vmobject(&self.key, &other_kv.key);
            let value_eq = try_eq_as_vmobject(&self.value, &other_kv.value);
            key_eq && value_eq
        } else {
            false
        }
    }
}

impl GCObject for VMNamed {
    fn free(&mut self) {
        self.traceable.remove_reference(&mut self.key);
        self.traceable.remove_reference(&mut self.value);
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMNamed {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        let mut new_key = try_deepcopy_as_vmobject(&mut self.key, gc_system)?;
        let mut new_value = try_deepcopy_as_vmobject(&mut self.value, gc_system)?;
        Ok(gc_system.new_object(VMNamed::new_with_alias(
            &mut new_key,
            &mut new_value,
            &self.alias,
        )))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMNamed::new_with_alias(
            &mut self.key,
            &mut self.value,
            &self.alias,
        )))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        let new_value = value.clone();
        self.traceable.remove_reference(&mut self.value);
        self.value = new_value;
        self.traceable.add_reference(&mut self.value);
        Ok(value)
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 元组类型
// 存储异构值的有序序列
// 支持索引访问、成员获取和值比较
// 可以包含命名元素和普通元素
#[derive(Debug)]
pub struct VMTuple {
    pub values: Vec<GCRef>,
    traceable: GCTraceable,
    alias: Vec<String>,
    auto_bind: bool,
}

impl VMTuple {
    pub fn new(values: Vec<&mut GCRef>) -> Self {
        let gc_traceable = GCTraceable::new(Some(&values));
        let mut cloned_refs = Vec::new();
        for value in values {
            cloned_refs.push(value.clone());
        }
        // 创建对象并设置引用跟踪
        VMTuple {
            values: cloned_refs,
            traceable: gc_traceable,
            alias: Vec::new(),
            auto_bind: false,
        }
    }

    pub fn new_with_alias(values: Vec<&mut GCRef>, alias: &Vec<String>) -> Self {
        // 创建对象并设置引用跟踪
        let gc_traceable = GCTraceable::new(Some(&values));
        let mut cloned_refs = Vec::new();
        for value in values {
            cloned_refs.push(value.clone());
        }
        VMTuple {
            values: cloned_refs,
            traceable: gc_traceable,
            alias: alias.clone(),
            auto_bind: false,
        }
    }

    pub fn get(&mut self, index: usize) -> Option<GCRef> {
        if index < self.values.len() {
            Some(self.values[index].clone())
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn eq(&self, other: &GCRef) -> bool {
        if other.isinstance::<VMTuple>() {
            let other_tuple = other.as_const_type::<VMTuple>();

            if self.values.len() != other_tuple.values.len() {
                return false;
            }

            // 比较每个元素
            for (i, val) in self.values.iter().enumerate() {
                let other_val = &other_tuple.values[i];

                // 使用元素的eq方法进行比较
                let eq = try_eq_as_vmobject(val, other_val);

                if !eq {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    pub fn get_member(&mut self, key: &GCRef) -> Result<&mut GCRef, VMVariableError> {
        for i in 0..self.values.len() {
            if self.values[i].isinstance::<VMKeyVal>() {
                let kv = self.values[i].as_const_type::<VMKeyVal>();
                if kv.check_key(key) {
                    return Ok(self.values[i].as_type::<VMKeyVal>().get_value());
                }
            } else if self.values[i].isinstance::<VMNamed>() {
                let named = self.values[i].as_const_type::<VMNamed>();
                if named.check_key(key) {
                    return Ok(self.values[i].as_type::<VMNamed>().get_value());
                }
            }
        }
        Err(VMVariableError::KeyNotFound(key.clone(), self.value_const_ref()?))
    }

    pub fn index_of(
        &mut self,
        index: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMVariableError> {
        if index.isinstance::<VMRange>() {
            let range = index.as_const_type::<VMRange>();
            let start = range.start;
            let end = range.end;
            if start < 0 || end > self.values.len() as i64 {
                return Err(VMVariableError::ValueError2Param(
                    GCRef::wrap_mut(self),
                    index.clone(),
                    "Index out of bounds".to_string(),
                ));
            }

            // Collect references first to avoid multiple mutable borrows
            let mut slice_refs: Vec<GCRef> = self.values[start as usize..end as usize]
                .iter_mut()
                .map(|r| r.clone())
                .collect();

            let refs_as_mut: Vec<&mut GCRef> = slice_refs.iter_mut().collect();

            let result = gc_system.new_object(VMTuple::new_with_alias(refs_as_mut, &self.alias));

            return Ok(result);
        } else if index.isinstance::<VMInt>() {
            let idx = index.as_const_type::<VMInt>().value;
            if idx < 0 {
                return Err(VMVariableError::ValueError(
                    index.clone(),
                    "Index must be a non-negative integer".to_string(),
                ));
            }
            let idx = idx as usize;
            if idx >= self.values.len() {
                return Err(VMVariableError::ValueError2Param(
                    GCRef::wrap_mut(self),
                    index.clone(),
                    "Index out of bounds".to_string(),
                ));
            }
            return Ok(self.values[idx].clone_ref());
        }
        Err(VMVariableError::TypeError(
            index.clone(),
            "Index must be an integer".to_string(),
        ))
    }
    /// 将另一个元组的成员赋值给当前元组
    /// 先尝试将所有 VMNamed 对象按照键进行赋值
    /// 剩下的值按照顺序赋值到非命名位置
    pub fn assign_members(&mut self, other: GCRef) -> Result<GCRef, VMVariableError> {
        // 确保参数是元组
        if !other.isinstance::<VMTuple>() {
            return Err(VMVariableError::ValueError2Param(
                GCRef::wrap_mut(self),
                other.clone(),
                "Expected a tuple".to_string(),
            ));
        }

        let other_tuple = other.as_const_type::<VMTuple>();

        // 分离命名参数和普通值
        let mut key_values = Vec::new();
        let mut normal_values = Vec::new();
        let mut assigned = vec![false; self.values.len()];

        for item in &other_tuple.values {
            if item.isinstance::<VMNamed>() {
                key_values.push(item.clone());
            } else {
                normal_values.push(item.clone());
            }
        }

        // 处理所有命名参数
        for kv in &mut key_values {
            let mut found: bool = false;
            // 在当前元组中查找匹配的键
            for i in 0..self.values.len() {
                if self.values[i].isinstance::<VMNamed>() {
                    let self_named = self.values[i].as_type::<VMNamed>();
                    let kv_named = kv.as_type::<VMNamed>();

                    // 检查键是否匹配
                    if try_eq_as_vmobject(self_named.get_const_key(), kv_named.get_const_key())
                    {
                        // 找到匹配的键，进行赋值
                        let value_ref = &mut self.values[i];
                        try_assign_as_vmobject(value_ref, kv_named.get_value())?;
                        assigned[i] = true;
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                // 如果没有找到匹配的键，添加新的键值对
                self.values.push(kv.clone());
                self.traceable
                    .add_reference(&mut self.values.last_mut().unwrap().clone());
            }
        }

        // 按顺序处理普通值
        let mut normal_index = 0;
        for value in normal_values.iter_mut() {
            // 寻找一个非命名且未赋值的位置
            while normal_index < assigned.len()
                && (self.values[normal_index].isinstance::<VMNamed>() && assigned[normal_index])
            {
                normal_index += 1;
            }

            if normal_index < self.values.len() {
                // 找到位置，进行赋值
                let value_ref = &mut self.values[normal_index];
                try_assign_as_vmobject(value_ref, value)?;
                normal_index += 1;
            } else {
                // 没有更多位置，追加到末尾
                self.values.push(value.clone());
                self.traceable
                    .add_reference(&mut self.values.last_mut().unwrap().clone());
            }
        }

        Ok(GCRef::wrap_mut(self))
    }

    pub fn add(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMTuple>() {
            let other_tuple = other.as_const_type::<VMTuple>();
            let mut new_values = self.values.clone();
            new_values.extend(other_tuple.values.clone());

            // Convert Vec<GCRef> to Vec<&mut GCRef>
            let refs_as_mut: Vec<&mut GCRef> = new_values.iter_mut().collect();

            let new_tuple = gc_system.new_object(VMTuple::new(refs_as_mut));

            return Ok(new_tuple);
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot add a value of non-tuple type".to_string(),
        ))
    }

    pub fn contains(&self, other: &GCRef) -> Result<bool, VMVariableError> {
        for value in &self.values {
            if try_eq_as_vmobject(value, other) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn set_lambda_self(&mut self) {
        self.auto_bind = true;
        let mut wrapped = GCRef::wrap(self);
        for val in &mut self.values {
            if val.isinstance::<VMNamed>()
                && val
                    .as_const_type::<VMNamed>()
                    .value
                    .isinstance::<VMLambda>()
            {
                let lambda = val.as_type::<VMNamed>().value.as_type::<VMLambda>();
                lambda.set_self_object(&mut wrapped);
            }
        }
    }

    pub fn append(&mut self, value: &mut GCRef) -> Result<GCRef, VMVariableError> {
        self.values.push(value.clone());
        self.traceable
            .add_reference(self.values.last().unwrap());
        Ok(GCRef::wrap_mut(self))
    }
}

impl GCObject for VMTuple {
    fn free(&mut self) {
        // 移除对所有元素的引用
        for value in &mut self.values {
            self.traceable.remove_reference(value);
        }
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMTuple {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        // 深拷贝元组中的每个元素
        let mut new_values = Vec::with_capacity(self.values.len());
        for value in &mut self.values {
            let copied_value = try_deepcopy_as_vmobject(value, gc_system)?;
            new_values.push(copied_value);
        }
        // 将 Vec<GCRef> 转换为 Vec<&mut GCRef>
        let refs_as_mut: Vec<&mut GCRef> = new_values.iter_mut().collect();

        // 创建新的元组对象
        let mut new_tuple = gc_system.new_object(VMTuple::new_with_alias(refs_as_mut, &self.alias));

        if self.auto_bind {
            new_tuple.as_type::<VMTuple>().set_lambda_self();
        }
        Ok(new_tuple)
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        // 浅拷贝元组中的每个元素
        let mut_refs = self.values.iter_mut().collect();
        let mut new_tuple = gc_system.new_object(VMTuple::new_with_alias(mut_refs, &self.alias));
        if self.auto_bind {
            new_tuple.as_type::<VMTuple>().set_lambda_self();
        }
        Ok(new_tuple)
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        if value.isinstance::<VMTuple>() {
            // 先克隆新元组的元素引用，确保它们不会在过程中被释放
            let other_tuple = value.as_type::<VMTuple>();
            let new_values = &mut other_tuple.values;

            // 为所有新元素增加引用计数，确保它们保持在线状态
            let mut cloned_values = Vec::with_capacity(new_values.len());
            for val in new_values {
                cloned_values.push(val.clone());
            }
            // 添加对新元素的引用
            for val in &cloned_values {
                self.traceable.add_reference(&mut val.clone());
            }

            // 现在移除对当前所有元素的引用
            for val in &mut self.values {
                self.traceable.remove_reference(val);
            }
            // 设置新的元素集合
            self.values = cloned_values;

            Ok(value)
        } else {
            Err(VMVariableError::ValueError2Param(
                GCRef::wrap_mut(self),
                value.clone(),
                "Cannot assign a value of non-tuple type".to_string(),
            ))
        }
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 指令集类型
// 存储IR指令和函数入口点映射
// 由编译器生成，不可直接修改
// 作为VMLambda的执行环境
#[derive(Debug)]
pub struct VMInstructions {
    pub instructions: Vec<IR>,
    pub func_ips: HashMap<String, usize>,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMInstructions {
    pub fn new(instructions: Vec<IR>, func_ips: HashMap<String, usize>) -> Self {
        VMInstructions {
            instructions,
            func_ips,
            traceable: GCTraceable::new(None),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(
        instructions: Vec<IR>,
        func_ips: HashMap<String, usize>,
        alias: &Vec<String>,
    ) -> Self {
        VMInstructions {
            instructions,
            func_ips,
            traceable: GCTraceable::new(None),
            alias: alias.clone(),
        }
    }
}

impl GCObject for VMInstructions {
    fn free(&mut self) {
        // 不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}
impl VMObject for VMInstructions {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMInstructions::new_with_alias(
            self.instructions.clone(),
            self.func_ips.clone(),
            &self.alias,
        )))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMInstructions::new_with_alias(
            self.instructions.clone(),
            self.func_ips.clone(),
            &self.alias,
        )))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap_mut(self),
            value.clone(),
            "Cannot assign a value to VMInstructions".to_string(),
        ))
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 函数/闭包类型
// 存储可执行代码、默认参数和环境(self对象)
// 支持函数调用，可保留闭包上下文
// 由 (params) -> { body } 语法创建
#[derive(Debug, PartialEq)]
pub enum VMCoroutineStatus {
    Running,
    Pending,
    Finished,
    Crashed,
}

impl VMCoroutineStatus {
    pub fn to_string(&self) -> String {
        match self {
            VMCoroutineStatus::Running => "Running".bright_green().bold().to_string(),
            VMCoroutineStatus::Pending => "Pending".bright_yellow().bold().to_string(),
            VMCoroutineStatus::Finished => "Finished".bright_yellow().bold().to_string(),
            VMCoroutineStatus::Crashed => "Crashed".bright_red().bold().to_string(),
        }
    }
}

#[derive(Debug)]
pub struct VMLambda {
    pub code_position: usize,
    pub signature: String,
    pub default_args_tuple: GCRef,
    pub self_object: Option<GCRef>,
    pub lambda_instructions: GCRef,
    pub result: GCRef,
    traceable: GCTraceable,
    pub coroutine_status: VMCoroutineStatus,
    alias: Vec<String>,
}

impl VMLambda {
    pub fn new(
        code_position: usize,
        signature: String,
        default_args_tuple: &mut GCRef,
        self_object: Option<&mut GCRef>,
        lambda_instructions: &mut GCRef,
        result: &mut GCRef,
    ) -> Self {
        if !lambda_instructions.isinstance::<VMInstructions>() {
            panic!("lambda_instructions must be a VMInstructions");
        }
        if !default_args_tuple.isinstance::<VMTuple>() {
            panic!("default_args_tuple must be a VMTuple");
        }
        let mut cloned_default_args_tuple = default_args_tuple.clone();
        let mut cloned_lambda_instructions = lambda_instructions.clone();
        let mut cloned_result = result.clone();

        // 创建引用向量
        let mut refs_vec = vec![
            &mut cloned_default_args_tuple,
            &mut cloned_lambda_instructions,
            &mut cloned_result,
        ];
        let mut cloned_obj;
        let cloned = match self_object {
            Some(obj) => {
                cloned_obj = obj.clone();
                let new = cloned_obj.clone();
                refs_vec.push(&mut cloned_obj);
                Some(new)
            }
            None => None,
        };

        VMLambda {
            code_position,
            signature,
            default_args_tuple: default_args_tuple.clone(),
            self_object: cloned,
            lambda_instructions: lambda_instructions.clone(),
            traceable: GCTraceable::new(Some(&refs_vec)),
            result: result.clone(),
            coroutine_status: VMCoroutineStatus::Running,
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(
        code_position: usize,
        signature: String,
        default_args_tuple: &mut GCRef,
        self_object: Option<&mut GCRef>,
        lambda_instructions: &mut GCRef,
        result: &mut GCRef,
        alias: &Vec<String>,
    ) -> Self {
        if !lambda_instructions.isinstance::<VMInstructions>() {
            panic!("lambda_instructions must be a VMInstructions");
        }
        if !default_args_tuple.isinstance::<VMTuple>() {
            panic!("default_args_tuple must be a VMTuple");
        }
        let mut cloned_default_args_tuple = default_args_tuple.clone();
        let mut cloned_lambda_instructions = lambda_instructions.clone();
        let mut cloned_result = result.clone();

        // 创建引用向量
        let mut refs_vec = vec![
            &mut cloned_default_args_tuple,
            &mut cloned_lambda_instructions,
            &mut cloned_result,
        ];
        let mut cloned_obj;
        let cloned = match self_object {
            Some(obj) => {
                cloned_obj = obj.clone();
                let new = cloned_obj.clone();
                refs_vec.push(&mut cloned_obj);
                Some(new)
            }
            None => None,
        };

        VMLambda {
            code_position,
            signature,
            default_args_tuple: default_args_tuple.clone(),
            self_object: cloned,
            lambda_instructions: lambda_instructions.clone(),
            traceable: GCTraceable::new(Some(&refs_vec)),
            result: result.clone(),
            coroutine_status: VMCoroutineStatus::Running,
            alias: alias.clone(),
        }
    }

    pub fn set_result(&mut self, result_object: &mut GCRef) {
        let mut result = self.result.clone();
        let mut new_result = result_object.clone();
        self.traceable.add_reference(&mut new_result);
        self.result = result_object.clone();
        self.traceable.remove_reference(&mut result);
    }

    pub fn set_self_object(&mut self, self_object: &mut GCRef) {
        if self.self_object.is_some() {
            self.traceable
                .remove_reference(self.self_object.as_mut().unwrap());
        }
        self.self_object = Some(self_object.clone());
        self.traceable
            .add_reference(self.self_object.as_mut().unwrap());
    }

    pub fn get_value(&mut self) -> &mut GCRef {
        &mut self.result
    }

    pub fn get_const_value(&self) -> &GCRef {
        &self.result
    }

    pub fn get_key(&mut self) -> &mut GCRef {
        &mut self.default_args_tuple
    }

    pub fn get_const_key(&self) -> &GCRef {
        &self.default_args_tuple
    }
}

impl GCObject for VMLambda {
    fn free(&mut self) {
        self.traceable
            .remove_reference(&mut self.default_args_tuple);
        self.traceable
            .remove_reference(&mut self.lambda_instructions);
        self.traceable.remove_reference(&mut self.result);
        if self.self_object.is_some() {
            self.traceable
                .remove_reference(self.self_object.as_mut().unwrap());
        }
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMLambda {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        let mut new_default_args_tuple = self
            .default_args_tuple
            .as_type::<VMTuple>()
            .deepcopy(gc_system)?;

        let mut new_result = self.result.as_type::<VMNull>().deepcopy(gc_system)?;
        let mut new_lambda_instructions = self
            .lambda_instructions
            .as_type::<VMInstructions>()
            .deepcopy(gc_system)?;

        Ok(gc_system.new_object(VMLambda::new_with_alias(
            self.code_position,
            self.signature.clone(),
            &mut new_default_args_tuple,
            self.self_object.as_mut(),
            &mut new_lambda_instructions,
            &mut new_result,
            &self.alias,
        )))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        let mut new_default_args_tuple = self
            .default_args_tuple
            .as_type::<VMTuple>()
            .copy(gc_system)?;

        let mut new_result = self.result.as_type::<VMNull>().copy(gc_system)?;

        Ok(gc_system.new_object(VMLambda::new_with_alias(
            self.code_position,
            self.signature.clone(),
            &mut new_default_args_tuple,
            self.self_object.as_mut(),
            &mut self.lambda_instructions,
            &mut new_result,
            &self.alias,
        )))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        if !value.isinstance::<VMLambda>() {
            return Err(VMVariableError::ValueError2Param(
                GCRef::wrap_mut(self),
                value.clone(),
                "Cannot assign a value of non-lambda type".to_string(),
            ));
        }

        // 先获取新值的引用并增加引用计数，确保不会在过程中被释放
        let v_lambda = value.as_type::<VMLambda>();
        let mut new_default_args_tuple = v_lambda.default_args_tuple.clone();
        let mut new_lambda_instructions = v_lambda.lambda_instructions.clone();
        let mut new_result = v_lambda.result.clone();
        let mut old_default_args_tuple = self.default_args_tuple.clone();
        let mut old_lambda_instructions = self.lambda_instructions.clone();
        let mut old_result = self.result.clone();

        // 移除旧引用
        self.get_traceable()
            .remove_reference(&mut old_default_args_tuple);

        self.get_traceable()
            .remove_reference(&mut old_lambda_instructions);

        self.get_traceable().remove_reference(&mut old_result);

        // 设置新引用
        self.default_args_tuple = new_default_args_tuple.clone();
        self.lambda_instructions = new_lambda_instructions.clone();
        self.result = new_result.clone();

        // 添加对新值的引用关系
        self.get_traceable()
            .add_reference(&mut new_default_args_tuple);
        self.get_traceable()
            .add_reference(&mut new_lambda_instructions);
        self.get_traceable().add_reference(&mut new_result);

        Ok(value)
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

// 原生函数类型
// 包装Rust实现的函数为VM可调用对象
// 用于提供内置函数和与宿主环境交互
#[derive(Debug)]
pub struct VMNativeFunction {
    // 包装rust函数， 函数定义为 fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>
    pub function: fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMNativeFunction {
    pub fn new(function: fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>) -> Self {
        VMNativeFunction {
            function,
            traceable: GCTraceable::new(None),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(
        function: fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
        alias: &Vec<String>,
    ) -> Self {
        VMNativeFunction {
            function,
            traceable: GCTraceable::new(None),
            alias: alias.clone(),
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
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMNativeFunction {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMNativeFunction::new_with_alias(self.function, &self.alias)))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMNativeFunction::new_with_alias(self.function, &self.alias)))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap_mut(self),
            value.clone(),
            "Cannot assign a value to VMNativeFunction".to_string(),
        ))
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

#[derive(Debug)]
pub struct VMRange {
    pub start: i64,
    pub end: i64,
    traceable: GCTraceable,
    alias: Vec<String>,
}
impl VMRange {
    pub fn new(start: i64, end: i64) -> Self {
        VMRange {
            start,
            end,
            traceable: GCTraceable::new(None),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(start: i64, end: i64, alias: &Vec<String>) -> Self {
        VMRange {
            start,
            end,
            traceable: GCTraceable::new(None),
            alias: alias.clone(),
        }
    }

    pub fn len(&self) -> i64 {
        self.end - self.start
    }

    pub fn add(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMRange::new(
                self.start + other_int.value,
                self.end + other_int.value,
            )));
        } else if other.isinstance::<VMRange>() {
            let other_range = other.as_const_type::<VMRange>();
            return Ok(gc_system.new_object(VMRange::new(
                self.start + other_range.start,
                self.end + other_range.end,
            )));
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot add a value of non-integer type".to_string(),
        ))
    }
    pub fn sub(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(gc_system.new_object(VMRange::new(
                self.start - other_int.value,
                self.end - other_int.value,
            )));
        } else if other.isinstance::<VMRange>() {
            let other_range = other.as_const_type::<VMRange>();
            return Ok(gc_system.new_object(VMRange::new(
                self.start - other_range.start,
                self.end - other_range.end,
            )));
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot subtract a value of non-integer type".to_string(),
        ))
    }

    pub fn contains(&self, other: &GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMInt>() {
            let other_int = other.as_const_type::<VMInt>();
            return Ok(self.start <= other_int.value && self.end >= other_int.value);
        } else if other.isinstance::<VMRange>() {
            let other_range = other.as_const_type::<VMRange>();
            return Ok(self.start <= other_range.start && self.end >= other_range.end);
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot check containment with a non-integer type".to_string(),
        ))
    }

    pub fn eq(&self, other: &GCRef) -> bool {
        if other.isinstance::<VMRange>() {
            let other_range = other.as_const_type::<VMRange>();
            self.start == other_range.start && self.end == other_range.end
        } else {
            false
        }
    }
}
impl GCObject for VMRange {
    fn free(&mut self) {
        // 不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMRange {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMRange::new_with_alias(self.start, self.end, &self.alias)))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMRange::new_with_alias(self.start, self.end, &self.alias)))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap_mut(self),
            value.clone(),
            "Cannot assign a value to VMRange".to_string(),
        ))
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}

#[derive(Debug)]
pub struct VMBytes {
    pub value: Vec<u8>,
    traceable: GCTraceable,
    alias: Vec<String>,
}

impl VMBytes {
    pub fn new(value: Vec<u8>) -> Self {
        VMBytes {
            value,
            traceable: GCTraceable::new(None),
            alias: Vec::new(),
        }
    }

    pub fn new_with_alias(value: Vec<u8>, alias: &Vec<String>) -> Self {
        VMBytes {
            value,
            traceable: GCTraceable::new(None),
            alias: alias.clone(),
        }
    }

    pub fn eq(&self, other: &GCRef) -> bool {
        if other.isinstance::<VMBytes>() {
            self.value == other.as_const_type::<VMBytes>().value
        } else {
            false
        }
    }

    pub fn to_string(&self) -> Result<String, VMVariableError> {
        // 尝试将字节转换为UTF-8字符串
        match String::from_utf8(self.value.clone()) {
            Ok(s) => Ok(s),
            Err(_) => Err(VMVariableError::ValueError(
                GCRef::wrap(self),
                "Cannot convert bytes to string: invalid UTF-8".to_string(),
            )),
        }
    }

    pub fn to_bool(&self) -> Result<bool, VMVariableError> {
        Ok(!self.value.is_empty())
    }

    pub fn len(&self) -> usize {
        self.value.len()
    }

    pub fn add(&self, other: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if other.isinstance::<VMBytes>() {
            let other_bytes = other.as_const_type::<VMBytes>();
            let mut new_value = self.value.clone();
            new_value.extend_from_slice(&other_bytes.value);
            return Ok(gc_system.new_object(VMBytes::new(new_value)));
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot add a value of non-bytes type".to_string(),
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
            let byte = self.value[index_int.value as usize];
            return Ok(gc_system.new_object(VMInt::new(byte as i64)));
        } else if index.isinstance::<VMRange>() {
            let range = index.as_const_type::<VMRange>();
            let start = range.start;
            let end = range.end;
            if start < 0 || end > self.value.len() as i64 {
                return Err(VMVariableError::IndexNotFound(
                    index.clone(),
                    GCRef::wrap(self),
                ));
            }
            let slice = &self.value[start as usize..end as usize];
            return Ok(gc_system.new_object(VMBytes::new(slice.to_vec())));
        }

        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            index.clone(),
            "Cannot index bytes with a non-integer type".to_string(),
        ))
    }

    pub fn contains(&self, other: GCRef) -> Result<bool, VMVariableError> {
        if other.isinstance::<VMBytes>() {
            let other_bytes = other.as_const_type::<VMBytes>();
            // 检查是否包含子序列
            return Ok(self
                .value
                .windows(other_bytes.value.len())
                .any(|window| window == other_bytes.value));
        } else if other.isinstance::<VMInt>() {
            let byte = other.as_const_type::<VMInt>().value;
            if !(0..=255).contains(&byte) {
                return Err(VMVariableError::ValueError(
                    other.clone(),
                    "Byte value must be between 0 and 255".to_string(),
                ));
            }
            return Ok(self.value.contains(&(byte as u8)));
        }
        Err(VMVariableError::ValueError2Param(
            GCRef::wrap(self),
            other.clone(),
            "Cannot check if bytes contain a non-bytes or non-integer type".to_string(),
        ))
    }
}

impl GCObject for VMBytes {
    fn free(&mut self) {
        // 字节不需要额外的释放操作
    }

    fn get_traceable(&mut self) -> &mut GCTraceable {
        &mut self.traceable
    }

    fn get_const_traceable(&self) -> &GCTraceable {
        &self.traceable
    }
}

impl VMObject for VMBytes {
    fn deepcopy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMBytes::new_with_alias(self.value.clone(), &self.alias)))
    }

    fn copy(&mut self, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        Ok(gc_system.new_object(VMBytes::new_with_alias(self.value.clone(), &self.alias)))
    }

    fn assign<'t>(&mut self, value: &'t mut GCRef) -> Result<&'t mut GCRef, VMVariableError> {
        if value.isinstance::<VMBytes>() {
            self.value = value.as_const_type::<VMBytes>().value.clone();
            Ok(value)
        } else if value.isinstance::<VMString>() {
            // 允许从字符串转换为字节序列
            self.value = value.as_const_type::<VMString>().value.as_bytes().to_vec();
            Ok(value)
        } else if value.isinstance::<VMKeyVal>() {
            // 允许从键值对转换为字节序列
            let kv = value.as_const_type::<VMKeyVal>();
            let index = kv.key.clone();
            let val = kv.value.clone();

            if index.isinstance::<VMInt>() {
                // 单字节修改
                let idx = index.as_const_type::<VMInt>().value;
                if idx < 0 || idx >= self.value.len() as i64 {
                    return Err(VMVariableError::IndexNotFound(
                        index.clone(),
                        GCRef::wrap_mut(self),
                    ));
                }

                // 获取要设置的字节值
                let byte_val: u8;
                if val.isinstance::<VMInt>() {
                    let int_val = val.as_const_type::<VMInt>().value;
                    if !(0..=255).contains(&int_val) {
                        return Err(VMVariableError::ValueError(
                            val.clone(),
                            "Byte value must be between 0 and 255".to_string(),
                        ));
                    }
                    byte_val = int_val as u8;
                } else if val.isinstance::<VMString>() {
                    // 处理单个索引位置写入字符串所有字节
                    let str_value = val.as_const_type::<VMString>().value.clone();
                    if str_value.is_empty() {
                        return Err(VMVariableError::ValueError(
                            val.clone(),
                            "Cannot use empty string as byte value".to_string(),
                        ));
                    }

                    // 获取字符串的字节表示
                    let str_bytes = str_value.as_bytes();

                    // 检查是否有足够空间
                    if (idx as usize) + str_bytes.len() > self.value.len() {
                        return Err(VMVariableError::ValueError(
                            val.clone(),
                            format!(
                                "Not enough space to write {} bytes at index {}",
                                str_bytes.len(),
                                idx
                            ),
                        ));
                    }

                    // 从指定索引位置开始写入所有字节
                    for (i, &byte) in str_bytes.iter().enumerate() {
                        self.value[(idx as usize) + i] = byte;
                    }

                    return Ok(value);
                } else if val.isinstance::<VMBytes>() {
                    // 处理单个索引位置写入字节序列所有字节
                    let bytes_value = val.as_const_type::<VMBytes>().value.clone();
                    if bytes_value.is_empty() {
                        return Err(VMVariableError::ValueError(
                            val.clone(),
                            "Cannot use empty bytes as byte value".to_string(),
                        ));
                    }

                    // 检查是否有足够空间
                    if (idx as usize) + bytes_value.len() > self.value.len() {
                        return Err(VMVariableError::ValueError(
                            val.clone(),
                            format!(
                                "Not enough space to write {} bytes at index {}",
                                bytes_value.len(),
                                idx
                            ),
                        ));
                    }

                    // 从指定索引位置开始写入所有字节
                    for (i, &byte) in bytes_value.iter().enumerate() {
                        self.value[(idx as usize) + i] = byte;
                    }

                    return Ok(value);
                } else {
                    return Err(VMVariableError::ValueError(
                        val.clone(),
                        "Cannot convert non-integer value to byte".to_string(),
                    ));
                }

                // 设置字节
                self.value[idx as usize] = byte_val;
                return Ok(value);
            } else if index.isinstance::<VMRange>() {
                // 范围修改
                let range = index.as_const_type::<VMRange>();
                let start = range.start;
                let end = range.end;

                if start < 0 || end > self.value.len() as i64 || start > end {
                    return Err(VMVariableError::IndexNotFound(
                        index.clone(),
                        GCRef::wrap_mut(self),
                    ));
                }

                // 获取要设置的字节序列
                let new_bytes: Vec<u8>;
                if val.isinstance::<VMBytes>() {
                    new_bytes = val.as_const_type::<VMBytes>().value.clone();
                } else if val.isinstance::<VMString>() {
                    new_bytes = val.as_const_type::<VMString>().value.as_bytes().to_vec();
                } else if val.isinstance::<VMInt>() {
                    // 如果是整数，将所有字节设为相同值
                    let int_val = val.as_const_type::<VMInt>().value;
                    if !(0..=255).contains(&int_val) {
                        return Err(VMVariableError::ValueError(
                            val.clone(),
                            "Byte value must be between 0 and 255".to_string(),
                        ));
                    }
                    new_bytes = vec![int_val as u8; (end - start) as usize];
                } else {
                    return Err(VMVariableError::ValueError(
                        val.clone(),
                        "Cannot convert value to bytes".to_string(),
                    ));
                }

                // 检查长度是否匹配
                if new_bytes.len() != (end - start) as usize {
                    return Err(VMVariableError::ValueError2Param(
                        index.clone(),
                        val.clone(),
                        format!(
                            "Slice length {} does not match range length {}",
                            new_bytes.len(),
                            (end - start)
                        ),
                    ));
                }

                // 替换范围内的字节
                for i in 0..(end - start) as usize {
                    self.value[(start as usize) + i] = new_bytes[i];
                }

                return Ok(value);
            } else {
                return Err(VMVariableError::ValueError(
                    index.clone(),
                    "Index must be an integer or range".to_string(),
                ));
            }
        } else {
            Err(VMVariableError::ValueError2Param(
                GCRef::wrap_mut(self),
                value.clone(),
                "Cannot assign a value of non-bytes type".to_string(),
            ))
        }
    }

    fn value_ref(&mut self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap_mut(self))
    }

    fn value_const_ref(&self) -> Result<GCRef, VMVariableError> {
        Ok(GCRef::wrap(self))
    }

    fn alias_const(&self) -> &Vec<String> {
        &self.alias
    }

    fn alias(&mut self) -> &mut Vec<String> {
        &mut self.alias
    }
}
