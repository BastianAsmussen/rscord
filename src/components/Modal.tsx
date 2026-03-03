import { Portal } from "solid-js/web";
import { onCleanup, onMount } from "solid-js";
import type { Accessor, JSX } from "solid-js";

type ModalProps = {
    open: Accessor<boolean>;
    onClose?: () => void;
    children: JSX.Element | JSX.Element[];
};

export function Modal(props: ModalProps) {
    const mount = document.getElementById("modal") ?? document.body;

    const handleKey = (e: KeyboardEvent) => {
        if (e.key === "Escape") {
            props.onClose?.();
        }
    };

    onMount(() => {
        window.addEventListener("keydown", handleKey);
    });

    onCleanup(() => {
        window.removeEventListener("keydown", handleKey);
    });

    return (
        <>
            {props.open() && (
                <Portal mount={mount} children={0}>
                    <div
                        class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm px-4 sm:px-6 py-8"
                        onClick={props.onClose}
                    >
                        <div
                            class="
                                relative
                                w-full
                                max-w-lg sm:max-w-2xl
                                bg-surface0
                                p-6 sm:p-8 md:p-10
                                rounded-2xl sm:rounded-3xl
                                shadow-2xl
                                border border-surface1
                                max-h-[85vh]
                                overflow-y-auto
                            "
                            onClick={(e) => e.stopPropagation()}
                        >
                            <button
                                type="button"
                                class="absolute top-4 right-4 text-subtext1 hover:text-white transition"
                                onClick={props.onClose}
                                aria-label="Close modal"
                            >
                                ✕
                            </button>

                            {props.children}
                        </div>
                    </div>
                </Portal>
            )}
        </>
    );
}