"use client";
import { create } from "zustand";

export type Session = { access_token: string; refresh_token: string };
type AuthState = { token?: string; refreshToken?: string; setSession: (session?: Session) => void };

export const useAuth = create<AuthState>((set) => ({
  token: typeof window === "undefined" ? undefined : localStorage.getItem("token") ?? undefined,
  refreshToken: typeof window === "undefined" ? undefined : localStorage.getItem("refresh_token") ?? undefined,
  setSession: (session) => {
    if (session) {
      localStorage.setItem("token", session.access_token);
      localStorage.setItem("refresh_token", session.refresh_token);
      document.cookie = `forgetrack_token=${encodeURIComponent(session.access_token)}; Path=/; SameSite=Lax`;
    } else {
      localStorage.removeItem("token");
      localStorage.removeItem("refresh_token");
      document.cookie = "forgetrack_token=; Path=/; Max-Age=0; SameSite=Lax";
    }
    set({ token: session?.access_token, refreshToken: session?.refresh_token });
  },
}));
