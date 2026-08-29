import Link from "next/link"; import AuthForm from "../../components/auth-form";
export default function LoginPage() { return <main><h1>Log in</h1><AuthForm/><p>New here? <Link href="/signup">Create an account</Link>.</p></main>; }
