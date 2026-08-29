import IssueDetail from "../../../components/issue-detail";
export default async function IssuePage({params}:{params:Promise<{id:string}>}) { const {id}=await params; return <main><IssueDetail issueId={id}/></main>; }
