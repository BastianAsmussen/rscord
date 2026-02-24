import { createSignal } from "solid-js";
import logo from "./assets/logo.svg";
import { invoke } from "@tauri-apps/api/core";
import { registerForPushNotifications } from '@choochmeque/tauri-plugin-notifications-api';
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = createSignal("");
  const [name, setName] = createSignal("");

  async function get_push_token(){
      try {
          const token = await registerForPushNotifications();
          console.log('Push token:', token);
          // Send this token to your server to send push notifications
          return token
      } catch (error) {
          console.error('Failed to register for push notifications:', error);
          return "epic fail"
      }
  }


  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
      get_push_token()
    setGreetMsg(await invoke("greet", { name: name() }));
  }

  return (
    <main class="container">
      <h1 class="bg-red-700">Welcome to Tauri + Solid</h1>
        
      <p>Click on the Tauri, Vite, and Solid logos to learn more.</p>

      <form
        class="row"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <button type="submit">Greet</button>
      </form>
      <p>{greetMsg()}</p>
    </main>
  );
}

export default App;
