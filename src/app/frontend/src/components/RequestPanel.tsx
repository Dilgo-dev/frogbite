import { createSignal, Show } from "solid-js";
import { activeRequest, updateActiveRequest } from "../lib/store";
import KeyValueEditor from "./KeyValueEditor";

type Tab = "params" | "headers" | "body" | "auth";

export default function RequestPanel() {
  const [tab, setTab] = createSignal<Tab>("params");
  const req = activeRequest;

  const tabs: { id: Tab; label: string }[] = [
    { id: "params", label: "Params" },
    { id: "headers", label: "Headers" },
    { id: "body", label: "Body" },
    { id: "auth", label: "Auth" },
  ];

  const headerCount = () => req()?.headers.filter((h) => h.key && h.enabled).length ?? 0;
  const paramCount = () => req()?.params.filter((p) => p.key && p.enabled).length ?? 0;

  return (
    <div class="flex flex-1 flex-col overflow-hidden">
      {/* Tabs */}
      <div class="flex items-center gap-0 border-b border-[var(--color-border)] px-4">
        {tabs.map((t) => {
          const count = () =>
            t.id === "headers" ? headerCount() : t.id === "params" ? paramCount() : 0;

          return (
            <button
              onClick={() => setTab(t.id)}
              class="btn-press relative flex items-center gap-1.5 px-3 py-2.5 text-[12px] font-medium"
              classList={{
                "text-[var(--color-fg)]": tab() === t.id,
                "text-[var(--color-fg-muted)] hover:text-[var(--color-fg-secondary)]": tab() !== t.id,
              }}
            >
              {t.label}
              <Show when={count() > 0}>
                <span class="flex h-4 min-w-4 items-center justify-center rounded-full bg-[var(--color-primary-dim)] px-1 text-[9px] font-bold text-[var(--color-primary)]">
                  {count()}
                </span>
              </Show>
              {/* Active indicator */}
              <Show when={tab() === t.id}>
                <div class="absolute bottom-0 left-3 right-3 h-[2px] rounded-full bg-[var(--color-primary)]" />
              </Show>
            </button>
          );
        })}
      </div>

      {/* Tab content */}
      <div class="flex-1 overflow-y-auto">
        <Show when={tab() === "params"}>
          <KeyValueEditor
            rows={req()?.params ?? []}
            onChange={(params) => updateActiveRequest({ params })}
            keyPlaceholder="Parameter"
            valuePlaceholder="Value"
          />
        </Show>

        <Show when={tab() === "headers"}>
          <KeyValueEditor
            rows={req()?.headers ?? []}
            onChange={(headers) => updateActiveRequest({ headers })}
            keyPlaceholder="Header"
            valuePlaceholder="Value"
          />
        </Show>

        <Show when={tab() === "body"}>
          <div class="p-3">
            <textarea
              value={req()?.body ?? ""}
              onInput={(e) => updateActiveRequest({ body: e.target.value })}
              placeholder="Request body (JSON, text, etc.)"
              spellcheck={false}
              class="h-full min-h-[200px] w-full resize-none rounded-md border border-[var(--color-border)] bg-[var(--color-bg-deep)] p-3 font-mono text-[12px] leading-relaxed text-[var(--color-fg)] placeholder:text-[var(--color-fg-muted)]/40 focus:outline-none focus:border-[var(--color-primary)]"
            />
          </div>
        </Show>

        <Show when={tab() === "auth"}>
          <div class="flex flex-col items-center justify-center py-16 text-center">
            <div class="mb-3 flex h-10 w-10 items-center justify-center rounded-lg bg-[var(--color-surface)]">
              <svg class="h-5 w-5 text-[var(--color-fg-muted)]" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                <path stroke-linecap="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
              </svg>
            </div>
            <p class="text-[13px] text-[var(--color-fg-muted)]">No authentication configured</p>
            <p class="mt-1 text-[11px] text-[var(--color-fg-muted)]/60">Use headers for Bearer tokens or API keys</p>
          </div>
        </Show>
      </div>
    </div>
  );
}
