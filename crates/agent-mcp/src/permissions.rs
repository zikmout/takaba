use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPermissions {
    #[serde(default = "default_true")]
    pub allow_read: bool,
    #[serde(default)]
    pub allow_write: bool,
    #[serde(default = "default_true")]
    pub require_confirmation_for_write: bool,
}

fn default_true() -> bool {
    true
}

impl Default for McpPermissions {
    fn default() -> Self {
        Self {
            allow_read: true,
            allow_write: false,
            require_confirmation_for_write: true,
        }
    }
}

impl McpPermissions {
    pub fn check(&self, is_write: bool) -> PermissionResult {
        if is_write {
            if !self.allow_write {
                return PermissionResult::Denied("write not allowed".into());
            }
            if self.require_confirmation_for_write {
                return PermissionResult::NeedsConfirmation;
            }
        } else if !self.allow_read {
            return PermissionResult::Denied("read not allowed".into());
        }

        PermissionResult::Allowed
    }
}

#[derive(Debug, Clone)]
pub enum PermissionResult {
    Allowed,
    Denied(String),
    NeedsConfirmation,
}
