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

macro_rules! hashset (
    { $($key:expr),* } => {
        {
            let mut _s = ::std::collections::HashSet::new();
            $(
                _s.insert($key);
            )*
            _s
        }
    }
);