import { createMemo, createSignal } from "solid-js";

import GuildSidebar from "../components/guild/GuildSidebar";
import ChannelSidebar from "../components/guild/ChannelSidebar";
import ChatView from "../components/guild/ChatView";
import MemberList from "../components/guild/MemberList";
import MessageInput from "../components/guild/MessageInput";

// todo: add real logic

type Guild = {
  id: number;
  name: string;
};

type Channel = {
  id: number;
  name: string;
  guild: number;
};

type Member = {
  id: number;
  name: string;
};

type Message = {
  id: number;
  user: string;
  text: string;
};

export default function GuildPage() {
  const guilds: Guild[] = [
    { id: 1, name: "test" },
    { id: 2, name: "ok lars" },
    { id: 3, name: "techcollege" },
  ];

  const channels: Channel[] = [
    { id: 1, name: "test", guild: 1 },
    { id: 2, name: "test2", guild: 1 },
    { id: 3, name: "test3", guild: 1 },

    { id: 4, name: "lars lars lars lars lars lars", guild: 2 },
    { id: 5, name: "lars ting", guild: 2 },
    { id: 6, name: "lars quotes", guild: 2 },

    { id: 7, name: "skema", guild: 3 },
    { id: 8, name: "elever", guild: 3 },
  ];

  const members: Member[] = [
    { id: 1, name: "bastian" },
    { id: 2, name: "jacob" },
    { id: 3, name: "mathias" },
    { id: 4, name: "casper" },
  ];

  const [activeGuild, setActiveGuild] = createSignal(1);
  const [activeChannel, setActiveChannel] = createSignal(1);

  const [messages, setMessages] = createSignal<Message[]>([
    { id: 1, user: "bastian", text: "jacob stop med at spille mercy!!!" },
    { id: 2, user: "jacob", text: "uwu <3" },
  ]);

  const [showMobileChannels, setShowMobileChannels] = createSignal(false);
  const [showMembers, setShowMembers] = createSignal(false);

  const filteredChannels = createMemo(() =>
      channels.filter((channel) => channel.guild === activeGuild())
  );

  const activeGuildName = createMemo(
      () => guilds.find((guild) => guild.id === activeGuild())?.name ?? "Guild"
  );

  const activeChannelName = createMemo(
      () => channels.find((channel) => channel.id === activeChannel())?.name ?? "channel"
  );

  const selectGuild = (id: number) => {
    setActiveGuild(id);

    const firstChannelForGuild = channels.find((channel) => channel.guild === id);
    if (firstChannelForGuild) {
      setActiveChannel(firstChannelForGuild.id);
    }

    setShowMobileChannels(true);
  };

  const selectChannel = (id: number) => {
    setActiveChannel(id);
    setShowMobileChannels(false);
  };

  const sendMessage = (text: string) => {
    setMessages((current) => [
      ...current,
      {
        id: Date.now(),
        user: "You",
        text,
      },
    ]);
  };

  return (
      <div class="flex h-screen bg-base text-text overflow-hidden">
        <GuildSidebar
            guilds={guilds}
            activeGuild={activeGuild()}
            onSelect={selectGuild}
        />

        <div class="hidden md:block shrink-0">
          <ChannelSidebar
              channels={filteredChannels()}
              activeChannel={activeChannel()}
              onSelect={selectChannel}
              title={activeGuildName()}
          />
        </div>

        <div class="flex flex-col flex-1 min-w-0">
          <div class="md:hidden flex items-center justify-between gap-3 p-3 border-b border-surface0 bg-mantle min-w-0">
            <button
                type="button"
                onClick={() => setShowMobileChannels(true)}
                class="px-3 py-1 bg-surface0 rounded text-text shrink-0"
            >
              channels
            </button>

            <div class="flex-1 min-w-0 text-center">
              <div class="font-bold text-primary truncate">{activeGuildName()}</div>
              <div class="text-sm text-subtext0 truncate"># {activeChannelName()}</div>
            </div>

            <button
                type="button"
                onClick={() => setShowMembers(true)}
                class="px-3 py-1 bg-surface0 rounded text-text shrink-0"
            >
              memebers
            </button>
          </div>

          <ChatView messages={messages()} />
          <MessageInput onSend={sendMessage} />
        </div>

        <div class="hidden md:block shrink-0">
          <MemberList members={members} />
        </div>

        {showMobileChannels() && (
            <div
                class="fixed inset-0 backdrop md:hidden z-40"
                onClick={() => setShowMobileChannels(false)}
            >
              <div
                  class="absolute left-16 top-0 bottom-0 w-64 bg-mantle"
                  onClick={(e) => e.stopPropagation()}
              >
                <div class="flex items-center justify-between p-4 border-b border-surface0">
                  <h2 class="font-bold text-subtext0 text-sm">{activeGuildName()}</h2>

                  <button
                      type="button"
                      class="px-2 py-1 rounded bg-surface0 hover:bg-surface1 text-text"
                      onClick={() => setShowMobileChannels(false)}
                  >
                    ✕
                  </button>
                </div>

                <ChannelSidebar
                    channels={filteredChannels()}
                    activeChannel={activeChannel()}
                    onSelect={selectChannel}
                />
              </div>
            </div>
        )}

        {showMembers() && (
            <div
                class="fixed inset-0 backdrop md:hidden z-40"
                onClick={() => setShowMembers(false)}
            >
              <div
                  class="absolute right-0 top-0 bottom-0"
                  onClick={(e) => e.stopPropagation()}
              >
                <MemberList
                    members={members}
                    mobile={true}
                    onClose={() => setShowMembers(false)}
                />
              </div>
            </div>
        )}
      </div>
  );
}