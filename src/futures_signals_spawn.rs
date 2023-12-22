use futures_signals::signal_vec::VecDiff;

pub trait SignalSpawn<A> {
    #[cfg(feature = "spawn")]
    fn spawn<F>(self, f: F)
    where
        Self: Send,
        F: Fn(A) + Send + 'static;

    #[cfg(feature = "spawn-local")]
    fn spawn_local<F>(self, f: F)
    where
        F: Fn(A) + 'static;
}

pub trait SignalVecSpawn<A> {
    #[cfg(feature = "spawn")]
    fn spawn<F>(self, f: F)
    where
        Self: Send,
        F: Fn(VecDiff<A>) + Send + 'static;

    #[cfg(feature = "spawn-local")]
    fn spawn_local<F>(self, f: F)
    where
        F: Fn(VecDiff<A>) + 'static;
}

#[cfg(not(target_os = "unknown"))]
mod os {
    use std::future::ready;

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
            async_global_executor::init();
            async_global_executor::spawn(self.for_each(move |new| {
                f(new);
                ready(())
            }))
            .detach();
        }

        #[cfg(feature = "spawn-local")]
        fn spawn_local<F>(self, f: F)
        where
            F: Fn(A) + 'static,
        {
            async_global_executor::init();
            async_global_executor::spawn_local(self.for_each(move |new| {
                f(new);
                ready(())
            }))
            .detach();
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
            async_global_executor::init();
            async_global_executor::spawn(self.for_each(move |new| {
                f(new);
                ready(())
            }))
            .detach();
        }

        #[cfg(feature = "spawn-local")]
        fn spawn_local<F>(self, f: F)
        where
            F: Fn(VecDiff<A>) + 'static,
        {
            async_global_executor::init();
            async_global_executor::spawn_local(self.for_each(move |new| {
                f(new);
                ready(())
            }))
            .detach();
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "spawn-local"))]
mod wasm {
    use std::future::ready;

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
            unimplemented!()
        }

        #[cfg(feature = "spawn-local")]
        fn spawn_local<F>(self, f: F)
        where
            F: Fn(A) + 'static,
        {
            wasm_bindgen_futures::spawn_local(self.for_each(move |new| {
                f(new);
                ready(())
            }));
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
            unimplemented!()
        }

        #[cfg(feature = "spawn-local")]
        fn spawn_local<F>(self, f: F)
        where
            F: Fn(VecDiff<A>) + 'static,
        {
            wasm_bindgen_futures::spawn_local(self.for_each(move |new| {
                f(new);
                ready(())
            }));
        }
    }
}
