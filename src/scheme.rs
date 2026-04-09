use clap::ValueEnum;
use praxis_domains::technology::theming::schemes::SchemeType;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Color scheme types supported by Vogix.
///
/// Application wrapper around praxis SchemeType — adds CLI (clap),
/// serialization (serde), display, and parsing traits.
/// Use `.to_praxis()` for ontological reasoning.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    /// ANSI 16-color terminal scheme
    Ansi16,
    /// Base16 scheme (16 colors: base00-base0F)
    Base16,
    /// Base24 scheme (24 colors: base00-base17)
    Base24,
    /// Vogix16 native scheme with semantic color names
    #[default]
    Vogix16,
}

impl Scheme {
    /// Convert to praxis SchemeType for ontological reasoning.
    #[allow(dead_code)]
    pub fn to_praxis(self) -> SchemeType {
        match self {
            Scheme::Ansi16 => SchemeType::Ansi16,
            Scheme::Base16 => SchemeType::Base16,
            Scheme::Base24 => SchemeType::Base24,
            Scheme::Vogix16 => SchemeType::Vogix16,
        }
    }

    /// Create from praxis SchemeType.
    #[allow(dead_code)]
    pub fn from_praxis(st: SchemeType) -> Self {
        match st {
            SchemeType::Ansi16 => Scheme::Ansi16,
            SchemeType::Base16 => Scheme::Base16,
            SchemeType::Base24 => Scheme::Base24,
            SchemeType::Vogix16 => Scheme::Vogix16,
        }
    }

    /// Number of color slots for this scheme (from ontology).
    #[allow(dead_code)]
    pub fn slot_count(self) -> usize {
        self.to_praxis().slot_count()
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scheme::Vogix16 => write!(f, "vogix16"),
            Scheme::Base16 => write!(f, "base16"),
            Scheme::Base24 => write!(f, "base24"),
            Scheme::Ansi16 => write!(f, "ansi16"),
        }
    }
}

impl FromStr for Scheme {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "vogix16" => Ok(Scheme::Vogix16),
            "base16" => Ok(Scheme::Base16),
            "base24" => Ok(Scheme::Base24),
            "ansi16" => Ok(Scheme::Ansi16),
            _ => Err(format!(
                "Unknown scheme: {}. Valid schemes: vogix16, base16, base24, ansi16",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheme_display() {
        assert_eq!(Scheme::Vogix16.to_string(), "vogix16");
        assert_eq!(Scheme::Base16.to_string(), "base16");
        assert_eq!(Scheme::Base24.to_string(), "base24");
        assert_eq!(Scheme::Ansi16.to_string(), "ansi16");
    }

    #[test]
    fn test_scheme_from_str() {
        assert_eq!("vogix16".parse::<Scheme>().unwrap(), Scheme::Vogix16);
        assert_eq!("BASE16".parse::<Scheme>().unwrap(), Scheme::Base16);
        assert_eq!("Base24".parse::<Scheme>().unwrap(), Scheme::Base24);
        assert_eq!("ANSI16".parse::<Scheme>().unwrap(), Scheme::Ansi16);
        assert!("invalid".parse::<Scheme>().is_err());
    }

    #[test]
    fn test_scheme_default() {
        assert_eq!(Scheme::default(), Scheme::Vogix16);
    }

    #[test]
    fn test_praxis_roundtrip() {
        for scheme in [Scheme::Ansi16, Scheme::Base16, Scheme::Base24, Scheme::Vogix16] {
            assert_eq!(Scheme::from_praxis(scheme.to_praxis()), scheme);
        }
    }

    #[test]
    fn test_slot_count_from_ontology() {
        assert_eq!(Scheme::Base16.slot_count(), 16);
        assert_eq!(Scheme::Base24.slot_count(), 24);
        assert_eq!(Scheme::Vogix16.slot_count(), 16);
        assert_eq!(Scheme::Ansi16.slot_count(), 16);
    }
}
