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
            </div>
        </div>
    );
}