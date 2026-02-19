import { Navigate } from "@solidjs/router";

export default function AuthGuard(props) {
    const session = localStorage.getItem("session");

    if (!session) {
        return <Navigate href="/signin" />;
    }

    return props.children;
}