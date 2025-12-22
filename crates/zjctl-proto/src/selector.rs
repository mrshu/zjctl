//! Pane selector parsing and types.

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

/// Errors from selector parsing
#[derive(Debug, Error)]
pub enum SelectorError {
    #[error("invalid selector format: {0}")]
    InvalidFormat(String),
    #[error("invalid pane type: {0} (expected 'terminal' or 'plugin')")]
    InvalidPaneType(String),
    #[error("invalid pane id: {0}")]
    InvalidPaneId(String),
    #[error("invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),
}

/// Pane selector for addressing panes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PaneSelector {
    /// Select by explicit pane ID: `id:terminal:N` or `id:plugin:N`
    Id { pane_type: PaneType, id: u32 },
    /// Select the currently focused pane: `focused`
    Focused,
    /// Select by title pattern: `title:/regex/` or `title:substring`
    Title { pattern: StringPattern },
    /// Select by command pattern: `cmd:/regex/` or `cmd:substring`
    Command { pattern: StringPattern },
    /// Select by tab index and pane index within tab: `tab:N:index:M`
    TabIndex { tab: usize, index: usize },
}

/// Pane type discriminator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaneType {
    Terminal,
    Plugin,
}

impl FromStr for PaneType {
    type Err = SelectorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "terminal" => Ok(PaneType::Terminal),
            "plugin" => Ok(PaneType::Plugin),
            _ => Err(SelectorError::InvalidPaneType(s.to_string())),
        }
    }
}

/// String matching pattern - either substring or regex
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StringPattern {
    /// Substring match (case-insensitive)
    Substring { value: String },
    /// Regex match
    Regex { pattern: String },
}

impl StringPattern {
    /// Test if this pattern matches the given string
    pub fn matches(&self, s: &str) -> Result<bool, regex::Error> {
        match self {
            StringPattern::Substring { value } => {
                Ok(s.to_lowercase().contains(&value.to_lowercase()))
            }
            StringPattern::Regex { pattern } => {
                let re = regex::Regex::new(pattern)?;
                Ok(re.is_match(s))
            }
        }
    }
}

impl FromStr for PaneSelector {
    type Err = SelectorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // focused
        if s == "focused" {
            return Ok(PaneSelector::Focused);
        }

        // id:terminal:N or id:plugin:N
        if let Some(rest) = s.strip_prefix("id:") {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(SelectorError::InvalidFormat(
                    "id selector requires format id:terminal:N or id:plugin:N".to_string(),
                ));
            }
            let pane_type = PaneType::from_str(parts[0])?;
            let id: u32 = parts[1]
                .parse()
                .map_err(|_| SelectorError::InvalidPaneId(parts[1].to_string()))?;
            return Ok(PaneSelector::Id { pane_type, id });
        }

        // title:/regex/ or title:substring
        if let Some(rest) = s.strip_prefix("title:") {
            let pattern = parse_string_pattern(rest)?;
            return Ok(PaneSelector::Title { pattern });
        }

        // cmd:/regex/ or cmd:substring
        if let Some(rest) = s.strip_prefix("cmd:") {
            let pattern = parse_string_pattern(rest)?;
            return Ok(PaneSelector::Command { pattern });
        }

        // tab:N:index:M
        if let Some(rest) = s.strip_prefix("tab:") {
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() == 3 && parts[1] == "index" {
                let tab: usize = parts[0]
                    .parse()
                    .map_err(|_| SelectorError::InvalidFormat("invalid tab index".to_string()))?;
                let index: usize = parts[2]
                    .parse()
                    .map_err(|_| SelectorError::InvalidFormat("invalid pane index".to_string()))?;
                return Ok(PaneSelector::TabIndex { tab, index });
            }
            return Err(SelectorError::InvalidFormat(
                "tab selector requires format tab:N:index:M".to_string(),
            ));
        }

        Err(SelectorError::InvalidFormat(format!(
            "unknown selector format: {}",
            s
        )))
    }
}

/// Parse a string pattern - /regex/ or plain substring
fn parse_string_pattern(s: &str) -> Result<StringPattern, SelectorError> {
    if s.starts_with('/') && s.ends_with('/') && s.len() > 2 {
        let pattern = &s[1..s.len() - 1];
        // Validate the regex
        regex::Regex::new(pattern)?;
        Ok(StringPattern::Regex {
            pattern: pattern.to_string(),
        })
    } else {
        Ok(StringPattern::Substring {
            value: s.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_focused() {
        let sel: PaneSelector = "focused".parse().unwrap();
        assert!(matches!(sel, PaneSelector::Focused));
    }

    #[test]
    fn test_parse_id_terminal() {
        let sel: PaneSelector = "id:terminal:42".parse().unwrap();
        match sel {
            PaneSelector::Id { pane_type, id } => {
                assert_eq!(pane_type, PaneType::Terminal);
                assert_eq!(id, 42);
            }
            _ => panic!("expected Id selector"),
        }
    }

    #[test]
    fn test_parse_id_plugin() {
        let sel: PaneSelector = "id:plugin:7".parse().unwrap();
        match sel {
            PaneSelector::Id { pane_type, id } => {
                assert_eq!(pane_type, PaneType::Plugin);
                assert_eq!(id, 7);
            }
            _ => panic!("expected Id selector"),
        }
    }

    #[test]
    fn test_parse_title_substring() {
        let sel: PaneSelector = "title:vim".parse().unwrap();
        match sel {
            PaneSelector::Title { pattern } => {
                assert!(matches!(pattern, StringPattern::Substring { .. }));
            }
            _ => panic!("expected Title selector"),
        }
    }

    #[test]
    fn test_parse_title_regex() {
        let sel: PaneSelector = "title:/^vim.*$/".parse().unwrap();
        match sel {
            PaneSelector::Title { pattern } => {
                assert!(matches!(pattern, StringPattern::Regex { .. }));
            }
            _ => panic!("expected Title selector"),
        }
    }

    #[test]
    fn test_parse_cmd_substring() {
        let sel: PaneSelector = "cmd:cargo".parse().unwrap();
        match sel {
            PaneSelector::Command { pattern } => {
                assert!(matches!(pattern, StringPattern::Substring { .. }));
            }
            _ => panic!("expected Command selector"),
        }
    }

    #[test]
    fn test_parse_tab_index() {
        let sel: PaneSelector = "tab:2:index:0".parse().unwrap();
        match sel {
            PaneSelector::TabIndex { tab, index } => {
                assert_eq!(tab, 2);
                assert_eq!(index, 0);
            }
            _ => panic!("expected TabIndex selector"),
        }
    }

    #[test]
    fn test_pattern_matching() {
        let substr = StringPattern::Substring {
            value: "vim".to_string(),
        };
        assert!(substr.matches("nvim").unwrap());
        assert!(substr.matches("VIM").unwrap());
        assert!(!substr.matches("nano").unwrap());

        let regex = StringPattern::Regex {
            pattern: "^cargo".to_string(),
        };
        assert!(regex.matches("cargo build").unwrap());
        assert!(!regex.matches("run cargo").unwrap());
    }
}
