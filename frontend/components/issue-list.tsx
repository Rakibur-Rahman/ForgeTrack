"use client";
import Link from "next/link";
import { FormEvent, useState } from "react";
import { useRouter } from "next/navigation";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { useAuth } from "../lib/auth";

type Project={id:string;organization_id:string;name:string;key:string;description?:string};
type Issue={id:string;title:string;status:string;priority:string};
type Member={user_id:string;name:string;email:string;role:string};
type Label={id:string;name:string;color:string};

export default function IssueList({projectId}:{projectId:string}) {
 const token=useAuth(s=>s.token),client=useQueryClient(),router=useRouter(); const [error,setError]=useState("");
 const project=useQuery({queryKey:["project",projectId,token],queryFn:()=>api<Project>(`/projects/${projectId}`,token),enabled:!!token});
 const issues=useQuery({queryKey:["issues",projectId,token],queryFn:()=>api<Issue[]>(`/projects/${projectId}/issues`,token),enabled:!!token});
 const members=useQuery({queryKey:["project-members",projectId,token],queryFn:()=>api<Member[]>(`/projects/${projectId}/members`,token),enabled:!!token});
 const labels=useQuery({queryKey:["labels",projectId,token],queryFn:()=>api<Label[]>(`/projects/${projectId}/labels`,token),enabled:!!token});
 if(!token)return <p>Please <Link href="/login">log in</Link>.</p>;
 async function perform(action:()=>Promise<unknown>,keys:(string|undefined)[][],form?:HTMLFormElement){try{setError("");await action();form?.reset();keys.forEach(queryKey=>client.invalidateQueries({queryKey}));}catch(cause){setError(cause instanceof Error?cause.message:"Request failed");}}
 function createIssue(e:FormEvent<HTMLFormElement>){e.preventDefault();const form=e.currentTarget,f=new FormData(form);void perform(()=>api(`/projects/${projectId}/issues`,token,{method:"POST",body:JSON.stringify({title:f.get("title"),description:f.get("description"),priority:f.get("priority"),assignee_id:f.get("assignee_id")||undefined})}),[["issues",projectId,token]],form);}
 function addMember(e:FormEvent<HTMLFormElement>){e.preventDefault();const form=e.currentTarget,f=new FormData(form);void perform(()=>api(`/projects/${projectId}/members`,token,{method:"POST",body:JSON.stringify({email:f.get("email"),role:f.get("role")})}),[["project-members",projectId,token]],form);}
 function createLabel(e:FormEvent<HTMLFormElement>){e.preventDefault();const form=e.currentTarget,f=new FormData(form);void perform(()=>api(`/projects/${projectId}/labels`,token,{method:"POST",body:JSON.stringify({name:f.get("name"),color:f.get("color")})}),[["labels",projectId,token]],form);}
 function updateProject(e:FormEvent<HTMLFormElement>){e.preventDefault();const f=new FormData(e.currentTarget);void perform(()=>api(`/projects/${projectId}`,token,{method:"PATCH",body:JSON.stringify({name:f.get("name"),description:f.get("description")})}),[["project",projectId,token],["projects",token]]);}
 async function deleteProject(){if(!confirm("Delete this project and all its issues?"))return;await perform(()=>api(`/projects/${projectId}`,token,{method:"DELETE"}),[["projects",token]]);router.push("/projects");}
 return <><Link href="/projects">← Projects</Link><h1>{project.data?`${project.data.key} · ${project.data.name}`:"Project"}</h1>
 {project.data&&<details><summary>Project settings</summary><form onSubmit={updateProject}><input name="name" defaultValue={project.data.name} required/><textarea name="description" defaultValue={project.data.description}/><button>Save project</button><button className="danger" type="button" onClick={deleteProject}>Delete project</button></form></details>}
 <h2>Issues</h2>{issues.isLoading&&<p>Loading issues…</p>}{issues.data?.length===0&&<p className="muted">No issues yet.</p>}{issues.data?.map(i=><Link className="card" href={`/issues/${i.id}`} key={i.id}><strong>{i.title}</strong><div className="muted">{i.status} · {i.priority}</div></Link>)}
 <h3>Create issue</h3><form onSubmit={createIssue}><input name="title" placeholder="Issue title" required/><textarea name="description" placeholder="Description"/><select name="priority" defaultValue="medium"><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option><option value="urgent">Urgent</option></select><select name="assignee_id" defaultValue=""><option value="">Unassigned</option>{members.data?.map(m=><option key={m.user_id} value={m.user_id}>{m.name}</option>)}</select><button>Create issue</button></form>
 <section className="columns"><div><h2>Project teammates</h2>{members.data?.map(m=><div className="card" key={m.user_id}><strong>{m.name}</strong><div className="muted">{m.email} · {m.role}</div></div>)}<form onSubmit={addMember}><input name="email" type="email" placeholder="Organization member email" required/><select name="role" defaultValue="reporter"><option value="reporter">Reporter</option><option value="developer">Developer</option><option value="maintainer">Maintainer</option></select><button>Add or update member</button></form></div>
 <div><h2>Labels</h2><div>{labels.data?.map(l=><span className="label" style={{borderColor:l.color}} key={l.id}>{l.name}</span>)}</div><form onSubmit={createLabel}><input name="name" placeholder="Label name" required/><input name="color" type="color" defaultValue="#6b7280"/><button>Create label</button></form></div></section>{error&&<p className="error">{error}</p>}</>;
}
