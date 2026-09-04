"use client";
import { FormEvent, useState } from "react";
import { useRouter } from "next/navigation";
import { api } from "../lib/api";
import { Session, useAuth } from "../lib/auth";
export default function AuthForm({ signup = false }: { signup?: boolean }) { const router=useRouter(), setSession=useAuth((s)=>s.setSession); const [error,setError]=useState(""); async function submit(e:FormEvent<HTMLFormElement>) { e.preventDefault(); const form=new FormData(e.currentTarget); try { const result=await api<Session>(signup?"/auth/signup":"/auth/login",undefined,{method:"POST",body:JSON.stringify({email:form.get("email"),password:form.get("password"),...(signup?{name:form.get("name")}:{})})}); setSession(result); router.push("/projects"); } catch(e) { setError(e instanceof Error?e.message:"Unable to authenticate"); } } return <form onSubmit={submit}>{signup&&<input name="name" placeholder="Name" required maxLength={100}/>}<input name="email" type="email" placeholder="Email" required/><input name="password" type="password" placeholder="Password (8+ characters)" required minLength={8}/>{error&&<p className="error">{error}</p>}<button>{signup?"Create account":"Log in"}</button></form>; }
