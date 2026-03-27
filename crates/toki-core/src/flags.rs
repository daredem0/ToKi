use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FlagValue {
    Bool(bool),
    Int(i32),
    String(String),
}

impl FlagValue {
    pub const fn as_int(&self) -> Option<i32> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameFlags {
    #[serde(default)]
    flags: HashMap<String, FlagValue>,
}

impl GameFlags {
    pub fn get(&self, flag: &str) -> Option<&FlagValue> {
        self.flags.get(flag)
    }

    pub fn set(&mut self, flag: impl Into<String>, value: FlagValue) {
        self.flags.insert(flag.into(), value);
    }

    pub fn clear(&mut self, flag: &str) -> bool {
        self.flags.remove(flag).is_some()
    }

    pub fn increment(&mut self, flag: impl Into<String>, amount: i32) -> bool {
        let flag = flag.into();
        match self.flags.get_mut(&flag) {
            Some(FlagValue::Int(value)) => {
                *value = value.saturating_add(amount);
                true
            }
            Some(_) => false,
            None => {
                self.flags.insert(flag, FlagValue::Int(amount));
                true
            }
        }
    }

    pub fn is_set(&self, flag: &str) -> bool {
        self.flags.contains_key(flag)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &FlagValue)> {
        self.flags.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{FlagValue, GameFlags};

    #[test]
    fn increment_updates_existing_integer_flags() {
        let mut flags = GameFlags::default();
        flags.set("coins", FlagValue::Int(2));

        assert!(flags.increment("coins", 3));
        assert_eq!(flags.get("coins"), Some(&FlagValue::Int(5)));
    }

    #[test]
    fn increment_creates_missing_integer_flags() {
        let mut flags = GameFlags::default();

        assert!(flags.increment("coins", 3));
        assert_eq!(flags.get("coins"), Some(&FlagValue::Int(3)));
    }

    #[test]
    fn increment_is_no_op_for_non_integer_flags() {
        let mut flags = GameFlags::default();
        flags.set("quest", FlagValue::String("done".to_string()));

        assert!(!flags.increment("quest", 3));
        assert_eq!(flags.get("quest"), Some(&FlagValue::String("done".to_string())));
    }
}
