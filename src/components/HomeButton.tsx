export default function HomeButton() {
    return (
        // todo: change path to homepage

        <button
            type="button"
            class="flex items-center justify-center w-10 h-10 md:w-12 md:h-12 rounded-xl bg-surface0 hover:bg-primary transition text-text hover:text-crust"
            onClick={() => (window.location.href = "/")}
        >
            home
        </button>
    );
}