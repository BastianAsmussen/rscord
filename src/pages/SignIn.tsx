export default function SignIn() {

    // TODO: Implement real login logic
    const login = () => {
        localStorage.setItem("session", "logged-in");
        window.location.href = "/";
    };

    return (
        <div>
            <h1>Sign In</h1>

            <button onClick={login}>Login</button>
        </div>
    );
}