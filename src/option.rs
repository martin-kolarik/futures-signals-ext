use std::ops::Deref;

use futures_signals::signal::{Mutable, Signal};

use crate::MutableExt;

#[derive(Debug)]
pub struct MutableOption<T>(Mutable<Option<T>>);

impl<T> Default for MutableOption<T> {
    fn default() -> Self {
        Self(Mutable::new(None))
    }
}

impl<T> Clone for MutableOption<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for MutableOption<T> {
    type Target = Mutable<Option<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> MutableOption<T> {
    pub fn new_empty() -> Self {
        Self(Mutable::new(None))
    }

    pub fn new_default() -> Self
    where
        T: Default,
    {
        Self(Mutable::new(Some(T::default())))
    }

    pub fn new_value(value: Option<T>) -> Self {
        Self(Mutable::new(value))
    }

    pub fn new_some_value(value: T) -> Self {
        Self(Mutable::new(Some(value)))
    }

    pub fn is_none(&self) -> bool {
        self.0.lock_ref().is_none()
    }

    pub fn is_some(&self) -> bool {
        self.0.lock_ref().is_some()
    }

    pub fn take(self) -> Option<T> {
        self.0.take()
    }

    pub fn take_if_value(&self, value: &T) -> Option<T>
    where
        T: PartialEq,
    {
        let mut current = self.0.lock_mut();
        match &*current {
            Some(current_value) if current_value == value => current.take(),
            _ => None,
        }
    }

    pub fn as_mutable(&self) -> Mutable<Option<T>> {
        self.0.clone()
    }

    pub fn map<F>(&self, f: impl FnOnce(&T) -> F) -> Option<F> {
        self.0.lock_ref().as_ref().map(f)
    }

    pub fn map_or<U>(&self, f: impl FnOnce(&T) -> U, default: U) -> U {
        self.map(f).unwrap_or(default)
    }

    pub fn map_or_else<U, D>(&self, f: impl FnOnce(&T) -> U, default: D) -> U
    where
        D: FnOnce() -> U,
    {
        self.map(f).unwrap_or_else(default)
    }

    pub fn map_or_default<U>(&self, f: impl FnOnce(&T) -> U) -> U
    where
        U: Default,
    {
        self.map(f).unwrap_or_default()
    }

    pub fn and_then<U>(&self, f: impl FnOnce(&T) -> Option<U>) -> Option<U> {
        self.0.lock_ref().as_ref().and_then(f)
    }

    pub fn signal_some_default(&self) -> impl Signal<Item = T>
    where
        T: Default + Copy,
    {
        self.signal_map_some_default(|v| *v)
    }

    pub fn signal_cloned_some_default(&self) -> impl Signal<Item = T>
    where
        T: Default + Clone,
    {
        self.signal_map_some_default(|v| v.clone())
    }

    pub fn signal_map<F, U>(&self, mut f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(Option<&T>) -> Option<U>,
    {
        self.0.signal_ref(move |v| f(v.as_ref()))
    }

    pub fn signal_map_some<F, U>(&self, mut f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(&T) -> U,
    {
        self.0.signal_ref(move |v| v.as_ref().map(&mut f))
    }

    pub fn signal_and_then_some<F, U>(&self, mut f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(&T) -> Option<U>,
    {
        self.0.signal_ref(move |v| v.as_ref().and_then(&mut f))
    }

    pub fn signal_and_then_some_or<F, U>(&self, mut f: F, default: U) -> impl Signal<Item = U>
    where
        F: FnMut(&T) -> Option<U>,
        U: Clone,
    {
        self.0.signal_ref(move |v| {
            v.as_ref()
                .and_then(&mut f)
                .unwrap_or_else(|| default.clone())
        })
    }

    pub fn signal_and_then_some_or_else<F, D, U>(
        &self,
        mut f: F,
        default: D,
    ) -> impl Signal<Item = U>
    where
        F: FnMut(&T) -> Option<U>,
        D: FnOnce() -> U + Clone,
    {
        self.0
            .signal_ref(move |v| v.as_ref().and_then(&mut f).unwrap_or_else(default.clone()))
    }

    pub fn signal_map_some_or<F, U>(&self, mut f: F, default: U) -> impl Signal<Item = U>
    where
        F: FnMut(&T) -> U,
        U: Clone,
    {
        self.0
            .signal_ref(move |v| v.as_ref().map(&mut f).unwrap_or(default.clone()))
    }

    pub fn signal_map_some_or_else<F, D, U>(&self, mut f: F, default: D) -> impl Signal<Item = U>
    where
        F: FnMut(&T) -> U,
        D: FnOnce() -> U + Clone,
    {
        self.0
            .signal_ref(move |v| v.as_ref().map(&mut f).unwrap_or_else(default.clone()))
    }

    pub fn signal_map_some_default<F, U>(&self, mut f: F) -> impl Signal<Item = U>
    where
        F: FnMut(&T) -> U,
        U: Default,
    {
        self.0
            .signal_ref(move |v| v.as_ref().map(&mut f).unwrap_or_default())
    }
}
