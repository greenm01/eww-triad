//! Rust client for Triad native IPC.
//!
//! Most Rust users should start with [`Client`]:
//!
//! ```no_run
//! use eww_triad::{Client, QueryRequest};
//!
//! # fn main() -> eww_triad::Result<()> {
//! let client = Client::connect_default()?;
//! let state = client.eww_state_once()?;
//! let capabilities = client.query(QueryRequest::Capabilities)?;
//! # let _ = (state, capabilities);
//! # Ok(())
//! # }
//! ```
//!
//! The client talks to Triad's Unix socket directly. The `eww-triad` binary
//! wraps the same client for shell and Eww use.

#[cfg(feature = "cli")]
#[doc(hidden)]
pub mod cli;
pub mod client;
pub mod error;
#[doc(hidden)]
pub mod ipc;
#[doc(hidden)]
pub mod protocol;
#[doc(hidden)]
pub mod state;
#[doc(hidden)]
pub mod view;

#[cfg(feature = "tokio")]
pub use client::AsyncClient;
pub use client::{BindingKind, Client, EventFilter, LayoutTarget, QueryRequest};
pub use error::{Error, Result};
pub use view::{EwwState, OutputFormat};
