"use client";
import Link from "next/link";
import { FormEvent, useState } from "react";
import { useRouter } from "next/navigation";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { useAuth } from "../lib/auth";
type Issue={id:string;project_id:string;title:string;description:string;status:string;priority:string;assignee_id?:string};
type Member={user_id:string;name:string}; type Label={id:string;name:string;color:string};
export default function IssueDetail({issueId}:{issueId:string}){
 const token=useAuth(s=>s.token),client=useQueryClient(),router=useRouter(),[error,setError]=useState("");
 const issue=useQuery({queryKey:["issue",issueId,token],queryFn:()=>api<Issue>(`/issues/${issueId}`,token),enabled:!!token});
 const projectId=issue.data?.project_id;
 const members=useQuery({queryKey:["project-members",projectId,token],queryFn:()=>api<Member[]>(`/projects/${projectId}/members`,token),enabled:!!token&&!!projectId});
 const labels=useQuery({queryKey:["labels",projectId,token],queryFn:()=>api<Label[]>(`/projects/${projectId}/labels`,token),enabled:!!token&&!!projectId});
 const attached=useQuery({queryKey:["issue-labels",issueId,token],queryFn:()=>api<Label[]>(`/issues/${issueId}/labels`,token),enabled:!!token});
 if(!token)return <p>Please <Link href="/login">log in</Link>.</p>;if(issue.isLoading)return <p>Loading issue…</p>;if(!issue.data)return <p>Issue not found.</p>;
 async function submit(e:FormEvent<HTMLFormElement>){e.preventDefault();const f=new FormData(e.currentTarget),assignee=f.get("assignee_id");try{setError("");await api(`/issues/${issueId}`,token,{method:"PATCH",body:JSON.stringify({title:f.get("title"),description:f.get("description"),status:f.get("status"),priority:f.get("priority"),assignee_id:assignee||undefined,clear_assignee:!assignee})});client.invalidateQueries({queryKey:["issue",issueId,token]});}catch(cause){setError(cause instanceof Error?cause.message:"Unable to update issue");}}
 async function addLabel(e:FormEvent<HTMLFormElement>){e.preventDefault();const f=new FormData(e.currentTarget);try{await api(`/issues/${issueId}/labels`,token,{method:"POST",body:JSON.stringify({label_id:f.get("label_id")})});client.invalidateQueries({queryKey:["issue-labels",issueId,token]});}catch(cause){setError(cause instanceof Error?cause.message:"Unable to attach label");}}
 async function removeLabel(id:string){try{await api(`/issues/${issueId}/labels/${id}`,token,{method:"DELETE"});client.invalidateQueries({queryKey:["issue-labels",issueId,token]});}catch(cause){setError(cause instanceof Error?cause.message:"Unable to remove label");}}
 async function deleteIssue(){if(!confirm("Delete this issue?"))return;try{await api(`/issues/${issueId}`,token,{method:"DELETE"});router.push(`/projects/${issue.data?.project_id}`);}catch(cause){setError(cause instanceof Error?cause.message:"Unable to delete issue");}}
 const i=issue.data,attachedIds=new Set(attached.data?.map(l=>l.id));return <><Link href={`/projects/${i.project_id}`}>← Issues</Link><h1>{i.title}</h1><form onSubmit={submit}><input name="title" defaultValue={i.title} required/><textarea name="description" defaultValue={i.description}/><select name="status" defaultValue={i.status}><option value="open">Open</option><option value="in_progress">In progress</option><option value="closed">Closed</option></select><select name="priority" defaultValue={i.priority}><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option><option value="urgent">Urgent</option></select><select name="assignee_id" defaultValue={i.assignee_id??""}><option value="">Unassigned</option>{members.data?.map(m=><option key={m.user_id} value={m.user_id}>{m.name}</option>)}</select><button>Save changes</button><button type="button" className="danger" onClick={deleteIssue}>Delete issue</button></form>
 <h2>Labels</h2><div>{attached.data?.map(l=><button type="button" className="label" style={{borderColor:l.color}} onClick={()=>removeLabel(l.id)} key={l.id}>{l.name} ×</button>)}</div><form className="inline" onSubmit={addLabel}><select name="label_id" required defaultValue=""><option value="" disabled>Choose label</option>{labels.data?.filter(l=>!attachedIds.has(l.id)).map(l=><option key={l.id} value={l.id}>{l.name}</option>)}</select><button>Attach label</button></form>{error&&<p className="error">{error}</p>}</>;
}
