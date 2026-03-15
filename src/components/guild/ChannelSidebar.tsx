import {For} from "solid-js";

type Channel = {
    id: number; name: string; guild_id: number; type: string;
};

export default function ChannelSidebar(props: {
    channels: Channel[]; activeChannel: number; onSelect: (id: number) => void; title?: string;
}) {
    return (<div class="w-64 bg-mantle p-4 border-r border-surface0 h-full">
        <h2 class="font-bold mb-4 text-subtext0 text-sm">
            {props.title ?? "Channels"}
        </h2>

        <div class="flex flex-col gap-2">
            <For each={props.channels}>
                {(channel) => {
                    const isText = channel.type === "Text";

                    return (<button
                        type="button"
                        onClick={() => isText && props.onSelect(channel.id)}
                        class={`text-left px-3 py-2 rounded transition flex items-center gap-2
                                ${!isText ? "opacity-40 cursor-not-allowed" : "cursor-pointer"}
                                ${props.activeChannel === channel.id ? "bg-surface0 text-primary font-bold border-l-2 border-primary"
                            : "hover:bg-surface0 text-text"}`}
                    >
                        {isText ? "# " : "🔊 "} {channel.name}
                    </button>);
                }}
            </For>
        </div>
    </div>);
}