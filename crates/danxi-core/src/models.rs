use serde::{Deserialize, Serialize};

/// Access/refresh token pair issued by the community auth service.
///
/// It never crosses the runtime boundary: commands and the web gateway only
/// persist it behind a [crate::SessionStore] and attach it to upstream calls.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct TokenPair {
    pub access: String,
    pub refresh: String,
}

impl TokenPair {
    pub fn is_valid(&self) -> bool {
        !self.access.is_empty() && !self.refresh.is_empty()
    }
}

/// Public profile of the signed-in community account.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityUser {
    pub user_id: i64,
    pub nickname: String,
    pub is_admin: bool,
}

/// What the UI may know about the current authentication state.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub community_logged_in: bool,
    pub community_user: Option<CommunityUser>,
    pub campus_logged_in: bool,
    pub campus_id: Option<String>,
    pub campus_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumTag {
    pub tag_id: i64,
    pub name: String,
    pub temperature: f32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumDivision {
    pub division_id: i64,
    pub name: String,
    pub description: String,
}

/// Trimmed-down floor preview embedded in hole listings.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumFloorPreview {
    pub floor_id: i64,
    pub content: String,
    pub anonyname: String,
    pub time_created: String,
    pub mentions: Vec<ForumFloorMention>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumFloorMention {
    pub floor_id: i64,
    pub hole_id: i64,
    pub content: String,
    pub anonyname: String,
    pub deleted: bool,
}

/// One complete floor in a forum thread.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumFloor {
    pub floor_id: i64,
    pub hole_id: i64,
    pub content: String,
    pub anonyname: String,
    pub time_created: String,
    pub time_updated: String,
    pub special_tag: String,
    pub deleted: bool,
    pub is_me: bool,
    pub liked: bool,
    pub like: i64,
    pub disliked: bool,
    pub dislike: i64,
    pub modified: i64,
    pub fold: Vec<String>,
    pub mentions: Vec<ForumFloorMention>,
}

/// Hole listing item shown by the forum panel.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumHole {
    pub hole_id: i64,
    pub division_id: i64,
    pub time_created: String,
    pub time_updated: String,
    pub view: i64,
    pub reply: i64,
    pub favorite_count: i64,
    pub locked: bool,
    pub tags: Vec<ForumTag>,
    pub first_floor: Option<ForumFloorPreview>,
    pub last_floor: Option<ForumFloorPreview>,
}

/// A page of readable thread content returned to desktop and web clients.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumThreadPage {
    pub hole: ForumHole,
    pub floors: Vec<ForumFloor>,
    pub offset: u32,
    pub next_offset: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumSearchPage {
    pub floors: Vec<ForumFloor>,
    pub offset: u32,
    pub next_offset: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationCourseGroup {
    pub group_id: i64,
    pub name: String,
    pub code: String,
    pub department: String,
    pub week_hour: Option<i64>,
    pub credits: Vec<f64>,
    pub course_count: i64,
    pub review_count: i64,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationRating {
    pub overall: Option<i64>,
    pub content: Option<i64>,
    pub workload: Option<i64>,
    pub assessment: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationReview {
    pub review_id: i64,
    pub title: String,
    pub content: String,
    pub time_created: String,
    pub time_updated: String,
    pub rating: EvaluationRating,
    pub vote: i64,
    pub remark: i64,
    pub course_group_id: Option<i64>,
    pub course_name: String,
    pub teachers: String,
    pub term: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationCourseDetail {
    pub group: EvaluationCourseGroup,
    pub reviews: Vec<EvaluationReview>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryOccupancy {
    pub campus_name: String,
    pub people: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiningVenueOccupancy {
    pub name: String,
    pub current: i64,
    pub capacity: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiningCrowdedness {
    pub available: bool,
    pub venues: Vec<DiningVenueOccupancy>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampusBus {
    pub id: String,
    pub start_campus: String,
    pub end_campus: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub direction: i64,
    pub holiday_run: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyClassroom {
    pub room_name: String,
    pub seats: Option<i64>,
    pub busy: Vec<bool>,
}

/// Sort orders accepted by `GET /api/holes`, matching the upstream values.
#[derive(Clone, Copy, Debug, Default)]
pub enum HoleSortOrder {
    #[default]
    LastReplied,
    LastCreated,
}

impl HoleSortOrder {
    pub fn as_str(&self) -> &'static str {
        match self {
            HoleSortOrder::LastReplied => "time_updated",
            HoleSortOrder::LastCreated => "time_created",
        }
    }
}
