#![forbid(unsafe_code)]
//! Tool registry and built-in tools.
//!
//! A tool declares a JSON schema, executes against an explicit [`ToolCtx`], and
//! returns model-visible text. Registry execution validates arguments and
//! spills oversized output to a file instead of flooding the context.

pub mod registry;
pub mod schema;
mod tools;

#[cfg(test)]
mod test_support;

pub use registry::{Tool, ToolCtx, ToolError, ToolOutcome, ToolRegistry, resolve_path};
pub use tools::{
    BashTool, EditTool, GlobTool, GrepTool, PresentTool, ReadTool, TodoTool, WebFetchTool,
    WebSearchTool, WriteTool,
};
