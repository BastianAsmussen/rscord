import { A } from "@solidjs/router";
import {invoke} from "@tauri-apps/api/core";
import {createSignal} from "solid-js";

export default function Home() {
    const [token, setToken] = createSignal("");

    const saveToken = async () => {
        const t = token();
        localStorage.setItem("session", t);
        await invoke("set_token", { token: t });
        alert("Token gemt og sat i backend!");
    };

    const removeToken = async () => {
        localStorage.removeItem("session");
        setToken("");
        await invoke("remove_token");
        alert("Token fjernet fra backend!");
    };

    const logout = () => {
        localStorage.removeItem("session");
        invoke("remove_token");
        window.location.href = "/signin";
    };

    return (
        <div>
            <h1>Forsiden</h1>

            <div style={{ margin: "20px 0", padding: "10px", border: "1px solid #ccc" }}>
                <h3>Test: Sæt Session Token</h3>
                <input
                    type="text"
                    placeholder="Indsæt token..."
                    value={token()}
                    onInput={(e) => setToken(e.currentTarget.value)}
                />
                <button onClick={saveToken}>Gem token</button>
                <button onClick={removeToken} style={{ "margin-left": "10px" }}>Fjern token</button>
            </div>

            <nav>
                <A href="/guild">Gå til Guilds</A>
                <br />
                <A href="/settings">Gå til Indstillinger</A>
            </nav>

            <button onClick={logout}>Log ud</button>
        </div>
    );
}