export type NavigationSection =
  | "overview"
  | "timetable"
  | "campus"
  | "forum"
  | "evaluation"
  | "settings";

export interface CommunityUser {
  userId: number;
  nickname: string;
  isAdmin: boolean;
}

export interface SessionStatus {
  communityLoggedIn: boolean;
  communityUser: CommunityUser | null;
  campusLoggedIn: boolean;
  campusId: string | null;
  campusName: string | null;
}

export interface CampusStatus {
  loggedIn: boolean;
  id: string | null;
  name: string | null;
}

export type CampusLoginResult =
  | { state: "authenticated"; status: CampusStatus }
  | { state: "requiresSecondFactor"; message: string };

export interface TimetableCourse {
  courseName: string;
  roomName: string | null;
  teacherNames: string[];
  /** 1 = Monday … 7 = Sunday. */
  weekday: number;
  startUnit: number;
  endUnit: number;
  /** 1-based week numbers. */
  weeks: number[];
}

export interface Timetable {
  courses: TimetableCourse[];
  /** Monday of week 1, `YYYY-MM-DD`; null when the system doesn't provide it. */
  semesterStartDate: string | null;
}

export type CampusLocation = "handan" | "fenglin" | "jiangwan" | "zhangjiang";

export interface LibraryOccupancy {
  campusName: string;
  people: number;
}

export interface DiningVenueOccupancy {
  name: string;
  current: number;
  capacity: number;
}

export interface DiningCrowdedness {
  available: boolean;
  venues: DiningVenueOccupancy[];
}

export interface CampusBus {
  id: string;
  startCampus: string;
  endCampus: string;
  startTime: string | null;
  endTime: string | null;
  /** Flutter enum: 0 none, 1 dual, 2 backward, 3 forward. */
  direction: number;
  holidayRun: boolean;
}

export interface EmptyClassroom {
  roomName: string;
  seats: number | null;
  /** Thirteen teaching periods; true means occupied. */
  busy: boolean[];
}

export interface ForumTag {
  tagId: number;
  name: string;
  temperature: number;
}

export interface ForumDivision {
  divisionId: number;
  name: string;
  description: string;
}

export interface ForumFloorPreview {
  floorId: number;
  content: string;
  anonyname: string;
  timeCreated: string;
}

export interface ForumFloor {
  floorId: number;
  holeId: number;
  content: string;
  anonyname: string;
  timeCreated: string;
  timeUpdated: string;
  specialTag: string;
  deleted: boolean;
  isMe: boolean;
  liked: boolean;
  like: number;
  disliked: boolean;
  dislike: number;
  modified: number;
  fold: string[];
}

export interface ForumHole {
  holeId: number;
  divisionId: number;
  timeCreated: string;
  timeUpdated: string;
  view: number;
  reply: number;
  favoriteCount: number;
  locked: boolean;
  tags: ForumTag[];
  firstFloor: ForumFloorPreview | null;
  lastFloor: ForumFloorPreview | null;
}

export interface ForumThreadPage {
  hole: ForumHole;
  floors: ForumFloor[];
  offset: number;
  nextOffset: number | null;
}

export interface HoleListQuery {
  divisionId?: number;
  size?: number;
  order?: "time_updated" | "time_created";
  before?: string;
}

export type ForumReaction = -1 | 0 | 1;

export interface EvaluationCourseGroup {
  groupId: number;
  name: string;
  code: string;
  department: string;
  weekHour: number | null;
  credits: number[];
  courseCount: number;
  reviewCount: number;
}

export interface EvaluationRating {
  overall: number | null;
  content: number | null;
  workload: number | null;
  assessment: number | null;
}

export interface EvaluationReview {
  reviewId: number;
  title: string;
  content: string;
  timeCreated: string;
  timeUpdated: string;
  rating: EvaluationRating;
  vote: number;
  remark: number;
  courseGroupId: number | null;
  courseName: string;
  teachers: string;
  term: string;
}

export interface EvaluationCourseDetail {
  group: EvaluationCourseGroup;
  reviews: EvaluationReview[];
}

/** Serialized AppError shape shared by both transports. */
export interface BackendError {
  kind: string;
  message: string;
}
