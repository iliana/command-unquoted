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
//! implementation), quotes are added only where necessary.
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

use std::ffi::OsStr;
use std::fmt::{self, Debug, Display};
use std::process::Command;

const RESERVED_COMMAND_WORDS: &[&str] = &[
    "case", "do", "done", "elif", "else", "esac", "fi", // POSIX-1.2018
    "for", "function", "if", "in", "select", "then", // POSIX-1.2018
    "time", // Bash
    "until", "while", // POSIX-1.2018
];

/// A wrapper for [`std::process::Command`] with a nicer-looking [`Debug`]
/// implementation.
///
/// See [the crate-level documentation][crate] for more details.
pub struct Unquoted<'a>(pub &'a Command);

impl Debug for Unquoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`")?;
        for (name, value_opt) in self.0.get_envs() {
            if let Some(value) = value_opt {
                write!(f, "{}={} ", Quoted(name), Quoted(value))?;
            }
        }

        let program = self.0.get_program();
        if let Some(s) = program
            .to_str()
            .filter(|s| RESERVED_COMMAND_WORDS.binary_search(s).is_ok())
        {
            write!(f, "'{}'", s)?;
        } else {
            write!(f, "{}", Quoted(program))?;
        }

        for arg in self.0.get_args() {
            write!(f, " {}", Quoted(arg))?;
        }
        write!(f, "`")
    }
}

struct Quoted<'a>(&'a OsStr);

impl Display for Quoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "''");
        }

        let s = self.0.to_string_lossy();
        let has_single_quote = s.contains('\'');
        let has_special_within_double = s.contains(
            [
                '$', '`', '\\', '"', // POSIX-1.2018
                '@', // Special within Bash double quotes per docs (unsure why); also extglob
                '!', // Bash history expansion
            ]
            .as_slice(),
        );
        let has_special = has_single_quote
            || has_special_within_double
            || s.contains(
                [
                    '|', '&', ';', '<', '>', '(', ')', ' ', '\t', '\n', // POSIX-1.2018
                    '*', '?', '[', '#', '~', '%', // POSIX-1.2018
                    // Technically '=' is in the above list of "may need
                    // to be quoted under certain circumstances" but those
                    // circumstances are generally variable assignments or are
                    // otherwise covered by other characters here.
                    ']', // Bash glob patterns
                    '{', '}', // Bash brace expansion
                ]
                .as_slice(),
            );

        if has_single_quote && !has_special_within_double {
            // Use double quotes
            write!(f, r#""{}""#, s)
        } else if has_special {
            // Use single quotes
            if has_single_quote {
                write!(f, "'")?;
                for c in s.chars() {
                    if c == '\'' {
                        write!(f, "'\\''")?;
                    } else {
                        write!(f, "{}", c)?;
                    }
                }
                write!(f, "'")
            } else {
                write!(f, "'{}'", s)
            }
        } else {
            // Use no quotes
            write!(f, "{}", s)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, process::Command};

    use crate::{Quoted, Unquoted, RESERVED_COMMAND_WORDS};

    #[test]
    fn command_words_sorted() {
        assert!(RESERVED_COMMAND_WORDS.windows(2).all(|s| s[0] < s[1]));
    }

    macro_rules! assert_q {
        ($left:expr, $right:expr) => {
            assert_eq!(Quoted(OsStr::new($left)).to_string(), $right)
        };
    }

    #[test]
    fn quoted() {
        assert_q!("", r"''");

        // special characters, generally: use single quotes
        assert_q!("meow", r"meow");
        assert_q!("|meow", "'|meow'");
        assert_q!("meow&", "'meow&'");
        assert_q!("meow;", "'meow;'");
        assert_q!("<meow", "'<meow'");
        assert_q!(">meow", "'>meow'");
        assert_q!("(meow", "'(meow'");
        assert_q!("meow)", "'meow)'");
        assert_q!("$meow", "'$meow'");
        assert_q!("`meow`", "'`meow`'");
        assert_q!(r"\meow", r"'\meow'");
        assert_q!(r#""meow""#, r#"'"meow"'"#);
        assert_q!("meow meow", "'meow meow'");
        assert_q!("meow\tmeow", "'meow\tmeow'");
        assert_q!("meow\nmeow", "'meow\nmeow'");
        assert_q!("meow*", "'meow*'");
        assert_q!("meow?", "'meow?'");
        assert_q!("[meow", "'[meow'");
        assert_q!("meow]", "'meow]'");
        assert_q!("{meow", "'{meow'");
        assert_q!("meow}", "'meow}'");
        assert_q!("#meow", "'#meow'");
        assert_q!("~meow", "'~meow'");
        assert_q!("%meow", "'%meow'");
        assert_q!("@meow", "'@meow'");
        assert_q!("!meow", "'!meow'");

        // single-quote with no other special characters: use double quotes
        assert_q!("meow's", r#""meow's""#);
        // single-quote with special characters that don't have special meaning
        // inside double quotes: use double quotes
        assert_q!("|meow's", r#""|meow's""#);
        assert_q!("meow's&", r#""meow's&""#);
        assert_q!("meow's;", r#""meow's;""#);
        assert_q!("<meow's", r#""<meow's""#);
        assert_q!(">meow's", r#"">meow's""#);
        assert_q!("(meow's", r#""(meow's""#);
        assert_q!("meow's)", r#""meow's)""#);
        assert_q!("meow's meow", r#""meow's meow""#);
        assert_q!("meow's\tmeow", "\"meow's\tmeow\"");
        assert_q!("meow's\nmeow", "\"meow's\nmeow\"");
        assert_q!("meow's*", r#""meow's*""#);
        assert_q!("meow's?", r#""meow's?""#);
        assert_q!("[meow's", r#""[meow's""#);
        assert_q!("meow's]", r#""meow's]""#);
        assert_q!("{meow's", r#""{meow's""#);
        assert_q!("meow's}", r#""meow's}""#);
        assert_q!("#meow's", r##""#meow's""##);
        assert_q!("~meow's", r#""~meow's""#);
        assert_q!("%meow's", r#""%meow's""#);
        // single-quote with special characters that _do_ have special meaning
        // inside double quotes: use single quotes
        assert_q!("$meow's", r"'$meow'\''s'");
        assert_q!("`meow's`", r"'`meow'\''s`'");
        assert_q!(r"\meow's", r"'\meow'\''s'");
        assert_q!(r#""meow's""#, r#"'"meow'\''s"'"#);
        assert_q!("@meow's", r"'@meow'\''s'");
        assert_q!("!meow's", r"'!meow'\''s'");
    }

    macro_rules! assert_u {
        ($value:expr, $display:expr) => {
            assert_eq!(format!("{:?}", Unquoted(&$value)), $display)
        };
    }

    #[test]
    fn program_only() {
        assert_u!(Command::new("program"), "`program`");
        assert_u!(Command::new("programn't"), r#"`"programn't"`"#);
        assert_u!(Command::new("case"), "`'case'`");
    }

    #[test]
    fn args() {
        assert_u!(
            Command::new("program").args(["arg1", "arg b", "arg'c", r#"arg"d"#, "arg$e"]),
            r#"`program arg1 'arg b' "arg'c" 'arg"d' 'arg$e'`"#
        );
    }

    #[test]
    fn env() {
        assert_u!(
            Command::new("program")
                .env("BLAH1", "blah")
                .env("BLAH2", "\"blah's blah\"")
                .env("BLAH3", r#"\"blah's blah\""#),
            r#"`BLAH1=blah BLAH2='"blah'\''s blah"' BLAH3='\"blah'\''s blah\"' program`"#
        );
    }
}
