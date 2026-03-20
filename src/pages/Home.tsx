import { A } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";

export default function Home() {
  const logout = () => {
    localStorage.removeItem("session");
    invoke("remove_token");
    window.location.href = "/signin";
  };

  return (
    <div>
      <h1>Forsiden</h1>

      <nav>
        <A href="/guild">Gå til Guilds</A>
        <br />
        <A href="/settings">Gå til Indstillinger</A>
      </nav>

      <button onClick={logout}>Log ud</button>
    </div>
  );
}
