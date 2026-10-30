//! Shell completions generation command.
//!
//! Restricts completion output strictly to `bash` and `zsh` per project standards.

use clap::{Args, Command, ValueEnum};
use clap_complete::{generate, Shell};
use std::io;

/// Supported shell targets for completions (defaulting to bash and zsh).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ShellChoice {
    /// Bourne Again SHell (bash).
    Bash,
    /// Z Shell (zsh).
    Zsh,
}

impl From<ShellChoice> for Shell {
    fn from(choice: ShellChoice) -> Self {
        match choice {
            ShellChoice::Bash => Shell::Bash,
            ShellChoice::Zsh => Shell::Zsh,
        }
    }
}

/// Arguments for the `completions` subcommand.
#[derive(Debug, Clone, Args)]
pub struct CompletionsArgs {
    /// Target shell to generate completions for.
    #[arg(value_enum)]
    pub shell: ShellChoice,
}

/// Generates shell completion script to standard output.
pub fn run_completions(args: CompletionsArgs, cmd: &mut Command) {
    let shell: Shell = args.shell.into();
    generate(shell, cmd, "jira", &mut io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_choice_conversion() {
        let bash: Shell = ShellChoice::Bash.into();
        assert_eq!(bash, Shell::Bash);
        let zsh: Shell = ShellChoice::Zsh.into();
        assert_eq!(zsh, Shell::Zsh);
    }
}
