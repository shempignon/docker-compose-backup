pub fn stringify<T>(err: T) -> String
where
    T: ToString,
{
    err.to_string()
}
