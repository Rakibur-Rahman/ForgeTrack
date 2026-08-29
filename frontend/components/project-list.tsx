"use client";

import Link from "next/link";
import { FormEvent, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { useAuth } from "../lib/auth";

type Organization = { id: string; name: string; slug: string };
type Project = { id: string; organization_id: string; name: string; key: string; description?: string };

export default function ProjectList({ initialProjects }: { initialProjects?: Project[] }) {
  const token = useAuth((state) => state.token);
  const client = useQueryClient();
  const [error, setError] = useState("");
  const organizations = useQuery({ queryKey: ["organizations", token], queryFn: () => api<Organization[]>("/organizations", token), enabled: !!token });
  const projects = useQuery({ queryKey: ["projects", token], queryFn: () => api<Project[]>("/projects", token), enabled: !!token, initialData: initialProjects });

  if (!token) return <p>Please <Link href="/login">log in</Link> to view projects.</p>;

  async function createOrganization(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    try {
      await api("/organizations", token, { method: "POST", body: JSON.stringify({ name: form.get("name"), slug: form.get("slug") }) });
      event.currentTarget.reset();
      client.invalidateQueries({ queryKey: ["organizations", token] });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Unable to create organization");
    }
  }

  async function createProject(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    try {
      await api("/projects", token, { method: "POST", body: JSON.stringify({ organization_id: form.get("organization_id"), name: form.get("name"), key: form.get("key"), description: form.get("description") || undefined }) });
      event.currentTarget.reset();
      client.invalidateQueries({ queryKey: ["projects", token] });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Unable to create project");
    }
  }

  return <>
    <h1>Projects</h1>
    {projects.data?.map((project) => <Link className="card" href={`/projects/${project.id}`} key={project.id}><strong>{project.key} · {project.name}</strong><div className="muted">{project.description}</div></Link>)}
    {projects.isLoading && <p>Loading projects…</p>}

    <h2>New organization</h2>
    <form onSubmit={createOrganization}>
      <input name="name" placeholder="Organization name" required maxLength={100} />
      <input name="slug" placeholder="URL-friendly slug, e.g. acme" required minLength={2} maxLength={80} pattern="[a-z0-9-]+" />
      <button>Create organization</button>
    </form>

    <h2>New project</h2>
    {organizations.data?.length === 0 ? <p>Create an organization above before creating a project.</p> : <form onSubmit={createProject}>
      <select name="organization_id" required defaultValue=""><option value="" disabled>Organization</option>{organizations.data?.map((organization) => <option value={organization.id} key={organization.id}>{organization.name}</option>)}</select>
      <input name="name" placeholder="Project name" required />
      <input name="key" placeholder="Key, e.g. FORGE" required minLength={2} maxLength={20} />
      <textarea name="description" placeholder="Description" />
      <button>Create project</button>
    </form>}
    {error && <p className="error">{error}</p>}
  </>;
}
