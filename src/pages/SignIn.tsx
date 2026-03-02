import { createSignal } from "solid-js";

export default function SignIn() {
    const [username, setUsername] = createSignal("");
    const [password, setPassword] = createSignal("");

    const login = (e: Event) => {
        e.preventDefault();
        console.log("Login:", username(), password());
    };
}