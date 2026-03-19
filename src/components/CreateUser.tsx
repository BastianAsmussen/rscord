import { createSignal } from "solid-js";
import type { JSX } from "solid-js";
import Modal from "./Modal";
import { invoke } from "@tauri-apps/api/core";

export function CreateUser() {
    const [open, setOpen] = createSignal(false);

    const [username, setUsername] = createSignal("");
    const [email, setEmail] = createSignal("");
    const [password, setPassword] = createSignal("");

    const register: JSX.EventHandler<HTMLFormElement, SubmitEvent> = async (e) => {
        e.preventDefault();

        try {
            const user = await invoke("sign_up", {
                email: email(),
                handle: username(),
                password: password(),
            });

            setUsername("");
            setEmail("");
            setPassword("");

            setOpen(false);

        } catch (err) {
            console.error("Registration failed:", err);
            alert("Failed to create user");
        }
    };

    return (
        <>
            <button
                type="button"
                onClick={() => setOpen(true)}
                class="text-primary font-semibold hover:underline"
            >
                create one
            </button>

            <Modal open={open} onClose={() => setOpen(false)} children={0}>
                <div class="w-full max-w-2xl mx-auto px-6 py-8 md:px-10 md:py-12">

                    <h2 class="text-4xl md:text-5xl font-bold text-center mb-8 md:mb-12 text-[#f38ba8]">
                        Create User
                    </h2>

                    <form onSubmit={register} class="flex flex-col gap-6 md:gap-8">

                        <input
                            type="text"
                            placeholder="Username"
                            value={username()}
                            onInput={(e) => setUsername(e.currentTarget.value)}
                            class="w-full px-5 py-3 text-white placeholder:text-subtext0 rounded-2xl bg-surface1 border border-surface2 focus:border-primary focus:outline-none"
                            required
                        />

                        <input
                            type="email"
                            placeholder="Email"
                            value={email()}
                            onInput={(e) => setEmail(e.currentTarget.value)}
                            class="w-full px-5 py-3 text-white placeholder:text-subtext0 rounded-2xl bg-surface1 border border-surface2 focus:border-primary focus:outline-none"
                            required
                        />

                        <input
                            type="password"
                            placeholder="Password"
                            value={password()}
                            onInput={(e) => setPassword(e.currentTarget.value)}
                            class="w-full px-5 py-3 text-white placeholder:text-subtext0 rounded-2xl bg-surface1 border border-surface2 focus:border-primary focus:outline-none"
                            required
                        />

                        <button
                            type="submit"
                            class="w-full py-3 bg-primary text-black font-semibold rounded-2xl hover:opacity-90 transition"
                        >
                            Create Account
                        </button>
                    </form>

                </div>
            </Modal>
        </>
    );
}