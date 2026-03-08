-- add users
INSERT INTO public.users (id, email, password_digest, user_handle, settings, email_verified, created_at, updated_at)
VALUES (DEFAULT, 'kurt@test.com', 'deadbeef', 'kurt', DEFAULT, true, DEFAULT, DEFAULT);

INSERT INTO public.users (id, email, password_digest, user_handle, settings, email_verified, created_at, updated_at)
VALUES (DEFAULT, 'sigurd@test.com', 'deadbeef', 'sigurd', DEFAULT, true, DEFAULT, DEFAULT);

INSERT INTO public.users (id, email, password_digest, user_handle, settings, email_verified, created_at, updated_at)
VALUES (DEFAULT, 'peter@test.com', 'deadbeef', 'peter', DEFAULT, true, DEFAULT, DEFAULT);

-- add sessions
INSERT INTO public.sessions (user_id, token, expires_at)
VALUES (1, 'kurt_test_token', '2030-01-01 00:00:00');
INSERT INTO public.sessions (user_id, token, expires_at)
VALUES (2, 'sigurd_test_token', '2030-01-01 00:00:00');

-- add displayed users
INSERT INTO public.displayed_users (id, user_id, display_name, icon_url, status, created_at, updated_at)
VALUES (DEFAULT, 1, 'xXx_kurt-xXx', null, 'online'::user_status, DEFAULT, DEFAULT);

INSERT INTO public.displayed_users (id, user_id, display_name, icon_url, status, created_at, updated_at)
VALUES (DEFAULT, 2, 'Sigurd the killer', null, 'dnd'::user_status, DEFAULT, DEFAULT);

INSERT INTO public.displayed_users (id, user_id, display_name, icon_url, status, created_at, updated_at)
VALUES (DEFAULT, 3, 'Postmand per', null, 'idle'::user_status, DEFAULT, DEFAULT);


-- add relationship
INSERT INTO public.relationships (id, sender_id, receiver_id, status, created_at, updated_at)
VALUES (DEFAULT, 1, 2, 'accepted'::relationship_status, DEFAULT, null);

INSERT INTO public.relationships (id, sender_id, receiver_id, status, created_at, updated_at)
VALUES (DEFAULT, 1, 3, 'accepted'::relationship_status, DEFAULT, null);

INSERT INTO public.relationships (id, sender_id, receiver_id, status, created_at, updated_at)
VALUES (DEFAULT, 3, 2, 'pending'::relationship_status, DEFAULT, null);


-- add channels
INSERT INTO public.channels (id, type, guild_id, name, topic, position, permission, created_at, updated_at)
VALUES (DEFAULT, 'dm'::channel_type, null, null, null, null, null, DEFAULT, DEFAULT);

INSERT INTO public.channels (id, type, guild_id, name, topic, position, permission, created_at, updated_at)
VALUES (DEFAULT, 'dm'::channel_type, null, null, null, null, null, DEFAULT, DEFAULT);

INSERT INTO public.channels (id, type, guild_id, name, topic, position, permission, created_at, updated_at)
VALUES (DEFAULT, 'dm'::channel_type, null, null, null, null, null, DEFAULT, DEFAULT);

INSERT INTO public.channels (id, type, guild_id, name, topic, position, permission, created_at, updated_at)
VALUES (DEFAULT, 'group_dm'::channel_type, null, null, null, null, null, DEFAULT, DEFAULT);


-- add channel members
-- dm for Kurt and Sigurd
INSERT INTO public.channel_members (channel_id, user_id, created_at, updated_at)
VALUES (1, 1, DEFAULT, DEFAULT);
INSERT INTO public.channel_members (channel_id, user_id, created_at, updated_at)
VALUES (1, 2, DEFAULT, DEFAULT);

--dm for Kurt and Peter
INSERT INTO public.channel_members (channel_id, user_id, created_at, updated_at)
VALUES (2, 1, DEFAULT, DEFAULT);
INSERT INTO public.channel_members (channel_id, user_id, created_at, updated_at)
VALUES (2, 3, DEFAULT, DEFAULT);

-- dm for Sigurd and Peter
INSERT INTO public.channel_members (channel_id, user_id, created_at, updated_at)
VALUES (3, 2, DEFAULT, DEFAULT);
INSERT INTO public.channel_members (channel_id, user_id, created_at, updated_at)
VALUES (3, 3, DEFAULT, DEFAULT);

-- group dm for Kurt, Sigurd and Peter
INSERT INTO public.channel_members (channel_id, user_id, created_at, updated_at)
VALUES (4, 1, DEFAULT, DEFAULT);
INSERT INTO public.channel_members (channel_id, user_id, created_at, updated_at)
VALUES (4, 2, DEFAULT, DEFAULT);
INSERT INTO public.channel_members (channel_id, user_id, created_at, updated_at)
VALUES (4, 3, DEFAULT, DEFAULT);


-- messages between kurt and sigurd
-- the dummy messages ciphertext is just ASCHII to HEX
INSERT INTO public.direct_messages (id, author_id, reply_to_id, channel_id, ciphertext, nonce, ratchet_key_id,
                                    created_at)
VALUES (DEFAULT, 1, null, 1, decode('48656A', 'hex'),
        decode('48656A', 'hex'), 0, DEFAULT);

INSERT INTO public.direct_messages (id, author_id, reply_to_id, channel_id, ciphertext, nonce, ratchet_key_id,
                                    created_at)
VALUES (DEFAULT, 2, 1, 1, decode('476F64646167', 'hex'),
        decode('476F64646167', 'hex'), 0, DEFAULT);


-- pinned the second message
INSERT INTO public.pinned_direct_messages (channel_id, message_id, pinned_by, pinned_at)
VALUES (1, 2, 1, DEFAULT);