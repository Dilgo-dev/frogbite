export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";

export interface KeyValue {
  key: string;
  value: string;
  enabled: boolean;
}

export interface RequestConfig {
  id: string;
  name: string;
  method: HttpMethod;
  url: string;
  headers: KeyValue[];
  params: KeyValue[];
  body: string;
  folderId: string | null;
}

export interface Folder {
  id: string;
  name: string;
  expanded: boolean;
}

export interface ResponseData {
  status: number;
  status_text: string;
  headers: Record<string, string>;
  body: string;
  duration_ms: number;
  error: string | null;
}

export const METHOD_COLORS: Record<HttpMethod, string> = {
  GET: "var(--color-primary)",
  POST: "var(--color-secondary)",
  PUT: "var(--color-accent)",
  PATCH: "var(--color-warning)",
  DELETE: "var(--color-error)",
  HEAD: "var(--color-info)",
  OPTIONS: "var(--color-fg-muted)",
};
