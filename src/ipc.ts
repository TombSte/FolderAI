import { Channel, invoke } from "@tauri-apps/api/core";

export type RequestContext = {
  panelId: string;
  tabId: string;
  requestId: string;
};

export type Location = { path: string; label: string };

export type Directory = {
  location: Location;
  breadcrumbs: Location[];
  entries: {
    name: string;
    location: Location | null;
    kind: "directory" | "file" | "link" | "other";
    size: number | null;
    modifiedMs: number | null;
    createdMs?: number | null;
  }[];
  skipped: number;
};

export type BrowseError = {
  code: "invalid-request" | "not-found" | "permission-denied" | "not-directory" | "cancelled" | "unavailable";
  message: string;
};

type Target = { kind: "home" } | { kind: "path"; path: string };
type BrowseStarted = { kind: "started"; context: RequestContext };
type BrowseResponse = { context: RequestContext; directory: Directory };

function sameContext(first: RequestContext, second: RequestContext) {
  return first.panelId === second.panelId && first.tabId === second.tabId && first.requestId === second.requestId;
}

function isBrowseError(error: unknown): error is BrowseError {
  if (!error || typeof error !== "object" || !("code" in error) || !("message" in error)) {
    return false;
  }
  const codes = ["invalid-request", "not-found", "permission-denied", "not-directory", "cancelled", "unavailable"];
  return typeof error.message === "string" && typeof error.code === "string" && codes.includes(error.code);
}

const cancelled: BrowseError = { code: "cancelled", message: "Browsing was cancelled." };
const unavailable: BrowseError = { code: "unavailable", message: "The folder could not be read." };

export async function browse(target: Target, context: RequestContext, signal: AbortSignal, showHidden = false): Promise<Directory> {
  if (signal.aborted) throw cancelled;
  let started = false;
  const cancel = () => {
    if (started) void invoke("cancel_browse", { context }).catch(() => {});
  };
  const onStatus = new Channel<BrowseStarted>((event) => {
    if (event.kind === "started" && sameContext(event.context, context)) {
      started = true;
      if (signal.aborted) cancel();
    }
  });
  signal.addEventListener("abort", cancel, { once: true });

  try {
    const result = await invoke<BrowseResponse>("browse", { request: { target, context, ...(showHidden ? { showHidden } : {}) }, onStatus });
    if (signal.aborted) throw cancelled;
    if (!sameContext(result.context, context)) throw unavailable;
    return result.directory;
  } catch (error) {
    if (signal.aborted) throw cancelled;
    throw isBrowseError(error) ? error : unavailable;
  } finally {
    signal.removeEventListener("abort", cancel);
  }
}
