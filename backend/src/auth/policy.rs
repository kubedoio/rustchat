//! Authorization policy engine
//!
//! Provides a centralized permission system for access control.
//! Supports both role-based and resource-based permissions.

use crate::constants::*;
use std::collections::HashSet;
use uuid::Uuid;

/// Resource types that can be protected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    User,
    Team,
    Channel,
    Post,
    File,
    System,
    Admin,
}

/// Actions that can be performed on resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Create,
    Read,
    Update,
    Delete,
    List,
    Manage,
}

/// Permission combining a resource and action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Permission {
    pub resource: Resource,
    pub action: Action,
}

impl Permission {
    pub const fn new(resource: Resource, action: Action) -> Self {
        Self { resource, action }
    }
}

/// Predefined permissions for common operations
pub mod permissions {
    use super::*;

    // User permissions
    pub const USER_READ: Permission = Permission::new(Resource::User, Action::Read);
    pub const USER_UPDATE: Permission = Permission::new(Resource::User, Action::Update);
    pub const USER_DELETE: Permission = Permission::new(Resource::User, Action::Delete);
    pub const USER_MANAGE: Permission = Permission::new(Resource::User, Action::Manage);

    // Team permissions
    pub const TEAM_CREATE: Permission = Permission::new(Resource::Team, Action::Create);
    pub const TEAM_READ: Permission = Permission::new(Resource::Team, Action::Read);
    pub const TEAM_UPDATE: Permission = Permission::new(Resource::Team, Action::Update);
    pub const TEAM_DELETE: Permission = Permission::new(Resource::Team, Action::Delete);
    pub const TEAM_MANAGE: Permission = Permission::new(Resource::Team, Action::Manage);

    // Channel permissions
    pub const CHANNEL_CREATE: Permission = Permission::new(Resource::Channel, Action::Create);
    pub const CHANNEL_READ: Permission = Permission::new(Resource::Channel, Action::Read);
    pub const CHANNEL_UPDATE: Permission = Permission::new(Resource::Channel, Action::Update);
    pub const CHANNEL_DELETE: Permission = Permission::new(Resource::Channel, Action::Delete);
    pub const CHANNEL_MANAGE: Permission = Permission::new(Resource::Channel, Action::Manage);

    // Post permissions
    pub const POST_CREATE: Permission = Permission::new(Resource::Post, Action::Create);
    pub const POST_READ: Permission = Permission::new(Resource::Post, Action::Read);
    pub const POST_UPDATE: Permission = Permission::new(Resource::Post, Action::Update);
    pub const POST_DELETE: Permission = Permission::new(Resource::Post, Action::Delete);

    // File permissions
    pub const FILE_CREATE: Permission = Permission::new(Resource::File, Action::Create);
    pub const FILE_READ: Permission = Permission::new(Resource::File, Action::Read);
    pub const FILE_DELETE: Permission = Permission::new(Resource::File, Action::Delete);

    // System permissions
    pub const SYSTEM_READ: Permission = Permission::new(Resource::System, Action::Read);
    pub const SYSTEM_MANAGE: Permission = Permission::new(Resource::System, Action::Manage);

    // Admin permissions
    pub const ADMIN_FULL: Permission = Permission::new(Resource::Admin, Action::Manage);
}

/// Role definitions with their permissions
#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<Permission>,
}

impl Role {
    /// Create a system admin role with all permissions
    pub fn system_admin() -> Self {
        use permissions::*;
        let mut permissions = HashSet::new();

        // Admin has all permissions
        permissions.insert(ADMIN_FULL);
        permissions.insert(USER_MANAGE);
        permissions.insert(TEAM_MANAGE);
        permissions.insert(CHANNEL_MANAGE);
        permissions.insert(SYSTEM_MANAGE);
        permissions.insert(POST_DELETE); // Can delete any post

        Self {
            name: ROLE_SYSTEM_ADMIN.to_string(),
            permissions,
        }
    }

    /// Create a team admin role
    pub fn team_admin() -> Self {
        use permissions::*;
        let mut permissions = HashSet::new();

        permissions.insert(TEAM_CREATE);
        permissions.insert(TEAM_MANAGE);
        permissions.insert(CHANNEL_MANAGE);
        permissions.insert(USER_READ);
        permissions.insert(POST_DELETE); // Can delete posts in their teams

        Self {
            name: ROLE_TEAM_ADMIN.to_string(),
            permissions,
        }
    }

    /// Create an organization admin role.
    ///
    /// This maps to the existing `org_admin` semantics used in API handlers.
    pub fn org_admin() -> Self {
        use permissions::*;
        let mut permissions = HashSet::new();

        permissions.insert(TEAM_CREATE);
        permissions.insert(TEAM_MANAGE);
        permissions.insert(CHANNEL_MANAGE);
        permissions.insert(USER_MANAGE);
        permissions.insert(POST_DELETE);

        Self {
            name: ROLE_ORG_ADMIN.to_string(),
            permissions,
        }
    }

    /// Create a standard member role
    pub fn member() -> Self {
        use permissions::*;
        let mut permissions = HashSet::new();

        // Basic permissions for regular users
        permissions.insert(USER_READ);
        permissions.insert(USER_UPDATE); // Own profile only
        permissions.insert(TEAM_READ);
        permissions.insert(CHANNEL_READ);
        permissions.insert(CHANNEL_CREATE); // Can create channels
        permissions.insert(POST_CREATE);
        permissions.insert(POST_READ);
        permissions.insert(POST_UPDATE); // Own posts only
        permissions.insert(FILE_CREATE);
        permissions.insert(FILE_READ);
        permissions.insert(SYSTEM_READ);

        Self {
            name: ROLE_MEMBER.to_string(),
            permissions,
        }
    }

    /// Create a guest role with limited permissions
    pub fn guest() -> Self {
        use permissions::*;
        let mut permissions = HashSet::new();

        permissions.insert(USER_READ);
        permissions.insert(TEAM_READ);
        permissions.insert(CHANNEL_READ);
        permissions.insert(POST_READ);
        permissions.insert(FILE_READ);

        Self {
            name: ROLE_GUEST.to_string(),
            permissions,
        }
    }

    /// Check if role has a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Get role by name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            ROLE_SYSTEM_ADMIN => Some(Self::system_admin()),
            ROLE_TEAM_ADMIN => Some(Self::team_admin()),
            ROLE_ORG_ADMIN => Some(Self::org_admin()),
            ROLE_ADMIN => Some(Self::org_admin()), // Alias
            ROLE_MEMBER => Some(Self::member()),
            ROLE_GUEST => Some(Self::guest()),
            _ => None,
        }
    }
}

/// Authorization check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthzResult {
    Allow,
    Deny(&'static str),
}

/// Policy engine for authorization decisions
pub struct PolicyEngine;

impl PolicyEngine {
    fn parse_roles(role: &str) -> impl Iterator<Item = &str> {
        role.split(|c: char| c.is_whitespace() || c == ',')
            .map(str::trim)
            .filter(|r| !r.is_empty())
    }

    /// Check if user has permission
    pub fn check_permission(role: &str, permission: &Permission) -> AuthzResult {
        let mut has_known_role = false;

        for role_name in Self::parse_roles(role) {
            let Some(role_def) = Role::from_name(role_name) else {
                continue;
            };
            has_known_role = true;

            // Admin role has all permissions
            if role_def.has_permission(&permissions::ADMIN_FULL) {
                return AuthzResult::Allow;
            }

            if role_def.has_permission(permission) {
                return AuthzResult::Allow;
            }
        }

        if has_known_role {
            AuthzResult::Deny("Permission denied")
        } else {
            AuthzResult::Deny("Unknown role")
        }
    }

    /// Check if user can access a resource they own
    pub fn check_ownership(
        role: &str,
        permission: &Permission,
        user_id: Uuid,
        resource_owner_id: Uuid,
    ) -> AuthzResult {
        // Owner can always perform actions on their own resources
        if user_id == resource_owner_id {
            return AuthzResult::Allow;
        }

        // Otherwise check role permission
        Self::check_permission(role, permission)
    }
}

/// Helper macros for authorization
#[macro_export]
macro_rules! require_permission {
    ($auth_user:expr, $permission:expr) => {
        match $crate::auth::policy::PolicyEngine::check_permission(&$auth_user.role, &$permission) {
            $crate::auth::policy::AuthzResult::Allow => {}
            $crate::auth::policy::AuthzResult::Deny(reason) => {
                return Err($crate::error::AppError::Forbidden(reason.to_string()));
            }
        }
    };
}

#[macro_export]
macro_rules! require_admin {
    ($auth_user:expr) => {
        match $auth_user.role.as_str() {
            "system_admin" | "admin" => {}
            _ => {
                return Err($crate::error::AppError::Forbidden(
                    "Admin access required".to_string(),
                ))
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use permissions::*;

    #[test]
    fn test_system_admin_has_all_permissions() {
        let admin = Role::system_admin();

        assert!(admin.has_permission(&ADMIN_FULL));
        assert!(admin.has_permission(&USER_MANAGE));
        assert!(admin.has_permission(&SYSTEM_MANAGE));
        assert!(admin.has_permission(&POST_DELETE));
    }

    #[test]
    fn test_member_limited_permissions() {
        let member = Role::member();

        assert!(!member.has_permission(&TEAM_CREATE));
        assert!(member.has_permission(&POST_CREATE));
        assert!(member.has_permission(&CHANNEL_READ));
        assert!(!member.has_permission(&USER_MANAGE));
        assert!(!member.has_permission(&SYSTEM_MANAGE));
    }

    #[test]
    fn test_guest_readonly() {
        let guest = Role::guest();

        assert!(guest.has_permission(&POST_READ));
        assert!(!guest.has_permission(&POST_CREATE));
        assert!(!guest.has_permission(&CHANNEL_CREATE));
    }

    #[test]
    fn test_policy_engine_check() {
        assert_eq!(
            PolicyEngine::check_permission("system_admin", &POST_DELETE),
            AuthzResult::Allow
        );

        assert_eq!(
            PolicyEngine::check_permission("member", &POST_CREATE),
            AuthzResult::Allow
        );

        assert_eq!(
            PolicyEngine::check_permission("member", &TEAM_CREATE),
            AuthzResult::Deny("Permission denied")
        );

        assert!(matches!(
            PolicyEngine::check_permission("guest", &POST_CREATE),
            AuthzResult::Deny(_)
        ));
    }

    #[test]
    fn test_policy_engine_multi_role_support() {
        // org_admin does NOT grant SYSTEM_MANAGE (removed to prevent privilege escalation)
        assert_eq!(
            PolicyEngine::check_permission("member org_admin", &permissions::SYSTEM_MANAGE),
            AuthzResult::Deny("Permission denied")
        );
        assert_eq!(
            PolicyEngine::check_permission("member,org_admin", &permissions::SYSTEM_MANAGE),
            AuthzResult::Deny("Permission denied")
        );
        // org_admin still grants USER_MANAGE and TEAM_MANAGE
        assert_eq!(
            PolicyEngine::check_permission("org_admin", &permissions::USER_MANAGE),
            AuthzResult::Allow
        );
        assert_eq!(
            PolicyEngine::check_permission("org_admin", &permissions::TEAM_MANAGE),
            AuthzResult::Allow
        );
        // system_admin retains SYSTEM_MANAGE
        assert_eq!(
            PolicyEngine::check_permission("system_admin", &permissions::SYSTEM_MANAGE),
            AuthzResult::Allow
        );
    }

    #[test]
    fn test_ownership_check() {
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        // Owner can access their own resource
        assert_eq!(
            PolicyEngine::check_ownership("guest", &POST_UPDATE, user1, user1),
            AuthzResult::Allow
        );

        // Non-owner needs permission
        assert_eq!(
            PolicyEngine::check_ownership("member", &POST_UPDATE, user1, user2),
            AuthzResult::Allow // Members have POST_UPDATE
        );

        // Guest can't update others' posts
        assert!(matches!(
            PolicyEngine::check_ownership("guest", &POST_UPDATE, user1, user2),
            AuthzResult::Deny(_)
        ));
    }
}
