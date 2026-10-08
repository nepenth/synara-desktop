//! Closed-vocabulary FFI enums with their stable wire spelling.
//!
//! Swift receives a real enum. Rust callers and tests keep the snake_case
//! wire label through `as_str`, `Display` and `PartialEq<&str>`, and parse
//! with `from_wire`, which fails closed on anything outside the vocabulary.

macro_rules! wire_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident => $wire:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
        pub enum $name {
            $( $(#[$vmeta])* $variant ),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$( $name::$variant ),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $wire ),+
                }
            }

            pub fn from_wire(value: &str) -> Option<Self> {
                match value {
                    $( $wire => Some($name::$variant), )+
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }
    };
}

pub(crate) use wire_enum;
