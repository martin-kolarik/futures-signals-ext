#[cfg(any(feature = "spawn", feature = "spawn-local"))]
mod spawn;
#[cfg(any(feature = "spawn", feature = "spawn-local"))]
pub use spawn::*;

mod entry;
pub use entry::*;

mod ext;
pub use ext::*;

mod flatten;
pub use flatten::*;

#[cfg(feature = "option")]
mod option;
#[cfg(feature = "option")]
pub use option::*;

#[cfg(all(target_arch = "wasm32", feature = "spawn"))]
compile_error!("'spawn' feature is not available for 'wasm32'");
