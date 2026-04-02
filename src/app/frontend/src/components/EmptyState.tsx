export default function EmptyState() {
  return (
    <div class="flex h-full flex-col items-center justify-center">
      <div class="mb-5 flex h-16 w-16 items-center justify-center rounded-2xl bg-[var(--color-surface)]">
        <svg class="h-8 w-8 text-[var(--color-fg-muted)]/30" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
          <path stroke-linecap="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
        </svg>
      </div>
      <p class="mb-1 text-[15px] font-medium text-[var(--color-fg-muted)]">No request selected</p>
      <p class="max-w-[240px] text-center text-[12px] leading-relaxed text-[var(--color-fg-muted)]/50">
        Select a request from the sidebar or create a new one to get started.
      </p>
    </div>
  );
}
