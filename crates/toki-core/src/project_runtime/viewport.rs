use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum RuntimeViewportMode {
    AspectFit {
        #[serde(default = "default_aspect_fit_percent")]
        fit_percent: u16,
    },
    IntegerScale {
        #[serde(default)]
        factor: IntegerScaleFactor,
    },
}

impl Default for RuntimeViewportMode {
    fn default() -> Self {
        default_runtime_viewport_mode()
    }
}

impl RuntimeViewportMode {
    pub fn fit_percent(self) -> u16 {
        match self {
            Self::AspectFit { fit_percent } => fit_percent.max(1),
            Self::IntegerScale { .. } => default_aspect_fit_percent(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerScaleFactor {
    Auto,
    Fixed(u8),
}

impl Default for IntegerScaleFactor {
    fn default() -> Self {
        Self::Auto
    }
}

impl Serialize for IntegerScaleFactor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Auto => serializer.serialize_str("auto"),
            Self::Fixed(value) => serializer.serialize_u8(*value),
        }
    }
}

impl<'de> Deserialize<'de> for IntegerScaleFactor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IntegerScaleFactorVisitor;

        impl Visitor<'_> for IntegerScaleFactorVisitor {
            type Value = IntegerScaleFactor;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(r#""auto" or a positive integer scale factor"#)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.eq_ignore_ascii_case("auto") {
                    Ok(IntegerScaleFactor::Auto)
                } else {
                    Err(E::invalid_value(Unexpected::Str(value), &self))
                }
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let value = u8::try_from(value)
                    .map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &self))?;
                if value == 0 {
                    return Err(E::invalid_value(Unexpected::Unsigned(0), &self));
                }
                Ok(IntegerScaleFactor::Fixed(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let value = u64::try_from(value)
                    .map_err(|_| E::invalid_value(Unexpected::Signed(value), &self))?;
                self.visit_u64(value)
            }
        }

        deserializer.deserialize_any(IntegerScaleFactorVisitor)
    }
}

pub const fn default_aspect_fit_percent() -> u16 {
    100
}

pub const fn default_runtime_viewport_mode() -> RuntimeViewportMode {
    RuntimeViewportMode::IntegerScale {
        factor: IntegerScaleFactor::Auto,
    }
}
