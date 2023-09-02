#![feature(return_position_impl_trait_in_trait)]

#[cfg(any(feature = "spawn", feature = "spawn-local"))]
mod futures_signals_spawn;
#[cfg(any(feature = "spawn", feature = "spawn-local"))]
pub use futures_signals_spawn::*;

mod futures_signals_ext;
pub use futures_signals_ext::*;

#[cfg(feature = "option")]
mod option;
#[cfg(feature = "option")]
pub use option::*;

#[cfg(all(target_arch = "wasm32", feature = "spawn"))]
compile_error!("'spawn' feature is not available for 'wasm32'");
