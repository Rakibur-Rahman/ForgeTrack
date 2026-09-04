import { useAuth } from "./auth";

export const apiUrl = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:3001";
type TokenPair = { access_token: string; refresh_token: string };

function request(path: string, token?: string, init?: RequestInit) {
  return fetch(`${apiUrl}${path}`, { ...init, headers: { "content-type": "application/json", ...(token ? { authorization: `Bearer ${token}` } : {}), ...init?.headers } });
}

export async function api<T>(path: string, token?: string, init?: RequestInit): Promise<T> {
  let response = await request(path, token, init);
  if (response.status === 401 && token && typeof window !== "undefined") {
    const refreshToken = localStorage.getItem("refresh_token");
    if (refreshToken) {
      const refreshed = await request("/auth/refresh", undefined, { method: "POST", body: JSON.stringify({ refresh_token: refreshToken }) });
      if (refreshed.ok) {
        const session = await refreshed.json() as TokenPair;
        useAuth.getState().setSession(session);
        response = await request(path, session.access_token, init);
      }
    }
  }
  if (!response.ok) { const body = await response.json().catch(() => ({})); throw new Error(body.error ?? "Request failed"); }
  return response.status === 204 ? undefined as T : response.json();
}
