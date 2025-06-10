/// Attribute Value Types
/// 
/// Type-safe attribute value storage with support for various data types.
/// Includes comparison, arithmetic, and serialization support.

use crate::instance::InstanceId;
use serde::{Serialize, Deserialize};
use std::fmt;

/// Supported value types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueType {
    Null = 0,
    Bool = 1,
    Integer = 2,
    Float = 3,
    String = 4,
    Vector2 = 5,
    Vector3 = 6,
    Color = 7,
    InstanceRef = 8,
    List = 9,
    Map = 10,
}

/// Attribute value variants
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttributeValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Vector2([f32; 2]),
    Vector3([f32; 3]),
    Color([u8; 4]), // RGBA
    InstanceRef(InstanceId),
    List(Vec<AttributeValue>),
    Map(std::collections::HashMap<String, AttributeValue>),
}

impl AttributeValue {
    /// Get the value type
    pub fn value_type(&self) -> ValueType {
        match self {
            AttributeValue::Null => ValueType::Null,
            AttributeValue::Bool(_) => ValueType::Bool,
            AttributeValue::Integer(_) => ValueType::Integer,
            AttributeValue::Float(_) => ValueType::Float,
            AttributeValue::String(_) => ValueType::String,
            AttributeValue::Vector2(_) => ValueType::Vector2,
            AttributeValue::Vector3(_) => ValueType::Vector3,
            AttributeValue::Color(_) => ValueType::Color,
            AttributeValue::InstanceRef(_) => ValueType::InstanceRef,
            AttributeValue::List(_) => ValueType::List,
            AttributeValue::Map(_) => ValueType::Map,
        }
    }
    
    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, AttributeValue::Null)
    }
    
    /// Convert to boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            AttributeValue::Bool(v) => Some(*v),
            AttributeValue::Integer(v) => Some(*v != 0),
            AttributeValue::Float(v) => Some(*v != 0.0),
            _ => None,
        }
    }
    
    /// Convert to integer
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            AttributeValue::Integer(v) => Some(*v),
            AttributeValue::Float(v) => Some(*v as i64),
            AttributeValue::Bool(v) => Some(if *v { 1 } else { 0 }),
            _ => None,
        }
    }
    
    /// Convert to float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            AttributeValue::Float(v) => Some(*v),
            AttributeValue::Integer(v) => Some(*v as f64),
            AttributeValue::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            _ => None,
        }
    }
    
    /// Convert to string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            AttributeValue::String(v) => Some(v),
            _ => None,
        }
    }
    
    /// Add two values
    pub fn add(&self, other: &AttributeValue) -> Option<AttributeValue> {
        match (self, other) {
            (AttributeValue::Integer(a), AttributeValue::Integer(b)) => {
                Some(AttributeValue::Integer(a + b))
            }
            (AttributeValue::Float(a), AttributeValue::Float(b)) => {
                Some(AttributeValue::Float(a + b))
            }
            (AttributeValue::String(a), AttributeValue::String(b)) => {
                Some(AttributeValue::String(format!("{}{}", a, b)))
            }
            (AttributeValue::Vector2([x1, y1]), AttributeValue::Vector2([x2, y2])) => {
                Some(AttributeValue::Vector2([x1 + x2, y1 + y2]))
            }
            (AttributeValue::Vector3([x1, y1, z1]), AttributeValue::Vector3([x2, y2, z2])) => {
                Some(AttributeValue::Vector3([x1 + x2, y1 + y2, z1 + z2]))
            }
            _ => None,
        }
    }
    
    /// Multiply value by scalar
    pub fn multiply(&self, scalar: f64) -> Option<AttributeValue> {
        match self {
            AttributeValue::Integer(v) => {
                Some(AttributeValue::Integer((*v as f64 * scalar) as i64))
            }
            AttributeValue::Float(v) => {
                Some(AttributeValue::Float(v * scalar))
            }
            AttributeValue::Vector2([x, y]) => {
                Some(AttributeValue::Vector2([
                    (x * scalar as f32),
                    (y * scalar as f32),
                ]))
            }
            AttributeValue::Vector3([x, y, z]) => {
                Some(AttributeValue::Vector3([
                    (x * scalar as f32),
                    (y * scalar as f32),
                    (z * scalar as f32),
                ]))
            }
            _ => None,
        }
    }
    
    /// Compare values
    pub fn compare(&self, other: &AttributeValue) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (AttributeValue::Integer(a), AttributeValue::Integer(b)) => Some(a.cmp(b)),
            (AttributeValue::Float(a), AttributeValue::Float(b)) => {
                a.partial_cmp(b)
            }
            (AttributeValue::String(a), AttributeValue::String(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }
    
    /// Check if greater than or equal
    pub fn greater_than_or_equal(&self, other: &AttributeValue) -> bool {
        matches!(self.compare(other), Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal))
    }
    
    /// Check if less than or equal
    pub fn less_than_or_equal(&self, other: &AttributeValue) -> bool {
        matches!(self.compare(other), Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal))
    }
    
    /// Clamp value between min and max
    pub fn clamp(&self, min: &AttributeValue, max: &AttributeValue) -> AttributeValue {
        match (self, min, max) {
            (AttributeValue::Integer(v), AttributeValue::Integer(min_v), AttributeValue::Integer(max_v)) => {
                AttributeValue::Integer(v.clamp(*min_v, *max_v))
            }
            (AttributeValue::Float(v), AttributeValue::Float(min_v), AttributeValue::Float(max_v)) => {
                AttributeValue::Float(v.clamp(*min_v, *max_v))
            }
            _ => self.clone(),
        }
    }
    
    /// Linear interpolation
    pub fn lerp(&self, other: &AttributeValue, t: f32) -> Option<AttributeValue> {
        match (self, other) {
            (AttributeValue::Float(a), AttributeValue::Float(b)) => {
                Some(AttributeValue::Float(a + (b - a) * t as f64))
            }
            (AttributeValue::Vector2([x1, y1]), AttributeValue::Vector2([x2, y2])) => {
                Some(AttributeValue::Vector2([
                    x1 + (x2 - x1) * t,
                    y1 + (y2 - y1) * t,
                ]))
            }
            (AttributeValue::Vector3([x1, y1, z1]), AttributeValue::Vector3([x2, y2, z2])) => {
                Some(AttributeValue::Vector3([
                    x1 + (x2 - x1) * t,
                    y1 + (y2 - y1) * t,
                    z1 + (z2 - z1) * t,
                ]))
            }
            (AttributeValue::Color([r1, g1, b1, a1]), AttributeValue::Color([r2, g2, b2, a2])) => {
                Some(AttributeValue::Color([
                    (r1 + ((r2 - r1) as f32 * t) as u8),
                    (g1 + ((g2 - g1) as f32 * t) as u8),
                    (b1 + ((b2 - b1) as f32 * t) as u8),
                    (a1 + ((a2 - a1) as f32 * t) as u8),
                ]))
            }
            _ => None,
        }
    }
}

impl Default for AttributeValue {
    fn default() -> Self {
        AttributeValue::Null
    }
}

impl fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttributeValue::Null => write!(f, "null"),
            AttributeValue::Bool(v) => write!(f, "{}", v),
            AttributeValue::Integer(v) => write!(f, "{}", v),
            AttributeValue::Float(v) => write!(f, "{:.2}", v),
            AttributeValue::String(v) => write!(f, "\"{}\"", v),
            AttributeValue::Vector2(v) => write!(f, "({:.2}, {:.2})", v[0], v[1]),
            AttributeValue::Vector3(v) => write!(f, "({:.2}, {:.2}, {:.2})", v[0], v[1], v[2]),
            AttributeValue::Color(v) => write!(f, "rgba({}, {}, {}, {})", v[0], v[1], v[2], v[3]),
            AttributeValue::InstanceRef(id) => write!(f, "ref({})", id),
            AttributeValue::List(v) => write!(f, "[{} items]", v.len()),
            AttributeValue::Map(v) => write!(f, "{{{} entries}}", v.len()),
        }
    }
}

/// Type-safe value wrapper
#[derive(Debug, Clone)]
pub enum TypedValue<T> {
    Some(T),
    None,
}

impl<T> TypedValue<T> {
    pub fn new(value: T) -> Self {
        TypedValue::Some(value)
    }
    
    pub fn is_some(&self) -> bool {
        matches!(self, TypedValue::Some(_))
    }
    
    pub fn unwrap(self) -> T {
        match self {
            TypedValue::Some(v) => v,
            TypedValue::None => panic!("unwrap on None value"),
        }
    }
    
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            TypedValue::Some(v) => v,
            TypedValue::None => default,
        }
    }
}

/// Value conversion traits
pub trait FromAttributeValue: Sized {
    fn from_attribute_value(value: &AttributeValue) -> Option<Self>;
}

pub trait ToAttributeValue {
    fn to_attribute_value(&self) -> AttributeValue;
}

// Implement conversions for common types
impl FromAttributeValue for bool {
    fn from_attribute_value(value: &AttributeValue) -> Option<Self> {
        value.as_bool()
    }
}

impl ToAttributeValue for bool {
    fn to_attribute_value(&self) -> AttributeValue {
        AttributeValue::Bool(*self)
    }
}

impl FromAttributeValue for i64 {
    fn from_attribute_value(value: &AttributeValue) -> Option<Self> {
        value.as_integer()
    }
}

impl ToAttributeValue for i64 {
    fn to_attribute_value(&self) -> AttributeValue {
        AttributeValue::Integer(*self)
    }
}

impl FromAttributeValue for f64 {
    fn from_attribute_value(value: &AttributeValue) -> Option<Self> {
        value.as_float()
    }
}

impl ToAttributeValue for f64 {
    fn to_attribute_value(&self) -> AttributeValue {
        AttributeValue::Float(*self)
    }
}

impl FromAttributeValue for String {
    fn from_attribute_value(value: &AttributeValue) -> Option<Self> {
        value.as_string().map(|s| s.to_string())
    }
}

impl ToAttributeValue for String {
    fn to_attribute_value(&self) -> AttributeValue {
        AttributeValue::String(self.clone())
    }
}

impl ToAttributeValue for &str {
    fn to_attribute_value(&self) -> AttributeValue {
        AttributeValue::String(self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_value_types() {
        assert_eq!(AttributeValue::Bool(true).value_type(), ValueType::Bool);
        assert_eq!(AttributeValue::Integer(42).value_type(), ValueType::Integer);
        assert_eq!(AttributeValue::Float(3.14).value_type(), ValueType::Float);
    }
    
    #[test]
    fn test_arithmetic() {
        let a = AttributeValue::Integer(10);
        let b = AttributeValue::Integer(5);
        
        assert_eq!(a.add(&b), Some(AttributeValue::Integer(15)));
        
        let c = AttributeValue::Float(2.5);
        assert_eq!(c.multiply(2.0), Some(AttributeValue::Float(5.0)));
    }
    
    #[test]
    fn test_comparison() {
        let a = AttributeValue::Float(10.0);
        let b = AttributeValue::Float(5.0);
        
        assert!(a.greater_than_or_equal(&b));
        assert!(!a.less_than_or_equal(&b));
    }
    
    #[test]
    fn test_lerp() {
        let a = AttributeValue::Vector3([0.0, 0.0, 0.0]);
        let b = AttributeValue::Vector3([10.0, 10.0, 10.0]);
        
        let result = a.lerp(&b, 0.5).unwrap();
        match result {
            AttributeValue::Vector3(v) => {
                assert_eq!(v, [5.0, 5.0, 5.0]);
            }
            _ => panic!("Wrong type"),
        }
    }
}