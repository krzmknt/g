//! Detection of tmux clients that cannot show colored underlines.
//!
//! The selected row's bottom border is a colored underline (SGR 58). tmux
//! only forwards the color when the outer terminal is declared to support
//! underline styles (`usstyle`); otherwise the underline is drawn in the
//! text color. This module produces a hint so the user can fix their
//! tmux configuration.

use std::process::Command;

/// Build the hint from raw environment values. `tmux` is the `$TMUX`
/// variable, `termname` and `termfeatures` come from tmux's
/// `client_termname` / `client_termfeatures` formats.
pub fn underline_color_hint(
    tmux: Option<&str>,
    termname: Option<&str>,
    termfeatures: Option<&str>,
) -> Option<String> {
    if tmux.is_none_or(str::is_empty) {
        return None;
    }
    let supported = termfeatures
        .unwrap_or("")
        .split(',')
        .any(|feature| feature.trim() == "usstyle");
    if supported {
        return None;
    }
    let termname = termname
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("xterm-256color");
    Some(format!(
        "tmux drops underline colors: add `set -as terminal-features \",{}:usstyle\"` to tmux.conf and restart tmux",
        termname
    ))
}

/// Ask the running tmux server about the current client and return a hint
/// when it will not pass underline colors through.
pub fn detect_underline_color_hint() -> Option<String> {
    let tmux = std::env::var("TMUX").ok();
    if tmux.as_deref().is_none_or(str::is_empty) {
        return None;
    }
    let output = Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "#{client_termname}\t#{client_termfeatures}",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text.trim_end().splitn(2, '\t');
    let termname = parts.next();
    let termfeatures = parts.next();
    underline_color_hint(tmux.as_deref(), termname, termfeatures)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_hint_outside_tmux() {
        assert_eq!(
            underline_color_hint(None, Some("xterm-256color"), Some("RGB")),
            None
        );
        assert_eq!(
            underline_color_hint(Some(""), Some("xterm-256color"), Some("RGB")),
            None
        );
    }

    #[test]
    fn no_hint_when_client_supports_underline_style() {
        let hint = underline_color_hint(
            Some("/tmp/tmux-501/default,1,0"),
            Some("xterm-256color"),
            Some("RGB,title,usstyle"),
        );
        assert_eq!(hint, None);
    }

    #[test]
    fn hint_names_the_client_terminal() {
        let hint = underline_color_hint(
            Some("/tmp/tmux-501/default,1,0"),
            Some("xterm-256color"),
            Some("bpaste,ccolour,clipboard,cstyle,focus,RGB,sixel,title"),
        )
        .expect("hint");
        assert!(hint.contains("set -as terminal-features \",xterm-256color:usstyle\""));
    }

    #[test]
    fn hint_falls_back_to_a_generic_terminal_name() {
        let hint =
            underline_color_hint(Some("/tmp/tmux-501/default,1,0"), None, None).expect("hint");
        assert!(hint.contains("\",xterm-256color:usstyle\""));
    }
}
