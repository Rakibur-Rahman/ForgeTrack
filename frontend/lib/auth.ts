"use client";
import { create } from "zustand";
type AuthState = { token?: string; setToken: (token?: string) => void };
export const useAuth = create<AuthState>((set) => ({ token: typeof window === "undefined" ? undefined : localStorage.getItem("token") ?? undefined, setToken: (token) => { if (token) { localStorage.setItem("token", token); document.cookie = `forgetrack_token=${encodeURIComponent(token)}; Path=/; SameSite=Lax`; } else { localStorage.removeItem("token"); document.cookie = "forgetrack_token=; Path=/; Max-Age=0; SameSite=Lax"; } set({ token }); } }));
