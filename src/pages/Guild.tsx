import {createEffect, createMemo, createSignal, onCleanup, onMount} from "solid-js";
import {invoke} from "@tauri-apps/api/core";
import {useNavigate} from "@solidjs/router";

import GuildSidebar from "../components/guild/GuildSidebar";
import ChannelSidebar from "../components/guild/ChannelSidebar";
import ChatView from "../components/guild/ChatView";
import MemberList from "../components/guild/MemberList";
import MessageInput from "../components/guild/MessageInput";
import {listen} from "@tauri-apps/api/event";

type Guild = { id: number; name: string };
type Channel = { id: number; name: string; guild_id: number; type: string; };
type Role = { id: number; name: string };
type Member = { user_id: number; user_handle: string; roles: Role[] };
type GuildMessage = {
    id: number;
    author_id: number;
    reply_to_id: number | null;
    channel_id: number;
    contents: string | null;
    edited_at: string;
    created_at: string;
};

export default function GuildPage() {
    const navigate = useNavigate();
    const [guilds, setGuilds] = createSignal<Guild[]>([]);
    const [channels, setChannels] = createSignal<Channel[]>([]);
    const [members, setMembers] = createSignal<Member[]>([]);
    const [messages, setMessages] = createSignal<GuildMessage[]>([]);

    const [activeGuild, setActiveGuild] = createSignal<number | null>(null);
    const [activeChannel, setActiveChannel] = createSignal<number | null>(null);

    const [showMobileChannels, setShowMobileChannels] = createSignal(false);
    const [showMembers, setShowMembers] = createSignal(false);

    onMount(async () => {
        // Check that a valid (non-expired) session still exists in localStorage
        // so we redirect to sign-in before attempting any API calls.
        const raw = localStorage.getItem("session");
        if (!raw) {
            navigate("/signin");
            return;
        }
        let session: { token: string; expires: string } | null = null;
        try {
            session = JSON.parse(raw);
        } catch {
            localStorage.removeItem("session");
            navigate("/signin");
            return;
        }
        if (!session?.token || new Date(session.expires) <= new Date()) {
            localStorage.removeItem("session");
            navigate("/signin");
            return;
        }
        // The token is passed explicitly to init_websocket so the Rust backend
        // has it in state for all subsequent commands (list_my_guilds, etc.).
        // It is also persisted to the on-disk store so app restarts work without
        // requiring another login.
        try {
            await invoke("init_websocket", { token: session.token });
        } catch (e) {
            console.error("Failed to init WebSocket:", e);
        }

        const unlisten = await listen<GuildMessage>("guild-message", (event) => {
            const newMessage = event.payload;
            if (newMessage.channel_id === activeChannel()) {
                setMessages((prev) => [...prev, newMessage]);
            }
        });

        onCleanup(() => unlisten());

        try {
            const data: Guild[] = await invoke("list_my_guilds");
            setGuilds(data);
            if (data.length > 0) setActiveGuild(data[0].id);
        } catch (e) {
            console.error("Failed to load guilds:", e);
        }
    });

    const memberDisplayList = createMemo(() =>
        members().map(m => ({
            id: m.user_id,
            name: m.user_handle
        }))
    );

    // When activeGuild changes, fetch channels and members
    createEffect(async () => {
        const guildId = activeGuild();
        if (!guildId) return;

        try {
            const [newChannels, newMembers] = await Promise.all([
                invoke<Channel[]>("get_guild_channels", { id: guildId }),
                invoke<Member[]>("get_guild_members", { id: guildId })
            ]);

            setChannels(newChannels);

            setMembers(newMembers);

            if (newChannels.length > 0) setActiveChannel(newChannels[0].id);
        } catch (e) {
            console.error("Failed to load guild data:", e);
        }
    });

    const activeGuildName = createMemo(() => {
        const guild = guilds().find((g) => g.id === activeGuild());
        return guild ? guild.name : "Guild";
    });

    const activeChannelName = createMemo(() => {
        const channel = channels().find((c) => c.id === activeChannel());
        return channel ? channel.name : "channel";
    });

    const selectGuild = (id: number) => {
        setActiveGuild(id);
        setShowMobileChannels(true);
    };

    const selectChannel = async (id: number) => {
        const channel = channels().find(c => c.id === id);
        if (channel && channel.type === "Text") {
            setActiveChannel(id);
            setShowMobileChannels(false);

            try {
                setMessages([]); // Clear old channel's messages
                const history = await invoke<GuildMessage[]>("get_messages", { channelId: id });
                setMessages(history);
            } catch (e) {
                console.error("Failed to load channel history:", e);
            }
        }
    };

    const sendMessage = async (text: string) => {
        const cid = activeChannel();
        if (!cid) return;
        try {
            await invoke("send_message", { channelId: cid, content: text });
            // The sent message is delivered back via the WebSocket connection,
            // so we do not add it to state here to avoid duplicates.
        } catch (e) {
            console.error("Send failed:", e);
        }
    };

    return (<div class="flex h-screen bg-base text-text overflow-hidden">
        <GuildSidebar
            guilds={guilds()}
            activeGuild={activeGuild() ?? 0}
            onSelect={selectGuild}
        />

        <div class="hidden md:block shrink-0">
            <ChannelSidebar
                channels={channels()}
                activeChannel={activeChannel() ?? 0}
                onSelect={selectChannel}
                title={activeGuildName()}
            />
        </div>

        <div class="flex flex-col flex-1 min-w-0">
            <div
                class="md:hidden flex items-center justify-between gap-3 p-3 border-b border-surface0 bg-mantle min-w-0">
                <button
                    type="button"
                    onClick={() => setShowMobileChannels(true)}
                    class="px-3 py-1 bg-surface0 rounded text-text shrink-0"
                >
                    channels
                </button>

                <div class="flex-1 min-w-0 text-center">
                    <div class="font-bold text-primary truncate">{activeGuildName()}</div>
                    <div class="text-sm text-subtext0 truncate"># {activeChannelName()}</div>
                </div>

                <button
                    type="button"
                    onClick={() => setShowMembers(true)}
                    class="px-3 py-1 bg-surface0 rounded text-text shrink-0"
                >
                    memebers
                </button>
            </div>

            <ChatView messages={messages()}/>
            <MessageInput onSend={sendMessage}/>
        </div>

        <div class="hidden md:block shrink-0">
            <MemberList members={memberDisplayList()}/>
        </div>

        {showMobileChannels() && (<div
            class="fixed inset-0 backdrop md:hidden z-40"
            onClick={() => setShowMobileChannels(false)}
        >
            <div
                class="absolute left-16 top-0 bottom-0 w-64 bg-mantle"
                onClick={(e) => e.stopPropagation()}
            >
                <div class="flex items-center justify-between p-4 border-b border-surface0">
                    <h2 class="font-bold text-subtext0 text-sm">{activeGuildName()}</h2>

                    <button
                        type="button"
                        class="px-2 py-1 rounded bg-surface0 hover:bg-surface1 text-text"
                        onClick={() => setShowMobileChannels(false)}
                    >
                        ✕
                    </button>
                </div>

                <ChannelSidebar
                    channels={channels()}
                    activeChannel={activeChannel() ?? 0}
                    onSelect={selectChannel}
                />
            </div>
        </div>)}

        {showMembers() && (<div
            class="fixed inset-0 backdrop md:hidden z-40"
            onClick={() => setShowMembers(false)}
        >
            <div
                class="absolute right-0 top-0 bottom-0"
                onClick={(e) => e.stopPropagation()}
            >
                <MemberList
                    members={memberDisplayList()}
                    mobile={true}
                    onClose={() => setShowMembers(false)}
                />
            </div>
        </div>)}
    </div>);
}
