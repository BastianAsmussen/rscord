import { Portal } from "solid-js/web";
import type { Accessor, JSX } from "solid-js";

type ModalProps = {
    open: Accessor<boolean>;
    onClose?: () => void;
    children: JSX.Element;
};

export function Modal(props: ModalProps) {
    const mount = document.getElementById("modal") ?? document.body;

    return (
        <>
            {props.open() && (
                <Portal
                    mount={mount}
                    children={
                        <div
                            class="fixed inset-0 backdrop flex items-center justify-center"
                            onClick={props.onClose}
                        >
                            <div
                                class="relative bg-surface1 p-6 w-full h-fit"
                                onClick={(e) => e.stopPropagation()}
                            >
                                <button
                                    type="button"
                                    class="absolute top-3 right-3"
                                    onClick={props.onClose}
                                >
                                    ✕
                                </button>

                                {props.children}
                            </div>
                        </div>
                    }
                />
            )}
        </>
    );
}