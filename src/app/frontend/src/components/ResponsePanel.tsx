import { createSignal, Show } from "solid-js";
import { response, loading } from "../lib/store";

type Tab = "body" | "headers";

function StatusBadge(props: { status: number; text: string }) {
  const color = () => {
    const s = props.status;
    if (s >= 200 && s < 300) return { bg: "var(--color-primary-dim)", fg: "var(--color-primary)" };
    if (s >= 300 && s < 400) return { bg: "var(--color-info-dim)", fg: "var(--color-info)" };
    if (s >= 400 && s < 500) return { bg: "var(--color-warning-dim)", fg: "var(--color-warning)" };
    return { bg: "var(--color-error-dim)", fg: "var(--color-error)" };
  };

  return (
    <span
      class="inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-[12px] font-bold tracking-wide"
      style={{ background: color().bg, color: color().fg }}
    >
      <span class="h-1.5 w-1.5 rounded-full" style={{ background: color().fg }} />
      {props.status} {props.text.replace(`${props.status} `, "")}
    </span>
  );
}

function formatBody(body: string): string {
  try {
    return JSON.stringify(JSON.parse(body), null, 2);
  } catch {
    return body;
  }
}

export default function ResponsePanel() {
  const [tab, setTab] = createSignal<Tab>("body");
  const res = response;

  const headerCount = () => Object.keys(res()?.headers ?? {}).length;

  return (
    <div class="flex flex-1 flex-col overflow-hidden border-t border-[var(--color-border)]">
      {/* Header bar */}
      <div class="flex items-center justify-between border-b border-[var(--color-border)] px-4 py-2">
        <div class="flex items-center gap-3">
          <span class="text-[11px] font-medium uppercase tracking-wider text-[var(--color-fg-muted)]">
            Response
          </span>

          <Show when={res() && !res()!.error}>
            <StatusBadge status={res()!.status} text={res()!.status_text} />
          </Show>

          <Show when={res()?.error}>
            <span class="text-[12px] font-medium text-[var(--color-error)]">Error</span>
          </Show>
        </div>

        <Show when={res() && !res()!.error}>
          <span class="font-mono text-[11px] text-[var(--color-fg-muted)]">
            {res()!.duration_ms}ms
          </span>
        </Show>
      </div>

      {/* Loading state */}
      <Show when={loading()}>
        <div class="flex-1 p-4">
          <div class="skeleton mb-3 h-4 w-24" />
          <div class="skeleton mb-2 h-3 w-full" />
          <div class="skeleton mb-2 h-3 w-4/5" />
          <div class="skeleton mb-2 h-3 w-3/5" />
          <div class="skeleton mb-2 h-3 w-full" />
          <div class="skeleton h-3 w-2/5" />
        </div>
      </Show>

      {/* Empty state */}
      <Show when={!res() && !loading()}>
        <div class="flex flex-1 flex-col items-center justify-center">
          <div class="mb-3 flex h-12 w-12 items-center justify-center rounded-xl bg-[var(--color-surface)]">
            <svg class="h-6 w-6 text-[var(--color-fg-muted)]/50" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path stroke-linecap="round" d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
            </svg>
          </div>
          <p class="text-[13px] text-[var(--color-fg-muted)]">Send a request to see the response</p>
          <p class="mt-1 text-[11px] text-[var(--color-fg-muted)]/50">Press Enter in the URL bar or click Send</p>
        </div>
      </Show>

      {/* Response content */}
      <Show when={res() && !loading()}>
        {/* Tabs */}
        <div class="flex items-center gap-0 border-b border-[var(--color-border)] px-4">
          <button
            onClick={() => setTab("body")}
            class="btn-press relative px-3 py-2 text-[12px] font-medium"
            classList={{
              "text-[var(--color-fg)]": tab() === "body",
              "text-[var(--color-fg-muted)]": tab() !== "body",
            }}
          >
            Body
            <Show when={tab() === "body"}>
              <div class="absolute bottom-0 left-3 right-3 h-[2px] rounded-full bg-[var(--color-primary)]" />
            </Show>
          </button>
          <button
            onClick={() => setTab("headers")}
            class="btn-press relative flex items-center gap-1.5 px-3 py-2 text-[12px] font-medium"
            classList={{
              "text-[var(--color-fg)]": tab() === "headers",
              "text-[var(--color-fg-muted)]": tab() !== "headers",
            }}
          >
            Headers
            <span class="flex h-4 min-w-4 items-center justify-center rounded-full bg-[var(--color-surface)] px-1 text-[9px] font-bold text-[var(--color-fg-muted)]">
              {headerCount()}
            </span>
            <Show when={tab() === "headers"}>
              <div class="absolute bottom-0 left-3 right-3 h-[2px] rounded-full bg-[var(--color-primary)]" />
            </Show>
          </button>
        </div>

        {/* Body */}
        <Show when={tab() === "body"}>
          <div class="flex-1 overflow-auto p-4">
            <Show when={res()?.error}>
              <p class="font-mono text-[12px] text-[var(--color-error)]">{res()!.error}</p>
            </Show>
            <Show when={!res()?.error}>
              <pre class="whitespace-pre-wrap font-mono text-[12px] leading-relaxed text-[var(--color-fg-secondary)]">
                {formatBody(res()!.body)}
              </pre>
            </Show>
          </div>
        </Show>

        {/* Headers */}
        <Show when={tab() === "headers"}>
          <div class="flex-1 overflow-auto">
            <div class="divide-y divide-[var(--color-border-subtle)]">
              {Object.entries(res()?.headers ?? {}).map(([key, value]) => (
                <div class="grid grid-cols-[200px_1fr] gap-0 px-4 py-2 hover:bg-[var(--color-surface)]">
                  <span class="font-mono text-[12px] font-medium text-[var(--color-fg-muted)]">{key}</span>
                  <span class="font-mono text-[12px] text-[var(--color-fg-secondary)]">{value}</span>
                </div>
              ))}
            </div>
          </div>
        </Show>
      </Show>
    </div>
  );
}
