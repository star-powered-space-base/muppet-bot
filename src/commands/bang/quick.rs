//! Quick bang commands: !ping, !features

use crate::features;

/// Generate ping response
pub fn ping() -> String {
    "🏓 Pong!".to_string()
}

/// Generate features list response
pub fn features_list() -> String {
    features::format_features_list()
}
