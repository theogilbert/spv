macro_rules! hashmap (
    { $($key:expr => $value:expr),* } => {
        {
            let mut _m = ::std::collections::HashMap::new();
            $(
                _m.insert($key, $value);
            )*
            _m
        }
    }
);
