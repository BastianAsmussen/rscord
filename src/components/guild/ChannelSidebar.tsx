import { For } from "solid-js";

type Channel = {
    // todo: add real logic
    id: number;
    name: string;
    guild: number;
};

export default function ChannelSidebar(props: {
    // todo: add real logic
    channels: Channel[];
    activeChannel: number;
    onSelect: (id: number) => void;
    title?: string;
}) {
    return (
        <div class="w-64 bg-mantle p-4 border-r border-surface0 h-full">
            <h2 class="font-bold mb-4 text-subtext0 text-sm">
                {props.title ?? "Channels"}
            </h2>

            <div class="flex flex-col gap-2">
                <For each={props.channels}>
                    {(channel) => (
                        <button
                            type="button"
                            onClick={() => props.onSelect(channel.id)}
                            class={`text-left px-3 py-2 rounded transition
                ${
                                props.activeChannel === channel.id
                                    ? "bg-surface0 text-primary"
                                    : "hover:bg-surface0 text-text"
                            }`}
                        >
                            # {channel.name}
                        </button>
                    )}
                </For>
            </div>
        </div>
    );
}