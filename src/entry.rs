use std::ops::Deref;

use futures_signals::signal_vec::{MutableVec, MutableVecLockMut};

pub struct Entry<'a, V> {
    key: Option<usize>,
    lock: MutableVecLockMut<'a, V>,
}

impl<'a, V: Copy> Entry<'a, V> {
    fn existing(self, key: usize) -> Value<'a, V> {
        let value = self.lock.get(key).copied().unwrap();
        Value::existing(self, value)
    }

    pub fn is_vacant(&self) -> bool {
        self.key.is_none()
    }

    pub fn is_occupied(&self) -> bool {
        self.key.is_some()
    }

    pub fn key(&self) -> Option<usize> {
        self.key
    }

    pub fn value(self) -> Option<Value<'a, V>> {
        self.key.map(|key| self.existing(key))
    }

    pub fn or_insert(self, value: V) -> Value<'a, V> {
        match self.key {
            Some(key) => self.existing(key),
            None => Value::new(self, value),
        }
    }

    pub fn or_insert_with<F: FnOnce() -> V>(self, value: F) -> Value<'a, V> {
        self.or_insert(value())
    }

    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut Value<'a, V>),
    {
        match self.key {
            Some(key) => {
                let mut existing = self.existing(key);
                f(&mut existing);
                existing.entry.take().unwrap()
            }
            None => self,
        }
    }

    pub fn and_set(mut self, value: V) -> Self {
        match self.key {
            Some(key) => {
                self.lock.set(key, value);
                self
            }
            None => self,
        }
    }

    pub fn or_insert_entry(mut self, value: V) -> Self {
        self.set(value);
        self
    }

    fn set(&mut self, value: V) {
        match self.key {
            Some(index) => {
                self.lock.set(index, value);
            }
            None => {
                let index = self.lock.len();
                self.lock.push(value);
                self.key = Some(index);
            }
        }
    }

    pub fn remove(mut self) -> Option<V> {
        self.key.map(|key| self.lock.remove(key))
    }
}

impl<'a, V: Copy + Default> Entry<'a, V> {
    pub fn or_default(self) -> Value<'a, V> {
        self.or_insert(V::default())
    }
}

pub struct Value<'a, V: Copy> {
    entry: Option<Entry<'a, V>>,
    value: V,
    modified: bool,
}

impl<'a, V: Copy> Drop for Value<'a, V> {
    fn drop(&mut self) {
        if self.modified
            && let Some(mut entry) = self.entry.take()
        {
            entry.set(self.value);
        }
    }
}

impl<'a, V: Copy> Value<'a, V> {
    fn new(entry: Entry<'a, V>, value: V) -> Self {
        Self {
            entry: Some(entry),
            value,
            modified: true,
        }
    }

    fn existing(entry: Entry<'a, V>, value: V) -> Self {
        Self {
            entry: Some(entry),
            value,
            modified: false,
        }
    }

    pub fn inspect_mut<F>(&mut self, f: F) -> bool
    where
        F: FnOnce(&mut V) -> bool,
    {
        self.modified = f(&mut self.value);
        self.modified
    }

    pub fn set(&mut self, value: V) -> bool {
        self.value = value;
        self.modified = true;
        self.modified
    }

    pub fn set_neq(&mut self, value: V) -> bool
    where
        V: PartialEq,
    {
        if &self.value != &value {
            self.value = value;
            self.modified = true;
        }
        self.modified
    }
}

impl<'a, V: Copy> Deref for Value<'a, V> {
    type Target = V;

    fn deref(&self) -> &V {
        &self.value
    }
}

pub struct EntryCloned<'a, V> {
    key: Option<usize>,
    lock: MutableVecLockMut<'a, V>,
}

impl<'a, V: Clone> EntryCloned<'a, V> {
    fn existing(self, key: usize) -> ValueCloned<'a, V> {
        let value = self.lock.get(key).cloned().unwrap();
        ValueCloned::existing(self, value)
    }

    pub fn is_vacant(&self) -> bool {
        self.key.is_none()
    }

    pub fn is_occupied(&self) -> bool {
        self.key.is_some()
    }

    pub fn key(&self) -> Option<usize> {
        self.key
    }

    pub fn value(self) -> Option<ValueCloned<'a, V>> {
        self.key.map(|key| self.existing(key))
    }

    pub fn or_insert(self, value: V) -> ValueCloned<'a, V> {
        match self.key {
            Some(key) => self.existing(key),
            None => ValueCloned::new(self, value),
        }
    }

    pub fn or_insert_with<F: FnOnce() -> V>(self, value: F) -> ValueCloned<'a, V> {
        self.or_insert(value())
    }

    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut ValueCloned<'a, V>),
    {
        match self.key {
            Some(key) => {
                let mut existing = self.existing(key);
                f(&mut existing);
                existing.entry.take().unwrap()
            }
            None => self,
        }
    }

    pub fn and_set(mut self, value: V) -> Self {
        match self.key {
            Some(key) => {
                self.lock.set_cloned(key, value);
                self
            }
            None => self,
        }
    }

    pub fn or_insert_entry(mut self, value: V) -> Self {
        self.set(value);
        self
    }

    fn set(&mut self, value: V) {
        match self.key {
            Some(index) => {
                self.lock.set_cloned(index, value);
            }
            None => {
                let index = self.lock.len();
                self.lock.push_cloned(value);
                self.key = Some(index);
            }
        }
    }

    pub fn remove(mut self) -> Option<V> {
        self.key.map(|key| self.lock.remove(key))
    }
}

impl<'a, V: Clone + Default> EntryCloned<'a, V> {
    pub fn or_default(self) -> ValueCloned<'a, V> {
        self.or_insert(V::default())
    }
}

pub struct ValueCloned<'a, V: Clone> {
    entry: Option<EntryCloned<'a, V>>,
    value: V,
    modified: bool,
}

impl<'a, V: Clone> ValueCloned<'a, V> {
    fn new(entry: EntryCloned<'a, V>, value: V) -> Self {
        Self {
            entry: Some(entry),
            value,
            modified: true,
        }
    }

    fn existing(entry: EntryCloned<'a, V>, value: V) -> Self {
        Self {
            entry: Some(entry),
            value,
            modified: false,
        }
    }

    pub fn inspect_mut<F>(&mut self, f: F) -> bool
    where
        F: FnOnce(&mut V) -> bool,
    {
        self.modified = f(&mut self.value);
        self.modified
    }

    pub fn set(&mut self, value: V) -> bool {
        self.value = value;
        self.modified = true;
        self.modified
    }

    pub fn set_neq(&mut self, value: V) -> bool
    where
        V: PartialEq,
    {
        if &self.value != &value {
            self.value = value;
            self.modified = true;
        }
        self.modified
    }
}

impl<'a, V: Clone> Deref for ValueCloned<'a, V> {
    type Target = V;

    fn deref(&self) -> &V {
        &self.value
    }
}

impl<'a, V: Clone> Drop for ValueCloned<'a, V> {
    fn drop(&mut self) {
        if self.modified
            && let Some(mut entry) = self.entry.take()
        {
            entry.set(self.value.clone());
        }
    }
}

pub trait MutableVecEntry<V> {
    fn entry<'a, F>(&'a self, f: F) -> Entry<'a, V>
    where
        F: FnMut(&V) -> bool;

    fn entry_cloned<'a, F>(&'a self, f: F) -> EntryCloned<'a, V>
    where
        F: FnMut(&V) -> bool;
}

impl<V> MutableVecEntry<V> for MutableVec<V> {
    fn entry<'a, F>(&'a self, f: F) -> Entry<'a, V>
    where
        F: FnMut(&V) -> bool,
    {
        let lock = self.lock_mut();
        let key = lock.iter().position(f);
        Entry { key, lock }
    }

    fn entry_cloned<'a, F>(&'a self, f: F) -> EntryCloned<'a, V>
    where
        F: FnMut(&V) -> bool,
    {
        let lock = self.lock_mut();
        let key = lock.iter().position(f);
        EntryCloned { key, lock }
    }
}
