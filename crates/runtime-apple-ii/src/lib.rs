//! Original Apple II control through the shared Emu198x script and MCP shell.
mod debug;
mod input;
mod media;
pub mod profiles;
mod queries;
mod runtime;
pub use queries::AppleIIQueries;
pub use runtime::AppleIIRuntime;

#[cfg(test)]
mod tests;
