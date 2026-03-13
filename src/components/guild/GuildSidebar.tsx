import { For } from "solid-js";
import HomeButton from "../HomeButton";

type Guild = {
    // todo: add real logic

    id: number;
    name: string;
};

export default function GuildSidebar(props: {
    // todo: add real logic

    guilds: Guild[];
    activeGuild: number;
    onSelect: (id: number) => void;
}) {
    return (
        <div class="w-16 md:w-20 bg-crust flex flex-col items-center py-3 md:py-4 gap-3 md:gap-4 border-r border-surface0 shrink-0">
            <HomeButton />

            <For each={props.guilds}>
                {(guild) => (
                    <button
                        type="button"
                        onClick={() => props.onSelect(guild.id)}
                        class={`w-10 h-10 md:w-12 md:h-12 rounded-xl transition font-bold
              ${
                            props.activeGuild === guild.id
                                ? "bg-primary text-crust"
                                : "bg-surface0 hover:bg-surface1 text-text"
                        }`}
                        title={guild.name}
                    >
                        {guild.name[0]}
                    </button>
                )}
            </For>
        </div>
    );
}