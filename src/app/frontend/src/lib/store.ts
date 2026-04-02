import { createSignal } from "solid-js";
import type { RequestConfig, Folder, ResponseData } from "./types";

const defaultRequest: RequestConfig = {
  id: "req-1",
  name: "Get Users",
  method: "GET",
  url: "https://jsonplaceholder.typicode.com/users/1",
  headers: [
    { key: "Content-Type", value: "application/json", enabled: true },
    { key: "", value: "", enabled: true },
  ],
  params: [{ key: "", value: "", enabled: true }],
  body: "",
  folderId: "folder-1",
};

const defaultRequests: RequestConfig[] = [
  defaultRequest,
  {
    id: "req-2",
    name: "Create User",
    method: "POST",
    url: "https://jsonplaceholder.typicode.com/users",
    headers: [
      { key: "Content-Type", value: "application/json", enabled: true },
      { key: "", value: "", enabled: true },
    ],
    params: [{ key: "", value: "", enabled: true }],
    body: '{\n  "name": "Ada Lovelace",\n  "email": "ada@example.com"\n}',
    folderId: "folder-1",
  },
  {
    id: "req-3",
    name: "Update User",
    method: "PUT",
    url: "https://jsonplaceholder.typicode.com/users/1",
    headers: [
      { key: "Content-Type", value: "application/json", enabled: true },
      { key: "", value: "", enabled: true },
    ],
    params: [{ key: "", value: "", enabled: true }],
    body: '{\n  "name": "Grace Hopper"\n}',
    folderId: "folder-1",
  },
  {
    id: "req-4",
    name: "Delete User",
    method: "DELETE",
    url: "https://jsonplaceholder.typicode.com/users/1",
    headers: [{ key: "", value: "", enabled: true }],
    params: [{ key: "", value: "", enabled: true }],
    body: "",
    folderId: null,
  },
  {
    id: "req-5",
    name: "Health Check",
    method: "GET",
    url: "https://jsonplaceholder.typicode.com/posts/1",
    headers: [{ key: "", value: "", enabled: true }],
    params: [{ key: "", value: "", enabled: true }],
    body: "",
    folderId: "folder-2",
  },
];

const defaultFolders: Folder[] = [
  { id: "folder-1", name: "Users API", expanded: true },
  { id: "folder-2", name: "Monitoring", expanded: false },
];

export const [requests, setRequests] = createSignal<RequestConfig[]>(defaultRequests);
export const [folders, setFolders] = createSignal<Folder[]>(defaultFolders);
export const [activeRequestId, setActiveRequestId] = createSignal<string | null>("req-1");
export const [response, setResponse] = createSignal<ResponseData | null>(null);
export const [loading, setLoading] = createSignal(false);

export function activeRequest() {
  const id = activeRequestId();
  return requests().find((r) => r.id === id) ?? null;
}

export function updateActiveRequest(patch: Partial<RequestConfig>) {
  const id = activeRequestId();
  if (!id) return;
  setRequests((prev) => prev.map((r) => (r.id === id ? { ...r, ...patch } : r)));
}

export function toggleFolder(folderId: string) {
  setFolders((prev) =>
    prev.map((f) => (f.id === folderId ? { ...f, expanded: !f.expanded } : f))
  );
}
