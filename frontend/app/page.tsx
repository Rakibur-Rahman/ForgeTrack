import Link from "next/link";
export default function Home() { return <main><h1>ForgeTrack</h1><p className="muted">Plan projects. Track issues. Ship confidently.</p><Link href="/login">Log in</Link>{" · "}<Link href="/signup">Create account</Link></main>; }
