use futures_signals::{
    signal::{Mutable, Signal, SignalExt},
    signal_vec::{
        Filter, FilterMap, FilterSignalCloned, MutableSignalVec, MutableVec, MutableVecLockMut,
        SignalVec, SignalVecExt,
    },
};
use pin_project_lite::pin_project;
use std::{
    collections::VecDeque,
    hash::Hash,
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Flatten, MutableVecEntry, SignalVecSpawn};

#[cfg(feature = "ahash")]
type Hasher = ahash::RandomState;
#[cfg(not(feature = "ahash"))]
type Hasher = std::hash::RandomState;

type HashMap<K, V> = std::collections::HashMap<K, V, Hasher>;

fn collect_hash_map<K, V, I>(iter: I) -> HashMap<K, V>
where
    K: Eq + Hash,
    I: Iterator<Item = (K, V)>,
{
    #[cfg(feature = "ahash")]
    {
        let mut map = HashMap::with_hasher(Hasher::with_seed(250402117));
        map.extend(iter);
        map
    }
    #[cfg(not(feature = "ahash"))]
    iter.collect()
}

pub trait MutableExt<A> {
    fn inspect(&self, f: impl FnMut(&A));
    fn inspect_mut(&self, f: impl FnMut(&mut A));

    fn map<B>(&self, f: impl FnOnce(&A) -> B) -> B;
    fn map_mut<B>(&self, f: impl FnOnce(&mut A) -> B) -> B;

    fn into_inner(self) -> A
    where
        A: Default,
        Self: Sized,
    {
        self.map_mut(mem::take)
    }

    fn take(&self) -> A
    where
        A: Default,
    {
        self.map_mut(mem::take)
    }
}

impl<A> MutableExt<A> for Mutable<A> {
    fn inspect(&self, mut f: impl FnMut(&A)) {
        f(&self.lock_ref())
    }

    fn inspect_mut(&self, mut f: impl FnMut(&mut A)) {
        f(&mut self.lock_mut())
    }

    fn map<B>(&self, f: impl FnOnce(&A) -> B) -> B {
        f(&self.lock_ref())
    }

    fn map_mut<B>(&self, f: impl FnOnce(&mut A) -> B) -> B {
        f(&mut self.lock_mut())
    }
}

pub trait MutableVecExt<A> {
    fn inspect_vec(&self, f: impl FnMut(&[A]));
    fn inspect_vec_mut(&self, f: impl FnMut(&mut MutableVecLockMut<A>));

    fn map_vec<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&[A]) -> U;

    fn map_vec_mut<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&mut MutableVecLockMut<A>) -> U;

    fn find_inspect_mut<P, F>(&self, predicate: P, f: F) -> Option<bool>
    where
        A: Copy,
        P: FnMut(&A) -> bool,
        F: FnMut(&mut A) -> bool;

    fn find_inspect_mut_cloned<P, F>(&self, predicate: P, f: F) -> Option<bool>
    where
        A: Clone,
        P: FnMut(&A) -> bool,
        F: FnMut(&mut A) -> bool;

    fn map<F, U>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&A) -> U;

    fn enumerate_map<F, U>(&self, f: F) -> Vec<U>
    where
        F: FnMut(usize, &A) -> U;

    fn filter<P>(&self, p: P) -> Vec<A>
    where
        A: Copy,
        P: FnMut(&A) -> bool;

    fn filter_cloned<P>(&self, p: P) -> Vec<A>
    where
        A: Clone,
        P: FnMut(&A) -> bool;

    fn filter_map<P, U>(&self, p: P) -> Vec<U>
    where
        P: FnMut(&A) -> Option<U>;

    fn find<P>(&self, p: P) -> Option<A>
    where
        A: Copy,
        P: FnMut(&A) -> bool;

    fn find_cloned<P>(&self, p: P) -> Option<A>
    where
        A: Clone,
        P: FnMut(&A) -> bool;

    fn find_map<P, U>(&self, p: P) -> Option<U>
    where
        P: FnMut(&A) -> Option<U>;

    fn find_set<P>(&self, p: P, item: A) -> bool
    where
        A: Copy,
        P: FnMut(&A) -> bool;

    fn find_set_cloned<P>(&self, p: P, item: A) -> bool
    where
        A: Clone,
        P: FnMut(&A) -> bool;

    fn find_set_or_add<P>(&self, p: P, item: A)
    where
        A: Copy,
        P: FnMut(&A) -> bool;

    fn find_set_or_add_cloned<P>(&self, p: P, item: A)
    where
        A: Clone,
        P: FnMut(&A) -> bool;

    fn find_set_if<P, F, I>(&self, p: P, item: F, i: I) -> bool
    where
        A: Copy,
        F: FnMut() -> A,
        P: FnMut(&A) -> bool,
        I: FnMut(&A) -> bool;

    fn find_set_if_cloned<P, F, I>(&self, p: P, item: F, i: I) -> bool
    where
        A: Clone,
        F: FnMut() -> A,
        P: FnMut(&A) -> bool,
        I: FnMut(&A) -> bool;

    fn find_set_if_or_add<P, F, I>(&self, p: P, item: F, i: I)
    where
        A: Copy,
        F: FnMut() -> A,
        P: FnMut(&A) -> bool,
        I: FnMut(&A) -> bool;

    fn find_set_if_or_add_cloned<P, F, I>(&self, p: P, item: F, i: I)
    where
        A: Clone,
        F: FnMut() -> A,
        P: FnMut(&A) -> bool,
        I: FnMut(&A) -> bool;

    fn find_remove<P>(&self, p: P) -> bool
    where
        A: Copy,
        P: FnMut(&A) -> bool;

    fn find_remove_cloned<P>(&self, p: P) -> bool
    where
        A: Clone,
        P: FnMut(&A) -> bool;

    fn extend(&self, source: impl IntoIterator<Item = A>)
    where
        A: Copy;

    fn extend_cloned(&self, source: impl IntoIterator<Item = A>)
    where
        A: Clone;

    fn replace<P>(&self, what: P, with: impl IntoIterator<Item = A>)
    where
        A: Copy,
        P: FnMut(&A) -> bool;

    fn replace_cloned<P>(&self, what: P, with: impl IntoIterator<Item = A>)
    where
        A: Clone,
        P: FnMut(&A) -> bool;

    fn replace_keyed<F, K>(&self, key: F, source: impl IntoIterator<Item = A>) -> bool
    where
        A: Copy,
        F: FnMut(&A) -> K,
        K: Eq + Hash;

    fn replace_keyed_cloned<F, K>(&self, f: F, source: impl IntoIterator<Item = A>) -> bool
    where
        A: Clone,
        F: FnMut(&A) -> K,
        K: Eq + Hash;

    fn synchronize<F, K>(&self, key: F, source: impl IntoIterator<Item = A>) -> bool
    where
        A: Copy,
        F: FnMut(&A) -> K,
        K: Eq + Hash;

    fn synchronize_cloned<F, K>(&self, key: F, source: impl IntoIterator<Item = A>) -> bool
    where
        A: Clone,
        F: FnMut(&A) -> K,
        K: Eq + Hash;

    fn take(&self) -> Vec<A>;

    #[cfg(feature = "spawn")]
    fn feed(&self, source: impl SignalVec<Item = A> + Send + 'static)
    where
        A: Copy + Send + Sync + 'static;

    #[cfg(feature = "spawn")]
    fn feed_cloned(&self, source: impl SignalVec<Item = A> + Send + 'static)
    where
        A: Clone + Send + Sync + 'static;

    #[cfg(feature = "spawn-local")]
    fn feed_local(&self, source: impl SignalVec<Item = A> + 'static)
    where
        A: Copy + 'static;

    #[cfg(feature = "spawn-local")]
    fn feed_local_cloned(&self, source: impl SignalVec<Item = A> + 'static)
    where
        A: Clone + 'static;

    fn signal_vec_filter<P>(&self, p: P) -> Filter<MutableSignalVec<A>, P>
    where
        A: Copy,
        P: FnMut(&A) -> bool;

    fn signal_vec_filter_cloned<P>(&self, p: P) -> Filter<MutableSignalVec<A>, P>
    where
        A: Clone,
        P: FnMut(&A) -> bool;

    fn signal_vec_filter_signal<P, S>(&self, p: P) -> FilterSignalCloned<MutableSignalVec<A>, S, P>
    where
        A: Copy,
        P: FnMut(&A) -> S,
        S: Signal<Item = bool>;

    fn signal_vec_filter_signal_cloned<P, S>(
        &self,
        p: P,
    ) -> FilterSignalCloned<MutableSignalVec<A>, S, P>
    where
        A: Clone,
        P: FnMut(&A) -> S,
        S: Signal<Item = bool>;

    fn signal_vec_filter_map<P, U>(&self, p: P) -> FilterMap<MutableSignalVec<A>, P>
    where
        A: Copy,
        P: FnMut(A) -> Option<U>;

    fn signal_vec_filter_map_cloned<P, U>(&self, p: P) -> FilterMap<MutableSignalVec<A>, P>
    where
        A: Clone,
        P: FnMut(A) -> Option<U>;
}

impl<A> MutableVecExt<A> for MutableVec<A> {
    #[inline]
    fn inspect_vec(&self, mut f: impl FnMut(&[A])) {
        f(&self.lock_ref())
    }

    #[inline]
    fn inspect_vec_mut(&self, mut f: impl FnMut(&mut MutableVecLockMut<A>)) {
        f(&mut self.lock_mut())
    }

    fn map_vec<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&[A]) -> U,
    {
        f(&self.lock_ref())
    }

    fn map_vec_mut<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&mut MutableVecLockMut<A>) -> U,
    {
        f(&mut self.lock_mut())
    }

    /// Return parameter of F (changed) drives if the value should be written back,
    /// and cause MutableVec change. If F returns false, no change is induced neither
    /// reported.
    fn find_inspect_mut<P, F>(&self, predicate: P, f: F) -> Option<bool>
    where
        A: Copy,
        P: FnMut(&A) -> bool,
        F: FnMut(&mut A) -> bool,
    {
        self.entry(predicate)
            .value()
            .map(|mut value| value.inspect_mut(f))
    }

    /// Return parameter of F (changed) drives if the value should be written back,
    /// and cause MutableVec change. If F returns false, no change is induced neither
    /// reported.
    fn find_inspect_mut_cloned<P, F>(&self, predicate: P, f: F) -> Option<bool>
    where
        A: Clone,
        P: FnMut(&A) -> bool,
        F: FnMut(&mut A) -> bool,
    {
        self.entry_cloned(predicate)
            .value()
            .map(|mut value| value.inspect_mut(f))
    }

    fn map<F, U>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&A) -> U,
    {
        self.lock_ref().iter().map(f).collect()
    }

    fn enumerate_map<F, U>(&self, mut f: F) -> Vec<U>
    where
        F: FnMut(usize, &A) -> U,
    {
        self.lock_ref()
            .iter()
            .enumerate()
            .map(|(index, item)| f(index, item))
            .collect()
    }

    fn filter<P>(&self, mut p: P) -> Vec<A>
    where
        A: Copy,
        P: FnMut(&A) -> bool,
    {
        self.lock_ref().iter().filter(|&a| p(a)).copied().collect()
    }

    fn filter_cloned<P>(&self, mut p: P) -> Vec<A>
    where
        A: Clone,
        P: FnMut(&A) -> bool,
    {
        self.lock_ref().iter().filter(|&a| p(a)).cloned().collect()
    }

    fn filter_map<P, U>(&self, p: P) -> Vec<U>
    where
        P: FnMut(&A) -> Option<U>,
    {
        self.lock_ref().iter().filter_map(p).collect()
    }

    fn find<P>(&self, mut p: P) -> Option<A>
    where
        A: Copy,
        P: FnMut(&A) -> bool,
    {
        self.lock_ref().iter().find(|&a| p(a)).copied()
    }

    fn find_cloned<P>(&self, mut p: P) -> Option<A>
    where
        A: Clone,
        P: FnMut(&A) -> bool,
    {
        self.lock_ref().iter().find(|&a| p(a)).cloned()
    }

    fn find_map<P, U>(&self, p: P) -> Option<U>
    where
        P: FnMut(&A) -> Option<U>,
    {
        self.lock_ref().iter().find_map(p)
    }

    fn find_set<P>(&self, p: P, item: A) -> bool
    where
        A: Copy,
        P: FnMut(&A) -> bool,
    {
        self.entry(p).and_set(item).is_occupied()
    }

    fn find_set_cloned<P>(&self, p: P, item: A) -> bool
    where
        A: Clone,
        P: FnMut(&A) -> bool,
    {
        self.entry_cloned(p).and_set(item).is_occupied()
    }

    fn find_set_or_add<P>(&self, p: P, item: A)
    where
        A: Copy,
        P: FnMut(&A) -> bool,
    {
        self.entry(p).and_set_or_insert(item);
    }

    fn find_set_or_add_cloned<P>(&self, p: P, item: A)
    where
        A: Clone,
        P: FnMut(&A) -> bool,
    {
        self.entry_cloned(p).and_set_or_insert(item);
    }

    fn find_set_if<P, F, I>(&self, p: P, mut item: F, mut i: I) -> bool
    where
        A: Copy,
        F: FnMut() -> A,
        P: FnMut(&A) -> bool,
        I: FnMut(&A) -> bool,
    {
        self.entry(p)
            .and_modify(|existing| {
                existing.inspect_mut(|existing| {
                    if i(existing) {
                        *existing = item();
                        true
                    } else {
                        false
                    }
                });
            })
            .is_occupied()
    }

    fn find_set_if_cloned<P, F, I>(&self, p: P, mut item: F, mut i: I) -> bool
    where
        A: Clone,
        F: FnMut() -> A,
        P: FnMut(&A) -> bool,
        I: FnMut(&A) -> bool,
    {
        self.entry_cloned(p)
            .and_modify(|existing| {
                existing.inspect_mut(|existing| {
                    if i(existing) {
                        *existing = item();
                        true
                    } else {
                        false
                    }
                });
            })
            .is_occupied()
    }

    fn find_set_if_or_add<P, F, I>(&self, p: P, mut item: F, mut i: I)
    where
        A: Copy,
        F: FnMut() -> A,
        P: FnMut(&A) -> bool,
        I: FnMut(&A) -> bool,
    {
        self.entry(p)
            .and_modify(|existing| {
                existing.inspect_mut(|existing| {
                    if i(existing) {
                        *existing = item();
                        true
                    } else {
                        false
                    }
                });
            })
            .or_insert_with(item);
    }

    fn find_set_if_or_add_cloned<P, F, I>(&self, p: P, mut item: F, mut i: I)
    where
        A: Clone,
        F: FnMut() -> A,
        P: FnMut(&A) -> bool,
        I: FnMut(&A) -> bool,
    {
        self.entry_cloned(p)
            .and_modify(|existing| {
                existing.inspect_mut(|existing| {
                    if i(existing) {
                        *existing = item();
                        true
                    } else {
                        false
                    }
                });
            })
            .or_insert_with(item);
    }

    fn find_remove<P>(&self, p: P) -> bool
    where
        A: Copy,
        P: FnMut(&A) -> bool,
    {
        self.entry(p).remove().is_some()
    }

    fn find_remove_cloned<P>(&self, p: P) -> bool
    where
        A: Clone,
        P: FnMut(&A) -> bool,
    {
        self.entry_cloned(p).remove().is_some()
    }

    fn extend(&self, source: impl IntoIterator<Item = A>)
    where
        A: Copy,
    {
        let mut lock = self.lock_mut();
        for item in source.into_iter() {
            lock.push(item);
        }
    }

    fn extend_cloned(&self, source: impl IntoIterator<Item = A>)
    where
        A: Clone,
    {
        let mut lock = self.lock_mut();
        for item in source.into_iter() {
            lock.push_cloned(item);
        }
    }

    fn replace<P>(&self, mut what: P, with: impl IntoIterator<Item = A>)
    where
        A: Copy,
        P: FnMut(&A) -> bool,
    {
        let mut lock = self.lock_mut();
        lock.retain(|item| !what(item));
        for item in with.into_iter() {
            lock.push(item);
        }
    }

    fn replace_cloned<P>(&self, mut what: P, with: impl IntoIterator<Item = A>)
    where
        A: Clone,
        P: FnMut(&A) -> bool,
    {
        let mut lock = self.lock_mut();
        lock.retain(|item| !what(item));
        for item in with.into_iter() {
            lock.push_cloned(item);
        }
    }

    fn replace_keyed<F, K>(&self, mut key: F, source: impl IntoIterator<Item = A>) -> bool
    where
        A: Copy,
        F: FnMut(&A) -> K,
        K: Eq + Hash,
    {
        let source = source.into_iter().map(|item| (key(&item), item));
        let mut source = collect_hash_map(source);

        let mut lock = self.lock_mut();

        let to_replace = lock
            .iter()
            .enumerate()
            .filter_map(|(index, item)| source.remove(&key(item)).map(|item| (index, item)))
            .collect::<Vec<_>>();
        for (index, item) in to_replace {
            lock.set(index, item)
        }

        let extended = !source.is_empty();
        for item in source.into_values() {
            lock.push(item);
        }

        extended
    }

    fn replace_keyed_cloned<F, K>(&self, mut key: F, source: impl IntoIterator<Item = A>) -> bool
    where
        A: Clone,
        F: FnMut(&A) -> K,
        K: Eq + Hash,
    {
        let source = source.into_iter().map(|item| (key(&item), item));
        let mut source = collect_hash_map(source);

        let mut lock = self.lock_mut();

        let to_replace = lock
            .iter()
            .enumerate()
            .filter_map(|(index, item)| source.remove(&key(item)).map(|item| (index, item)))
            .collect::<Vec<_>>();
        for (index, item) in to_replace {
            lock.set_cloned(index, item)
        }

        let extended = !source.is_empty();
        for item in source.into_values() {
            lock.push_cloned(item);
        }

        extended
    }

    fn synchronize<F, K>(&self, mut key: F, source: impl IntoIterator<Item = A>) -> bool
    where
        A: Copy,
        F: FnMut(&A) -> K,
        K: Eq + Hash,
    {
        let source = source.into_iter().map(|item| (key(&item), item));
        let mut source = collect_hash_map(source);

        let mut lock = self.lock_mut();

        let to_remove: Vec<_> = lock
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(index, item)| match source.remove(&key(item)) {
                Some(_) => None,
                None => Some(index),
            })
            .collect();
        // indexes go down, no need to calculate them anyhow
        for index in to_remove.into_iter() {
            lock.remove(index);
        }

        let extended = !source.is_empty();
        for item in source.into_values() {
            lock.push(item);
        }

        extended
    }

    fn synchronize_cloned<F, K>(&self, mut key: F, source: impl IntoIterator<Item = A>) -> bool
    where
        A: Clone,
        F: FnMut(&A) -> K,
        K: Eq + Hash,
    {
        let source = source.into_iter().map(|item| (key(&item), item));
        let mut source = collect_hash_map(source);

        let mut lock = self.lock_mut();

        let to_remove = lock
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(index, item)| match source.remove(&key(item)) {
                Some(_) => None,
                None => Some(index),
            })
            .collect::<Vec<_>>();
        // indexes go down, no need to calculate them anyhow
        for index in to_remove.into_iter() {
            lock.remove(index);
        }

        let extended = !source.is_empty();
        for item in source.into_values() {
            lock.push_cloned(item);
        }

        extended
    }

    fn take(&self) -> Vec<A> {
        self.lock_mut().drain(..).collect()
    }

    #[cfg(feature = "spawn")]
    fn feed(&self, source: impl SignalVec<Item = A> + Send + 'static)
    where
        A: Copy + Send + Sync + 'static,
    {
        source.feed(self.clone());
    }

    #[cfg(feature = "spawn")]
    fn feed_cloned(&self, source: impl SignalVec<Item = A> + Send + 'static)
    where
        A: Clone + Send + Sync + 'static,
    {
        source.feed_cloned(self.clone());
    }

    #[cfg(feature = "spawn-local")]
    fn feed_local(&self, source: impl SignalVec<Item = A> + 'static)
    where
        A: Copy + 'static,
    {
        source.feed_local(self.clone());
    }

    #[cfg(feature = "spawn-local")]
    fn feed_local_cloned(&self, source: impl SignalVec<Item = A> + 'static)
    where
        A: Clone + 'static,
    {
        source.feed_local_cloned(self.clone());
    }

    #[inline]
    fn signal_vec_filter<P>(&self, p: P) -> Filter<MutableSignalVec<A>, P>
    where
        A: Copy,
        P: FnMut(&A) -> bool,
    {
        self.signal_vec().filter(p)
    }

    #[inline]
    fn signal_vec_filter_cloned<P>(&self, p: P) -> Filter<MutableSignalVec<A>, P>
    where
        A: Clone,
        P: FnMut(&A) -> bool,
    {
        self.signal_vec_cloned().filter(p)
    }

    #[inline]
    fn signal_vec_filter_signal<P, S>(&self, p: P) -> FilterSignalCloned<MutableSignalVec<A>, S, P>
    where
        A: Copy,
        P: FnMut(&A) -> S,
        S: Signal<Item = bool>,
    {
        self.signal_vec().filter_signal_cloned(p)
    }

    #[inline]
    fn signal_vec_filter_signal_cloned<P, S>(
        &self,
        p: P,
    ) -> FilterSignalCloned<MutableSignalVec<A>, S, P>
    where
        A: Clone,
        P: FnMut(&A) -> S,
        S: Signal<Item = bool>,
    {
        self.signal_vec_cloned().filter_signal_cloned(p)
    }

    #[inline]
    fn signal_vec_filter_map<P, U>(&self, p: P) -> FilterMap<MutableSignalVec<A>, P>
    where
        A: Copy,
        P: FnMut(A) -> Option<U>,
    {
        self.signal_vec().filter_map(p)
    }

    #[inline]
    fn signal_vec_filter_map_cloned<P, U>(&self, p: P) -> FilterMap<MutableSignalVec<A>, P>
    where
        A: Clone,
        P: FnMut(A) -> Option<U>,
    {
        self.signal_vec_cloned().filter_map(p)
    }
}

pub trait SignalVecFinalizerExt: SignalVec + Sized {
    #[inline]
    fn first(self) -> impl Signal<Item = Option<Self::Item>>
    where
        Self::Item: Copy,
    {
        self.first_map(|i| *i)
    }

    fn first_cloned(self) -> impl Signal<Item = Option<Self::Item>>
    where
        Self::Item: Clone,
    {
        self.first_map(|i| i.clone())
    }

    fn first_map<F, U>(self, mut f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(&Self::Item) -> U,
    {
        self.to_signal_map(move |items| items.first().map(&mut f))
    }

    #[inline]
    fn last(self) -> impl Signal<Item = Option<Self::Item>>
    where
        Self::Item: Copy,
    {
        self.last_map(|i| *i)
    }

    fn last_cloned(self) -> impl Signal<Item = Option<Self::Item>>
    where
        Self::Item: Clone,
    {
        self.last_map(|i| i.clone())
    }

    fn last_map<F, U>(self, mut f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(&Self::Item) -> U,
    {
        self.to_signal_map(move |items| items.last().map(&mut f))
    }

    fn all<F>(self, mut f: F) -> impl Signal<Item = bool>
    where
        F: FnMut(&Self::Item) -> bool,
    {
        self.to_signal_map(move |items| items.iter().all(&mut f))
    }

    fn any<F>(self, mut f: F) -> impl Signal<Item = bool>
    where
        F: FnMut(&Self::Item) -> bool,
    {
        self.to_signal_map(move |items| items.iter().any(&mut f))
    }

    #[inline]
    fn any_item(self) -> impl Signal<Item = bool> {
        self.len().neq(0)
    }
}

impl<S: SignalVec + Sized> SignalVecFinalizerExt for S {}

pub trait SignalVecFlattenExt: SignalVec + Sized {
    fn flatten_ext(self) -> Flatten<Self>
    where
        Self::Item: SignalVec,
    {
        Flatten {
            signal: Some(self),
            inner: vec![],
            pending: VecDeque::new(),
        }
    }
}

impl<S: SignalVec + Sized> SignalVecFlattenExt for S {}

pub trait SignalTimeExt: Signal + Sized {
    #[inline]
    fn debounce<W, F>(
        self,
        window: W,
    ) -> Debounce<Self, W, Self::Item, impl FnMut(Self::Item, Self::Item) -> Self::Item, F> {
        Self::debounce_reduce(self, window, |_, value| -> Self::Item { value })
    }

    fn debounce_reduce<W, R, F>(self, window: W, reduce: R) -> Debounce<Self, W, Self::Item, R, F>
    where
        R: FnMut(Self::Item, Self::Item) -> Self::Item,
    {
        Debounce {
            signal: Some(self),
            window,
            acc: None,
            reduce,
            future: None,
            first: true,
        }
    }

    #[inline]
    fn throttle_ext<D, F>(
        self,
        delay: D,
    ) -> Throttle<Self, D, Self::Item, impl FnMut(Self::Item, Self::Item) -> Self::Item, F> {
        Self::throttle_reduce(self, delay, |_, value| value)
    }

    fn throttle_reduce<D, R, F>(self, delay: D, reduce: R) -> Throttle<Self, D, Self::Item, R, F> {
        Throttle {
            signal: Some(self),
            delay,
            acc: None,
            reduce,
            timeout: None,
        }
    }
}

impl<S: Signal + Sized> SignalTimeExt for S {}

pin_project! {
    #[derive(Debug)]
    #[must_use = "Signals do nothing unless polled"]
    pub struct Debounce<S, W, B, R, D> {
        #[pin]
        signal: Option<S>,
        window: W,
        acc: Option<B>,
        reduce: R,
        #[pin]
        future: Option<D>,
        first: bool,
    }
}

impl<S, W, B, R, F> Signal for Debounce<S, W, B, R, F>
where
    S: Signal<Item = B>,
    W: FnMut() -> F,
    F: Future<Output = ()>,
    R: FnMut(B, B) -> B,
{
    type Item = Option<B>;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        let mut done = false;

        loop {
            match this
                .signal
                .as_mut()
                .as_pin_mut()
                .map(|signal| signal.poll_change(cx))
            {
                None => {
                    done = true;
                }
                Some(Poll::Ready(None)) => {
                    this.signal.set(None);
                    this.future.set(Some((this.window)()));
                    done = true;
                }
                Some(Poll::Ready(Some(value))) => {
                    this.future.set(Some((this.window)()));
                    *this.acc = Some(match this.acc.take() {
                        None => value,
                        Some(acc) => (this.reduce)(acc, value),
                    });
                    continue;
                }
                Some(Poll::Pending) => {}
            }
            break;
        }

        match this
            .future
            .as_mut()
            .as_pin_mut()
            .map(|delay| delay.poll(cx))
        {
            None => {}
            Some(Poll::Ready(_)) => {
                this.future.set(None);
                match this.acc.take() {
                    None => {}
                    Some(value) => {
                        *this.first = false;
                        return Poll::Ready(Some(Some(value)));
                    }
                }
            }
            Some(Poll::Pending) => {
                done = false;
            }
        }

        if *this.first {
            *this.first = false;
            Poll::Ready(Some(None))
        } else if done {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "Signals do nothing unless polled"]
    pub struct Throttle<S, D, B, R, F> {
        #[pin]
        signal: Option<S>,
        delay: D,
        acc: Option<B>,
        reduce: R,
        #[pin]
        timeout: Option<F>,
    }
}

impl<S, D, B, R, F> Signal for Throttle<S, D, B, R, F>
where
    S: Signal<Item = B>,
    D: FnMut() -> F,
    F: Future<Output = ()>,
    R: FnMut(B, B) -> B,
{
    type Item = B;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        let mut done = false;

        loop {
            match this
                .signal
                .as_mut()
                .as_pin_mut()
                .map(|signal| signal.poll_change(cx))
            {
                None => {
                    done = true;
                }
                Some(Poll::Ready(None)) => {
                    this.signal.set(None);
                    done = true;
                }
                Some(Poll::Ready(Some(value))) => {
                    *this.acc = Some(match this.acc.take() {
                        None => value,
                        Some(acc) => (this.reduce)(acc, value),
                    });

                    if this.timeout.is_none() {
                        this.timeout.set(Some((this.delay)()));
                        if let Some(Poll::Ready(())) =
                            this.timeout.as_mut().as_pin_mut().map(|f| f.poll(cx))
                        {
                            this.timeout.set(None);
                        }

                        return Poll::Ready(this.acc.take());
                    }

                    continue;
                }
                Some(Poll::Pending) => {}
            }
            break;
        }

        match this
            .timeout
            .as_mut()
            .as_pin_mut()
            .map(|delay| delay.poll(cx))
        {
            None => {}
            Some(Poll::Ready(_)) => {
                this.timeout.set(None);

                match this.acc.take() {
                    None => {}
                    Some(value) => {
                        return Poll::Ready(Some(value));
                    }
                }
            }
            Some(Poll::Pending) => {
                done = false;
            }
        }

        if done {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

pub trait SignalExtMapBool
where
    Self: Sized,
{
    fn map_bool<T, TM: FnMut() -> T, FM: FnMut() -> T>(
        self,
        t: TM,
        f: FM,
    ) -> MapBool<Self, TM, FM> {
        MapBool {
            signal: self,
            true_mapper: t,
            false_mapper: f,
        }
    }

    fn map_option<T, TM: FnMut() -> T>(self, t: TM) -> MapOption<Self, TM> {
        MapOption {
            signal: self,
            true_mapper: t,
        }
    }
}

impl<S: Signal<Item = bool> + Sized> SignalExtMapBool for S {}

pin_project! {
    #[derive(Debug)]
    #[must_use = "Signals do nothing unless polled"]
    pub struct MapBool<S, TM, FM> {
        #[pin]
        signal: S,
        true_mapper: TM,
        false_mapper: FM,
    }
}

impl<T, S: Signal<Item = bool>, TM: FnMut() -> T, FM: FnMut() -> T> Signal for MapBool<S, TM, FM> {
    type Item = T;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();

        this.signal.poll_change(cx).map(|opt| {
            opt.map(|value| {
                if value {
                    (this.true_mapper)()
                } else {
                    (this.false_mapper)()
                }
            })
        })
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "Signals do nothing unless polled"]
    pub struct MapOption<S, TM> {
        #[pin]
        signal: S,
        true_mapper: TM,
    }
}

impl<T, S: Signal<Item = bool>, TM: FnMut() -> T> Signal for MapOption<S, TM> {
    type Item = Option<T>;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();

        this.signal
            .poll_change(cx)
            .map(|opt| opt.map(|value| value.then(this.true_mapper)))
    }
}

pub trait SignalExtMapOption<T>
where
    Self: Sized,
{
    fn map_some<F, U>(self, f: F) -> MapSome<Self, T, F, U>
    where
        F: FnMut(T) -> U,
    {
        MapSome {
            signal: self,
            mapper: f,
            pt: PhantomData,
            pu: PhantomData,
        }
    }

    fn map_some_default<F, U>(self, f: F) -> MapSomeDefault<Self, T, F, U>
    where
        F: FnMut(T) -> U,
        U: Default,
    {
        MapSomeDefault {
            signal: self,
            mapper: f,
            pt: PhantomData,
            pu: PhantomData,
        }
    }

    fn and_then_some<F, U>(self, f: F) -> AndThenSome<Self, T, F, U>
    where
        F: FnMut(T) -> Option<U>,
    {
        AndThenSome {
            signal: self,
            mapper: f,
            pt: PhantomData,
            pu: PhantomData,
        }
    }

    fn unwrap_or_default(self) -> UnwrapOrDefault<Self, T>
    where
        T: Default,
    {
        UnwrapOrDefault {
            signal: self,
            pt: PhantomData,
        }
    }
}

impl<T, S: Signal<Item = Option<T>> + Sized> SignalExtMapOption<T> for S {}

pin_project! {
    #[derive(Debug)]
    #[must_use = "Signals do nothing unless polled"]
    pub struct MapSome<S, T, F, U> {
        #[pin]
        signal: S,
        mapper: F,
        pt: PhantomData<T>,
        pu: PhantomData<U>,
    }
}

impl<T, S, F, U> Signal for MapSome<S, T, F, U>
where
    S: Signal<Item = Option<T>>,
    F: FnMut(T) -> U,
{
    type Item = Option<U>;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.signal
            .poll_change(cx)
            .map(|opt| opt.map(|opt| opt.map(this.mapper)))
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "Signals do nothing unless polled"]
    pub struct MapSomeDefault<S, T, F, U> {
        #[pin]
        signal: S,
        mapper: F,
        pt: PhantomData<T>,
        pu: PhantomData<U>,
    }
}

impl<T, S, F, U> Signal for MapSomeDefault<S, T, F, U>
where
    S: Signal<Item = Option<T>>,
    F: FnMut(T) -> U,
    U: Default,
{
    type Item = U;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.signal
            .poll_change(cx)
            .map(|opt| opt.map(|opt| opt.map(this.mapper).unwrap_or_default()))
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "Signals do nothing unless polled"]
    pub struct AndThenSome<S, T, F, U> {
        #[pin]
        signal: S,
        mapper: F,
        pt: PhantomData<T>,
        pu: PhantomData<U>,
    }
}

impl<T, S, F, U> Signal for AndThenSome<S, T, F, U>
where
    S: Signal<Item = Option<T>>,
    F: FnMut(T) -> Option<U>,
{
    type Item = Option<U>;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.signal
            .poll_change(cx)
            .map(|opt| opt.map(|opt| opt.and_then(this.mapper)))
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "Signals do nothing unless polled"]
    pub struct UnwrapOrDefault<S, T> {
        #[pin]
        signal: S,
        pt: PhantomData<T>,
    }
}

impl<T, S> Signal for UnwrapOrDefault<S, T>
where
    S: Signal<Item = Option<T>>,
    T: Default,
{
    type Item = T;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.project()
            .signal
            .poll_change(cx)
            .map(|opt| opt.map(|opt| opt.unwrap_or_default()))
    }
}

#[cfg(test)]
mod test {
    use futures_signals::signal_vec::MutableVec;

    use crate::MutableVecExt;

    #[test]
    fn replace_keyed() {
        let vec = MutableVec::new_with_values(vec![("a", 1), ("b", 2), ("c", 3)]);
        assert_eq!(vec.replace_keyed(|(k, _)| *k, [("b", 20), ("d", 4)]), true);
        assert_eq!(
            vec.lock_ref().as_slice(),
            &[("a", 1), ("b", 20), ("c", 3), ("d", 4)]
        );
    }

    #[test]
    fn replace_keyed_cloned() {
        let vec = MutableVec::new_with_values(vec![("a", 1), ("b", 2), ("c", 3)]);
        assert_eq!(
            vec.replace_keyed_cloned(|(k, _)| *k, [("b", 20), ("d", 4)]),
            true
        );
        assert_eq!(
            vec.lock_ref().as_slice(),
            &[("a", 1), ("b", 20), ("c", 3), ("d", 4)]
        );
    }
}
