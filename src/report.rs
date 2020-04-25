pub fn report<T>(err: T)
where
    T: ToString,
{
    error!("{}", err.to_string());
}
