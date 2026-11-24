import { invoke } from "@tauri-apps/api/core";
import type { AppFolder } from "./bindings/AppFolder";
import type { AuthStatus } from "./bindings/AuthStatus";
import type { Bootstrap } from "./bindings/Bootstrap";
import type { DeploymentInfo } from "./bindings/DeploymentInfo";
import type { DeploymentOrigin } from "./bindings/DeploymentOrigin";
import type { Diagnostics } from "./bindings/Diagnostics";
import type { DiscoverFeed } from "./bindings/DiscoverFeed";
import type { ExperienceDetail } from "./bindings/ExperienceDetail";
import type { FlagInput } from "./bindings/FlagInput";
import type { FlagOverview } from "./bindings/FlagOverview";
import type { InstalledDeployment } from "./bindings/InstalledDeployment";
import type { LaunchTarget } from "./bindings/LaunchTarget";
import type { LibraryOverview } from "./bindings/LibraryOverview";
import type { NetworkProbe } from "./bindings/NetworkProbe";
import type { PeopleOverview } from "./bindings/PeopleOverview";
import type { PerfPoint } from "./bindings/PerfPoint";
import type { PlaySession } from "./bindings/PlaySession";
import type { RecentExperience } from "./bindings/RecentExperience";
import type { RobloxInstall } from "./bindings/RobloxInstall";
import type { SearchPage } from "./bindings/SearchPage";
import type { Settings } from "./bindings/Settings";

export type SettingsPatch = {
  [Section in keyof Settings]?: Partial<Settings[Section]>;
};

export class IpcFailure extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "IpcFailure";
  }
}

function toFailure(error: unknown): IpcFailure {
  if (typeof error === "object" && error !== null && "code" in error && "message" in error) {
    return new IpcFailure(String(error.code), String(error.message));
  }
  return new IpcFailure("unknown", "Something went wrong in the app's backend. Check the logs for details.");
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toFailure(error);
  }
}

export const api = {
  bootstrap: () => call<Bootstrap>("get_bootstrap"),
  appReady: () => call<void>("app_ready"),
  updateSettings: (patch: SettingsPatch) => call<Settings>("update_settings", { patch }),
  resetSettings: () => call<Settings>("reset_settings"),
  diagnostics: () => call<Diagnostics>("get_diagnostics"),
  openFolder: (folder: AppFolder) => call<void>("open_app_folder", { folder }),
  listDeployments: () => call<InstalledDeployment[]>("list_deployments"),
  validateDeployment: (version: string) => call<DeploymentInfo>("validate_deployment", { version }),
  installDeployment: (version: string) => call<InstalledDeployment>("install_deployment", { version }),
  cancelDeploymentInstall: () => call<void>("cancel_deployment_install"),
  removeDeployment: (version: string) => call<void>("remove_deployment", { version }),
  launchDeployment: (version: string, origin: DeploymentOrigin) =>
    call<void>("launch_deployment", { version, origin }),
  fflags: () => call<FlagOverview>("get_fflags"),
  setActiveFlagProfile: (id: string) => call<FlagOverview>("set_active_fflag_profile", { id }),
  createFlagProfile: (name: string, copyFrom: string | null) =>
    call<FlagOverview>("create_fflag_profile", { name, copyFrom }),
  renameFlagProfile: (id: string, name: string) => call<FlagOverview>("rename_fflag_profile", { id, name }),
  deleteFlagProfile: (id: string) => call<FlagOverview>("delete_fflag_profile", { id }),
  saveFlag: (profileId: string, original: string | null, flag: FlagInput) =>
    call<FlagOverview>("save_fflag", { profileId, original, flag }),
  setFlagEnabled: (profileId: string, name: string, enabled: boolean) =>
    call<FlagOverview>("set_fflag_enabled", { profileId, name, enabled }),
  removeFlag: (profileId: string, name: string) => call<FlagOverview>("remove_fflag", { profileId, name }),
  importFlags: (profileId: string, text: string, replace: boolean) =>
    call<FlagOverview>("import_fflags", { profileId, text, replace }),
  flagsJson: (profileId: string) => call<string>("get_fflags_json", { profileId }),
  exportFlagsFile: (profileId: string) => call<string>("export_fflags_file", { profileId }),
  resetFlags: () => call<FlagOverview>("reset_fflags"),
  restoreFlagBackup: (id: number) => call<FlagOverview>("restore_fflag_backup", { id }),
  library: () => call<LibraryOverview>("get_library"),
  setGameFavorite: (target: LaunchTarget, favorite: boolean) =>
    call<LibraryOverview>("set_game_favorite", { target, favorite }),
  setGamePinned: (target: LaunchTarget, pinned: boolean) => call<LibraryOverview>("set_game_pinned", { target, pinned }),
  createCollection: (name: string) => call<LibraryOverview>("create_collection", { name }),
  renameCollection: (id: string, name: string) => call<LibraryOverview>("rename_collection", { id, name }),
  deleteCollection: (id: string) => call<LibraryOverview>("delete_collection", { id }),
  setGameInCollection: (id: string, target: LaunchTarget, included: boolean) =>
    call<LibraryOverview>("set_game_in_collection", { id, target, included }),
  clearLibrary: () => call<LibraryOverview>("clear_library"),
  people: () => call<PeopleOverview>("get_people"),
  addLocalProfile: (username: string) => call<PeopleOverview>("add_local_profile", { username }),
  removeLocalProfile: (userId: number) => call<PeopleOverview>("remove_local_profile", { userId }),
  setActiveLocalProfile: (userId: number) => call<PeopleOverview>("set_active_local_profile", { userId }),
  addPerson: (username: string) => call<PeopleOverview>("add_person", { username }),
  removePerson: (userId: number) => call<PeopleOverview>("remove_person", { userId }),
  clearPeople: () => call<void>("clear_people"),
  openUserProfile: (userId: number) => call<void>("open_user_profile", { userId }),
  performanceHistory: () => call<PerfPoint[]>("get_performance_history"),
  samplePerformance: () => call<PerfPoint | null>("sample_performance"),
  checkNetwork: () => call<NetworkProbe[]>("check_network"),
  discoverFeed: (refresh = false) => call<DiscoverFeed>("get_discover_feed", { refresh }),
  searchExperiences: (query: string, pageToken: string | null = null) =>
    call<SearchPage>("search_experiences", { query, pageToken }),
  experience: (universeId: number) => call<ExperienceDetail>("get_experience", { universeId }),
  robloxInstall: () => call<RobloxInstall>("get_roblox_install"),
  launchExperience: (target: LaunchTarget, force = false) =>
    call<PlaySession>("launch_experience", { target, force }),
  playSession: () => call<PlaySession | null>("get_play_session"),
  recentExperiences: () => call<RecentExperience[]>("list_recent_experiences"),
  syncPlayHistory: () => call<RecentExperience[]>("sync_play_history"),
  clearRecentExperiences: () => call<void>("clear_recent_experiences"),
  backgroundAnimation: () => call<boolean>("get_background_animation"),
  authStatus: () => call<AuthStatus>("get_auth_status"),
  setOAuthClientId: (clientId: string) => call<AuthStatus>("set_oauth_client_id", { clientId }),
  startSignIn: () => call<AuthStatus>("start_sign_in"),
  cancelSignIn: () => call<AuthStatus>("cancel_sign_in"),
  signOut: () => call<AuthStatus>("sign_out"),
  dismissWelcome: () => call<AuthStatus>("dismiss_welcome"),
  openOAuthDashboard: () => call<void>("open_oauth_dashboard"),
};

export function failureMessage(error: unknown, fallback: string): string {
  return error instanceof IpcFailure ? error.message : fallback;
}
