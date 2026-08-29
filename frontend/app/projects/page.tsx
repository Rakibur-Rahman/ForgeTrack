import ProjectList from "../../components/project-list";
import { serverProjects } from "../../lib/server-api";
export default async function ProjectsPage() { const projects = await serverProjects(); return <main><ProjectList initialProjects={projects}/></main>; }
