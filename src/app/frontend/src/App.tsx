import { Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import Sidebar from "./components/Sidebar";
import UrlBar from "./components/UrlBar";
import RequestPanel from "./components/RequestPanel";
import ResponsePanel from "./components/ResponsePanel";
import EmptyState from "./components/EmptyState";
import { activeRequest, setResponse, setLoading, loading } from "./lib/store";
import type { ResponseData } from "./lib/types";

export default function App() {
  const sendRequest = async () => {
    const req = activeRequest();
    if (!req || loading()) return;

    setLoading(true);
    setResponse(null);

    const headers: Record<string, string> = {};
    for (const h of req.headers) {
      if (h.enabled && h.key) headers[h.key] = h.value;
    }

    // Build URL with params
    let url = req.url;
    const enabledParams = req.params.filter((p) => p.enabled && p.key);
    if (enabledParams.length > 0) {
      const searchParams = new URLSearchParams();
      for (const p of enabledParams) searchParams.append(p.key, p.value);
      const separator = url.includes("?") ? "&" : "?";
      url = url + separator + searchParams.toString();
    }

    try {
      const res = await invoke<ResponseData>("send_request", {
        opts: {
          method: req.method,
          url,
          headers,
          body: req.body || null,
        },
      });
      setResponse(res);
    } catch (e: any) {
      setResponse({
        status: 0,
        status_text: "",
        headers: {},
        body: "",
        duration_ms: 0,
        error: e.toString(),
      });
    }

    setLoading(false);
  };

  return (
    <div class="grid h-screen grid-cols-[220px_1fr] bg-[var(--color-bg)]">
      {/* Sidebar */}
      <Sidebar />

      {/* Main area */}
      <Show
        when={activeRequest()}
        fallback={<EmptyState />}
      >
        <div class="grid grid-rows-[auto_1fr_1fr] overflow-hidden">
          {/* URL bar */}
          <UrlBar onSend={sendRequest} />

          {/* Request config */}
          <RequestPanel />

          {/* Response */}
          <ResponsePanel />
        </div>
      </Show>
    </div>
  );
}
