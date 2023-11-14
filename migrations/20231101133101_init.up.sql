-- SQL dump generated using DBML (dbml-lang.org)
-- Database: PostgreSQL
-- Generated at: 2023-11-13T08:33:46.241Z

CREATE TABLE "users" (
  "id" bigserial PRIMARY KEY,
  "username" varchar UNIQUE NOT NULL,
  "hashed_password" varchar NOT NULL,
  "avatar" varchar NOT NULL,
  "nickname" varchar NOT NULL,
  "role" varchar NOT NULL,
  "room_id" bigint NOT NULL,
  "deleted" boolean NOT NULL DEFAULT false,
  "create_at" timestamptz NOT NULL DEFAULT (now())
);

CREATE TABLE "friends" (
  "requester_id" bigint,
  "addressee_id" bigint,
  "room_id" bigint NOT NULL,
  "status" varchar NOT NULL,
  "create_at" timestamptz NOT NULL DEFAULT (now()),
  PRIMARY KEY ("requester_id", "addressee_id")
);

CREATE TABLE "rooms" (
  "id" bigserial PRIMARY KEY,
  "name" varchar NOT NULL,
  "cover" varchar NOT NULL,
  "category" varchar NOT NULL,
  "create_at" timestamptz NOT NULL DEFAULT (now())
);

CREATE TABLE "members" (
  "member_id" bigint,
  "room_id" bigint,
  "rank" varchar NOT NULL,
  "join_at" timestamptz NOT NULL DEFAULT (now()),
  PRIMARY KEY ("room_id", "member_id")
);

ALTER TABLE "users" ADD FOREIGN KEY ("room_id") REFERENCES "rooms" ("id");

ALTER TABLE "friends" ADD FOREIGN KEY ("requester_id") REFERENCES "users" ("id");

ALTER TABLE "friends" ADD FOREIGN KEY ("addressee_id") REFERENCES "users" ("id");

ALTER TABLE "friends" ADD FOREIGN KEY ("room_id") REFERENCES "rooms" ("id");

ALTER TABLE "members" ADD FOREIGN KEY ("member_id") REFERENCES "users" ("id");

ALTER TABLE "members" ADD FOREIGN KEY ("room_id") REFERENCES "rooms" ("id");
