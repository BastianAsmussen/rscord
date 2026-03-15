import { For } from "solid-js";

type GuildMessage = {
    id: number;
    author_id: number;
    reply_to_id: number | null;
    channel_id: number;
    contents: string | null;
    edited_at: string;
    created_at: string;
};

export default function ChatView(props: { messages: GuildMessage[] }) {
    return (
        <div class="flex-1 overflow-y-auto p-4 md:p-6 flex flex-col gap-4 min-w-0">
            <For each={props.messages}>
                {(msg) => (
                    <div class="flex flex-col min-w-0">
                        {/* TODO: Displaying author_id as a placeholder until linked to member names */}
                        <span class="font-bold text-primary">User {msg.author_id}</span>
                        <span class="text-text break-words">
                            {msg.contents ?? ""}
                        </span>
                    </div>
                )}
            </For>
        </div>
    );
}