import { Route } from "@solidjs/router";
import AuthGuard from "./auth/AuthGuard.jsx";
import Home from "./pages/Home";
import SignIn from "./pages/SignIn";
import GuildPage from "./pages/Guild";

export default function App() {
    return (
        <>
            <Route path="/signin" component={SignIn} />
            <Route path="/guild" component={GuildPage} />
            <Route
                path="/"
                component={() => (
                    <AuthGuard>
                        <Home />
                    </AuthGuard>
                )}
            />
        </>
    );
}