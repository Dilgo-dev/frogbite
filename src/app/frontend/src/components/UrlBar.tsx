import { Show } from "solid-js";
import { activeRequest, updateActiveRequest, loading } from "../lib/store";
import { METHOD_COLORS } from "../lib/types";
import type { HttpMethod } from "../lib/types";

const METHODS: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

export default function UrlBar(props: { onSend: () => void }) {
  const req = activeRequest;

  return (
    <div class="flex items-center gap-2 border-b border-[var(--color-border)] px-4 py-2.5">
      {/* Method selector */}
      <div class="relative">
        <select
          value={req()?.method ?? "GET"}
          onChange={(e) => updateActiveRequest({ method: e.target.value as HttpMethod })}
          class="btn-press h-8 appearance-none rounded-md border border-[var(--color-border)] bg-[var(--color-surface)] pl-2.5 pr-7 text-xs font-bold uppercase tracking-wide focus:outline-none focus:border-[var(--color-primary)]"
          style={{ color: METHOD_COLORS[req()?.method ?? "GET"] }}
        >
          {METHODS.map((m) => (
            <option value={m}>{m}</option>
          ))}
        </select>
        <svg
          class="pointer-events-none absolute right-2 top-1/2 h-3 w-3 -translate-y-1/2 text-[var(--color-fg-muted)]"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          stroke-width="2.5"
        >
          <path stroke-linecap="round" d="M6 9l6 6 6-6" />
        </svg>
      </div>

      {/* URL input */}
      <input
        type="text"
        value={req()?.url ?? ""}
        onInput={(e) => updateActiveRequest({ url: e.target.value })}
        onKeyDown={(e) => e.key === "Enter" && props.onSend()}
        placeholder="Enter request URL..."
        spellcheck={false}
        class="h-8 flex-1 rounded-md border border-[var(--color-border)] bg-[var(--color-bg-deep)] px-3 font-mono text-[13px] text-[var(--color-fg)] placeholder:text-[var(--color-fg-muted)] focus:outline-none focus:border-[var(--color-primary)]"
      />

      {/* Send button */}
      <button
        onClick={props.onSend}
        disabled={loading()}
        class="btn-press h-8 rounded-md px-5 text-xs font-bold tracking-wide text-[var(--color-bg-deep)] disabled:opacity-40"
        style={{ background: METHOD_COLORS[req()?.method ?? "GET"] }}
      >
        <Show when={!loading()} fallback={
          <div class="flex items-center gap-1.5">
            <div class="h-3 w-3 animate-spin rounded-full border-2 border-current border-t-transparent" />
            <span>Sending</span>
          </div>
        }>
          Send
        </Show>
      </button>
    </div>
  );
}
