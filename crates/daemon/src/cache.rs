use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
struct Entry {
    value: Vec<u8>,
    expires_at: Instant,
}

#[derive(Clone, Debug)]
pub struct Cache {
    max_size: usize,
    entries: BTreeMap<String, Entry>,
    insertion_order: VecDeque<String>,
}

impl Cache {
    pub fn new(max_size: usize) -> Self {
        Cache { max_size, entries: BTreeMap::new(), insertion_order: VecDeque::new() }
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.get_at(key, Instant::now())
    }

    pub fn get_at(&self, key: &str, now: Instant) -> Option<Vec<u8>> {
        let entry = self.entries.get(key)?;
        if now >= entry.expires_at {
            return None;
        }
        Some(entry.value.clone())
    }

    pub fn set(&mut self, key: String, value: Vec<u8>, ttl: Duration) {
        self.set_at(key, value, Instant::now(), ttl);
    }

    pub fn set_at(&mut self, key: String, value: Vec<u8>, now: Instant, ttl: Duration) {
        let expires_at = now + ttl;
        if self.entries.contains_key(&key) {
            self.entries.insert(key.clone(), Entry { value, expires_at });
        } else {
            self.entries.insert(key.clone(), Entry { value, expires_at });
            self.insertion_order.push_back(key);
            if self.entries.len() > self.max_size {
                if let Some(oldest) = self.insertion_order.pop_front() {
                    self.entries.remove(&oldest);
                }
            }
        }
    }

    pub fn evict(&mut self) {
        self.evict_at(Instant::now());
    }

    pub fn evict_at(&mut self, now: Instant) {
        self.entries.retain(|_k, v| v.expires_at > now);
        self.insertion_order.retain(|k| self.entries.contains_key(k));
        while self.entries.len() > self.max_size {
            if let Some(oldest) = self.insertion_order.pop_front() {
                self.entries.remove(&oldest);
            } else {
                break;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
