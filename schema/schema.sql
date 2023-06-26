-- SQL dump generated using DBML (dbml-lang.org)
-- Database: PostgreSQL
-- Generated at: 2023-03-11T03:47:57.306Z

CREATE TABLE "users" (
  "id" bigserial PRIMARY KEY,
  "username" varchar UNIQUE NOT NULL,
  "hashed_password" varchar NOT NULL,
  "nickname" varchar NOT NULL,
  "avatar" varchar NOT NULL DEFAULT 'blank',
  "bio" varchar NOT NULL DEFAULT 'blank',
  "role" varchar NOT NULL DEFAULT 'user',
  "deleted" boolean NOT NULL DEFAULT false,
  "room_id" bigint NOT NULL,
  "create_at" timestamptz NOT NULL DEFAULT (now())
);

CREATE TABLE "sessions" (
  "id" uuid PRIMARY KEY,
  "user_id" bigint NOT NULL,
  "refresh_token" varchar NOT NULL,
  "client_ip" varchar NOT NULL,
  "user_agent" varchar NOT NULL,
  "expire_at" timestamptz NOT NULL,
  "create_at" timestamptz NOT NULL DEFAULT (now())
);

CREATE TABLE "friendships" (
  "user_id" bigint,
  "friend_id" bigint,
  "status" varchar NOT NULL DEFAULT 'adding',
  "room_id" bigint NOT NULL,
  "create_at" timestamptz NOT NULL DEFAULT (now()),
  PRIMARY KEY ("user_id", "friend_id")
);

CREATE TABLE "rooms" (
  "id" bigserial PRIMARY KEY,
  "name" varchar NOT NULL,
  "cover" varchar NOT NULL DEFAULT 'blank',
  "category" varchar NOT NULL DEFAULT 'public',
  "create_at" timestamptz NOT NULL DEFAULT (now())
);

CREATE TABLE "room_members" (
  "room_id" bigint,
  "member_id" bigint,
  "rank" varchar NOT NULL DEFAULT 'member',
  "join_at" timestamptz NOT NULL DEFAULT (now()),
  PRIMARY KEY ("room_id", "member_id")
);

CREATE TABLE "messages" (
  "id" bigserial PRIMARY KEY,
  "room_id" bigint NOT NULL,
  "sender_id" bigint NOT NULL,
  "content" varchar NOT NULL,
  "kind" varchar NOT NULL DEFAULT 'text',
  "send_at" timestamptz NOT NULL DEFAULT (now())
);

CREATE TABLE "invitations" (
  "code" varchar PRIMARY KEY,
  "expire_at" timestamptz NOT NULL
);

ALTER TABLE "users" ADD FOREIGN KEY ("room_id") REFERENCES "rooms" ("id") ON DELETE CASCADE ON UPDATE NO ACTION;

ALTER TABLE "sessions" ADD FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE CASCADE ON UPDATE NO ACTION;

ALTER TABLE "friendships" ADD FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE CASCADE ON UPDATE NO ACTION;

ALTER TABLE "friendships" ADD FOREIGN KEY ("friend_id") REFERENCES "users" ("id") ON DELETE CASCADE ON UPDATE NO ACTION;

ALTER TABLE "friendships" ADD FOREIGN KEY ("room_id") REFERENCES "rooms" ("id") ON DELETE CASCADE ON UPDATE NO ACTION;

ALTER TABLE "room_members" ADD FOREIGN KEY ("room_id") REFERENCES "rooms" ("id") ON DELETE CASCADE ON UPDATE NO ACTION;

ALTER TABLE "room_members" ADD FOREIGN KEY ("member_id") REFERENCES "users" ("id") ON DELETE CASCADE ON UPDATE NO ACTION;

ALTER TABLE "messages" ADD FOREIGN KEY ("room_id") REFERENCES "rooms" ("id") ON DELETE CASCADE ON UPDATE NO ACTION;

ALTER TABLE "messages" ADD FOREIGN KEY ("sender_id") REFERENCES "users" ("id") ON DELETE CASCADE ON UPDATE NO ACTION;
