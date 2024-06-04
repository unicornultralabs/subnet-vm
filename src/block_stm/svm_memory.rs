use crate::svm::primitive_types::SVMPrimitives;
use dashmap::DashMap;
use std::{collections::HashMap, sync::Arc};

type Version = u64;

#[derive(Clone)]
pub struct SVMObject<T> {
    value: T,
    version: Version,
}

#[derive(Clone)]
pub struct SVMMemory {
    objects: Arc<DashMap<Vec<u8>, SVMObject<SVMPrimitives>>>,
}

impl SVMMemory {
    pub fn new() -> Self {
        Self {
            objects: Arc::new(DashMap::new()),
        }
    }

    pub fn get(&self, key: Vec<u8>) -> Option<SVMObject<SVMPrimitives>> {
        self.objects.get(&key).map(|x| x.value().clone())
    }

    pub fn set(&self, key: Vec<u8>, object: SVMObject<SVMPrimitives>) {
        self.objects.insert(key, object);
    }
}

struct Transaction<'a> {
    tm: &'a SVMMemory,
    read_set: HashMap<Vec<u8>, (SVMPrimitives, Version)>,
    write_set: HashMap<Vec<u8>, SVMPrimitives>,
}

impl<'a> Transaction<'a> {
    fn new(tm: &'a SVMMemory) -> Self {
        Transaction {
            tm,
            read_set: HashMap::new(),
            write_set: HashMap::new(),
        }
    }

    fn read(&mut self, key: Vec<u8>) -> Option<SVMPrimitives> {
        if let Some(value) = self.write_set.get(&key) {
            return Some(value.clone());
        }

        if let Some(tv) = self.tm.get(key.clone()) {
            self.read_set.insert(key, (tv.value.clone(), tv.version));
            return Some(tv.value);
        }

        None
    }

    fn write(&mut self, key: Vec<u8>, value: SVMPrimitives) {
        self.write_set.insert(key, value);
    }

    fn commit(self) -> bool {
        // Validate
        for (key, (value, version)) in self.read_set {
            if let Some(tv) = self.tm.objects.get(&key) {
                if tv.version != version {
                    return false; // Conflict detected, abort
                }
            }
        }

        // Commit
        for (key, value) in self.write_set {
            let version = self.tm.objects.get(&key).map_or(0, |tv| tv.version) + 1;
            self.tm.objects.insert(key, SVMObject { value, version });
        }

        true
    }
}
