import { backend, toTransportError } from "$lib/api";
import { communityNetwork } from "$lib/stores/community-network.svelte";
import type {
  CampusBus,
  CampusLocation,
  DiningCrowdedness,
  EmptyClassroom,
  LibraryOccupancy,
} from "$lib/types/app";

export type CampusResource<T> =
  | { phase: "idle" }
  | { phase: "loading" }
  | { phase: "unauthenticated" }
  | { phase: "enhancedAuth"; message: string }
  | { phase: "error"; message: string }
  | { phase: "ready"; data: T };

export const CAMPUS_OPTIONS: Array<{ value: CampusLocation; label: string }> = [
  { value: "handan", label: "邯郸" },
  { value: "fenglin", label: "枫林" },
  { value: "jiangwan", label: "江湾" },
  { value: "zhangjiang", label: "张江" },
];

export const CAMPUS_BUILDINGS: Record<CampusLocation, string[]> = {
  handan: ["HGD", "HGX", "H2", "H3", "H4", "H5", "H6"],
  fenglin: ["F1", "F2"],
  jiangwan: ["JA", "JB"],
  zhangjiang: ["Z2"],
};

const CAMPUS_STORAGE_KEY = "danxi.campus.selected";

function loadCampusPreference(): CampusLocation {
  try {
    const value = localStorage.getItem(CAMPUS_STORAGE_KEY);
    if (CAMPUS_OPTIONS.some((option) => option.value === value)) {
      return value as CampusLocation;
    }
  } catch {
    // Storage is unavailable during SSR or in restricted browser contexts.
  }
  return "handan";
}

function persistCampusPreference(campus: CampusLocation) {
  try {
    localStorage.setItem(CAMPUS_STORAGE_KEY, campus);
  } catch {
    // Best-effort persistence.
  }
}

const initialCampus = loadCampusPreference();

function today(): string {
  const date = new Date();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

class CampusServicesStore {
  campus = $state<CampusLocation>(initialCampus);
  building = $state(CAMPUS_BUILDINGS[initialCampus][0]);
  date = $state(today());
  library = $state<CampusResource<LibraryOccupancy[]>>({ phase: "idle" });
  dining = $state<CampusResource<DiningCrowdedness>>({ phase: "idle" });
  buses = $state<CampusResource<CampusBus[]>>({ phase: "idle" });
  classrooms = $state<CampusResource<EmptyClassroom[]>>({ phase: "idle" });
  #libraryRequest = 0;
  #diningRequest = 0;
  #busRequest = 0;
  #classroomRequest = 0;

  setCampus(campus: CampusLocation) {
    persistCampusPreference(campus);
    if (this.campus === campus) return;
    this.campus = campus;
    this.building = CAMPUS_BUILDINGS[campus][0];
    void this.loadDining(true);
    void this.loadClassrooms(true);
  }

  setBuilding(building: string) {
    if (this.building === building) return;
    this.building = building;
    void this.loadClassrooms(true);
  }

  setDate(date: string) {
    if (this.date === date) return;
    this.date = date;
    void this.loadClassrooms(true);
  }

  requireLogin() {
    this.#diningRequest += 1;
    this.#busRequest += 1;
    this.#classroomRequest += 1;
    this.dining = { phase: "unauthenticated" };
    this.buses = { phase: "unauthenticated" };
    this.classrooms = { phase: "unauthenticated" };
  }

  async loadAll(force = false) {
    await Promise.all([
      this.loadLibrary(force),
      this.loadDining(force),
      this.loadBuses(force),
      this.loadClassrooms(force),
    ]);
  }

  async loadLibrary(force = false) {
    if (!force && this.library.phase !== "idle" && this.library.phase !== "error") return;
    const request = ++this.#libraryRequest;
    this.library = { phase: "loading" };
    try {
      const data = await backend.loadLibraryOccupancy();
      if (request === this.#libraryRequest) this.library = { phase: "ready", data };
    } catch (raw) {
      if (request === this.#libraryRequest) this.library = errorState(raw);
    }
  }

  async loadDining(force = false) {
    if (
      !force &&
      this.dining.phase !== "idle" &&
      this.dining.phase !== "error" &&
      this.dining.phase !== "unauthenticated"
    ) return;
    const request = ++this.#diningRequest;
    const campus = this.campus;
    this.dining = { phase: "loading" };
    try {
      const data = await backend.loadDiningCrowdedness(campus);
      if (request === this.#diningRequest && campus === this.campus) {
        this.dining = { phase: "ready", data };
      }
    } catch (raw) {
      if (request === this.#diningRequest) this.dining = errorState(raw);
    }
  }

  async authenticateDining() {
    const request = ++this.#diningRequest;
    this.dining = { phase: "loading" };
    try {
      await backend.beginDiningEnhancedAuth();
      if (request !== this.#diningRequest) return;
      await this.loadDining(true);
    } catch (raw) {
      if (request !== this.#diningRequest) return;
      const error = toTransportError(raw);
      this.dining = error.kind === "auth" && error.message.includes("尚未登录")
        ? { phase: "unauthenticated" }
        : { phase: "enhancedAuth", message: error.message };
    }
  }

  async loadBuses(force = false) {
    if (
      !force &&
      this.buses.phase !== "idle" &&
      this.buses.phase !== "error" &&
      this.buses.phase !== "unauthenticated"
    ) return;
    const request = ++this.#busRequest;
    this.buses = { phase: "loading" };
    try {
      const day = new Date();
      const holiday = day.getDay() === 0 || day.getDay() === 6;
      const data = await backend.loadCampusBuses(holiday);
      if (request === this.#busRequest) this.buses = { phase: "ready", data };
    } catch (raw) {
      if (request === this.#busRequest) this.buses = errorState(raw);
    }
  }

  async loadClassrooms(force = false) {
    if (
      !force &&
      this.classrooms.phase !== "idle" &&
      this.classrooms.phase !== "error" &&
      this.classrooms.phase !== "unauthenticated"
    ) return;
    const request = ++this.#classroomRequest;
    const building = this.building;
    const date = this.date;
    this.classrooms = { phase: "loading" };
    try {
      const data = await backend.loadEmptyClassrooms(building, date, {
        useWebvpn: communityNetwork.useWebvpn,
      });
      if (request === this.#classroomRequest && building === this.building && date === this.date) {
        this.classrooms = { phase: "ready", data };
      }
    } catch (raw) {
      if (request === this.#classroomRequest) this.classrooms = errorState(raw);
    }
  }
}

function errorState<T>(raw: unknown): CampusResource<T> {
  const error = toTransportError(raw);
  if (error.kind === "auth") return { phase: "unauthenticated" };
  if (error.kind === "enhancedAuth") {
    return { phase: "enhancedAuth", message: error.message };
  }
  return { phase: "error", message: error.message };
}

export const campusServices = new CampusServicesStore();
