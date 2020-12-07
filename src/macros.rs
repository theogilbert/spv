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

macro_rules! vecdeque (
    { $($value:expr),* } => {
        {
            let mut _vd = ::std::collections::VecDeque::new();
            $(
                _vd.push_back($value);
            )*
            _vd
        }
    }
);