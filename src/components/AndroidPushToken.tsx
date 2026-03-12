import {registerForPushNotifications} from '@choochmeque/tauri-plugin-notifications-api';
import {platform} from "@tauri-apps/plugin-os";
import {invoke} from "@tauri-apps/api/core";

export async function add_push_token(user_id: number) {
    if (platform() == "android") {
        const token = await registerForPushNotifications()
            .catch((e) => console.error('Failed to register for push notifications:', e.toString()))
        console.log('Push token:', token);

        let response = invoke('add_push_token', {
            userId: user_id, token: token
        });

        response.catch((e) => e.toString());
        await response;
    }
    return true;
}
