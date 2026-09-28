const USE_WEBVPN_KEY = "danxi.community.use-webvpn";

function loadUseWebvpn(): boolean {
  try {
    return localStorage.getItem(USE_WEBVPN_KEY) !== "0";
  } catch {
    return true;
  }
}

class CommunityNetworkStore {
  useWebvpn = $state(loadUseWebvpn());

  setUseWebvpn(value: boolean) {
    this.useWebvpn = value;
    try {
      localStorage.setItem(USE_WEBVPN_KEY, value ? "1" : "0");
    } catch {
      // Persistence is best-effort; the default remains enabled.
    }
  }
}

export const communityNetwork = new CommunityNetworkStore();
