pub type Version = u64;

#[derive(Clone)]
pub struct SVMObject<T> {
    pub value: T,
    pub version: Version,
}
