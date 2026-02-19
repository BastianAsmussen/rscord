import { Route } from "@solidjs/router";
import AuthGuard from "./auth/AuthGuard.jsx";

import Home from "./pages/Home";
import SignIn from "./pages/SignIn";
import Settings from "./pages/Settings";

export default function App() {
    return (
        <>
            <Route path="/signin" component={SignIn} />

            <Route
                path="/"
                component={() => (
                    <AuthGuard>
                        <Home />
                    </AuthGuard>
                )}
            />

            <Route
                path="/settings"
                component={() => (
                    <AuthGuard>
                        <Settings />
                    </AuthGuard>
                )}
            />
        </>
    );
}