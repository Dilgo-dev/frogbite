import { For, Show, createMemo } from "solid-js";
import {
  requests,
  folders,
  activeRequestId,
  setActiveRequestId,
  toggleFolder,
} from "../lib/store";
import { METHOD_COLORS } from "../lib/types";
import type { HttpMethod } from "../lib/types";

function MethodBadge(props: { method: HttpMethod }) {
  return (
    <span
      class="inline-flex w-[42px] shrink-0 items-center justify-center rounded-[3px] px-1 py-[1px] text-[10px] font-bold uppercase tracking-wide"
      style={{
        color: METHOD_COLORS[props.method],
        background: METHOD_COLORS[props.method] + "15",
      }}
    >
      {props.method === "DELETE" ? "DEL" : props.method === "OPTIONS" ? "OPT" : props.method}
    </span>
  );
}

export default function Sidebar() {
  const ungrouped = createMemo(() => requests().filter((r) => !r.folderId));

  return (
    <aside class="flex h-full w-[220px] shrink-0 flex-col border-r border-[var(--color-border)]">
      {/* Header */}
      <div class="flex items-center justify-between px-4 py-3 border-b border-[var(--color-border)]">
        <span class="text-sm font-bold tracking-tight text-[var(--color-fg)]">
          frog<span class="text-[var(--color-primary)]">bite</span>
        </span>
        <button
          class="btn-press flex h-6 w-6 items-center justify-center rounded text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-raised)] hover:text-[var(--color-fg)]"
          title="New request"
        >
          <svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" d="M12 5v14m-7-7h14" />
          </svg>
        </button>
      </div>

      {/* Collection tree */}
      <div class="flex-1 overflow-y-auto py-2">
        <For each={folders()}>
          {(folder) => {
            const folderRequests = createMemo(() =>
              requests().filter((r) => r.folderId === folder.id)
            );
            return (
              <div>
                <button
                  onClick={() => toggleFolder(folder.id)}
                  class="btn-press flex w-full items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-[var(--color-fg-muted)] hover:text-[var(--color-fg)]"
                >
                  <svg
                    class="h-3 w-3 shrink-0 transition-transform"
                    classList={{ "rotate-90": folder.expanded }}
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    stroke-width="2.5"
                  >
                    <path stroke-linecap="round" d="M9 5l7 7-7 7" />
                  </svg>
                  <span class="truncate">{folder.name}</span>
                  <span class="ml-auto text-[10px] text-[var(--color-fg-muted)]">
                    {folderRequests().length}
                  </span>
                </button>

                <Show when={folder.expanded}>
                  <div class="ml-2">
                    <For each={folderRequests()}>
                      {(req) => (
                        <button
                          onClick={() => setActiveRequestId(req.id)}
                          class="btn-press flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left text-[13px]"
                          classList={{
                            "bg-[var(--color-surface-raised)] text-[var(--color-fg)]":
                              activeRequestId() === req.id,
                            "text-[var(--color-fg-secondary)] hover:bg-[var(--color-surface)]":
                              activeRequestId() !== req.id,
                          }}
                        >
                          <MethodBadge method={req.method} />
                          <span class="truncate">{req.name}</span>
                        </button>
                      )}
                    </For>
                  </div>
                </Show>
              </div>
            );
          }}
        </For>

        {/* Ungrouped requests */}
        <Show when={ungrouped().length > 0}>
          <div class="mt-1 border-t border-[var(--color-border-subtle)] pt-1">
            <For each={ungrouped()}>
              {(req) => (
                <button
                  onClick={() => setActiveRequestId(req.id)}
                  class="btn-press flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left text-[13px]"
                  classList={{
                    "bg-[var(--color-surface-raised)] text-[var(--color-fg)]":
                      activeRequestId() === req.id,
                    "text-[var(--color-fg-secondary)] hover:bg-[var(--color-surface)]":
                      activeRequestId() !== req.id,
                  }}
                >
                  <MethodBadge method={req.method} />
                  <span class="truncate">{req.name}</span>
                </button>
              )}
            </For>
          </div>
        </Show>
      </div>
    </aside>
  );
}
