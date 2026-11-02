//! Render Markdown as readable terminal text.
//!
//! ```
//! use mdcat::{render, RenderOptions};
//!
//! assert_eq!(render("**Hello**, terminal!", &RenderOptions::default()), "Hello, terminal!\n");
//! ```

mod render;

pub use render::{render, RenderOptions};
