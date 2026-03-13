-- add users
-- Passwords: Password123!
-- OPAQUE_SERVER_SETUP=a2723445193c68a16853f8d7ea5a414e0cbc9cfebfa33eefc7eed48710f505bf3e60447aa24126f58174f1cde15894968631fce9076836c76915838bfc55de1d9ac72f6824a11e71a07cdbc8fb1887b0d25372f415adbc61fbb53047c4293e0a7876e9fd240c2d8db73b4f46ada6e1c48a368b99cf305e357aeb92e820494f37
INSERT INTO public.users (id, email, opaque_record, user_handle, settings, email_verified, created_at, updated_at)
VALUES (DEFAULT, 'kurt@test.com', '\x0edb27faa03da246c46322023444fe625b94df728988d3ce74a13fd51ace3d69bac62e00f0e83d4f81ffd367e2a649aa01d55048c2194101aaeed4351ce7c3f393d1e243f4ecbcac401f735a16aa3c04967e81008fa2240931829341443cb3f9dc63f64e4ea60bfa7bbe741d087706ba896b9749594ec6f1a1c1658a8116b735f42ca05cbc79407658bfad31f11ccf9fc8b0c4527d99cc39ac72fe45bb92843cf6fe1d0df050664dc0b1da49a392e14a46d34a68323098ba53e7c4261bd0fcae', 'kurt', DEFAULT, true, DEFAULT, DEFAULT);

INSERT INTO public.users (id, email, opaque_record, user_handle, settings, email_verified, created_at, updated_at)
VALUES (DEFAULT, 'sigurd@test.com', '\x8e78a5f6d29a2cce7cff9cea4fc667fa377ee6ee1e46ad29695121b0c1b0e80f2c44e6d586b2ad9beaab243fc2b6a2851f71d010ab0caabf8263aeeb7a0cf7f0b03437baf64153e518f91c1aa0b291c3066da9daefce760267ea392b37ab900dac7898af497ee781bca5d91b85ed8db7ab72004fae02201c22c689fce0f51dc1c6c42ca2bc442f60d99430f2c76a9d7462f99fdee734f8409a3c886df1282a1054404b8bce513780802326cd48de4863a030a7fc23078b1c2c8ab5a910b53407', 'sigurd', DEFAULT, true, DEFAULT, DEFAULT);

INSERT INTO public.users (id, email, opaque_record, user_handle, settings, email_verified, created_at, updated_at)
VALUES (DEFAULT, 'peter@test.com', '\xa6d801a61b9fc256c46295e60e029540e09bb87aed55f1c4d2b2518ad5f0b912e2a36cb556099e2ea52369913e53d5990eacd3a5b7352ad4651abb8f309f60aace14998586c5269102013d43ec89bcfa2763057ab051f99b1f669e8641b89a853d4043df37ed50a5001b26707c986f31a4b793122a1aa5ec4012042853a314cd48feea2bf147b11a85fb611942ee300e6b9b751b77905f99f9388a1b1538e079ab0c62be345c28b386f6e0dd73370e858432e613a19acf71be059b15f0f8fcc9', 'peter', DEFAULT, true, DEFAULT, DEFAULT);

-- ----------------------------------------------------------------------

-- add sessions
INSERT INTO public.sessions (user_id, token, expires_at)
VALUES (1, 'kurt_test_token', '2030-01-01 00:00:00');
INSERT INTO public.sessions (user_id, token, expires_at)
VALUES (2, 'sigurd_test_token', '2030-01-01 00:00:00');

-- ----------------------------------------------------------------------

-- add displayed users
INSERT INTO public.displayed_users (id, user_id, display_name, icon_url, status, created_at, updated_at)
VALUES (DEFAULT, 1, 'xXx_kurt_xXx', null, 'online'::user_status, DEFAULT, DEFAULT);

INSERT INTO public.displayed_users (id, user_id, display_name, icon_url, status, created_at, updated_at)
VALUES (DEFAULT, 2, 'Sigurd the killer', null, 'dnd'::user_status, DEFAULT, DEFAULT);

INSERT INTO public.displayed_users (id, user_id, display_name, icon_url, status, created_at, updated_at)
VALUES (DEFAULT, 3, 'Postmand Per', null, 'idle'::user_status, DEFAULT, DEFAULT);

-- ----------------------------------------------------------------------

-- add relationship
INSERT INTO public.relationships (id, sender_id, receiver_id, status, created_at, updated_at)
VALUES (DEFAULT, 1, 2, 'accepted'::relationship_status, DEFAULT, null);

INSERT INTO public.relationships (id, sender_id, receiver_id, status, created_at, updated_at)
VALUES (DEFAULT, 1, 3, 'accepted'::relationship_status, DEFAULT, null);

INSERT INTO public.relationships (id, sender_id, receiver_id, status, created_at, updated_at)
VALUES (DEFAULT, 3, 2, 'pending'::relationship_status, DEFAULT, null);

-- ----------------------------------------------------------------------

-- Kurt's Guild
INSERT INTO public.guilds (id, owner_id, name, icon_url, created_at, updated_at)
VALUES (DEFAULT, 1, 'The Rust Den', 'https://example.com/rust.png', DEFAULT, DEFAULT);

-- Sigurd's Guild
INSERT INTO public.guilds (id, owner_id, name, icon_url, created_at, updated_at)
VALUES (DEFAULT, 2, 'Shyvana Mains', null, DEFAULT, DEFAULT);

-- ----------------------------------------------------------------------

-- Channels for (Guild 1 - Owner: Kurt)
INSERT INTO public.channels (guild_id, name, type, position, properties)
VALUES
    (1, 'General', 'text'::channel_type, 0, jsonb_build_object('topic', 'Default channel')),
    (1, 'Rust Stuff', 'text'::channel_type, 1, jsonb_build_object('topic', 'Programming')),
    (1, 'Chill', 'voice'::channel_type, 2, '{}'::jsonb),
    (1, 'announcements', 'text'::channel_type, 3, jsonb_build_object('topic', 'Official updates'));

-- Channels for (Guild 2 - Owner: Sigurd)
INSERT INTO public.channels (guild_id, name, type, position, properties)
VALUES
    (2, 'General', 'text'::channel_type, 0, jsonb_build_object('topic', 'default channel')),
    (2, 'Movies', 'text'::channel_type, 1, jsonb_build_object('topic', 'Write about movies')),
    (2, 'War', 'voice'::channel_type, 2, '{}'::jsonb),
    (2, 'Simping Corner', 'voice'::channel_type, 3, '{}'::jsonb);
-- ----------------------------------------------------------------------

-- Members for Guild 1
-- Kurt (1), Sigurd (2), and Peter (3)
INSERT INTO public.guild_members (guild_id, user_id, joined_at)
VALUES (1, 1, DEFAULT), -- Owner (Kurt)
       (1, 2, DEFAULT), -- Guest (Sigurd)
       (1, 3, DEFAULT); -- Guest (Peter)

-- Members for Guild 2
-- Sigurd (2), Kurt (1), and Peter (3)
INSERT INTO public.guild_members (guild_id, user_id, joined_at)
VALUES (2, 2, DEFAULT), -- Owner (Sigurd)
       (2, 1, DEFAULT), -- Guest (Kurt)
       (2, 3, DEFAULT); -- Guest (Peter)

-- ----------------------------------------------------------------------

-- DM: Kurt (1) and Sigurd (2)
WITH dm1 AS (INSERT INTO public.channels (type, name, guild_id, properties)
    VALUES ('dm'::channel_type, NULL, NULL, '{}') RETURNING id)
INSERT INTO public.channels_members (channel_id, user_id)
SELECT id, u FROM dm1, unnest(ARRAY[1, 2]) AS u;

-- DM: Kurt (1) and Peter (3)
WITH dm2 AS (INSERT INTO public.channels (type, name, guild_id, properties)
    VALUES ('dm'::channel_type, NULL, NULL, '{}') RETURNING id)
INSERT INTO public.channels_members (channel_id, user_id)
SELECT id, u FROM dm2, unnest(ARRAY[1, 3]) AS u;

-- DM: Sigurd (2) and Peter (3)
WITH dm3 AS (INSERT INTO public.channels (type, name, guild_id, properties)
    VALUES ('dm'::channel_type, NULL, NULL, '{}') RETURNING id)
INSERT INTO public.channels_members (channel_id, user_id)
SELECT id, u FROM dm3, unnest(ARRAY[2, 3]) AS u;

-- Group DM: Kurt (1), Sigurd (2), and Peter (3)
WITH gdm1 AS (INSERT INTO public.channels (type, name, guild_id, properties)
    VALUES ('group_dm'::channel_type, 'The Trio', NULL, '{}') RETURNING id)
INSERT INTO public.channels_members (channel_id, user_id)
SELECT id, u FROM gdm1, unnest(ARRAY[1, 2, 3]) AS u;
-- ----------------------------------------------------------------------

-- messages between kurt and sigurd (Private)
-- the dummy messages ciphertext is just ASCHII to HEX
INSERT INTO public.direct_messages (id, author_id, reply_to_id, channel_id, ciphertext, nonce, ratchet_key_id,
                                    created_at)
VALUES (DEFAULT, 1, null, 1, decode('48656A', 'hex'),
        decode('48656A', 'hex'), 0, DEFAULT);

INSERT INTO public.direct_messages (id, author_id, reply_to_id, channel_id, ciphertext, nonce, ratchet_key_id,
                                    created_at)
VALUES (DEFAULT, 2, 1, 1, decode('476F64646167', 'hex'),
        decode('476F64646167', 'hex'), 0, DEFAULT);

-- ----------------------------------------------------------------------

-- (Guild ID: 1)
-- messages between kurt and sigurd (Guild)
INSERT INTO public.guild_messages (author_id, channel_id, contents)
VALUES (1, 1, 'Welcome to the Rust Den!');

INSERT INTO public.guild_messages (author_id, channel_id, contents)
VALUES (2, 1, 'Thanks for the invite, Bro.');

INSERT INTO public.guild_messages (author_id, channel_id, contents)
VALUES (3, 2, 'Why is the compiler so nice?');

-- Kurt replies to Peter (Message ID 3) in Channel 2
INSERT INTO public.guild_messages (author_id, reply_to_id, channel_id, contents)
VALUES (1, 3, 2, 'Probably because you code in C#.');

-- (Guild ID: 2)
INSERT INTO public.guild_messages (author_id, channel_id, contents)
VALUES (2, 5, 'Reporting for duty in the Shyvana guild!');

INSERT INTO public.guild_messages (author_id, channel_id, contents)
VALUES (3, 5, 'Glad to be here!');

-- ----------------------------------------------------------------------

-- pinned the second message
INSERT INTO public.pinned_direct_messages (channel_id, message_id, pinned_by, pinned_at)
VALUES (1, 2, 1, DEFAULT);