import { backend, toTransportError } from "$lib/api";
import { session } from "$lib/stores/session.svelte";
import type { Timetable, TimetableCourse } from "$lib/types/app";

const START_DATE_OVERRIDE_KEY = "danxi.timetable.semesterStart";

export type TimetableState =
  | { phase: "idle" }
  | { phase: "loading" }
  | { phase: "unauthenticated" }
  | { phase: "error"; message: string }
  | { phase: "ready"; timetable: Timetable };

function loadOverride(): string | null {
  try {
    return localStorage.getItem(START_DATE_OVERRIDE_KEY);
  } catch {
    return null;
  }
}

/** Timetable state for the schedule panel, including week navigation. */
export class TimetableStore {
  state = $state<TimetableState>({ phase: "idle" });
  selectedWeek = $state(1);
  /** User-set semester start, used when the system doesn't provide one. */
  semesterStartOverride = $state<string | null>(loadOverride());
  #request = 0;

  requireLogin() {
    this.#request += 1;
    this.state = { phase: "unauthenticated" };
  }

  async load(force = false) {
    if (!session.status.campusLoggedIn) {
      this.requireLogin();
      return;
    }
    if (!force && (this.state.phase === "loading" || this.state.phase === "ready")) return;
    const request = ++this.#request;
    this.state = { phase: "loading" };
    try {
      const timetableData = await backend.loadTimetable();
      if (request !== this.#request) return;
      this.state = { phase: "ready", timetable: timetableData };
      const start = effectiveStart(timetableData, this.semesterStartOverride);
      this.selectedWeek = currentWeek(start) ?? 1;
    } catch (raw) {
      if (request !== this.#request) return;
      const error = toTransportError(raw);
      this.state =
        error.kind === "auth"
          ? { phase: "unauthenticated" }
          : { phase: "error", message: error.message };
    }
  }

  setSemesterStart(date: string) {
    this.semesterStartOverride = date;
    try {
      localStorage.setItem(START_DATE_OVERRIDE_KEY, date);
    } catch {
      // Best-effort persistence.
    }
    this.selectedWeek = currentWeek(date) ?? this.selectedWeek;
  }

  goToWeek(week: number, maxWeek: number) {
    this.selectedWeek = Math.min(Math.max(1, week), maxWeek);
  }
}

export const timetable = new TimetableStore();

/** The effective semester start: system-provided or user override. */
export function effectiveStart(
  timetableData: Timetable,
  override: string | null,
): string | null {
  return override ?? timetableData.semesterStartDate;
}

/** Monday-based weekday index (1-7). */
export function weekdayOfToday(): number {
  const day = new Date().getDay();
  return day === 0 ? 7 : day;
}

/**
 * Week number relative to the semester start, 1-based. Null when the start
 * date is unknown — callers then skip week filtering.
 */
export function currentWeek(semesterStartDate: string | null): number | null {
  if (!semesterStartDate) return null;
  const start = Date.parse(`${semesterStartDate}T00:00:00`);
  if (Number.isNaN(start)) return null;
  const diffDays = Math.floor((Date.now() - start) / 86_400_000);
  return Math.max(1, Math.floor(diffDays / 7) + 1);
}

/** The last week any course appears in. */
export function maxWeekOf(timetableData: Timetable): number {
  return Math.max(1, ...timetableData.courses.flatMap((course) => course.weeks));
}

export function todayCourses(
  timetableData: Timetable,
  override: string | null = null,
): TimetableCourse[] {
  const weekday = weekdayOfToday();
  const week = currentWeek(effectiveStart(timetableData, override));
  return timetableData.courses
    .filter(
      (course) =>
        course.weekday === weekday &&
        (week === null || course.weeks.length === 0 || course.weeks.includes(week)),
    )
    .sort((a, b) => a.startUnit - b.startUnit);
}

/** Courses of one week, grouped by weekday (1-7) and period-sorted. */
export function weekCourses(
  timetableData: Timetable,
  week: number,
): Map<number, TimetableCourse[]> {
  const grouped = new Map<number, TimetableCourse[]>();
  for (const course of timetableData.courses) {
    if (course.weeks.length > 0 && !course.weeks.includes(week)) continue;
    const list = grouped.get(course.weekday) ?? [];
    list.push(course);
    grouped.set(course.weekday, list);
  }
  for (const list of grouped.values()) {
    list.sort((a, b) => a.startUnit - b.startUnit);
  }
  return grouped;
}

export const WEEKDAY_LABELS = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];

/** A stable pastel palette per course name, calendar-app style. */
const COURSE_HUES = [210, 352, 158, 28, 262, 188, 95, 325, 42, 230];

function hashString(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i++) {
    hash = (hash * 31 + value.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

export interface CoursePalette {
  /** Tinted card background, works in light and dark themes. */
  background: string;
  /** Saturated accent for the left bar. */
  bar: string;
  /** Readable accent text for meta info. */
  text: string;
}

export function coursePalette(courseName: string): CoursePalette {
  const hue = COURSE_HUES[hashString(courseName) % COURSE_HUES.length];
  return {
    background: `hsl(${hue} 60% 50% / 0.09)`,
    bar: `hsl(${hue} 52% 48%)`,
    text: `hsl(${hue} 45% 38%)`,
  };
}

/** Calendar date of a weekday in a week, when the semester start is known. */
export function dateOfWeekday(semesterStart: string | null, week: number, weekday: number): string | null {
  if (!semesterStart) return null;
  const start = Date.parse(`${semesterStart}T00:00:00`);
  if (Number.isNaN(start)) return null;
  const date = new Date(start + (week - 1) * 7 * 86_400_000 + (weekday - 1) * 86_400_000);
  return `${date.getMonth() + 1}/${date.getDate()}`;
}
