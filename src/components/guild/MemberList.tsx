import { For, Show } from "solid-js";

type Member = {
    id: number;
    name: string;
};

export default function MemberList(props: {
    members: Member[];
    onClose?: () => void;
    mobile?: boolean;
}) {
    return (
        <div class="w-64 bg-mantle p-4 border-l border-surface0 h-full">
            <div class="flex items-center justify-between mb-4">
                <h2 class="font-bold text-subtext0 text-sm">Members</h2>

                <Show when={props.mobile}>
                    <button
                        type="button"
                        class="px-2 py-1 rounded bg-surface0 hover:bg-surface1 text-text"
                        onClick={props.onClose}
                    >
                        ✕
                    </button>
                </Show>
            </div>

            <div class="flex flex-col gap-2">
                <For each={props.members}>
                    {(member) => (
                        <div class="px-3 py-2 rounded bg-surface0 text-text">
                            {member.name}
                        </div>
                    )}
                </For>
            </div>
        </div>
    );
}