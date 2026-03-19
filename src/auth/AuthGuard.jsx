import { Navigate } from "@solidjs/router";

export default function AuthGuard(props) {
    const sessionRaw = localStorage.getItem("session");

    if (!sessionRaw || sessionRaw === "null") {
        return <Navigate href="/signin" />;
    }

    try {
        const session = JSON.parse(sessionRaw);

        if (!session.token) {
            return <Navigate href="/signin" />;
        }

        const now = new Date();
        if (new Date(session.expires) <= now) {
            localStorage.removeItem("session");
            return <Navigate href="/signin" />;
        }
    } catch (e) {
        console.error("Session parse error", e);
        localStorage.removeItem("session");
        return <Navigate href="/signin" />;
    }

    return props.children;
}