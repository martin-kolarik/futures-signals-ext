use async_std::future::ready;
#[cfg(feature = "spawn")]
use async_std::task::spawn;
#[cfg(feature = "spawn_local")]
use async_std::task::spawn_local;
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt, VecDiff},
};

pub trait SignalSpawn<A> {
    #[cfg(feature = "spawn")]
    fn spawn<F>(self, f: F)
    where
        Self: Send,
        F: Fn(A) + Send + 'static;

    #[cfg(feature = "spawn_local")]
    fn spawn_local<F>(self, f: F)
    where
        F: Fn(A) + 'static;
}

impl<A, S> SignalSpawn<A> for S
where
    S: Signal<Item = A> + 'static,
{
    #[cfg(feature = "spawn")]
    fn spawn<F>(self, f: F)
    where
        Self: Send,
        F: Fn(A) + Send + 'static,
    {
        spawn(self.for_each(move |new| {
            f(new);
            ready(())
        }));
    }

    #[cfg(feature = "spawn_local")]
    fn spawn_local<F>(self, f: F)
    where
        F: Fn(A) + 'static,
    {
        spawn_local(self.for_each(move |new| {
            f(new);
            ready(())
        }));
    }
}

pub trait SignalVecSpawn<A> {
    #[cfg(feature = "spawn")]
    fn spawn<F>(self, f: F)
    where
        Self: Send,
        F: Fn(VecDiff<A>) + Send + 'static;

    #[cfg(feature = "spawn_local")]
    fn spawn_local<F>(self, f: F)
    where
        F: Fn(VecDiff<A>) + 'static;
}

impl<A, S> SignalVecSpawn<A> for S
where
    S: SignalVec<Item = A> + 'static,
{
    #[cfg(feature = "spawn")]
    fn spawn<F>(self, f: F)
    where
        Self: Send,
        F: Fn(VecDiff<A>) + Send + 'static,
    {
        spawn(self.for_each(move |new| {
            f(new);
            ready(())
        }));
    }

    #[cfg(feature = "spawn_local")]
    fn spawn_local<F>(self, f: F)
    where
        F: Fn(VecDiff<A>) + 'static,
    {
        spawn_local(self.for_each(move |new| {
            f(new);
            ready(())
        }));
    }
}
