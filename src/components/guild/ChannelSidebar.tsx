import {createSignal, For} from "solid-js";
import {invoke} from "@tauri-apps/api/core";
import Modal from "../Modal";

type Channel = { id: number; name: string; guild_id: number; type: string; };

interface ChannelSidebarProps {
    channels: Channel[];
    activeChannel: number;
    activeGuild: number | null;
    onSelect: (id: number) => void;
    onChannelCreated: () => void;
    title?: string;
}

export default function ChannelSidebar(props: ChannelSidebarProps) {
    const [showModal, setShowModal] = createSignal(false);
    const [name, setName] = createSignal("");
    const [type, setType] = createSignal("Text");
    const [topic, setTopic] = createSignal("");
    const [loading, setLoading] = createSignal(false);

    const handleCreate = async (e: Event) => {
        e.preventDefault();
        if (!props.activeGuild || !name()) return;

        setLoading(true);
        try {
            await invoke("create_channel", {
                guildId: props.activeGuild, name: name(), channelType: type(), topic: topic()
            });
            setShowModal(false);
            setName("");
            setTopic("");
            props.onChannelCreated();
        } catch (err) {
            alert("Error creating channel: " + err);
        } finally {
            setLoading(false);
        }
    };

    return (<div class="w-64 bg-mantle p-4 border-r border-surface0 h-full flex flex-col">
        <div class="flex items-center justify-between mb-4">
            <h2 class="font-bold text-subtext0 text-sm uppercase tracking-wider">
                {props.title ?? "Channels"}
            </h2>
            {props.activeGuild && (<button
                onClick={() => setShowModal(true)}
                class="text-subtext1 hover:text-primary transition text-xl font-bold"
            >
                +
            </button>)}
        </div>

        <div class="flex flex-col gap-1">
            <For each={props.channels}>
                {(channel) => {
                    const isText = channel.type.toLowerCase() === "text";
                    return (<button
                        type="button"
                        onClick={() => isText && props.onSelect(channel.id)}
                        class={`text-left px-3 py-2 rounded transition flex items-center gap-2
                                    ${!isText ? "opacity-40 cursor-not-allowed" : "cursor-pointer"}
                                    ${props.activeChannel === channel.id ? "bg-surface0 text-primary font-bold border-l-2 border-primary" : "hover:bg-surface0 text-text"}`}
                    >
                                <span class="text-subtext1 w-4 text-center">
                                    {isText ? "#" : "🔊"}
                                </span>
                        <span class="truncate">{channel.name}</span>
                    </button>);
                }}
            </For>
        </div>

        <Modal open={showModal} onClose={() => setShowModal(false)}>
            <h2 class="text-2xl font-bold text-text mb-6">Create Channel</h2>
            <form onSubmit={handleCreate} class="flex flex-col gap-4">
                <div>
                    <label class="block text-xs font-bold text-subtext0 mb-2 uppercase">Channel Type</label>
                    <select
                        value={type()}
                        onChange={(e) => setType(e.currentTarget.value)}
                        class="w-full p-3 bg-base border border-surface1 rounded-xl text-text focus:border-primary outline-none"
                    >
                        <option value="Text">Text</option>
                        <option value="Voice">Voice</option>
                    </select>
                </div>

                <div>
                    <label class="block text-xs font-bold text-subtext0 mb-2 uppercase">Channel Name</label>
                    <input
                        type="text"
                        placeholder="new-channel"
                        value={name()}
                        onInput={(e) => setName(e.currentTarget.value)}
                        class="w-full p-3 bg-base border border-surface1 rounded-xl text-text focus:border-primary outline-none"
                        required
                    />
                </div>

                <div>
                    <label class="block text-xs font-bold text-subtext0 mb-2 uppercase">Topic</label>
                    <input
                        type="text"
                        placeholder="What's this channel about?"
                        value={topic()}
                        onInput={(e) => setTopic(e.currentTarget.value)}
                        class="w-full p-3 bg-base border border-surface1 rounded-xl text-text focus:border-primary outline-none"
                    />
                </div>

                <button
                    type="submit"
                    disabled={loading()}
                    class="mt-2 w-full py-3 bg-primary text-crust font-bold rounded-xl hover:opacity-90 transition disabled:opacity-50"
                >
                    {loading() ? "Creating..." : "Create Channel"}
                </button>
            </form>
        </Modal>
    </div>);
}