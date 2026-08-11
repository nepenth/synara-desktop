//! Typed `matrix_*` command registry (P2).
//!
//! P2 command-group slices register one async handler per existing desktop
//! command name. The desktop invoke list remains byte-compatible until P3;
//! the registry is the single core-side source of handler dispatch.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::{is_known_matrix_command, is_valid_command_name, CommandEnvelope, MatrixIpcError};
use crate::core::CoreState;

/// Owned future returned by a command handler.
pub type CommandFuture =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, MatrixIpcError>> + Send>>;

/// Async command-body seam. It receives internal core state (never a Tauri
/// type) and an already-validated request envelope.
pub trait CommandHandler: Send + Sync {
    fn handle(&self, state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture;
}

impl<F> CommandHandler for F
where
    F: Fn(Arc<CoreState>, CommandEnvelope) -> CommandFuture + Send + Sync,
{
    fn handle(&self, state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
        self(state, request)
    }
}

/// Registry construction errors (static / privacy-safe).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRegistryError {
    InvalidCommandName,
    UnknownCommand,
    DuplicateCommand,
}

/// Command-name → async handler table.
#[derive(Default)]
pub struct CommandRegistry {
    handlers: HashMap<String, Arc<dyn CommandHandler>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register exactly one handler for a census-listed `matrix_*` command name.
    pub fn register<H>(
        &mut self,
        command: impl Into<String>,
        handler: H,
    ) -> Result<(), CommandRegistryError>
    where
        H: CommandHandler + 'static,
    {
        let command = command.into();
        if !is_valid_command_name(&command) {
            return Err(CommandRegistryError::InvalidCommandName);
        }
        if !is_known_matrix_command(&command) {
            return Err(CommandRegistryError::UnknownCommand);
        }
        if self.handlers.contains_key(&command) {
            return Err(CommandRegistryError::DuplicateCommand);
        }
        self.handlers.insert(command, Arc::new(handler));
        Ok(())
    }

    pub fn handler(&self, command: &str) -> Option<Arc<dyn CommandHandler>> {
        self.handlers.get(command).cloned()
    }

    pub fn contains(&self, command: &str) -> bool {
        self.handlers.contains_key(command)
    }

    /// Stable lexical list used by P2 parity coverage tests.
    pub fn command_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.handlers.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_handler(
        _state: std::sync::Arc<crate::core::CoreState>,
        _request: CommandEnvelope,
    ) -> CommandFuture {
        Box::pin(async { Ok(serde_json::Value::Null) })
    }

    #[test]
    fn registration_accepts_census_names_and_rejects_unknown_names() {
        let mut registry = CommandRegistry::new();
        assert_eq!(
            registry.register("matrix_login_flows", test_handler),
            Ok(())
        );
        assert_eq!(
            registry.register("matrix_p2_not_in_desktop_invoke_list", test_handler),
            Err(CommandRegistryError::UnknownCommand)
        );
        assert_eq!(
            registry.register("desktop_show", test_handler),
            Err(CommandRegistryError::InvalidCommandName)
        );
        assert_eq!(registry.command_names(), vec!["matrix_login_flows"]);
    }
}
