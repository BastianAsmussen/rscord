import { createSignal } from "solid-js";
import logo from "./assets/logo.svg";
import { invoke } from "@tauri-apps/api/core";
import { registerForPushNotifications } from '@choochmeque/tauri-plugin-notifications-api';
import "./App.css";
import { Route } from "@solidjs/router";
import AuthGuard from "./auth/AuthGuard.jsx";

import Home from "./pages/Home";
import SignIn from "./pages/SignIn";
import Settings from "./pages/Settings";

async function get_push_token(){
    try {
        const token = await registerForPushNotifications();
        console.log('Push token:', token);
        // Send this token to your server to send push notifications
        return token
    } catch (error) {
        console.error('Failed to register for push notifications:', error);
        return "epic fail"
    }
}
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
