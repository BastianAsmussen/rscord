import { A } from "@solidjs/router";

export default function Home() {
    const logout = () => {
        localStorage.removeItem("session");
        window.location.href = "/signin";
    };

    return (
        <div>
            <h1>Forsiden</h1>

            <nav>
                <A href="/settings">Gå til Indstillinger</A>
            </nav>

            <button onClick={logout}>Log ud</button>
        </div>
    );
}