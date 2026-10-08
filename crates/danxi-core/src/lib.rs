mod campus;
mod campus_services;
mod error;
mod forum;
mod models;
mod session;
mod timetable;
mod webvpn;

pub use campus::{
    CampusAuthenticationResult, CampusCredentialStore, CampusCredentials, CampusLoginResult,
    CampusSecondFactorContext, CampusService, CampusSession, CampusStatus, DEFAULT_ID_HOST,
    merge_campus_status,
};
pub use campus_services::{CampusLifeService, CampusLocation, teaching_buildings};
pub use error::AppError;
pub use forum::{
    DEFAULT_AUTH_BASE_URL, DEFAULT_DANKE_BASE_URL, DEFAULT_FORUM_BASE_URL, ForumService,
    SessionManager,
};
pub use models::{
    CampusBus, CommunityUser, DiningCrowdedness, DiningVenueOccupancy, EmptyClassroom,
    EvaluationCourseDetail, EvaluationCourseGroup, EvaluationRating, EvaluationReview,
    ForumDivision, ForumFloor, ForumFloorMention, ForumFloorPreview, ForumHole, ForumTag,
    ForumThreadPage, HoleSortOrder, LibraryOccupancy, SessionStatus, TokenPair,
};
pub use session::{MemorySessionStore, SessionStore};
pub use timetable::{
    Timetable, TimetableCourse, parse_jwgl, parse_postgraduate, parse_semester_start,
};
