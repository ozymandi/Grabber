//! Launch arguments (design D8; spec texture-export "ShareX launch contract").
//!
//! `ripper.exe [source] [output]`: a second argument switches on ShareX mode, with that
//! path as the file ShareX continues with (`$output`).

use serde::Serialize;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Standalone,
    Sharex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub source: Option<PathBuf>,
    pub output: Option<PathBuf>,
}

impl Launch {
    pub fn mode(&self) -> Mode {
        if self.output.is_some() {
            Mode::Sharex
        } else {
            Mode::Standalone
        }
    }
}

/// Parses the arguments after the program name. Empty arguments count as absent.
pub fn parse<I: IntoIterator<Item = OsString>>(args: I) -> Launch {
    let mut it = args.into_iter().filter(|a| !a.is_empty()).map(PathBuf::from);
    let source = it.next();
    let output = if source.is_some() { it.next() } else { None };
    Launch { source, output }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<OsString> {
        v.iter().map(OsString::from).collect()
    }

    #[test]
    fn no_args_is_standalone() {
        let l = parse(args(&[]));
        assert_eq!(l, Launch { source: None, output: None });
        assert_eq!(l.mode(), Mode::Standalone);
    }

    #[test]
    fn one_arg_is_standalone_with_source() {
        let l = parse(args(&[r"C:\shots\a.png"]));
        assert_eq!(l.source, Some(PathBuf::from(r"C:\shots\a.png")));
        assert_eq!(l.mode(), Mode::Standalone);
    }

    #[test]
    fn two_args_is_sharex() {
        let l = parse(args(&[r"C:\shots\a.png", r"C:\shots\a_out.png"]));
        assert_eq!(l.output, Some(PathBuf::from(r"C:\shots\a_out.png")));
        assert_eq!(l.mode(), Mode::Sharex);
    }

    #[test]
    fn empty_args_are_ignored() {
        let l = parse(args(&["", r"C:\shots\a.png"]));
        assert_eq!(l.source, Some(PathBuf::from(r"C:\shots\a.png")));
        assert_eq!(l.mode(), Mode::Standalone);
    }
}
