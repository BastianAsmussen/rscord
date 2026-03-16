import { createSignal } from "solid-js";
import type { JSX } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { CreateUser } from "../components/CreateUser";

export default function SignIn() {
    const [username, setUsername] = createSignal("");
    const [password, setPassword] = createSignal("");
    const [error, setError] = createSignal("");

    const login: JSX.EventHandler<HTMLFormElement, SubmitEvent> = async (e) => {
        e.preventDefault();
        setError("");

        try {
            const user = await invoke("log_in", {
                email: username(),
                password: password(),
            });

            console.log(`Logged in as @${user.user.handle}.`);
        } catch (err) {
            console.error(err);
            setError(String(err));
        }
    };

    return (
        <div class="min-h-screen flex items-center justify-center bg-[#0f1220] px-4">
            <div class="w-full max-w-2xl bg-[#313244] p-8 md:p-12 rounded-3xl shadow-2xl border border-[#45475a]">
                <h1 class="text-4xl md:text-5xl font-bold text-center mb-10 text-[#f38ba8]">
                    Sign In
                </h1>

                <form onSubmit={login} class="flex flex-col gap-6 md:gap-8">
                    <input
                        type="text"
                        placeholder="Email"
                        value={username()}
                        onInput={(e) => setUsername(e.currentTarget.value)}
                        class="w-full px-5 py-3 text-white placeholder:text-[#a6adc8] rounded-2xl bg-[#45475a] border border-[#585b70] focus:border-[#f38ba8] focus:outline-none"
                        required
                    />

                    <input
                        type="password"
                        placeholder="Password"
                        value={password()}
                        onInput={(e) => setPassword(e.currentTarget.value)}
                        class="w-full px-5 py-3 text-white placeholder:text-[#a6adc8] rounded-2xl bg-[#45475a] border border-[#585b70] focus:border-[#f38ba8] focus:outline-none"
                        required
                    />

                    <button
                        type="submit"
                        class="w-full py-3 bg-[#f38ba8] text-black font-semibold rounded-2xl hover:opacity-90 transition"
                    >
                        Login
                    </button>
                </form>

                {error() && (
                    <p class="text-red-400 mt-4 text-center">
                        {error()}
                    </p>
                )}

                <div class="mt-10 text-center text-[#cdd6f4]">
                    If you don’t have a user,&nbsp;
                    <CreateUser />
                </div>
            </div>
        </div>
    );
}
