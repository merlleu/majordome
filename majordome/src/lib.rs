mod app;
mod error;
mod module;
mod signal;
mod compat;

pub use app::*;
pub use error::*;
pub use module::*;
pub use compat::*;

pub mod macros {
    pub use majordome_derive::*;
}