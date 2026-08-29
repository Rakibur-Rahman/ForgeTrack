import IssueList from "../../../components/issue-list";
export default async function ProjectPage({params}:{params:Promise<{id:string}>}) { const {id}=await params; return <main><IssueList projectId={id}/></main>; }
