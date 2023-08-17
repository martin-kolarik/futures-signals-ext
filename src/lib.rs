#![feature(lazy_cell)]
#![feature(return_position_impl_trait_in_trait)]

mod futures_signals_spawn;
pub use futures_signals_spawn::{SignalSpawn, SignalVecSpawn};

mod futures_signals_ext;
pub use futures_signals_ext::{MutableExt, MutableVecExt, SignalExtMapBool, SignalExtMapOption};

#[cfg(feature = "option")]
mod option;
#[cfg(feature = "option")]
pub use option::*;
