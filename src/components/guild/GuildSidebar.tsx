import { createSignal, For, onCleanup, Show } from "solid-js";
import HomeButton from "../HomeButton";
import Modal from "../Modal";
import { invoke } from "@tauri-apps/api/core";

type Guild = { id: number; name: string };

interface GuildSidebarProps {
    guilds: Guild[];
    activeGuild: number;
    onSelect: (id: number) => void;
    onGuildCreated: () => void;
}

export default function GuildSidebar(props: GuildSidebarProps) {
    const [showModal, setShowModal] = createSignal(false);
    const [newGuildName, setNewGuildName] = createSignal("");
    const [joinGuildId, setJoinGuildId] = createSignal("");
    const [loading, setLoading] = createSignal(false);

    const [contextMenu, setContextMenu] = createSignal<{ x: number, y: number, guildId: number } | null>(null);

    const handleContextMenu = (e: MouseEvent, guildId: number) => {
        e.preventDefault();
        setContextMenu({ x: e.clientX, y: e.clientY, guildId });
    };

    const closeMenu = () => setContextMenu(null);

    window.addEventListener("click", closeMenu);
    onCleanup(() => window.removeEventListener("click", closeMenu));

    const handleAction = async (action: "leave_guild" | "delete_guild") => {
        const menu = contextMenu();
        if (!menu) return;

        const isDelete = action === "delete_guild";
        const confirmMsg = isDelete
            ? "Are you sure you want to DELETE this guild? This cannot be undone and you must be the owner."
            : "Leave this guild? Note: Owners cannot leave, they must delete the guild.";

        if (!confirm(confirmMsg)) return;

        try {
            await invoke(action, { id: menu.guildId });
            props.onGuildCreated();
        } catch (err) {
            alert(`Action failed: ${err}`);
        }
        closeMenu();
    };

    const handleCreate = async (e: Event) => {
        e.preventDefault();
        if (!newGuildName()) return;

        setLoading(true);
        try {
            await invoke("create_guild", { name: newGuildName() });
            setNewGuildName("");
            setShowModal(false);
            props.onGuildCreated();
        } catch (err) {
            alert("Failed to create guild: " + err);
        } finally {
            setLoading(false);
        }
    };

    const handleJoin = async (e: Event) => {
        e.preventDefault();
        const id = parseInt(joinGuildId());
        if (isNaN(id)) return alert("Please enter a valid ID");

        setLoading(true);
        try {
            await invoke("join_guild", { id });
            setJoinGuildId("");
            setShowModal(false);
            props.onGuildCreated();
        } catch (err) {
            alert("Join failed: " + err);
        } finally {
            setLoading(false);
        }
    };

    return (
        <div class="w-16 md:w-20 bg-crust flex flex-col items-center py-3 md:py-4 gap-3 md:gap-4 border-r border-surface0 shrink-0 relative">
            <HomeButton />

            {/* Create Guild Button */}
            <button
                onClick={() => setShowModal(true)}
                class="w-10 h-10 md:w-12 md:h-12 rounded-xl bg-surface0 hover:bg-green text-green hover:text-crust transition flex items-center justify-center text-2xl font-bold"
                title="Guild Functions"
            >
                +
            </button>

            <div class="w-8 h-[2px] bg-surface0 rounded-full" />

            <For each={props.guilds}>
                {(guild) => (
                    <button
                        type="button"
                        onClick={() => props.onSelect(guild.id)}
                        onContextMenu={(e) => handleContextMenu(e, guild.id)}
                        class={`w-10 h-10 md:w-12 md:h-12 rounded-xl transition font-bold text-center flex items-center justify-center
                            ${props.activeGuild === guild.id ? "bg-primary text-crust" : "bg-surface0 hover:bg-surface1 text-text"}`}
                        title={guild.name}
                    >
                        {guild.name.charAt(0).toUpperCase()}
                    </button>
                )}
            </For>

            {/* Context Menu */}
            <Show when={contextMenu()}>
                {(menu) => (
                    <div
                        class="fixed z-[100] bg-mantle border border-surface0 rounded-lg shadow-2xl py-1 w-48 overflow-hidden"
                        style={{ left: `${menu().x}px`, top: `${menu().y}px` }}
                        onClick={(e) => e.stopPropagation()}
                    >
                        <button
                            onClick={() => handleAction("leave_guild")}
                            class="w-full text-left px-4 py-2 hover:bg-surface0 text-text text-sm transition-colors"
                        >
                            Leave Guild
                        </button>
                        <button
                            onClick={() => handleAction("delete_guild")}
                            class="w-full text-left px-4 py-2 hover:bg-red/10 text-red text-sm transition-colors font-semibold"
                        >
                            Delete Guild
                        </button>
                        <div class="border-t border-surface0 my-1" />
                        <div class="px-4 py-1.5 text-[10px] text-subtext0 uppercase font-bold tracking-wider select-none">
                            ID: {menu().guildId}
                        </div>
                    </div>
                )}
            </Show>

            <Modal open={showModal} onClose={() => setShowModal(false)}>
                <div class="flex flex-col gap-8">
                    {/* Section 1: Create */}
                    <section>
                        <h2 class="text-xl font-bold text-text mb-4">Create a New Guild</h2>
                        <form onSubmit={handleCreate} class="flex flex-col gap-3">
                            <input
                                type="text"
                                value={newGuildName()}
                                onInput={(e) => setNewGuildName(e.currentTarget.value)}
                                class="w-full p-3 bg-base border border-surface1 rounded-xl text-text focus:border-primary outline-none"
                                placeholder="Guild Name"
                            />
                            <button
                                type="submit"
                                disabled={loading()}
                                class="w-full py-2 bg-primary text-crust font-bold rounded-xl hover:opacity-90 transition disabled:opacity-50"
                            >
                                {loading() ? "Creating..." : "Create Guild"}
                            </button>
                        </form>
                    </section>

                    {/* Separator */}
                    <div class="flex items-center gap-4 text-subtext0">
                        <div class="flex-1 h-[1px] bg-surface1" />
                        <span class="text-xs font-bold uppercase">OR</span>
                        <div class="flex-1 h-[1px] bg-surface1" />
                    </div>

                    {/* Section 2: Join */}
                    <section>
                        <h2 class="text-xl font-bold text-text mb-4">Join via ID</h2>
                        <form onSubmit={handleJoin} class="flex flex-col gap-3">
                            <input
                                type="number"
                                value={joinGuildId()}
                                onInput={(e) => setJoinGuildId(e.currentTarget.value)}
                                class="w-full p-3 bg-base border border-surface1 rounded-xl text-text focus:border-green outline-none"
                                placeholder="Guild ID (e.g. 123)"
                            />
                            <button
                                type="submit"
                                disabled={loading()}
                                class="w-full py-2 bg-green text-crust font-bold rounded-xl hover:opacity-90 transition disabled:opacity-50"
                            >
                                {loading() ? "Joining..." : "Join Guild"}
                            </button>
                        </form>
                    </section>
                </div>
            </Modal>
        </div>
    );
}