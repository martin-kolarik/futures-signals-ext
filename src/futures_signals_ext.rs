use futures_signals::{
    signal::{Mutable, Signal},
    signal_vec::{
        Filter, FilterSignalCloned, MutableSignalVec, MutableVec, MutableVecLockMut,
        MutableVecLockRef, SignalVec, SignalVecExt,
    },
};
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{marker::PhantomData, mem};

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
    fn inspect(&self, f: impl FnOnce(&MutableVecLockRef<A>));
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

    fn filter_map<F, U>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&A) -> Option<U>;

    fn find_map<F, U>(&self, f: F) -> Option<U>
    where
        F: FnMut(&A) -> Option<U>;

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
    fn inspect(&self, f: impl FnOnce(&MutableVecLockRef<A>)) {
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
        let mut lock = self.lock_mut();
        if let Some((index, mut item)) = lock
            .iter()
            .position(predicate)
            .and_then(|index| lock.get(index).map(|item| (index, *item)))
        {
            if f(&mut item) {
                lock.set(index, item);
                Some(true)
            } else {
                Some(false)
            }
        } else {
            None
        }
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
        let mut lock = self.lock_mut();
        if let Some((index, item)) = lock
            .iter()
            .position(predicate)
            .and_then(|index| lock.get(index).map(|item| (index, item)))
        {
            let mut item = item.clone();
            if f(&mut item) {
                lock.set_cloned(index, item);
                Some(true)
            } else {
                Some(false)
            }
        } else {
            None
        }
    }

    fn map<F, U>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&A) -> U,
    {
        self.lock_ref().iter().map(f).collect()
    }

    fn filter_map<F, U>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&A) -> Option<U>,
    {
        self.lock_ref().iter().filter_map(f).collect()
    }

    fn find_map<F, U>(&self, f: F) -> Option<U>
    where
        F: FnMut(&A) -> Option<U>,
    {
        self.lock_ref().iter().find_map(f)
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

pub trait SignalVecFirstExt<A> {
    fn first(self) -> impl Signal<Item = Option<A>>
    where
        A: Copy,
        Self: Sized,
    {
        self.first_map(|i| *i)
    }

    fn first_cloned(self) -> impl Signal<Item = Option<A>>
    where
        A: Clone,
        Self: Sized,
    {
        self.first_map(|i| i.clone())
    }

    fn first_map<F, U>(self, f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(&A) -> U;
}

impl<A, S> SignalVecFirstExt<A> for S
where
    S: SignalVec<Item = A>,
{
    fn first_map<F, U>(self, mut f: F) -> impl Signal<Item = Option<U>>
    where
        F: FnMut(&A) -> U,
    {
        self.to_signal_map(move |items| items.first().map(&mut f))
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

pub trait SignalExtMapOption<T, U> {
    fn map_some<F>(self, f: F) -> MapSome<Self, T, U, F>
    where
        Self: Sized,
        F: FnMut(&T) -> U;

    fn map_some_default<F>(self, f: F) -> MapSomeDefault<Self, T, U, F>
    where
        Self: Sized,
        F: FnMut(&T) -> U,
        U: Default;

    fn and_then_some<F>(self, f: F) -> AndThenSome<Self, T, U, F>
    where
        Self: Sized,
        F: FnMut(&T) -> Option<U>;
}

impl<T, U, S> SignalExtMapOption<T, U> for S
where
    S: Signal<Item = Option<T>>,
{
    fn map_some<F>(self, f: F) -> MapSome<Self, T, U, F>
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

    fn map_some_default<F>(self, f: F) -> MapSomeDefault<Self, T, U, F>
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

    fn and_then_some<F>(self, f: F) -> AndThenSome<Self, T, U, F>
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
}

pin_project! {
#[derive(Debug)]
    #[must_use = "Signals do nothing unless polled"]
    pub struct MapSome<S, T, U, F> {
        #[pin]
        signal: S,
        mapper: F,
        pt: PhantomData<T>,
        pu: PhantomData<U>,
    }
}

impl<T, U, S, F> Signal for MapSome<S, T, U, F>
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
    pub struct MapSomeDefault<S, T, U, F> {
        #[pin]
        signal: S,
        mapper: F,
        pt: PhantomData<T>,
        pu: PhantomData<U>,
    }
}

impl<T, U, S, F> Signal for MapSomeDefault<S, T, U, F>
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
    pub struct AndThenSome<S, T, U, F> {
        #[pin]
        signal: S,
        mapper: F,
        pt: PhantomData<T>,
        pu: PhantomData<U>,
    }
}

impl<T, U, S, F> Signal for AndThenSome<S, T, U, F>
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
