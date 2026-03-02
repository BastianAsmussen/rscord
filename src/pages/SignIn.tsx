import { createSignal } from "solid-js";
import CreateUser from "../components/CreateUser";

export default function SignIn() {
    const [username, setUsername] = createSignal("");
    const [password, setPassword] = createSignal("");

    const login = (e: Event) => {
        e.preventDefault();
        console.log("Login:", username(), password());
    };

    return (
        <div class="min-h-screen flex items-center justify-center bg-[#0f1220] px-4">
            <div
                class="
                    w-full
                    max-w-2xl
                    bg-[#313244]
                    p-8 md:p-12
                    rounded-3xl
                    shadow-2xl
                    border border-[#45475a]
                "
            >
                <h1 class="text-4xl md:text-5xl font-bold text-center mb-10 text-[#f38ba8]">
                    Sign In
                </h1>

                <form onSubmit={login} class="flex flex-col gap-6 md:gap-8">
                    <input
                        type="text"
                        placeholder="Username"
                        value={username()}
                        onInput={(e) => setUsername(e.currentTarget.value)}
                        class="
                            w-full
                            px-5 py-3
                            text-white
                            placeholder:text-[#a6adc8]
                            rounded-2xl
                            bg-[#45475a]
                            border border-[#585b70]
                            focus:border-[#f38ba8]
                            focus:outline-none
                        "
                        required
                    />

                    <input
                        type="password"
                        placeholder="Password"
                        value={password()}
                        onInput={(e) => setPassword(e.currentTarget.value)}
                        class="
                            w-full
                            px-5 py-3
                            text-white
                            placeholder:text-[#a6adc8]
                            rounded-2xl
                            bg-[#45475a]
                            border border-[#585b70]
                            focus:border-[#f38ba8]
                            focus:outline-none
                        "
                        required
                    /></form>
            </div>
        </div>
    );
}