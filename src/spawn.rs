use std::future::Future;

use futures_signals::signal_vec::{MutableVec, MutableVecLockMut, VecDiff};

pub trait SignalSpawn<A> {
    #[cfg(feature = "spawn")]
    fn spawn<F>(self, f: F)
    where
        Self: Send,
        F: Fn(A) + Send + 'static;

    #[cfg(feature = "spawn")]
    fn spawn_fut<F, W>(self, f: F)
    where
        Self: Send,
        F: Fn(A) -> W + Send + 'static,
        W: Future<Output = ()> + Send + 'static;

    #[cfg(feature = "spawn-local")]
    fn spawn_local<F>(self, f: F)
    where
        F: Fn(A) + 'static;

    #[cfg(feature = "spawn-local")]
    fn spawn_local_fut<F, W>(self, f: F)
    where
        F: Fn(A) -> W + 'static,
        W: Future<Output = ()> + 'static;
}

pub trait SignalVecSpawn<A> {
    #[cfg(feature = "spawn")]
    fn spawn<F>(self, f: F)
    where
        Self: Send,
        F: Fn(VecDiff<A>) + Send + 'static;

    #[cfg(feature = "spawn")]
    fn feed(self, target: MutableVec<A>)
    where
        Self: Sized + Send,
        A: Copy + Send + Sync + 'static,
    {
        self.spawn(move |diff| {
            MutableVecLockMut::apply_vec_diff(&mut target.lock_mut(), diff);
        });
    }

    #[cfg(feature = "spawn")]
    fn feed_cloned(self, target: MutableVec<A>)
    where
        Self: Sized + Send,
        A: Clone + Send + Sync + 'static,
    {
        self.spawn(move |diff| {
            MutableVecLockMut::apply_vec_diff(&mut target.lock_mut(), diff);
        });
    }

    #[cfg(feature = "spawn-local")]
    fn spawn_local<F>(self, f: F)
    where
        F: Fn(VecDiff<A>) + 'static;

    #[cfg(feature = "spawn-local")]
    fn feed_local(self, target: MutableVec<A>)
    where
        Self: Sized,
        A: Copy + 'static,
    {
        self.spawn_local(move |diff| {
            MutableVecLockMut::apply_vec_diff(&mut target.lock_mut(), diff);
        });
    }

    #[cfg(feature = "spawn-local")]
    fn feed_local_cloned(self, target: MutableVec<A>)
    where
        Self: Sized,
        A: Clone + 'static,
    {
        self.spawn_local(move |diff| {
            MutableVecLockMut::apply_vec_diff(&mut target.lock_mut(), diff);
        });
    }
}

#[cfg(not(target_os = "unknown"))]
mod os {
    use std::future::{Future, ready};

    use futures_signals::{
        signal::{Signal, SignalExt},
        signal_vec::{SignalVec, SignalVecExt, VecDiff},
    };

    use crate::{SignalSpawn, SignalVecSpawn};

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
            self.spawn_fut(move |new| {
                f(new);
                ready(())
            });
        }

        #[cfg(feature = "spawn")]
        fn spawn_fut<F, W>(self, f: F)
        where
            Self: Send,
            F: Fn(A) -> W + Send + 'static,
            W: Future<Output = ()> + Send + 'static,
        {
            artwrap::spawn(self.for_each(move |new| f(new)));
        }

        #[cfg(feature = "spawn-local")]
        fn spawn_local<F>(self, f: F)
        where
            F: Fn(A) + 'static,
        {
            self.spawn_local_fut(move |new| {
                f(new);
                ready(())
            });
        }

        #[cfg(feature = "spawn-local")]
        fn spawn_local_fut<F, W>(self, f: F)
        where
            F: Fn(A) -> W + 'static,
            W: Future<Output = ()> + 'static,
        {
            artwrap::spawn_local(self.for_each(f));
        }
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
            artwrap::spawn(self.for_each(move |new| {
                f(new);
                ready(())
            }));
        }

        #[cfg(feature = "spawn-local")]
        fn spawn_local<F>(self, f: F)
        where
            F: Fn(VecDiff<A>) + 'static,
        {
            artwrap::spawn_local(self.for_each(move |new| {
                f(new);
                ready(())
            }));
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "spawn-local"))]
mod wasm {
    use std::future::{Future, ready};

    use futures_signals::{
        signal::{Signal, SignalExt},
        signal_vec::{SignalVec, SignalVecExt, VecDiff},
    };

    use crate::{SignalSpawn, SignalVecSpawn};

    impl<A, S> SignalSpawn<A> for S
    where
        S: Signal<Item = A> + 'static,
    {
        #[cfg(feature = "spawn")]
        fn spawn<F>(self, _: F)
        where
            Self: Send,
            F: Fn(A) + Send + 'static,
        {
            const { unimplemented!() }
        }

        #[cfg(feature = "spawn")]
        fn spawn_fut<F, W>(self, _: F)
        where
            Self: Send,
            F: Fn(A) -> W + Send + 'static,
            W: Future<Output = ()> + Send + 'static,
        {
            const { unimplemented!() }
        }

        #[cfg(feature = "spawn-local")]
        fn spawn_local<F>(self, f: F)
        where
            F: Fn(A) + 'static,
        {
            self.spawn_local_fut(move |new| {
                f(new);
                ready(())
            });
        }

        #[cfg(feature = "spawn-local")]
        fn spawn_local_fut<F, W>(self, f: F)
        where
            F: Fn(A) -> W + 'static,
            W: Future<Output = ()> + 'static,
        {
            artwrap::spawn_local(self.for_each(move |new| f(new)));
        }
    }

    impl<A, S> SignalVecSpawn<A> for S
    where
        S: SignalVec<Item = A> + 'static,
    {
        #[cfg(feature = "spawn")]
        fn spawn<F>(self, _: F)
        where
            Self: Send,
            F: Fn(VecDiff<A>) + Send + 'static,
        {
            const { unimplemented!() }
        }

        #[cfg(feature = "spawn-local")]
        fn spawn_local<F>(self, f: F)
        where
            F: Fn(VecDiff<A>) + 'static,
        {
            artwrap::spawn_local(self.for_each(move |new| {
                f(new);
                ready(())
            }));
        }
    }
}
