// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2024 Oxide Computer Company

//! command-unquoted provides [a wrapper struct][Unquoted] for
//! [`std::process::Command`] that provides a nicer-looking [`Debug`]
//! implementation and is useful for user-facing error messages.
//!
//! Instead of quoting all strings (as done in the Unix `Command`
//! implementation), the [shell-words][shell_words] crate is used to add quotes
//! only where necessary.
//!
//! As with `Command`'s `Debug` implementation, this format only approximates an
//! appropriate shell invocation of the program with the provided environment.
//! It may be particularly unsuitable for Windows (patches welcome). Non-UTF-8
//! data is lossily converted using the UTF-8 replacement character. This format
//! **is not stable** and may change between releases; only the API of this
//! crate is stable.
//!
//! To keep the resulting output friendlier (and sometimes due to Rust standard
//! library limitations), the result of these methods are not displayed in this
//! implementation:
//! - [`Command::current_dir`]
//! - [`Command::env_clear`] and [`Command::env_remove`]
//! - [`Command::stdin`], [`Command::stdout`], and [`Command::stderr`]
//! - all methods of all `CommandExt` traits

#![warn(clippy::pedantic)]

use std::fmt::{self, Debug};
use std::process::Command;

/// A wrapper for [`std::process::Command`] with a nicer-looking [`Debug`]
/// implementation.
///
/// See [the crate-level documentation][crate] for more details.
pub struct Unquoted<'a>(pub &'a Command);

macro_rules! quote {
    ($s:expr) => {
        shell_words::quote(&$s.to_string_lossy())
    };
}

impl Debug for Unquoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, value_opt) in self.0.get_envs() {
            if let Some(value) = value_opt {
                write!(f, "{}={} ", quote!(name), quote!(value))?;
            }
        }
        write!(f, "{}", quote!(self.0.get_program()))?;
        for arg in self.0.get_args() {
            write!(f, " {}", quote!(arg))?;
        }
        Ok(())
    }
}
