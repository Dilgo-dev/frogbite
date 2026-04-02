import { For } from "solid-js";
import type { KeyValue } from "../lib/types";

interface Props {
  rows: KeyValue[];
  onChange: (rows: KeyValue[]) => void;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
}

export default function KeyValueEditor(props: Props) {
  const update = (index: number, patch: Partial<KeyValue>) => {
    const next = props.rows.map((r, i) => (i === index ? { ...r, ...patch } : r));
    // Auto-add empty row at bottom
    const last = next[next.length - 1];
    if (last && (last.key !== "" || last.value !== "")) {
      next.push({ key: "", value: "", enabled: true });
    }
    props.onChange(next);
  };

  const remove = (index: number) => {
    if (props.rows.length <= 1) return;
    props.onChange(props.rows.filter((_, i) => i !== index));
  };

  return (
    <div class="divide-y divide-[var(--color-border-subtle)]">
      {/* Column headers */}
      <div class="grid grid-cols-[28px_1fr_1fr_28px] gap-0 px-3 py-1.5 text-[10px] font-medium uppercase tracking-wider text-[var(--color-fg-muted)]">
        <span />
        <span>{props.keyPlaceholder ?? "Key"}</span>
        <span>{props.valuePlaceholder ?? "Value"}</span>
        <span />
      </div>

      <For each={props.rows}>
        {(row, i) => (
          <div
            class="group grid grid-cols-[28px_1fr_1fr_28px] items-center gap-0 hover:bg-[var(--color-surface)]"
            classList={{ "opacity-40": !row.enabled }}
          >
            {/* Toggle */}
            <div class="flex items-center justify-center">
              <input
                type="checkbox"
                checked={row.enabled}
                onChange={(e) => update(i(), { enabled: e.target.checked })}
                class="h-3 w-3 rounded border-[var(--color-border)] accent-[var(--color-primary)]"
              />
            </div>

            {/* Key */}
            <input
              type="text"
              value={row.key}
              onInput={(e) => update(i(), { key: e.target.value })}
              placeholder={i() === props.rows.length - 1 ? props.keyPlaceholder ?? "Key" : ""}
              spellcheck={false}
              class="h-8 border-r border-[var(--color-border-subtle)] bg-transparent px-2 font-mono text-[12px] text-[var(--color-fg)] placeholder:text-[var(--color-fg-muted)]/40 focus:outline-none"
            />

            {/* Value */}
            <input
              type="text"
              value={row.value}
              onInput={(e) => update(i(), { value: e.target.value })}
              placeholder={i() === props.rows.length - 1 ? props.valuePlaceholder ?? "Value" : ""}
              spellcheck={false}
              class="h-8 bg-transparent px-2 font-mono text-[12px] text-[var(--color-fg-secondary)] placeholder:text-[var(--color-fg-muted)]/40 focus:outline-none"
            />

            {/* Delete */}
            <button
              onClick={() => remove(i())}
              class="btn-press flex h-8 w-7 items-center justify-center text-[var(--color-fg-muted)] opacity-0 group-hover:opacity-100 hover:text-[var(--color-error)]"
            >
              <svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        )}
      </For>
    </div>
  );
}
