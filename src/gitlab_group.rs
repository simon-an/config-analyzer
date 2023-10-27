use gitlab::{GroupId, GroupStatistics, Project, VisibilityLevel};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Group {
    /// The ID of the group.
    pub id: GroupId,
    /// The name of the group.
    pub name: String,
    /// The path to the group.
    pub path: String,
    /// The description of the group.
    pub description: Option<String>,
    /// Whether the project is public, internal, or private.
    pub visibility: VisibilityLevel,
    /// Whether LFS is enabled for the group.
    pub lfs_enabled: bool,
    /// The URL to the group avatar.
    pub avatar_url: Option<String>,
    /// The URL to the group's profile page.
    pub web_url: String,
    /// Whether membership requests are allowed for the group.
    pub request_access_enabled: bool,
    pub full_name: String,
    pub full_path: String,
    pub parent_id: Option<GroupId>,
    /// Statistics about the group.
    pub statistics: Option<GroupStatistics>,

    pub projects: Option<Vec<Project>>, // TODO: create MR for gitlab crate to add this field
}
