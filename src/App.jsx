import { createSignal } from "solid-js";
import logo from "./assets/logo.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
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
        </>
    );
  }
