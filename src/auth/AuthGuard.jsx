import { Navigate } from "@solidjs/router";

export default function AuthGuard(props) {
    const session = localStorage.getItem("session"); // TODO: needs to be saved and authed though cookies

    if (!session) {
        return <Navigate href="/signin" />;
    }

    return props.children;
}
