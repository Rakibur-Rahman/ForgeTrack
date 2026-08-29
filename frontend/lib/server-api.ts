import { cookies } from "next/headers";
import { apiUrl } from "./api";

export type ServerProject = { id: string; organization_id: string; name: string; key: string; description?: string };

export async function serverProjects(): Promise<ServerProject[] | undefined> {
  const token = (await cookies()).get("forgetrack_token")?.value;
  if (!token) return undefined;
  const response = await fetch(`${apiUrl}/projects`, { headers: { authorization: `Bearer ${token}` }, cache: "no-store" });
  return response.ok ? response.json() : undefined;
}
