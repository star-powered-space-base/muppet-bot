//! Admin bang commands: !toggle, !reload, !sync

use crate::features;

/// Response for toggle command
pub struct ToggleResponse {
    pub success: bool,
    pub message: String,
}

/// Handle toggle command
pub fn toggle(feature_id: &str) -> ToggleResponse {
    // Check if feature exists
    match features::get_feature(feature_id) {
        Some(feature) => {
            if !feature.toggleable {
                ToggleResponse {
                    success: false,
                    message: format!(
                        "❌ Cannot toggle '{}' - this feature is not toggleable.",
                        feature.name
                    ),
                }
            } else {
                // In the actual implementation, this would toggle in the database
                // For now, return a message indicating what would happen
                ToggleResponse {
                    success: true,
                    message: format!(
                        "🔄 Feature '{}' toggle requested. Database update required.",
                        feature.name
                    ),
                }
            }
        }
        None => {
            let valid_features: Vec<&str> = features::get_toggleable_features()
                .map(|f| f.id)
                .collect();

            ToggleResponse {
                success: false,
                message: format!(
                    "❌ Unknown feature: '{}'\n\nToggleable features: {}",
                    feature_id,
                    valid_features.join(", ")
                ),
            }
        }
    }
}

/// Handle reload command response
pub fn reload() -> String {
    "🔄 Reloading guild settings from database...".to_string()
}

/// Handle sync command response
pub fn sync() -> String {
    "🔄 Syncing slash commands to this guild...".to_string()
}

/// Get list of toggleable features for help
pub fn list_toggleable() -> String {
    let mut output = String::from("**Toggleable Features:**\n\n");

    for feature in features::get_toggleable_features() {
        output.push_str(&format!(
            "• `{}` - {} (v{})\n",
            feature.id, feature.description, feature.version
        ));
    }

    output.push_str("\nUse `!toggle <feature_id>` to enable/disable.");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_nonexistent_feature() {
        let response = toggle("nonexistent");
        assert!(!response.success);
        assert!(response.message.contains("Unknown feature"));
    }

    #[test]
    fn test_toggle_non_toggleable_feature() {
        let response = toggle("personas");
        assert!(!response.success);
        assert!(response.message.contains("not toggleable"));
    }

    #[test]
    fn test_toggle_valid_feature() {
        let response = toggle("reminders");
        assert!(response.success);
        assert!(response.message.contains("toggle requested"));
    }
}
