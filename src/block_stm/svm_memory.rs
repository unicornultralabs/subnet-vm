use crate::svm::{
    object::{SVMObject, Version},
    primitive_types::SVMPrimitives,
};
use dashmap::DashMap;
use std::{collections::HashMap, sync::Arc, thread::sleep, time::Duration};

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

pub struct Transaction<'a> {
    tm: &'a SVMMemory,
    read_set: HashMap<Vec<u8>, (SVMPrimitives, Version)>,
    write_set: HashMap<Vec<u8>, SVMPrimitives>,
}

impl<'a> Transaction<'a> {
    pub fn new(tm: &'a Arc<SVMMemory>) -> Self {
        Transaction {
            tm,
            read_set: HashMap::new(),
            write_set: HashMap::new(),
        }
    }

    pub fn read(&mut self, key: Vec<u8>) -> Option<SVMPrimitives> {
        if let Some(value) = self.write_set.get(&key) {
            return Some(value.clone());
        }

        if let Some(tv) = self.tm.get(key.clone()) {
            self.read_set.insert(key, (tv.value.clone(), tv.version));
            return Some(tv.value);
        }

        None
    }

    pub fn write(&mut self, key: Vec<u8>, value: SVMPrimitives) {
        self.write_set.insert(key, value);
    }

    fn commit(&self) -> Result<(), &'static str> {
        // Validate
        for (key, (_, version)) in &self.read_set {
            if let Some(tv) = self.tm.objects.get(key) {
                if tv.version != *version {
                    return Err("Conflict detected, transaction aborted");
                }
            }
        }

        // Commit
        for (key, value) in &self.write_set {
            let version = self.tm.objects.get(key).map_or(0, |tv| tv.version) + 1;
            self.tm.objects.insert(
                key.clone(),
                SVMObject {
                    value: value.clone(),
                    version,
                },
            );
        }

        Ok(())
    }

    fn rollback(&mut self) {
        self.read_set.clear();
        self.write_set.clear();
    }
}

pub fn retry_transaction<F>(tm: Arc<SVMMemory>, transaction_fn: F) -> Result<(), String>
where
    F: Fn(&mut Transaction) -> Result<(), String>,
{
    loop {
        let mut txn = Transaction::new(&tm);
        if let Err(e) = transaction_fn(&mut txn) {
            return Err("transaction_fn execution failed".to_owned());
        }

        match txn.commit() {
            Ok(_) => return Ok(()),
            Err(_) => {
                txn.rollback();
                sleep(Duration::from_millis(10)); // Simple backoff strategy
            }
        }
    }
}
