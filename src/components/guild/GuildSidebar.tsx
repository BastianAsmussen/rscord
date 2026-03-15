import { Accessor, For, JSX} from "solid-js";
import HomeButton from "../HomeButton";

type Guild = {
    id: number;
    name: string;
};

interface GuildSidebarProps {
    guilds: Guild[];
    activeGuild: number;
    onSelect: (id: number) => void;
}

export default function GuildSidebar(props: GuildSidebarProps) {
    return (
        <div
            class="w-16 md:w-20 bg-crust flex flex-col items-center py-3 md:py-4 gap-3 md:gap-4 border-r border-surface0 shrink-0">
            <HomeButton/>

            <For each={props.guilds} children={function (item: Guild, index: Accessor<number>): JSX.Element {
                throw new Error("Function not implemented.");
            }}>
                {(guild) => (
                    <button
                        type="button"
                        onClick={() => props.onSelect(guild.id)}
                        class={`w-10 h-10 md:w-12 md:h-12 rounded-xl transition font-bold text-center flex items-center justify-center
                        ${
                            props.activeGuild === guild.id
                                ? "bg-primary text-crust"
                                : "bg-surface0 hover:bg-surface1 text-text"
                        }`}
                        title={guild.name}
                    >
                        {guild.name.charAt(0).toUpperCase()}
                    </button>
                )}
            </For>
        </div>
    );
}