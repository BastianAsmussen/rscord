import { A } from "@solidjs/router";

export default function Home() {
    const logout = () => {
        localStorage.removeItem("session");
        window.location.href = "/signin";
    };

    return (
        <div>
            <h1>Home Page</h1>

            <nav>
                <A href="/settings">Go to Settings</A>
            </nav>

            <button onClick={logout}>Logout</button>
        </div>
    );
}