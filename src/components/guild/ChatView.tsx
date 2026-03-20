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

type Member = { id: number; name: string };

export default function ChatView(props: { messages: GuildMessage[]; members: Member[] }) {
    const authorName = (authorId: number) => {
        const member = props.members.find(m => m.id === authorId);
        return member ? member.name : `User ${authorId}`;
    };

    return (
        <div class="flex-1 overflow-y-auto p-4 md:p-6 flex flex-col gap-4 min-w-0">
            <For each={props.messages}>
                {(msg) => (
                    <div class="flex flex-col min-w-0">
                        <span class="font-bold text-primary">{authorName(msg.author_id)}</span>
                        <span class="text-text break-words">
                            {msg.contents ?? ""}
                        </span>
                    </div>
                )}
            </For>
        </div>
    );
}
