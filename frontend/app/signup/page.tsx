import Link from "next/link"; import AuthForm from "../../components/auth-form";
export default function SignupPage() { return <main><h1>Create account</h1><AuthForm signup/><p>Already registered? <Link href="/login">Log in</Link>.</p></main>; }
