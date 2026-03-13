import { createSignal } from "solid-js";

export default function MessageInput(props: { onSend: (msg: string) => void }) {
    // todo: add real logic
    const [input, setInput] = createSignal("");

    const send = () => {
        if (!input().trim()) return;

        props.onSend(input().trim());
        setInput("");
    };

    return (
        <div class="p-3 md:p-4 border-t border-surface0 flex gap-3 bg-mantle">
            <input
                value={input()}
                onInput={(e) => setInput(e.currentTarget.value)}
                onKeyDown={(e) => {
                    if (e.key === "Enter") {
                        e.preventDefault();
                        send();
                    }
                }}
                placeholder="message"
                class="flex-1 min-w-0 px-4 py-2 rounded bg-surface0 text-text placeholder-subtext0 focus:outline-none"
            />

            <button
                type="button"
                onClick={send}
                class="px-4 py-2 bg-primary text-crust rounded hover:opacity-90 transition"
            >
                Send
            </button>
        </div>
    );
}