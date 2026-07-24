macro_rules! string_kind {
    (
        $(#[$meta:meta])*
        $name:ident, $category:literal, {
            $($variant:ident => $value:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[non_exhaustive]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $($variant,)+
            Custom($crate::String),
        }

        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $value,)+
                    Self::Custom(value) => value,
                }
            }

            /// Creates an application-specific kind without changing this crate.
            ///
            /// # Errors
            ///
            /// Returns an error for empty values or surrounding whitespace.
            pub fn custom(value: impl Into<$crate::String>) -> crate::Result<Self> {
                Self::parse_owned(value.into())
            }

            fn parse_owned(value: $crate::String) -> crate::Result<Self> {
                if value.is_empty() || value.trim() != value {
                    return Err(crate::GraphError::InvalidKind {
                        category: $category,
                        value,
                    });
                }
                Ok(match value.as_str() {
                    $($value => Self::$variant,)+
                    _ => Self::Custom(value),
                })
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl core::str::FromStr for $name {
            type Err = crate::GraphError;

            fn from_str(value: &str) -> crate::Result<Self> {
                Self::parse_owned($crate::String::from(value))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::de::Error as _;

                let value = <$crate::String as serde::Deserialize>::deserialize(deserializer)?;
                Self::parse_owned(value).map_err(D::Error::custom)
            }
        }
    };
}
