use std::fmt;

/// Club-level permission catalogue (foundation set). Business modules extend
/// this enum in their own slices. Grants live in `club_role_permissions`;
/// unknown strings read from the database are ignored, never granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    ClubManage,
    ClubMembersView,
    ClubMembersManage,
    ClubRolesManage,
    ClubIntakeManage,
    ClubImpersonate,
}

impl Permission {
    pub const ALL: [Permission; 6] = [
        Permission::ClubManage,
        Permission::ClubMembersView,
        Permission::ClubMembersManage,
        Permission::ClubRolesManage,
        Permission::ClubIntakeManage,
        Permission::ClubImpersonate,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::ClubManage => "club.manage",
            Permission::ClubMembersView => "club.members.view",
            Permission::ClubMembersManage => "club.members.manage",
            Permission::ClubRolesManage => "club.roles.manage",
            Permission::ClubIntakeManage => "club.intake.manage",
            Permission::ClubImpersonate => "club.impersonate",
        }
    }

    pub fn parse(value: &str) -> Option<Permission> {
        Permission::ALL.into_iter().find(|p| p.as_str() == value)
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_every_permission() {
        for p in Permission::ALL {
            assert_eq!(Permission::parse(p.as_str()), Some(p));
        }
    }

    #[test]
    fn unknown_permission_is_never_granted() {
        assert_eq!(Permission::parse("club.everything"), None);
        assert_eq!(Permission::parse(""), None);
    }
}
