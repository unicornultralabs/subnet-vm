use dashmap::DashMap;

use crate::svm::primitive_types::SVMPrimitives;

pub struct SVMMemory {
    objects: DashMap<Vec<u8>, SVMObject>,
}

impl SVMMemory {
    pub fn new() -> Self {
        Self {
            objects: DashMap::new(),
        }
    }

    pub fn set(&self, object: SVMObject) {
        self.objects.insert(object.key.as_bytes().to_vec(), object);
    }

    pub fn unset(&self, key: Vec<u8>) {
        self.objects.remove(&key);
    }
}

pub struct SVMObject {
    key: String,
    inner: SVMPrimitives,
}
