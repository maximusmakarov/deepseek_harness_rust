use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{DshrError, Result};

/// Time-sortable identifiers with a human-readable prefix.
///
/// Wire format: `<prefix>_<uuid-v7>`, e.g. `ses_0192f3...`. UUID v7 embeds a
/// millisecond timestamp, so ids sort in creation order without a central
/// sequence.
macro_rules! id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new() -> Self {
                Self(format!("{}_{}", $prefix, uuid::Uuid::now_v7()))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = DshrError;

            fn from_str(value: &str) -> Result<Self> {
                let Some(rest) = value.strip_prefix(concat!($prefix, "_")) else {
                    return Err(DshrError::invalid_input(format!(
                        "invalid {}: expected prefix '{}_'",
                        stringify!($name),
                        $prefix
                    )));
                };
                uuid::Uuid::parse_str(rest).map_err(|_| {
                    DshrError::invalid_input(format!("invalid {}: '{}'", stringify!($name), value))
                })?;
                Ok(Self(value.to_owned()))
            }
        }
    };
}

id_type!(SessionId, "ses");
id_type!(TurnId, "turn");
id_type!(StepId, "step");
id_type!(ToolCallId, "call");
id_type!(JobId, "job");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_through_strings() {
        let id = SessionId::new();
        let parsed: SessionId = id.to_string().parse().expect("valid id");
        assert_eq!(id, parsed);
    }

    #[test]
    fn ids_sort_by_creation_time() {
        let first = TurnId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = TurnId::new();
        assert!(first < second);
    }

    #[test]
    fn wrong_prefix_is_rejected() {
        let err = "turn_0192f3c0-0000-7000-8000-000000000000"
            .parse::<SessionId>()
            .expect_err("wrong prefix");
        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn malformed_uuid_is_rejected() {
        assert!("ses_not-a-uuid".parse::<SessionId>().is_err());
    }

    #[test]
    fn serde_is_transparent() {
        let id = JobId::new();
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, format!("\"{id}\""));
        let back: JobId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, back);
    }
}
