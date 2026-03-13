import { For } from "solid-js";

type Message = {
    // todo: add real logic

    id: number;
    user: string;
    text: string;
};

export default function ChatView(props: { messages: Message[] }) {
    return (
        <div class="flex-1 overflow-y-auto p-4 md:p-6 flex flex-col gap-4 min-w-0">
            <For each={props.messages}>
                {(msg) => (
                    <div class="flex flex-col min-w-0">
                        <span class="font-bold text-primary">{msg.user}</span>
                        <span class="text-text break-words">{msg.text}</span>
                    </div>
                )}
            </For>
        </div>
    );
}