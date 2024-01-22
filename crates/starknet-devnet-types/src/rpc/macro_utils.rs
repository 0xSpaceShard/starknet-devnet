#[macro_export]
macro_rules! impl_wrapper_serialize {
    ($wrapper_name:ident) => {
        impl serde::Serialize for $wrapper_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                self.inner.serialize(serializer)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_wrapper_deserialize {
    ($wrapper_name:ident, $name:ident) => {
        impl<'de> serde::Deserialize<'de> for $wrapper_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Ok($wrapper_name { inner: $name::deserialize(deserializer)? })
            }
        }
    };
}
