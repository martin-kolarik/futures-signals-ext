use futures_signals::{
    signal::{Mutable, Signal},
    signal_vec::{
        Filter, FilterSignalCloned, MutableSignalVec, MutableVec, MutableVecLockMut, SignalVec,
        SignalVecExt,
    },
};
use pin_project_lite::pin_project;
use std::{
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{MutableVecEntry, SignalVecSpawn};

pub trait MutableExt<A> {
    fn inspect(&self, f: impl FnOnce(&A));
    fn inspect_mut(&self, f: impl FnOnce(&mut A));

    fn map<B>(&self, f: impl FnOnce(&A) -> B) -> B;
    fn map_mut<B>(&self, f: impl FnOnce(&mut A) -> B) -> B;

    fn apply(&self, f: impl FnOnce(A) -> A)
    where
        A: Copy;
    fn apply_cloned(&self, f: impl FnOnce(A) -> A)
    where
        A: Clone;

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
    fn inspect(&self, f: impl FnOnce(&A)) {
        f(&self.lock_ref())
    }

    fn inspect_mut(&self, f: impl FnOnce(&mut A)) {
        f(&mut self.lock_mut())
    }

    fn map<B>(&self, f: impl FnOnce(&A) -> B) -> B {
        f(&self.lock_ref())
    }

    fn map_mut<B>(&self, f: impl FnOnce(&mut A) -> B) -> B {
        f(&mut self.lock_mut())
    }

    fn apply(&self, f: impl FnOnce(A) -> A)
    where
        A: Copy,
    {
        self.set(f(self.get()))
    }

    fn apply_cloned(&self, f: impl FnOnce(A) -> A)
    where
        A: Clone,
    {
        self.set(f(self.get_cloned()))
    }
}

pub trait MutableVecExt<A> {
    fn map_vec<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&[A]) -> U;

    fn map_vec_mut<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&mut MutableVecLockMut<A>) -> U;

    fn inspect(&self, f: impl FnOnce(&[A]));
    fn inspect_mut(&self, f: impl FnOnce(&mut MutableVecLockMut<A>));

    fn find_inspect_mut<P, F>(&self, predicate: P, f: F) -> Option<bool>
    where
        A: Copy,
        P: FnMut(&A) -> bool,
        F: FnOnce(&mut A) -> bool;

    fn find_inspect_mut_cloned<P, F>(&self, predicate: P, f: F) -> Option<bool>
    where
        A: Clone,
        P: FnMut(&A) -> bool,
        F: FnOnce(&mut A) -> bool;

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

    fn replace_or_extend_keyed<F, K>(&self, f: F, source: impl IntoIterator<Item = A>) -> bool
    where
        A: Copy,
        F: FnMut(&A) -> K,
        K: Eq + Hash;

    fn replace_or_extend_keyed_cloned<F, K>(
        &self,
        f: F,
        source: impl IntoIterator<Item = A>,
    ) -> bool
    where
        A: Clone,
        F: FnMut(&A) -> K,
        K: Eq + Hash;

    #[cfg(feature = "spawn")]
    fn feed(&self, source: impl SignalVec<Item = A> + 'static)
    where
        A: Copy + 'static;

    #[cfg(feature = "spawn")]
    fn feed_cloned(&self, source: impl SignalVec<Item = A> + 'static)
    where
        A: Clone + 'static;

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
}

impl<A> MutableVecExt<A> for MutableVec<A> {
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

    fn inspect(&self, f: impl FnOnce(&[A])) {
        f(&self.lock_ref())
    }

    fn inspect_mut(&self, f: impl FnOnce(&mut MutableVecLockMut<A>)) {
        f(&mut self.lock_mut())
    }

    /// Return parameter of F (changed) drives if the value should be written back,
    /// and cause MutableVec change. If F returns false, no change is induced neither
    /// reported.
    fn find_inspect_mut<P, F>(&self, predicate: P, f: F) -> Option<bool>
    where
        A: Copy,
        P: FnMut(&A) -> bool,
        F: FnOnce(&mut A) -> bool,
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
        F: FnOnce(&mut A) -> bool,
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
        self.entry(p).or_insert_entry(item);
    }

    fn find_set_or_add_cloned<P>(&self, p: P, item: A)
    where
        A: Clone,
        P: FnMut(&A) -> bool,
    {
        self.entry_cloned(p).or_insert_entry(item);
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

    fn replace<P>(&self, mut p: P, with: impl IntoIterator<Item = A>)
    where
        A: Copy,
        P: FnMut(&A) -> bool,
    {
        let mut lock = self.lock_mut();
        lock.retain(|item| !p(item));
        for item in with.into_iter() {
            lock.push(item);
        }
    }

    fn replace_cloned<P>(&self, mut p: P, with: impl IntoIterator<Item = A>)
    where
        A: Clone,
        P: FnMut(&A) -> bool,
    {
        let mut lock = self.lock_mut();
        lock.retain(|item| !p(item));
        for item in with.into_iter() {
            lock.push_cloned(item);
        }
    }

    fn replace_or_extend_keyed<F, K>(&self, mut f: F, source: impl IntoIterator<Item = A>) -> bool
    where
        A: Copy,
        F: FnMut(&A) -> K,
        K: Eq + Hash,
    {
        let mut source = source
            .into_iter()
            .map(|item| (f(&item), item))
            .collect::<HashMap<_, _>>();
        let mut lock = self.lock_mut();
        let indexes = lock
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let key = f(item);
                source.get(&key).map(|_| (index, key))
            })
            .collect::<Vec<_>>();
        for (index, item) in indexes
            .into_iter()
            .filter_map(|(index, key)| source.remove(&key).map(|item| (index, item)))
        {
            lock.set(index, item)
        }

        let extended = !source.is_empty();
        for (_, item) in source.into_iter() {
            lock.push(item);
        }

        extended
    }

    fn replace_or_extend_keyed_cloned<F, K>(
        &self,
        mut f: F,
        source: impl IntoIterator<Item = A>,
    ) -> bool
    where
        A: Clone,
        F: FnMut(&A) -> K,
        K: Eq + Hash,
    {
        let mut source = source
            .into_iter()
            .map(|item| (f(&item), item))
            .collect::<HashMap<_, _>>();
        let mut lock = self.lock_mut();
        let indexes = lock
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let key = f(item);
                source.get(&key).map(|_| (index, key))
            })
            .collect::<Vec<_>>();
        for (index, item) in indexes
            .into_iter()
            .filter_map(|(index, key)| source.remove(&key).map(|item| (index, item)))
        {
            lock.set_cloned(index, item)
        }

        let extended = !source.is_empty();
        for (_, item) in source.into_iter() {
            lock.push_cloned(item);
        }

        extended
    }

    #[cfg(feature = "spawn")]
    fn feed(&self, source: impl SignalVec<Item = A> + 'static)
    where
        A: Copy + 'static,
    {
        source.feed(self.clone());
    }

    #[cfg(feature = "spawn")]
    fn feed_cloned(&self, source: impl SignalVec<Item = A> + 'static)
    where
        A: Clone + 'static,
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

    fn signal_vec_filter<P>(&self, p: P) -> Filter<MutableSignalVec<A>, P>
    where
        A: Copy,
        P: FnMut(&A) -> bool,
    {
        self.signal_vec().filter(p)
    }

    fn signal_vec_filter_cloned<P>(&self, p: P) -> Filter<MutableSignalVec<A>, P>
    where
        A: Clone,
        P: FnMut(&A) -> bool,
    {
        self.signal_vec_cloned().filter(p)
    }

    fn signal_vec_filter_signal<P, S>(&self, p: P) -> FilterSignalCloned<MutableSignalVec<A>, S, P>
    where
        A: Copy,
        P: FnMut(&A) -> S,
        S: Signal<Item = bool>,
    {
        self.signal_vec().filter_signal_cloned(p)
    }

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
}

pub trait SignalVecFinalizerExt: SignalVec {
    fn first(self) -> impl Signal<Item = Option<Self::Item>>
    where
        Self::Item: Copy,
        Self: Sized,
    {
        self.first_map(|i| *i)
    }

    fn first_cloned(self) -> impl Signal<Item = Option<Self::Item>>
    where
        Self::Item: Clone,
        Self: Sized,
    {
        self.first_map(|i| i.clone())
    }

    fn first_map<F, U>(self, f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(&Self::Item) -> U;

    fn all<F>(self, f: F) -> impl Signal<Item = bool>
    where
        F: FnMut(&Self::Item) -> bool;

    fn any<F>(self, f: F) -> impl Signal<Item = bool>
    where
        F: FnMut(&Self::Item) -> bool;

    fn seq(self) -> Sequence<Self>
    where
        Self: Sized;
}

impl<S> SignalVecFinalizerExt for S
where
    S: SignalVec,
{
    fn first_map<F, U>(self, mut f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(&Self::Item) -> U,
    {
        self.to_signal_map(move |items| items.first().map(&mut f))
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

    fn seq(self) -> Sequence<Self>
    where
        Self: Sized,
    {
        Sequence {
            signal: self,
            sequence: 0,
        }
    }
}

pub trait SignalExtMapBool {
    fn map_bool<T, TM: FnMut() -> T, FM: FnMut() -> T>(self, t: TM, f: FM) -> MapBool<Self, TM, FM>
    where
        Self: Sized;

    fn map_option<T, TM: FnMut() -> T>(self, t: TM) -> MapOption<Self, TM>
    where
        Self: Sized;
}

impl<S: Signal<Item = bool>> SignalExtMapBool for S {
    fn map_bool<T, TM: FnMut() -> T, FM: FnMut() -> T>(self, t: TM, f: FM) -> MapBool<Self, TM, FM>
    where
        Self: Sized,
    {
        MapBool {
            signal: self,
            true_mapper: t,
            false_mapper: f,
        }
    }

    fn map_option<T, TM: FnMut() -> T>(self, t: TM) -> MapOption<Self, TM>
    where
        Self: Sized,
    {
        MapOption {
            signal: self,
            true_mapper: t,
        }
    }
}

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

pub trait SignalExtMapOption<T> {
    fn map_some<F, U>(self, f: F) -> MapSome<Self, T, F, U>
    where
        Self: Sized,
        F: FnMut(&T) -> U;

    fn map_some_default<F, U>(self, f: F) -> MapSomeDefault<Self, T, F, U>
    where
        Self: Sized,
        F: FnMut(&T) -> U,
        U: Default;

    fn and_then_some<F, U>(self, f: F) -> AndThenSome<Self, T, F, U>
    where
        Self: Sized,
        F: FnMut(&T) -> Option<U>;

    fn unwrap_or_default(self) -> UnwrapOrDefault<Self, T>
    where
        Self: Sized,
        T: Default;
}

impl<T, S> SignalExtMapOption<T> for S
where
    S: Signal<Item = Option<T>>,
{
    fn map_some<F, U>(self, f: F) -> MapSome<Self, T, F, U>
    where
        Self: Sized,
        F: FnMut(&T) -> U,
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
        Self: Sized,
        F: FnMut(&T) -> U,
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
        Self: Sized,
        F: FnMut(&T) -> Option<U>,
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
        Self: Sized,
        T: Default,
    {
        UnwrapOrDefault {
            signal: self,
            pt: PhantomData,
        }
    }
}

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
    F: FnMut(&T) -> U,
{
    type Item = Option<U>;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.signal
            .as_mut()
            .poll_change(cx)
            .map(|opt| opt.map(|opt| opt.map(|value| (this.mapper)(&value))))
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
    F: FnMut(&T) -> U,
    U: Default,
{
    type Item = U;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.signal
            .as_mut()
            .poll_change(cx)
            .map(|opt| opt.map(|opt| opt.map(|value| (this.mapper)(&value)).unwrap_or_default()))
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
    F: FnMut(&T) -> Option<U>,
{
    type Item = Option<U>;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.signal
            .as_mut()
            .poll_change(cx)
            .map(|opt| opt.map(|opt| opt.and_then(|value| (this.mapper)(&value))))
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
        let mut this = self.project();
        this.signal
            .as_mut()
            .poll_change(cx)
            .map(|opt| opt.map(|opt| opt.unwrap_or_default()))
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "Signals do nothing unless polled"]
    pub struct Sequence<A>
    where
        A: SignalVec,
    {
        #[pin]
        signal: A,
        sequence: u64,
    }
}

impl<A> Signal for Sequence<A>
where
    A: SignalVec,
{
    type Item = u64;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        let mut changed = false;

        let done = loop {
            break match this.signal.as_mut().poll_vec_change(cx) {
                Poll::Ready(None) => true,
                Poll::Ready(Some(_)) => {
                    *this.sequence += 1;
                    changed = true;
                    continue;
                }
                Poll::Pending => false,
            };
        };

        if changed {
            Poll::Ready(Some(*this.sequence))
        } else if done {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}
