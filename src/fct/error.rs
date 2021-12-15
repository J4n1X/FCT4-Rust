// TODO: Add custom error type to replace the macro used here

// a macro that tries to unwrap a Result and print an error message if it fails
macro_rules! unwrap_or_return_error {
    ($e:expr, $m:expr) => (match $e {
        Ok(v) => v,
        Err(_) => {
            println!("{}", $m);
            return Err($m);
        }
    })
}

pub(crate) use unwrap_or_return_error;